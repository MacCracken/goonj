use crate::error::GoonjError;
use serde::{Deserialize, Serialize};

/// Frequency bands for absorption coefficients: 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz.
pub const FREQUENCY_BANDS: [f32; 6] = [125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0];

/// Acoustic material with frequency-dependent absorption and scattering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticMaterial {
    /// Material name.
    pub name: String,
    /// Absorption coefficients per frequency band (0.0 = fully reflective, 1.0 = fully absorptive).
    pub absorption: [f32; 6],
    /// Scattering coefficient (0.0 = specular, 1.0 = fully diffuse).
    pub scattering: f32,
}

impl AcousticMaterial {
    /// Create a new material with validated absorption and scattering coefficients.
    ///
    /// All absorption values and the scattering coefficient must be in the range 0.0–1.0.
    pub fn new(
        name: impl Into<String>,
        absorption: [f32; 6],
        scattering: f32,
    ) -> crate::error::Result<Self> {
        for (i, &a) in absorption.iter().enumerate() {
            if !(0.0..=1.0).contains(&a) {
                return Err(GoonjError::InvalidMaterial(format!(
                    "absorption[{i}] = {a} is outside 0.0–1.0"
                )));
            }
        }
        if !(0.0..=1.0).contains(&scattering) {
            return Err(GoonjError::InvalidMaterial(format!(
                "scattering = {scattering} is outside 0.0–1.0"
            )));
        }
        Ok(Self {
            name: name.into(),
            absorption,
            scattering,
        })
    }

    /// Average absorption coefficient across all bands.
    #[must_use]
    #[inline]
    pub fn average_absorption(&self) -> f32 {
        self.absorption.iter().sum::<f32>() / self.absorption.len() as f32
    }

    /// Absorption at a specific band index (0–5).
    #[must_use]
    #[inline]
    pub fn absorption_at_band(&self, band: usize) -> f32 {
        if band < 6 { self.absorption[band] } else { 0.0 }
    }

    /// Concrete: hard, highly reflective.
    #[must_use]
    pub fn concrete() -> Self {
        Self {
            name: "concrete".into(),
            absorption: [0.01, 0.01, 0.02, 0.02, 0.02, 0.03],
            scattering: 0.10,
        }
    }

    /// Carpet: soft, highly absorptive at high frequencies.
    #[must_use]
    pub fn carpet() -> Self {
        Self {
            name: "carpet".into(),
            absorption: [0.08, 0.24, 0.57, 0.69, 0.71, 0.73],
            scattering: 0.40,
        }
    }

    /// Glass: reflective at low frequencies, less at high.
    #[must_use]
    pub fn glass() -> Self {
        Self {
            name: "glass".into(),
            absorption: [0.35, 0.25, 0.18, 0.12, 0.07, 0.04],
            scattering: 0.05,
        }
    }

    /// Wood paneling.
    #[must_use]
    pub fn wood() -> Self {
        Self {
            name: "wood".into(),
            absorption: [0.15, 0.11, 0.10, 0.07, 0.06, 0.07],
            scattering: 0.15,
        }
    }

    /// Heavy curtain / drape.
    #[must_use]
    pub fn curtain() -> Self {
        Self {
            name: "curtain".into(),
            absorption: [0.07, 0.31, 0.49, 0.75, 0.70, 0.60],
            scattering: 0.50,
        }
    }

    /// Drywall / gypsum board.
    #[must_use]
    pub fn drywall() -> Self {
        Self {
            name: "drywall".into(),
            absorption: [0.29, 0.10, 0.05, 0.04, 0.07, 0.09],
            scattering: 0.10,
        }
    }

    /// Ceramic tile.
    #[must_use]
    pub fn tile() -> Self {
        Self {
            name: "tile".into(),
            absorption: [0.01, 0.01, 0.01, 0.01, 0.02, 0.02],
            scattering: 0.05,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorption_in_range() {
        let materials = [
            AcousticMaterial::concrete(),
            AcousticMaterial::carpet(),
            AcousticMaterial::glass(),
            AcousticMaterial::wood(),
            AcousticMaterial::curtain(),
            AcousticMaterial::drywall(),
            AcousticMaterial::tile(),
        ];
        for m in &materials {
            for &a in &m.absorption {
                assert!(
                    (0.0..=1.0).contains(&a),
                    "{}: absorption {} out of range",
                    m.name,
                    a
                );
            }
            assert!(
                (0.0..=1.0).contains(&m.scattering),
                "{}: scattering out of range",
                m.name
            );
        }
    }

    #[test]
    fn carpet_more_absorptive_than_concrete() {
        assert!(
            AcousticMaterial::carpet().average_absorption()
                > AcousticMaterial::concrete().average_absorption()
        );
    }

    #[test]
    fn average_absorption_concrete() {
        let c = AcousticMaterial::concrete();
        let avg = c.average_absorption();
        assert!((avg - 0.0183).abs() < 0.01);
    }

    #[test]
    fn absorption_at_band_valid() {
        let w = AcousticMaterial::wood();
        assert!((w.absorption_at_band(0) - 0.15).abs() < f32::EPSILON);
        assert!((w.absorption_at_band(6)).abs() < f32::EPSILON); // out of range
    }

    #[test]
    fn serde_roundtrip() {
        let m = AcousticMaterial::carpet();
        let json = serde_json::to_string(&m).unwrap();
        let back: AcousticMaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn new_valid_material() {
        let m = AcousticMaterial::new("test", [0.1; 6], 0.5);
        assert!(m.is_ok());
        assert_eq!(m.unwrap().name, "test");
    }

    #[test]
    fn new_rejects_absorption_above_one() {
        let m = AcousticMaterial::new("bad", [0.1, 0.2, 1.5, 0.1, 0.1, 0.1], 0.5);
        assert!(m.is_err());
    }

    #[test]
    fn new_rejects_negative_absorption() {
        let m = AcousticMaterial::new("bad", [-0.1, 0.2, 0.3, 0.1, 0.1, 0.1], 0.5);
        assert!(m.is_err());
    }

    #[test]
    fn new_rejects_scattering_out_of_range() {
        let m = AcousticMaterial::new("bad", [0.1; 6], 1.5);
        assert!(m.is_err());
    }
}
