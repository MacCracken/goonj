//! Vibroacoustics — coupling between structural vibration and acoustic radiation.
//!
//! Computes sound radiated by vibrating surfaces. Takes surface velocity
//! distributions (from a structural dynamics solver like impetus) and
//! estimates the resulting acoustic field using radiation impedance models.
//!
//! The coupling flow is:
//! ```text
//! impetus (structural FEM/modal) → surface velocities → goonj vibroacoustics → sound field
//! ```

use crate::material::NUM_BANDS;
use crate::propagation::speed_of_sound;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A vibrating surface element for acoustic radiation computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VibratingSurface {
    /// Centre position of the surface element.
    pub position: Vec3,
    /// Outward normal of the surface.
    pub normal: Vec3,
    /// Area of the surface element in m².
    pub area: f32,
    /// Normal velocity amplitude in m/s (complex magnitude).
    pub velocity: f32,
    /// Vibration frequency in Hz.
    pub frequency: f32,
}

/// A structural vibration mode shape for modal radiation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VibrationMode {
    /// Natural frequency of this mode in Hz.
    pub frequency: f32,
    /// Modal damping ratio (0.0–1.0, typically 0.001–0.05 for metals).
    pub damping: f32,
    /// Per-element normal velocity amplitude for this mode shape.
    /// Each entry corresponds to a surface element.
    pub mode_shape: Vec<f32>,
}

/// Result of acoustic radiation computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadiationResult {
    /// Radiated sound power in Watts.
    pub sound_power_watts: f32,
    /// Sound power level in dB (re 1 pW).
    pub sound_power_level_db: f32,
    /// Per-band radiated power in Watts.
    pub power_per_band: [f32; NUM_BANDS],
}

/// Radiation efficiency of a baffled rectangular plate.
///
/// Below the critical frequency, radiation efficiency is low (σ << 1).
/// At the critical frequency, σ peaks. Above, σ → 1.
///
/// # Arguments
/// * `frequency` — vibration frequency in Hz
/// * `critical_frequency` — coincidence/critical frequency of the plate in Hz
/// * `plate_area` — surface area of the plate in m²
/// * `plate_perimeter` — perimeter of the plate in m
/// * `temperature_celsius` — air temperature
#[must_use]
#[inline]
pub fn radiation_efficiency(
    frequency: f32,
    critical_frequency: f32,
    plate_area: f32,
    plate_perimeter: f32,
    temperature_celsius: f32,
) -> f32 {
    if frequency <= 0.0 || critical_frequency <= 0.0 || plate_area <= 0.0 {
        return 0.0;
    }

    let c = speed_of_sound(temperature_celsius);
    let wavelength = c / frequency;
    let f_ratio = frequency / critical_frequency;

    if f_ratio < 0.5 {
        // Well below critical: radiation from edges and corners dominates
        // σ ≈ (perimeter × wavelength) / (4 × π × area) (edge radiation)
        let sigma = plate_perimeter * wavelength / (4.0 * std::f32::consts::PI * plate_area);
        sigma.clamp(0.0, 1.0)
    } else if f_ratio < 1.0 {
        // Approaching critical: rapid increase
        let sigma = (1.0 / (1.0 - f_ratio)).sqrt();
        sigma
            .clamp(0.0, 10.0)
            .min(1.0 / (1.0 - f_ratio + 0.01).sqrt())
            .clamp(0.0, 1.0)
    } else {
        // At or above critical: σ → 1
        let sigma = 1.0 / (1.0 - 1.0 / (f_ratio * f_ratio)).abs().sqrt().max(0.01);
        sigma.clamp(0.0, 1.0)
    }
}

/// Compute the sound power radiated by a vibrating surface.
///
/// Uses the Rayleigh integral approximation for a baffled piston:
/// `W = ρ₀c₀ × σ × A × <v²>` where σ is the radiation efficiency
/// and <v²> is the mean-square velocity.
///
/// # Arguments
/// * `surfaces` — array of vibrating surface elements
/// * `critical_frequency` — coincidence frequency of the structure
/// * `temperature_celsius` — air temperature
#[must_use]
#[tracing::instrument(skip(surfaces), fields(num_surfaces = surfaces.len()))]
pub fn radiated_sound_power(
    surfaces: &[VibratingSurface],
    critical_frequency: f32,
    temperature_celsius: f32,
) -> RadiationResult {
    if surfaces.is_empty() {
        return RadiationResult {
            sound_power_watts: 0.0,
            sound_power_level_db: f32::NEG_INFINITY,
            power_per_band: [0.0; NUM_BANDS],
        };
    }

    let c = speed_of_sound(temperature_celsius);
    let rho = 1.21_f32; // air density

    let total_area: f32 = surfaces.iter().map(|s| s.area).sum();
    let perimeter = 4.0 * total_area.sqrt(); // approximate perimeter for a square

    let mut total_power = 0.0_f32;
    let mut power_per_band = [0.0_f32; NUM_BANDS];

    for surface in surfaces {
        let sigma = radiation_efficiency(
            surface.frequency,
            critical_frequency,
            total_area,
            perimeter,
            temperature_celsius,
        );

        // Sound power from this element: W = ρ₀c × σ × A × v²
        let power = rho * c * sigma * surface.area * surface.velocity * surface.velocity;
        total_power += power;

        // Assign to nearest frequency band
        let band = nearest_band(surface.frequency);
        power_per_band[band] += power;
    }

    let swl = if total_power > 0.0 {
        10.0 * (total_power / 1e-12).log10() // dB re 1 pW
    } else {
        f32::NEG_INFINITY
    };

    RadiationResult {
        sound_power_watts: total_power,
        sound_power_level_db: swl,
        power_per_band,
    }
}

/// Compute radiated power from a set of vibration modes.
///
/// Each mode contributes independently (modal superposition).
/// The modal velocity is scaled by the mode shape amplitude at each element.
///
/// # Arguments
/// * `positions` — centre positions of surface elements
/// * `areas` — area of each surface element in m²
/// * `modes` — vibration modes with natural frequencies and mode shapes
/// * `excitation_amplitude` — amplitude of the driving force (scales all modes)
/// * `critical_frequency` — coincidence frequency
/// * `temperature_celsius` — air temperature
#[must_use]
pub fn modal_radiation(
    positions: &[Vec3],
    areas: &[f32],
    modes: &[VibrationMode],
    excitation_amplitude: f32,
    critical_frequency: f32,
    temperature_celsius: f32,
) -> RadiationResult {
    if positions.is_empty() || areas.is_empty() || modes.is_empty() {
        return RadiationResult {
            sound_power_watts: 0.0,
            sound_power_level_db: f32::NEG_INFINITY,
            power_per_band: [0.0; NUM_BANDS],
        };
    }

    let n = positions.len().min(areas.len());
    let mut surfaces = Vec::with_capacity(n * modes.len());

    for mode in modes {
        if mode.mode_shape.len() < n {
            continue;
        }

        // Modal response amplitude: A / (2ζω) at resonance
        let omega = std::f32::consts::TAU * mode.frequency;
        let modal_amp = if omega > 0.0 && mode.damping > 0.0 {
            excitation_amplitude / (2.0 * mode.damping * omega)
        } else {
            0.0
        };

        for i in 0..n {
            let velocity = modal_amp * mode.mode_shape[i];
            if velocity.abs() > f32::EPSILON {
                surfaces.push(VibratingSurface {
                    position: positions[i],
                    normal: Vec3::Y, // default up — caller should set correctly
                    area: areas[i],
                    velocity: velocity.abs(),
                    frequency: mode.frequency,
                });
            }
        }
    }

    radiated_sound_power(&surfaces, critical_frequency, temperature_celsius)
}

/// Find the nearest frequency band index for a given frequency.
#[must_use]
#[inline]
fn nearest_band(frequency: f32) -> usize {
    let bands = &crate::material::FREQUENCY_BANDS;
    let mut best = 0;
    let mut best_dist = (frequency - bands[0]).abs();
    for (i, &f) in bands.iter().enumerate().skip(1) {
        let dist = (frequency - f).abs();
        if dist < best_dist {
            best = i;
            best_dist = dist;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radiation_efficiency_below_critical() {
        // Large plate (10 m²) at low frequency well below critical
        let sigma = radiation_efficiency(100.0, 2500.0, 10.0, 12.0, 20.0);
        assert!(
            sigma > 0.0 && sigma < 0.5,
            "below critical should have low sigma for large plate, got {sigma}"
        );
    }

    #[test]
    fn radiation_efficiency_above_critical() {
        let sigma = radiation_efficiency(5000.0, 2500.0, 1.0, 4.0, 20.0);
        assert!(
            sigma > 0.5,
            "above critical should have high sigma, got {sigma}"
        );
    }

    #[test]
    fn radiation_efficiency_in_range() {
        for f in [63.0, 125.0, 500.0, 1000.0, 4000.0, 8000.0] {
            let sigma = radiation_efficiency(f, 2000.0, 2.0, 6.0, 20.0);
            assert!(
                (0.0..=1.0).contains(&sigma),
                "sigma should be [0,1] at {f} Hz, got {sigma}"
            );
        }
    }

    #[test]
    fn radiated_power_from_vibrating_plate() {
        let surfaces = vec![VibratingSurface {
            position: Vec3::ZERO,
            normal: Vec3::Y,
            area: 1.0,
            velocity: 0.001, // 1 mm/s
            frequency: 1000.0,
        }];
        let result = radiated_sound_power(&surfaces, 2500.0, 20.0);
        assert!(result.sound_power_watts > 0.0);
        assert!(result.sound_power_level_db.is_finite());
    }

    #[test]
    fn radiated_power_empty() {
        let result = radiated_sound_power(&[], 2500.0, 20.0);
        assert_eq!(result.sound_power_watts, 0.0);
    }

    #[test]
    fn radiated_power_increases_with_velocity() {
        let make = |v: f32| {
            radiated_sound_power(
                &[VibratingSurface {
                    position: Vec3::ZERO,
                    normal: Vec3::Y,
                    area: 1.0,
                    velocity: v,
                    frequency: 1000.0,
                }],
                2500.0,
                20.0,
            )
        };
        let low = make(0.001);
        let high = make(0.01);
        assert!(high.sound_power_watts > low.sound_power_watts);
    }

    #[test]
    fn modal_radiation_produces_output() {
        let positions = vec![Vec3::ZERO, Vec3::X, Vec3::new(0.0, 0.0, 1.0)];
        let areas = vec![0.5, 0.5, 0.5];
        let modes = vec![VibrationMode {
            frequency: 500.0,
            damping: 0.02,
            mode_shape: vec![1.0, 0.5, -0.5],
        }];
        let result = modal_radiation(&positions, &areas, &modes, 1.0, 2500.0, 20.0);
        assert!(result.sound_power_watts > 0.0);
    }

    #[test]
    fn modal_radiation_empty() {
        let result = modal_radiation(&[], &[], &[], 1.0, 2500.0, 20.0);
        assert_eq!(result.sound_power_watts, 0.0);
    }

    #[test]
    fn nearest_band_test() {
        assert_eq!(nearest_band(63.0), 0);
        assert_eq!(nearest_band(1000.0), 4);
        assert_eq!(nearest_band(8000.0), 7);
        assert_eq!(nearest_band(3000.0), 5); // closer to 2000 than 4000
    }

    #[test]
    fn radiation_result_serializes() {
        let result = RadiationResult {
            sound_power_watts: 0.001,
            sound_power_level_db: 90.0,
            power_per_band: [0.0; NUM_BANDS],
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: RadiationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }
}
