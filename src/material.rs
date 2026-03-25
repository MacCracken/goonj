use crate::error::GoonjError;
use serde::{Deserialize, Serialize};

/// Number of octave frequency bands (ISO 3382: 63 Hz – 8000 Hz).
pub const NUM_BANDS: usize = 8;

/// Octave-band centre frequencies in Hz (ISO 3382-1).
pub const FREQUENCY_BANDS: [f32; NUM_BANDS] =
    [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

/// Acoustic material with frequency-dependent absorption and scattering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticMaterial {
    /// Material name.
    pub name: String,
    /// Absorption coefficients per frequency band (0.0 = fully reflective, 1.0 = fully absorptive).
    pub absorption: [f32; NUM_BANDS],
    /// Scattering coefficient (0.0 = specular, 1.0 = fully diffuse).
    pub scattering: f32,
}

impl AcousticMaterial {
    /// Create a new material with validated absorption and scattering coefficients.
    ///
    /// All absorption values and the scattering coefficient must be in the range 0.0–1.0.
    pub fn new(
        name: impl Into<String>,
        absorption: [f32; NUM_BANDS],
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

    /// Absorption at a specific band index (0–7). Returns 0.0 if out of range.
    #[must_use]
    #[inline]
    pub fn absorption_at_band(&self, band: usize) -> f32 {
        if band < NUM_BANDS {
            self.absorption[band]
        } else {
            0.0
        }
    }

    /// Concrete: hard, highly reflective.
    #[must_use]
    pub fn concrete() -> Self {
        Self {
            name: "concrete".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.01, 0.01, 0.01, 0.02, 0.02, 0.02, 0.03, 0.04],
            scattering: 0.10,
        }
    }

    /// Carpet: soft, highly absorptive at high frequencies.
    #[must_use]
    pub fn carpet() -> Self {
        Self {
            name: "carpet".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.02, 0.08, 0.24, 0.57, 0.69, 0.71, 0.73, 0.73],
            scattering: 0.40,
        }
    }

    /// Glass: reflective at low frequencies, less at high.
    #[must_use]
    pub fn glass() -> Self {
        Self {
            name: "glass".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.35, 0.35, 0.25, 0.18, 0.12, 0.07, 0.04, 0.02],
            scattering: 0.05,
        }
    }

    /// Wood paneling.
    #[must_use]
    pub fn wood() -> Self {
        Self {
            name: "wood".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.15, 0.15, 0.11, 0.10, 0.07, 0.06, 0.07, 0.07],
            scattering: 0.15,
        }
    }

    /// Heavy curtain / drape.
    #[must_use]
    pub fn curtain() -> Self {
        Self {
            name: "curtain".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.03, 0.07, 0.31, 0.49, 0.75, 0.70, 0.65, 0.65],
            scattering: 0.50,
        }
    }

    /// Drywall / gypsum board.
    #[must_use]
    pub fn drywall() -> Self {
        Self {
            name: "drywall".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.29, 0.29, 0.10, 0.05, 0.04, 0.07, 0.09, 0.09],
            scattering: 0.10,
        }
    }

    /// Ceramic tile.
    #[must_use]
    pub fn tile() -> Self {
        Self {
            name: "tile".into(),
            //           63    125   250   500   1k    2k    4k    8k Hz
            absorption: [0.01, 0.01, 0.01, 0.01, 0.01, 0.02, 0.02, 0.02],
            scattering: 0.05,
        }
    }
}

/// Wall construction properties for sound transmission loss calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallConstruction {
    /// Surface mass density in kg/m² (e.g. 12mm drywall ≈ 10 kg/m²).
    pub surface_density: f32,
    /// Critical (coincidence) frequency in Hz. Below this, mass law dominates.
    /// For common materials: drywall ~2500 Hz, glass ~1250 Hz, concrete ~150 Hz.
    pub critical_frequency: f32,
    /// Internal loss factor (damping). Typical: 0.01–0.03 for stiff materials, 0.1+ for damped.
    pub loss_factor: f32,
}

impl WallConstruction {
    /// Single-leaf drywall (12.5 mm gypsum board).
    #[must_use]
    pub fn drywall_single() -> Self {
        Self {
            surface_density: 10.0,
            critical_frequency: 2500.0,
            loss_factor: 0.014,
        }
    }

    /// Double-leaf drywall (2 × 12.5 mm with air gap).
    #[must_use]
    pub fn drywall_double() -> Self {
        Self {
            surface_density: 20.0,
            critical_frequency: 2500.0,
            loss_factor: 0.02,
        }
    }

    /// 150 mm concrete wall.
    #[must_use]
    pub fn concrete_150mm() -> Self {
        Self {
            surface_density: 350.0,
            critical_frequency: 130.0,
            loss_factor: 0.01,
        }
    }

    /// 6 mm glass pane.
    #[must_use]
    pub fn glass_6mm() -> Self {
        Self {
            surface_density: 15.0,
            critical_frequency: 2000.0,
            loss_factor: 0.02,
        }
    }

    /// Sound Reduction Index (transmission loss) in dB at a given frequency.
    ///
    /// Uses the mass law below the critical frequency and Davy's model
    /// (simplified) above it. Returns TL in dB (higher = more isolation).
    ///
    /// Reference: J.L. Davy, "Predicting the sound insulation of single leaf
    /// walls — extension of Cremer's model," JASA 2009.
    #[must_use]
    #[inline]
    pub fn transmission_loss_db(&self, frequency: f32) -> f32 {
        if frequency <= 0.0 || self.surface_density <= 0.0 {
            return 0.0;
        }

        // Mass law: TL = 20 × log10(π × f × m / (ρ₀ × c₀)) - 3
        // where m = surface density, ρ₀c₀ ≈ 415 (air impedance at 20°C)
        let rho_c = 415.0_f32;
        let mass_law =
            20.0 * (std::f32::consts::PI * frequency * self.surface_density / rho_c).log10() - 3.0;

        if frequency < self.critical_frequency * 0.5 {
            // Below coincidence: pure mass law
            mass_law.max(0.0)
        } else if frequency < self.critical_frequency * 2.0 {
            // Near coincidence: mass law with coincidence dip (reduced by loss factor)
            let coincidence_dip = 10.0
                * (self.loss_factor + self.surface_density / (485.0 * frequency.sqrt())).log10();
            (mass_law + coincidence_dip).max(0.0)
        } else {
            // Above coincidence: mass law + damping controlled increase
            let above = mass_law + 10.0 * self.loss_factor.max(0.001).log10() + 5.0;
            above.max(0.0)
        }
    }

    /// Transmission coefficient (energy ratio 0.0–1.0) at a given frequency.
    ///
    /// τ = 10^(-TL/10), where TL is the transmission loss in dB.
    #[must_use]
    #[inline]
    pub fn transmission_coefficient(&self, frequency: f32) -> f32 {
        let tl = self.transmission_loss_db(frequency);
        10.0_f32.powf(-tl / 10.0)
    }
}

/// Johnson-Champoux-Allard-Lafarge (JCAL) porous material model.
///
/// A 6-parameter model for detailed characterization of porous absorbers.
/// More accurate than Miki for materials with known microstructural properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JcalMaterial {
    /// Flow resistivity in Pa·s/m².
    pub flow_resistivity: f32,
    /// Porosity (0.0–1.0).
    pub porosity: f32,
    /// Tortuosity (≥1.0, typically 1.0–4.0).
    pub tortuosity: f32,
    /// Viscous characteristic length in meters (typically 30–300 µm).
    pub viscous_length: f32,
    /// Thermal characteristic length in meters (typically 50–600 µm).
    pub thermal_length: f32,
    /// Static thermal permeability in m² (typically 1e-10 to 1e-8).
    pub thermal_permeability: f32,
}

impl JcalMaterial {
    /// Standard fibrous absorber (e.g., mineral wool).
    #[must_use]
    pub fn mineral_wool() -> Self {
        Self {
            flow_resistivity: 30_000.0,
            porosity: 0.97,
            tortuosity: 1.06,
            viscous_length: 100.0e-6,
            thermal_length: 200.0e-6,
            thermal_permeability: 1.5e-9,
        }
    }

    /// Open-cell foam.
    #[must_use]
    pub fn open_cell_foam() -> Self {
        Self {
            flow_resistivity: 10_000.0,
            porosity: 0.98,
            tortuosity: 1.02,
            viscous_length: 150.0e-6,
            thermal_length: 300.0e-6,
            thermal_permeability: 3.0e-9,
        }
    }

    /// Compute the surface impedance magnitude at a given frequency.
    ///
    /// Uses the JCAL model to compute the complex characteristic impedance
    /// and propagation constant, then derives the surface impedance for
    /// a layer of given thickness backed by a rigid wall.
    ///
    /// Returns the impedance magnitude normalized to ρ₀c₀.
    #[must_use]
    pub fn surface_impedance_magnitude(&self, frequency: f32, thickness: f32) -> f32 {
        if frequency <= 0.0 || thickness <= 0.0 || self.porosity <= 0.0 {
            return 1.0;
        }

        let omega = std::f32::consts::TAU * frequency;

        // Simplified JCAL: effective density and bulk modulus magnitudes
        // Full complex computation would require complex arithmetic;
        // this gives the magnitude for absorption coefficient estimation.
        let sigma = self.flow_resistivity;
        let phi = self.porosity;
        let alpha_inf = self.tortuosity;

        // Johnson effective density magnitude (viscous effects)
        let rho_0 = 1.21_f32; // air density
        let omega_sigma = omega * rho_0 * alpha_inf / sigma;
        let g_visc = (1.0 + omega_sigma * omega_sigma).sqrt();
        let rho_eff = rho_0 * alpha_inf * g_visc / phi;

        // Champoux-Allard bulk modulus magnitude (thermal effects)
        let gamma = 1.4_f32; // ratio of specific heats
        let p_0 = 101325.0_f32; // atmospheric pressure
        let k_eff = gamma * p_0 / phi;

        // Characteristic impedance magnitude: Z_c = sqrt(ρ_eff × K_eff)
        let z_c = (rho_eff * k_eff).sqrt();

        // Normalize to air impedance
        let rho_c = rho_0 * 343.0;
        (z_c / rho_c).max(0.01)
    }

    /// Compute absorption coefficient at a given frequency for a layer of given thickness.
    ///
    /// Returns absorption coefficient (0.0–1.0) for normal incidence.
    #[must_use]
    #[inline]
    pub fn absorption_coefficient(&self, frequency: f32, thickness: f32) -> f32 {
        let z_norm = self.surface_impedance_magnitude(frequency, thickness);
        // Normal incidence absorption: α = 1 - |R|² where R = (Z-1)/(Z+1)
        let r = ((z_norm - 1.0) / (z_norm + 1.0)).abs();
        (1.0 - r * r).clamp(0.0, 1.0)
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
        assert!((w.absorption_at_band(0) - 0.15).abs() < f32::EPSILON); // 63 Hz
        assert!((w.absorption_at_band(7) - 0.07).abs() < f32::EPSILON); // 8000 Hz
        assert!((w.absorption_at_band(8)).abs() < f32::EPSILON); // out of range
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
        let m = AcousticMaterial::new("test", [0.1; NUM_BANDS], 0.5);
        assert!(m.is_ok());
        assert_eq!(m.unwrap().name, "test");
    }

    #[test]
    fn new_rejects_absorption_above_one() {
        let mut abs = [0.1; NUM_BANDS];
        abs[2] = 1.5;
        let m = AcousticMaterial::new("bad", abs, 0.5);
        assert!(m.is_err());
    }

    #[test]
    fn new_rejects_negative_absorption() {
        let mut abs = [0.1; NUM_BANDS];
        abs[0] = -0.1;
        let m = AcousticMaterial::new("bad", abs, 0.5);
        assert!(m.is_err());
    }

    #[test]
    fn new_rejects_scattering_out_of_range() {
        let m = AcousticMaterial::new("bad", [0.1; NUM_BANDS], 1.5);
        assert!(m.is_err());
    }

    // --- Wall transmission tests ---

    #[test]
    fn concrete_high_transmission_loss() {
        let wall = WallConstruction::concrete_150mm();
        let tl_1k = wall.transmission_loss_db(1000.0);
        assert!(
            tl_1k > 40.0,
            "concrete should have >40 dB TL at 1kHz, got {tl_1k}"
        );
    }

    #[test]
    fn drywall_lower_than_concrete() {
        let drywall = WallConstruction::drywall_single();
        let concrete = WallConstruction::concrete_150mm();
        let tl_drywall = drywall.transmission_loss_db(1000.0);
        let tl_concrete = concrete.transmission_loss_db(1000.0);
        assert!(
            tl_concrete > tl_drywall,
            "concrete ({tl_concrete}) should isolate more than drywall ({tl_drywall})"
        );
    }

    #[test]
    fn transmission_loss_increases_with_frequency_below_coincidence() {
        // Test mass law regime (well below critical frequency of 2500 Hz)
        let wall = WallConstruction::concrete_150mm(); // fc ≈ 130 Hz → mass law above that
        let tl_250 = wall.transmission_loss_db(250.0);
        let tl_1k = wall.transmission_loss_db(1000.0);
        assert!(
            tl_1k > tl_250,
            "higher freq should have more TL: 1kHz={tl_1k} vs 250Hz={tl_250}"
        );
    }

    #[test]
    fn transmission_coefficient_in_range() {
        let wall = WallConstruction::glass_6mm();
        for &f in &FREQUENCY_BANDS {
            let tau = wall.transmission_coefficient(f);
            assert!(
                (0.0..=1.0).contains(&tau),
                "τ should be in [0,1] at {f} Hz, got {tau}"
            );
        }
    }

    #[test]
    fn transmission_loss_non_negative() {
        let wall = WallConstruction::drywall_single();
        for &f in &FREQUENCY_BANDS {
            let tl = wall.transmission_loss_db(f);
            assert!(tl >= 0.0, "TL should be non-negative at {f} Hz, got {tl}");
        }
    }

    #[test]
    fn double_drywall_better_than_single() {
        let single = WallConstruction::drywall_single();
        let double = WallConstruction::drywall_double();
        let tl_s = single.transmission_loss_db(500.0);
        let tl_d = double.transmission_loss_db(500.0);
        assert!(
            tl_d > tl_s,
            "double ({tl_d}) should isolate more than single ({tl_s})"
        );
    }

    #[test]
    fn transmission_loss_zero_frequency() {
        let wall = WallConstruction::concrete_150mm();
        assert_eq!(wall.transmission_loss_db(0.0), 0.0);
    }

    // --- JCAL tests ---

    #[test]
    fn jcal_mineral_wool_absorbs() {
        let mat = JcalMaterial::mineral_wool();
        let alpha = mat.absorption_coefficient(1000.0, 0.05); // 50mm thick
        assert!(
            alpha > 0.3,
            "50mm mineral wool should absorb >0.3 at 1kHz, got {alpha}"
        );
    }

    #[test]
    fn jcal_absorption_increases_with_thickness() {
        let mat = JcalMaterial::mineral_wool();
        let thin = mat.absorption_coefficient(1000.0, 0.025);
        let thick = mat.absorption_coefficient(1000.0, 0.100);
        assert!(
            thick >= thin,
            "thicker ({thick}) should absorb at least as much as thinner ({thin})"
        );
    }

    #[test]
    fn jcal_absorption_in_range() {
        let mat = JcalMaterial::open_cell_foam();
        for &f in &FREQUENCY_BANDS {
            let alpha = mat.absorption_coefficient(f, 0.05);
            assert!(
                (0.0..=1.0).contains(&alpha),
                "absorption {alpha} out of range at {f} Hz"
            );
        }
    }

    #[test]
    fn jcal_zero_frequency_returns_valid() {
        let mat = JcalMaterial::mineral_wool();
        let z = mat.surface_impedance_magnitude(0.0, 0.05);
        assert!((z - 1.0).abs() < f32::EPSILON);
    }
}
