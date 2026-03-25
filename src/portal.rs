//! Portal-based sound propagation through openings between rooms.
//!
//! Models sound transmission through doorways, windows, and other apertures.
//! A portal connects two rooms and allows energy to flow between them with
//! frequency-dependent attenuation based on aperture size and diffraction.

use crate::material::NUM_BANDS;
use crate::propagation::speed_of_sound;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// An acoustic portal (opening) connecting two spaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portal {
    /// Centre position of the portal opening.
    pub position: Vec3,
    /// Normal direction of the portal (points into the destination room).
    pub normal: Vec3,
    /// Width of the opening in meters.
    pub width: f32,
    /// Height of the opening in meters.
    pub height: f32,
}

impl Portal {
    /// Area of the portal opening in m².
    #[must_use]
    #[inline]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Frequency-dependent transmission factor through this portal.
    ///
    /// At low frequencies (wavelength >> aperture size), sound diffracts freely
    /// through the opening. At high frequencies (wavelength << aperture), the
    /// opening acts as a transparent window. The transition follows the
    /// aperture diffraction model.
    ///
    /// Returns per-band transmission factor (0.0–1.0).
    #[must_use]
    pub fn transmission_factor(&self, temperature_celsius: f32) -> [f32; NUM_BANDS] {
        let c = speed_of_sound(temperature_celsius);
        let characteristic_size = (self.width * self.height).sqrt();

        if characteristic_size <= 0.0 {
            return [0.0; NUM_BANDS];
        }

        std::array::from_fn(|band| {
            let freq = crate::material::FREQUENCY_BANDS[band];
            let wavelength = c / freq;
            let ratio = characteristic_size / wavelength;

            // Below: diffraction limited (partial transmission)
            // Above: geometric transmission (full)
            // Transition: smooth sigmoid around ratio = 1
            (ratio / (1.0 + ratio)).clamp(0.0, 1.0)
        })
    }
}

/// Energy transmitted through a portal from source to listener.
///
/// Combines portal area, distance attenuation, and frequency-dependent
/// aperture diffraction. Returns per-band energy scaling factors.
#[must_use]
pub fn portal_energy_transfer(
    source: Vec3,
    portal: &Portal,
    listener: Vec3,
    temperature_celsius: f32,
) -> [f32; NUM_BANDS] {
    let to_portal = portal.position - source;
    let from_portal = listener - portal.position;
    let d_source = to_portal.length();
    let d_listener = from_portal.length();

    if d_source < f32::EPSILON || d_listener < f32::EPSILON {
        return [0.0; NUM_BANDS];
    }

    // Directional coupling: how well aligned is the source-portal-listener path?
    let cos_in = (to_portal / d_source).dot(portal.normal).abs();
    let cos_out = (from_portal / d_listener).dot(portal.normal).abs();
    let directional = cos_in * cos_out;

    // Distance attenuation (inverse square through portal)
    let total_distance = d_source + d_listener;
    let distance_atten = 1.0 / (4.0 * std::f32::consts::PI * total_distance * total_distance);

    // Portal area factor (larger opening → more energy transfer)
    let area_factor = portal.area();

    let freq_factors = portal.transmission_factor(temperature_celsius);

    std::array::from_fn(|band| {
        (freq_factors[band] * directional * distance_atten * area_factor).clamp(0.0, 1.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_door() -> Portal {
        Portal {
            position: Vec3::new(5.0, 1.0, 0.0),
            normal: Vec3::Z,
            width: 0.9,
            height: 2.1,
        }
    }

    #[test]
    fn door_area() {
        let door = standard_door();
        assert!((door.area() - 1.89).abs() < 0.01);
    }

    #[test]
    fn transmission_factor_high_freq_near_one() {
        let door = standard_door();
        let factors = door.transmission_factor(20.0);
        // At 8 kHz, wavelength ≈ 0.043 m, door ≈ 1.4 m → ratio ≈ 32 → near 1.0
        assert!(
            factors[7] > 0.9,
            "8 kHz through door should be near 1.0, got {}",
            factors[7]
        );
    }

    #[test]
    fn transmission_factor_low_freq_lower() {
        let door = standard_door();
        let factors = door.transmission_factor(20.0);
        // At 63 Hz, wavelength ≈ 5.4 m, door ≈ 1.4 m → ratio ≈ 0.26 → ~0.2
        assert!(
            factors[0] < factors[7],
            "low freq ({}) should transmit less than high ({})",
            factors[0],
            factors[7]
        );
    }

    #[test]
    fn portal_energy_on_axis() {
        let door = standard_door();
        let source = Vec3::new(5.0, 1.0, -3.0);
        let listener = Vec3::new(5.0, 1.0, 3.0);
        let energy = portal_energy_transfer(source, &door, listener, 20.0);
        for &e in &energy {
            assert!(
                (0.0..=1.0).contains(&e),
                "energy should be in [0,1], got {e}"
            );
        }
        // On-axis should have some energy transfer
        assert!(energy[4] > 0.0, "1 kHz on-axis should have energy transfer");
    }

    #[test]
    fn portal_energy_off_axis_lower() {
        let door = standard_door();
        let on_axis = portal_energy_transfer(
            Vec3::new(5.0, 1.0, -3.0),
            &door,
            Vec3::new(5.0, 1.0, 3.0),
            20.0,
        );
        let off_axis = portal_energy_transfer(
            Vec3::new(0.0, 1.0, -3.0), // far off to the side
            &door,
            Vec3::new(5.0, 1.0, 3.0),
            20.0,
        );
        let on_sum: f32 = on_axis.iter().sum();
        let off_sum: f32 = off_axis.iter().sum();
        assert!(
            on_sum > off_sum,
            "on-axis ({on_sum}) should have more energy than off-axis ({off_sum})"
        );
    }

    #[test]
    fn zero_size_portal_no_transmission() {
        let tiny = Portal {
            position: Vec3::ZERO,
            normal: Vec3::Z,
            width: 0.0,
            height: 0.0,
        };
        let factors = tiny.transmission_factor(20.0);
        for &f in &factors {
            assert_eq!(f, 0.0);
        }
    }

    #[test]
    fn portal_serializes() {
        let door = standard_door();
        let json = serde_json::to_string(&door).unwrap();
        let back: Portal = serde_json::from_str(&json).unwrap();
        assert_eq!(door, back);
    }
}
