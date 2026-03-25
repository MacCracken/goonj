//! Source directivity patterns for frequency-dependent sound radiation.
//!
//! Models how sound sources radiate energy non-uniformly in space. Supports
//! analytical patterns (omnidirectional, cardioid, dipole) and tabulated
//! balloon data for measured directivity (e.g., CLF/CF2 format).

use crate::material::NUM_BANDS;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A directivity pattern describing how a source radiates sound vs angle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DirectivityPattern {
    /// Equal radiation in all directions.
    Omnidirectional,
    /// Cardioid: `gain = 0.5 × (1 + cos θ)` where θ is angle from front axis.
    Cardioid,
    /// Subcardioid (wide cardioid): `gain = 0.75 + 0.25 × cos θ`.
    Subcardioid,
    /// Supercardioid: `gain = 0.37 + 0.63 × cos θ`.
    Supercardioid,
    /// Figure-8 / dipole: `gain = |cos θ|`.
    Figure8,
    /// Tabulated per-band directivity data from measured balloon.
    Tabulated(Box<DirectivityBalloon>),
}

/// Measured directivity data as a table of gain values indexed by angle and frequency band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectivityBalloon {
    /// Azimuth angles in radians (typically −π to π).
    pub azimuths: Vec<f32>,
    /// Elevation angles in radians (typically −π/2 to π/2).
    pub elevations: Vec<f32>,
    /// Per-band gain values, indexed as `gains[band][elevation_idx * num_azimuths + azimuth_idx]`.
    /// Values in linear scale (0.0–1.0+ where 1.0 = on-axis reference level).
    pub gains: [Vec<f32>; NUM_BANDS],
}

impl DirectivityPattern {
    /// Compute the directivity gain for a given direction relative to the source's
    /// front axis. Returns gain in linear scale (1.0 = on-axis reference).
    ///
    /// `direction` is the unit vector from source toward the evaluation point.
    /// `front` is the unit vector of the source's main radiation axis.
    #[must_use]
    #[inline]
    pub fn gain(&self, direction: Vec3, front: Vec3) -> f32 {
        match self {
            Self::Omnidirectional => 1.0,
            Self::Cardioid => {
                let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
                (0.5 * (1.0 + cos_theta)).max(0.0)
            }
            Self::Subcardioid => {
                let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
                0.75 + 0.25 * cos_theta
            }
            Self::Supercardioid => {
                let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
                (0.37 + 0.63 * cos_theta).max(0.0)
            }
            Self::Figure8 => {
                let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
                cos_theta.abs()
            }
            Self::Tabulated(balloon) => balloon.interpolate_broadband(direction, front),
        }
    }

    /// Compute per-band directivity gain.
    ///
    /// For analytical patterns, all bands are equal. For tabulated data,
    /// each band has its own spatial pattern.
    #[must_use]
    pub fn gain_per_band(&self, direction: Vec3, front: Vec3) -> [f32; NUM_BANDS] {
        match self {
            Self::Tabulated(balloon) => balloon.interpolate_per_band(direction, front),
            _ => {
                let g = self.gain(direction, front);
                [g; NUM_BANDS]
            }
        }
    }
}

impl DirectivityBalloon {
    /// Interpolate broadband gain (average across bands) for a direction.
    #[must_use]
    fn interpolate_broadband(&self, direction: Vec3, front: Vec3) -> f32 {
        let per_band = self.interpolate_per_band(direction, front);
        per_band.iter().sum::<f32>() / NUM_BANDS as f32
    }

    /// Interpolate per-band gain for a direction using nearest-neighbor lookup.
    #[must_use]
    fn interpolate_per_band(&self, direction: Vec3, front: Vec3) -> [f32; NUM_BANDS] {
        if self.azimuths.is_empty() || self.elevations.is_empty() {
            return [1.0; NUM_BANDS];
        }

        // Convert direction to azimuth/elevation relative to front
        let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
        let theta = cos_theta.acos(); // polar angle from front

        // For simplicity, map to elevation=0 and azimuth=theta (axisymmetric fallback)
        let target_az = theta;
        let target_el = 0.0_f32;

        // Nearest-neighbor lookup
        let az_idx = self
            .azimuths
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((**a - target_az).abs())
                    .partial_cmp(&((**b - target_az).abs()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let el_idx = self
            .elevations
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((**a - target_el).abs())
                    .partial_cmp(&((**b - target_el).abs()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let flat_idx = el_idx * self.azimuths.len() + az_idx;

        std::array::from_fn(|band| {
            if flat_idx < self.gains[band].len() {
                self.gains[band][flat_idx]
            } else {
                1.0
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omnidirectional_always_one() {
        let p = DirectivityPattern::Omnidirectional;
        assert_eq!(p.gain(Vec3::X, Vec3::Z), 1.0);
        assert_eq!(p.gain(-Vec3::Z, Vec3::Z), 1.0);
    }

    #[test]
    fn cardioid_on_axis_max() {
        let p = DirectivityPattern::Cardioid;
        let on_axis = p.gain(Vec3::Z, Vec3::Z);
        assert!(
            (on_axis - 1.0).abs() < 0.01,
            "on-axis should be ~1.0, got {on_axis}"
        );
    }

    #[test]
    fn cardioid_rear_null() {
        let p = DirectivityPattern::Cardioid;
        let rear = p.gain(-Vec3::Z, Vec3::Z);
        assert!(rear.abs() < 0.01, "rear should be ~0.0, got {rear}");
    }

    #[test]
    fn cardioid_side_half() {
        let p = DirectivityPattern::Cardioid;
        let side = p.gain(Vec3::X, Vec3::Z);
        assert!((side - 0.5).abs() < 0.01, "90° should be ~0.5, got {side}");
    }

    #[test]
    fn figure8_on_axis_one() {
        let p = DirectivityPattern::Figure8;
        assert!((p.gain(Vec3::Z, Vec3::Z) - 1.0).abs() < 0.01);
    }

    #[test]
    fn figure8_side_zero() {
        let p = DirectivityPattern::Figure8;
        let side = p.gain(Vec3::X, Vec3::Z);
        assert!(side.abs() < 0.01, "90° should be ~0.0, got {side}");
    }

    #[test]
    fn figure8_rear_one() {
        let p = DirectivityPattern::Figure8;
        assert!((p.gain(-Vec3::Z, Vec3::Z) - 1.0).abs() < 0.01);
    }

    #[test]
    fn supercardioid_on_axis() {
        let p = DirectivityPattern::Supercardioid;
        let on = p.gain(Vec3::Z, Vec3::Z);
        assert!((on - 1.0).abs() < 0.01);
    }

    #[test]
    fn gain_per_band_analytical_uniform() {
        let p = DirectivityPattern::Cardioid;
        let bands = p.gain_per_band(Vec3::Z, Vec3::Z);
        for &g in &bands {
            assert!((g - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn tabulated_empty_returns_one() {
        let balloon = DirectivityBalloon {
            azimuths: vec![],
            elevations: vec![],
            gains: std::array::from_fn(|_| vec![]),
        };
        let p = DirectivityPattern::Tabulated(Box::new(balloon));
        assert_eq!(p.gain(Vec3::Z, Vec3::Z), 1.0);
    }

    #[test]
    fn tabulated_single_point() {
        let balloon = DirectivityBalloon {
            azimuths: vec![0.0],
            elevations: vec![0.0],
            gains: std::array::from_fn(|_| vec![0.8]),
        };
        let p = DirectivityPattern::Tabulated(Box::new(balloon));
        let g = p.gain(Vec3::Z, Vec3::Z);
        assert!(
            (g - 0.8).abs() < 0.01,
            "single-point should return 0.8, got {g}"
        );
    }

    #[test]
    fn directivity_pattern_serializes() {
        let p = DirectivityPattern::Cardioid;
        let json = serde_json::to_string(&p).unwrap();
        let back: DirectivityPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
