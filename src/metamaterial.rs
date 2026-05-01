//! Acoustic metamaterials — engineered structures with frequency-dependent
//! effective parameters.
//!
//! Three analytical models plus a lookup-table form for manufacturer data:
//!
//! - `Metamaterial::NegativeStiffness` — bulk modulus K(ω) goes negative
//!   near resonance (e.g. Helmholtz arrays, Fang et al. 2006). Density is
//!   a positive constant.
//! - `Metamaterial::NegativeDensity` — effective density ρ(ω) goes
//!   negative near resonance (membrane-mass / dipole-resonant inclusions,
//!   Yang et al. 2008). Bulk modulus is a positive constant.
//! - `Metamaterial::DoublyNegative` — both ρ_eff and K_eff resonant;
//!   supports backward waves / negative refraction (Lee et al. 2010).
//! - `Metamaterial::LookupTable` — frequency vs. absorption rows from a
//!   measurement or datasheet, linearly interpolated.
//!
//! All models produce a per-band absorption profile in goonj's 8 ISO
//! octave bands so a metamaterial panel plugs into the existing
//! `AcousticMaterial` pipeline as a wall material.
//!
//! Bulk-impedance approximation: at each frequency, Z_eff = √(ρ_eff·K_eff)
//! is the (complex) characteristic impedance; α = 1 − |R|² with
//! R = (Z_eff − Z₀)/(Z_eff + Z₀) for normal incidence against air at
//! Z₀ ≈ 413 rayls (20 °C, 1 atm). Treats the metamaterial as a
//! semi-infinite half-space — for thin panels with significant
//! transmission, pair with the existing `material::transmission_loss`.
//!
//! References:
//! - Liu et al., "Locally Resonant Sonic Materials," Science 289, 2000.
//! - Fang et al., "Ultrasonic metamaterials with negative modulus,"
//!   Nature Materials 5, 2006.
//! - Yang et al., "Membrane-type acoustic metamaterial with negative
//!   dynamic mass," Phys. Rev. Lett. 101, 2008.
//! - Lee et al., "Composite acoustic medium with simultaneously negative
//!   density and modulus," Phys. Rev. Lett. 104, 2010.

use crate::error::Result;
use crate::material::{AcousticMaterial, FREQUENCY_BANDS, NUM_BANDS};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Characteristic impedance of air, Z₀ = ρ₀·c at 20 °C, 1 atm (rayls).
pub const AIR_IMPEDANCE_RAYLS: f32 = 413.0;

/// Reference air density at 20 °C (kg/m³) — convenience for `background_value`.
pub const AIR_DENSITY: f32 = 1.205;

/// Reference air bulk modulus K₀ = ρ₀·c² at 20 °C (Pa) — convenience for
/// `background_value`.
pub const AIR_BULK_MODULUS: f32 = 1.42e5;

/// Lorentzian (Drude–Lorentz) resonance for a frequency-dependent effective
/// parameter:
///
/// `X(ω) = X_bg · [ 1 − ω_p² / (ω² − ω_0² + iγω) ]`
///
/// where `X_bg = background_value` is the off-resonance / high-frequency
/// limit. The real part dips below zero near `center_hz` when `plasma_hz`
/// is large enough — that's the "negative" regime of the metamaterial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LorentzianResonance {
    /// Resonance frequency f₀ = ω₀/(2π) (Hz).
    pub center_hz: f32,
    /// Plasma frequency f_p = ω_p/(2π) (Hz). Sets resonance strength.
    pub plasma_hz: f32,
    /// Damping rate f_γ = γ/(2π) (Hz). Higher = broader, shallower dip.
    pub damping_hz: f32,
    /// Off-resonance / high-frequency-limit value of X (the canonical
    /// Drude–Lorentz "ε_∞"). The DC value is
    /// `background_value · (1 + (plasma_hz / center_hz)²)`. Use the
    /// [`LorentzianResonance::with_dc_value`] constructor if you'd
    /// rather specify the DC value directly.
    pub background_value: f32,
}

impl LorentzianResonance {
    /// Construct a resonance and pick `background_value` so the DC
    /// value (ω→0) equals `dc_value` exactly. Convenience for users
    /// who think of static density / bulk modulus rather than the
    /// Drude–Lorentz background.
    #[must_use]
    pub fn with_dc_value(center_hz: f32, plasma_hz: f32, damping_hz: f32, dc_value: f32) -> Self {
        // X(0) = X_bg · (1 + (ω_p/ω_0)²); invert for X_bg.
        let center = center_hz.max(f32::EPSILON);
        let factor = 1.0 + (plasma_hz / center).powi(2);
        Self {
            center_hz,
            plasma_hz,
            damping_hz,
            background_value: dc_value / factor,
        }
    }

    /// Evaluate `X(ω)` at `frequency_hz`. Returns `(real, imag)`.
    #[must_use]
    pub fn evaluate(&self, frequency_hz: f32) -> (f32, f32) {
        let omega = TAU * frequency_hz;
        let omega_0 = TAU * self.center_hz;
        let omega_p = TAU * self.plasma_hz;
        let gamma = TAU * self.damping_hz;
        let omega2 = omega * omega;
        let omega_0_2 = omega_0 * omega_0;
        let omega_p_2 = omega_p * omega_p;
        let denom = (omega2 - omega_0_2).powi(2) + gamma * gamma * omega2;
        if denom < f32::EPSILON {
            return (self.background_value, 0.0);
        }
        let real = self.background_value * (1.0 - omega_p_2 * (omega2 - omega_0_2) / denom);
        let imag = self.background_value * omega_p_2 * gamma * omega / denom;
        (real, imag)
    }

    /// Real part of `X(ω)` — negative around resonance for strong enough `plasma_hz`.
    #[must_use]
    #[inline]
    pub fn real_part(&self, frequency_hz: f32) -> f32 {
        self.evaluate(frequency_hz).0
    }
}

/// Acoustic metamaterial model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Metamaterial {
    /// Negative-stiffness: K(ω) goes negative near resonance; ρ is constant.
    NegativeStiffness {
        /// Frequency-dependent effective bulk modulus (Pa).
        k_eff: LorentzianResonance,
        /// Effective density of the host medium (kg/m³).
        rho: f32,
    },
    /// Negative-density: ρ(ω) goes negative near resonance; K is constant.
    NegativeDensity {
        /// Frequency-dependent effective density (kg/m³).
        rho_eff: LorentzianResonance,
        /// Effective bulk modulus of the host medium (Pa).
        k_modulus: f32,
    },
    /// Doubly-negative: both ρ_eff and K_eff have Lorentzian resonances.
    /// Supports backward waves where both real parts are negative.
    DoublyNegative {
        /// Effective bulk modulus (Pa).
        k_eff: LorentzianResonance,
        /// Effective density (kg/m³).
        rho_eff: LorentzianResonance,
    },
    /// Manufacturer absorption table: `(frequency_hz, absorption_0..=1)`
    /// rows sorted ascending by frequency. Linear interpolation between
    /// rows; outside the range the endpoint absorption is held.
    LookupTable {
        /// Sorted (frequency_hz, absorption) rows. At least one required.
        rows: Vec<(f32, f32)>,
    },
}

impl Metamaterial {
    /// Absorption coefficient at frequency f (Hz), in 0..=1.
    #[must_use]
    pub fn absorption_at(&self, frequency_hz: f32) -> f32 {
        match self {
            Self::NegativeStiffness { k_eff, rho } => {
                impedance_absorption(k_eff.evaluate(frequency_hz), (*rho, 0.0))
            }
            Self::NegativeDensity { rho_eff, k_modulus } => {
                impedance_absorption((*k_modulus, 0.0), rho_eff.evaluate(frequency_hz))
            }
            Self::DoublyNegative { k_eff, rho_eff } => {
                impedance_absorption(k_eff.evaluate(frequency_hz), rho_eff.evaluate(frequency_hz))
            }
            Self::LookupTable { rows } => lookup_absorption(rows, frequency_hz),
        }
    }

    /// Absorption coefficients for the 8 ISO octave bands (63–8000 Hz).
    #[must_use]
    #[tracing::instrument(skip(self))]
    pub fn absorption_bands(&self) -> [f32; NUM_BANDS] {
        std::array::from_fn(|band| self.absorption_at(FREQUENCY_BANDS[band]))
    }

    /// Convert to an [`AcousticMaterial`] for use as a wall material.
    pub fn to_acoustic_material(
        &self,
        name: impl Into<String>,
        scattering: f32,
    ) -> Result<AcousticMaterial> {
        AcousticMaterial::new(name, self.absorption_bands(), scattering)
    }
}

/// Normal-incidence bulk absorption from complex `(K_eff, ρ_eff)` against air.
///
/// Z = √(ρ·K), R = (Z − Z₀)/(Z + Z₀), α = 1 − |R|².
fn impedance_absorption(k: (f32, f32), rho: (f32, f32)) -> f32 {
    // ρ·K (complex multiply)
    let prod_r = rho.0 * k.0 - rho.1 * k.1;
    let prod_i = rho.0 * k.1 + rho.1 * k.0;
    // Principal √(prod) = √|prod|·(cos(arg/2) + i·sin(arg/2))
    let mag = (prod_r * prod_r + prod_i * prod_i).sqrt();
    let phase = prod_i.atan2(prod_r);
    let sqrt_mag = mag.sqrt();
    let z_r = sqrt_mag * (phase * 0.5).cos();
    let z_i = sqrt_mag * (phase * 0.5).sin();
    // R = (Z − Z₀)/(Z + Z₀)
    let num_r = z_r - AIR_IMPEDANCE_RAYLS;
    let num_i = z_i;
    let den_r = z_r + AIR_IMPEDANCE_RAYLS;
    let den_i = z_i;
    let den_mag2 = den_r * den_r + den_i * den_i;
    if den_mag2 < f32::EPSILON {
        return 1.0;
    }
    let r_mag2 = (num_r * num_r + num_i * num_i) / den_mag2;
    (1.0 - r_mag2).clamp(0.0, 1.0)
}

fn lookup_absorption(rows: &[(f32, f32)], frequency_hz: f32) -> f32 {
    if rows.is_empty() {
        return 0.0;
    }
    if rows.len() == 1 || frequency_hz <= rows[0].0 {
        return rows[0].1.clamp(0.0, 1.0);
    }
    let last = rows[rows.len() - 1];
    if frequency_hz >= last.0 {
        return last.1.clamp(0.0, 1.0);
    }
    for w in rows.windows(2) {
        let (f_a, a_a) = w[0];
        let (f_b, a_b) = w[1];
        if frequency_hz >= f_a && frequency_hz <= f_b {
            let span = (f_b - f_a).max(f32::EPSILON);
            let frac = (frequency_hz - f_a) / span;
            return (a_a + (a_b - a_a) * frac).clamp(0.0, 1.0);
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neg_stiffness_500() -> Metamaterial {
        Metamaterial::NegativeStiffness {
            k_eff: LorentzianResonance {
                center_hz: 500.0,
                plasma_hz: 800.0,
                damping_hz: 50.0,
                background_value: AIR_BULK_MODULUS,
            },
            rho: AIR_DENSITY,
        }
    }

    fn neg_density_400() -> Metamaterial {
        Metamaterial::NegativeDensity {
            rho_eff: LorentzianResonance {
                center_hz: 400.0,
                plasma_hz: 600.0,
                damping_hz: 40.0,
                background_value: AIR_DENSITY,
            },
            k_modulus: AIR_BULK_MODULUS,
        }
    }

    #[test]
    fn lorentzian_far_above_resonance_returns_background() {
        let r = LorentzianResonance {
            center_hz: 1000.0,
            plasma_hz: 500.0,
            damping_hz: 100.0,
            background_value: 1.205,
        };
        // f ≫ f₀: real → background_value, imag → 0
        let (re, im) = r.evaluate(100_000.0);
        assert!(
            (re - 1.205).abs() < 0.01,
            "real should ≈ background at f≫f₀, got {re}"
        );
        assert!(im.abs() < 0.01, "imag should ≈ 0 at f≫f₀, got {im}");
    }

    #[test]
    fn with_dc_value_recovers_specified_dc() {
        let r = LorentzianResonance::with_dc_value(500.0, 800.0, 50.0, 1.42e5);
        let (re, _) = r.evaluate(0.001);
        assert!(
            (re / 1.42e5 - 1.0).abs() < 0.01,
            "DC value should match specification, got {re}"
        );
    }

    #[test]
    fn lorentzian_above_resonance_can_go_negative() {
        // Plasma > center: real part dips below zero just above f_0
        let r = LorentzianResonance {
            center_hz: 500.0,
            plasma_hz: 800.0,
            damping_hz: 20.0,
            background_value: 1.0,
        };
        // Just above f_0 the real part should be negative
        let re = r.real_part(520.0);
        assert!(
            re < 0.0,
            "real part should go negative just above resonance, got {re}"
        );
    }

    #[test]
    fn lorentzian_zero_plasma_is_constant() {
        let r = LorentzianResonance {
            center_hz: 500.0,
            plasma_hz: 0.0,
            damping_hz: 50.0,
            background_value: 2.5,
        };
        for f in [10.0, 500.0, 5000.0] {
            let (re, im) = r.evaluate(f);
            assert!((re - 2.5).abs() < 1e-4);
            assert!(im.abs() < 1e-4);
        }
    }

    #[test]
    fn negative_stiffness_absorption_in_range() {
        let m = neg_stiffness_500();
        for &band in &FREQUENCY_BANDS {
            let a = m.absorption_at(band);
            assert!(
                (0.0..=1.0).contains(&a),
                "α at {band}Hz = {a} outside [0,1]"
            );
        }
    }

    #[test]
    fn negative_stiffness_shows_frequency_dependence() {
        // The bulk-impedance approach yields different α across bands,
        // even when the parameters give "stiffening" rather than peaking
        // — what matters for the physics is non-trivial dispersion.
        let bands = neg_stiffness_500().absorption_bands();
        let max = bands.iter().copied().fold(0.0_f32, f32::max);
        let min = bands.iter().copied().fold(1.0_f32, f32::min);
        assert!(
            max - min > 0.05,
            "negative-stiffness should be frequency-dependent (max-min>0.05), got bands={bands:?}"
        );
    }

    #[test]
    fn negative_density_shows_frequency_dependence() {
        let bands = neg_density_400().absorption_bands();
        let max = bands.iter().copied().fold(0.0_f32, f32::max);
        let min = bands.iter().copied().fold(1.0_f32, f32::min);
        assert!(
            max - min > 0.05,
            "negative-density should be frequency-dependent (max-min>0.05), got bands={bands:?}"
        );
    }

    #[test]
    fn dc_tuned_metamaterial_is_well_formed() {
        // Verify with_dc_value flows through to a usable metamaterial:
        // bands stay in [0,1] and the tuning gives meaningful dispersion.
        let target_hz = 500.0;
        let rho_dc = AIR_IMPEDANCE_RAYLS * AIR_IMPEDANCE_RAYLS / AIR_BULK_MODULUS;
        let m = Metamaterial::NegativeDensity {
            rho_eff: LorentzianResonance::with_dc_value(target_hz, 600.0, 100.0, rho_dc),
            k_modulus: AIR_BULK_MODULUS,
        };
        let bands = m.absorption_bands();
        for a in bands {
            assert!((0.0..=1.0).contains(&a));
        }
        let max = bands.iter().copied().fold(0.0_f32, f32::max);
        let min = bands.iter().copied().fold(1.0_f32, f32::min);
        assert!(
            max - min > 0.05,
            "DC-tuned metamaterial should be frequency-dependent, got bands={bands:?}"
        );
    }

    #[test]
    fn doubly_negative_in_range() {
        let m = Metamaterial::DoublyNegative {
            k_eff: LorentzianResonance {
                center_hz: 500.0,
                plasma_hz: 700.0,
                damping_hz: 60.0,
                background_value: AIR_BULK_MODULUS,
            },
            rho_eff: LorentzianResonance {
                center_hz: 500.0,
                plasma_hz: 600.0,
                damping_hz: 40.0,
                background_value: AIR_DENSITY,
            },
        };
        for &band in &FREQUENCY_BANDS {
            let a = m.absorption_at(band);
            assert!((0.0..=1.0).contains(&a));
        }
    }

    #[test]
    fn lookup_interpolates_between_rows() {
        let m = Metamaterial::LookupTable {
            rows: vec![(100.0, 0.10), (1000.0, 0.50), (10000.0, 0.90)],
        };
        assert!((m.absorption_at(100.0) - 0.10).abs() < 1e-5);
        assert!((m.absorption_at(1000.0) - 0.50).abs() < 1e-5);
        // Halfway between 100 and 1000 in linear space (not log)
        let mid = m.absorption_at(550.0);
        assert!(
            (mid - 0.30).abs() < 0.01,
            "linear interp at 550 Hz between (100,0.10) and (1000,0.50) ≈ 0.30, got {mid}"
        );
    }

    #[test]
    fn lookup_clamps_outside_range() {
        let m = Metamaterial::LookupTable {
            rows: vec![(100.0, 0.10), (1000.0, 0.50)],
        };
        assert!((m.absorption_at(10.0) - 0.10).abs() < 1e-5);
        assert!((m.absorption_at(20_000.0) - 0.50).abs() < 1e-5);
    }

    #[test]
    fn lookup_clamps_invalid_rows() {
        let m = Metamaterial::LookupTable {
            rows: vec![(100.0, -0.5), (1000.0, 1.5)],
        };
        assert_eq!(m.absorption_at(100.0), 0.0);
        assert_eq!(m.absorption_at(1000.0), 1.0);
    }

    #[test]
    fn lookup_empty_returns_zero() {
        let m = Metamaterial::LookupTable { rows: vec![] };
        assert_eq!(m.absorption_at(500.0), 0.0);
    }

    #[test]
    fn lookup_single_row_holds() {
        let m = Metamaterial::LookupTable {
            rows: vec![(500.0, 0.42)],
        };
        assert!((m.absorption_at(50.0) - 0.42).abs() < 1e-5);
        assert!((m.absorption_at(500.0) - 0.42).abs() < 1e-5);
        assert!((m.absorption_at(5_000.0) - 0.42).abs() < 1e-5);
    }

    #[test]
    fn absorption_bands_returns_eight() {
        let m = neg_stiffness_500();
        let bands = m.absorption_bands();
        assert_eq!(bands.len(), NUM_BANDS);
        for a in bands {
            assert!((0.0..=1.0).contains(&a));
        }
    }

    #[test]
    fn convert_to_acoustic_material() {
        let m = neg_stiffness_500();
        let am = m.to_acoustic_material("ns_500hz", 0.2).unwrap();
        assert_eq!(am.name, "ns_500hz");
        assert_eq!(am.scattering, 0.2);
        for a in am.absorption {
            assert!((0.0..=1.0).contains(&a));
        }
    }

    #[test]
    fn convert_rejects_invalid_scattering() {
        let m = neg_stiffness_500();
        assert!(m.to_acoustic_material("bad", 1.5).is_err());
    }

    #[test]
    fn impedance_match_zero_reflection() {
        // ρ·K with Z = Z₀ exactly: α should equal 1.0
        // Z₀² = AIR_IMPEDANCE_RAYLS² ≈ 1.706e5; choose ρ=1, K = Z₀²/ρ
        let z02 = AIR_IMPEDANCE_RAYLS * AIR_IMPEDANCE_RAYLS;
        let alpha = impedance_absorption((z02, 0.0), (1.0, 0.0));
        assert!(
            alpha > 0.99,
            "impedance-matched should give α≈1, got {alpha}"
        );
    }

    #[test]
    fn rigid_wall_high_reflection() {
        // Very high impedance vs air → low absorption
        let alpha = impedance_absorption((1e10, 0.0), (1e6, 0.0));
        assert!(alpha < 0.01, "rigid wall should reflect, α={alpha}");
    }

    #[test]
    fn metamaterial_serialization_roundtrip() {
        let m = neg_stiffness_500();
        let json = serde_json::to_string(&m).unwrap();
        let back: Metamaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn lookup_serialization_roundtrip() {
        let m = Metamaterial::LookupTable {
            rows: vec![(125.0, 0.05), (1000.0, 0.6), (8000.0, 0.4)],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Metamaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
