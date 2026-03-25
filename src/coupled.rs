//! Coupled room acoustics — multi-room energy exchange.
//!
//! Models sound energy flow between connected rooms through portals,
//! producing characteristic double-slope decay curves. Uses a statistical
//! energy analysis (SEA) approach with per-band coupling coefficients.

use crate::impulse::sabine_rt60;
use crate::portal::Portal;
use crate::propagation::speed_of_sound;
use crate::room::AcousticRoom;
use serde::{Deserialize, Serialize};

/// A pair of acoustically coupled rooms connected by a portal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoupledRooms {
    /// The source room.
    pub room_a: AcousticRoom,
    /// The receiving room.
    pub room_b: AcousticRoom,
    /// Portal connecting the two rooms.
    pub portal: Portal,
}

/// Result of coupled room energy decay analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoupledDecay {
    /// RT60 of the early (fast) decay component in seconds.
    pub rt60_early: f32,
    /// RT60 of the late (slow) decay component in seconds.
    pub rt60_late: f32,
    /// Relative amplitude of the early component (0.0–1.0).
    pub early_amplitude: f32,
    /// Coupling strength (0.0 = isolated, 1.0 = fully coupled).
    pub coupling_strength: f32,
}

/// Compute the coupled decay characteristics for two rooms joined by a portal.
///
/// Uses the statistical coupling model (Kuttruff): the coupled system produces
/// a double-slope decay where the early slope depends on the more absorptive
/// room and the late slope on the less absorptive one.
#[must_use]
#[tracing::instrument(skip(coupled))]
pub fn coupled_room_decay(coupled: &CoupledRooms) -> CoupledDecay {
    let vol_a = coupled.room_a.geometry.volume_shoebox();
    let vol_b = coupled.room_b.geometry.volume_shoebox();
    let abs_a = coupled.room_a.geometry.total_absorption();
    let abs_b = coupled.room_b.geometry.total_absorption();

    let rt60_a = sabine_rt60(vol_a, abs_a);
    let rt60_b = sabine_rt60(vol_b, abs_b);

    // Portal coupling coefficient: κ = (c × A_portal) / (4 × V)
    let c = speed_of_sound(coupled.room_a.temperature_celsius);
    let portal_area = coupled.portal.area();
    let kappa_a = if vol_a > 0.0 {
        c * portal_area / (4.0 * vol_a)
    } else {
        0.0
    };
    let kappa_b = if vol_b > 0.0 {
        c * portal_area / (4.0 * vol_b)
    } else {
        0.0
    };

    // Coupling strength: ratio of portal coupling to room absorption
    let coupling = if abs_a + abs_b > 0.0 {
        (c * portal_area / (abs_a + abs_b)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Decay rates (1/seconds)
    let gamma_a = if rt60_a > 0.0 && rt60_a.is_finite() {
        6.908 / rt60_a
    } else {
        0.0
    };
    let gamma_b = if rt60_b > 0.0 && rt60_b.is_finite() {
        6.908 / rt60_b
    } else {
        0.0
    };

    // Coupled decay eigenvalues
    let sum = gamma_a + kappa_a + gamma_b + kappa_b;
    let product = (gamma_a + kappa_a) * (gamma_b + kappa_b) - kappa_a * kappa_b;
    let discriminant = (sum * sum - 4.0 * product).max(0.0);
    let sqrt_disc = discriminant.sqrt();

    let lambda1 = (sum + sqrt_disc) * 0.5; // fast decay
    let lambda2 = (sum - sqrt_disc) * 0.5; // slow decay

    let rt60_early = if lambda1 > 0.0 { 6.908 / lambda1 } else { 0.0 };
    let rt60_late = if lambda2 > 0.0 { 6.908 / lambda2 } else { 0.0 };

    // Early component amplitude (energy partition)
    let early_amp = if lambda1 > lambda2 {
        (lambda1 - gamma_b - kappa_b) / (lambda1 - lambda2)
    } else {
        0.5
    };

    CoupledDecay {
        rt60_early,
        rt60_late,
        early_amplitude: early_amp.clamp(0.0, 1.0),
        coupling_strength: coupling,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use hisab::Vec3;

    fn two_rooms() -> CoupledRooms {
        CoupledRooms {
            room_a: AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete()),
            room_b: AcousticRoom::shoebox(8.0, 6.0, 3.0, AcousticMaterial::carpet()),
            portal: Portal {
                position: Vec3::new(10.0, 1.0, 4.0),
                normal: Vec3::X,
                width: 0.9,
                height: 2.1,
            },
        }
    }

    #[test]
    fn coupled_produces_double_slope() {
        let decay = coupled_room_decay(&two_rooms());
        assert!(decay.rt60_early > 0.0);
        assert!(decay.rt60_late > 0.0);
        assert!(
            decay.rt60_late > decay.rt60_early,
            "late ({}) should be longer than early ({})",
            decay.rt60_late,
            decay.rt60_early
        );
    }

    #[test]
    fn coupling_strength_in_range() {
        let decay = coupled_room_decay(&two_rooms());
        assert!(
            (0.0..=1.0).contains(&decay.coupling_strength),
            "coupling should be [0,1], got {}",
            decay.coupling_strength
        );
    }

    #[test]
    fn early_amplitude_in_range() {
        let decay = coupled_room_decay(&two_rooms());
        assert!((0.0..=1.0).contains(&decay.early_amplitude));
    }

    #[test]
    fn coupled_decay_serializes() {
        let decay = coupled_room_decay(&two_rooms());
        let json = serde_json::to_string(&decay);
        assert!(json.is_ok());
    }
}
