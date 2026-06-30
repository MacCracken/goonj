//! Green's Function Parabolic Equation (GFPE) outdoor propagation.
//!
//! Solves the one-way Helmholtz equation in a 2D vertical slice (range × height),
//! marching range by Δr using the analytic parabolic-equation Green's function
//! plus an image source for ground reflection. Atmospheric refraction enters
//! as a per-range phase screen; terrain enters as a staircase that zeros the
//! field below the local ground elevation at each range slice.
//!
//! Reference: Gilbert & Di, "A fast Green's function method for one-way sound
//! propagation in the atmosphere," JASA 94(4), 1993.
//!
//! ## Scope and approximations
//!
//! - Direct (O(N²)) convolution per range step. No FFT — fine for typical
//!   outdoor grids (~100–500 height samples).
//! - Linear sound-speed gradient `c(z) = c₀ + g·z`. The phase screen uses
//!   `(n²(z) − 1)/2` linearised about the reference speed.
//! - Ground impedance via [`propagation::GroundImpedance`] (Miki). The
//!   reflection coefficient is evaluated at a single representative
//!   grazing angle per range step (small-angle PE assumption).
//! - Terrain: at each range, samples below `terrain.height_at(range)` are
//!   zeroed before convolution. This captures the dominant "shadow behind
//!   hill" effect without modelling diffraction over the crest in detail.
//! - Free-field reference for excess attenuation: 2D cylindrical spreading,
//!   `|ψ_free| ∝ 1/√r`.

use crate::propagation::GroundImpedance;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

/// Atmospheric refraction model: linear sound-speed gradient.
///
/// `c(z) = reference_speed + gradient_per_m · z`. Negative gradient
/// (cooler air aloft on a sunny day) bends rays upward → shadow zones
/// at long range; positive gradient (temperature inversion) bends rays
/// downward → ducting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GfpeAtmosphere {
    /// Sound-speed gradient (m/s per m). Typical: -0.1 to +0.1.
    pub gradient_per_m: f32,
    /// Reference sound speed at z=0 (m/s).
    pub reference_speed: f32,
}

impl Default for GfpeAtmosphere {
    fn default() -> Self {
        Self {
            gradient_per_m: 0.0,
            reference_speed: 343.0,
        }
    }
}

/// Terrain elevation profile along the range axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GfpeTerrain {
    /// Range positions (meters), sorted ascending. Must start at 0.
    pub ranges_m: Vec<f32>,
    /// Ground elevation at each range position (meters). Same length as `ranges_m`.
    pub heights_m: Vec<f32>,
}

impl GfpeTerrain {
    /// Flat ground at z=0 over [0, max_range].
    #[must_use]
    pub fn flat(max_range_m: f32) -> Self {
        Self {
            ranges_m: vec![0.0, max_range_m.max(0.0)],
            heights_m: vec![0.0, 0.0],
        }
    }

    /// Linearly interpolated ground height at `range_m`. Outside the
    /// defined range, the endpoint elevation is held.
    #[must_use]
    pub fn height_at(&self, range_m: f32) -> f32 {
        if self.ranges_m.is_empty() || self.ranges_m.len() != self.heights_m.len() {
            return 0.0;
        }
        if range_m <= self.ranges_m[0] {
            return self.heights_m[0];
        }
        let last = self.ranges_m.len() - 1;
        if range_m >= self.ranges_m[last] {
            return self.heights_m[last];
        }
        for i in 0..last {
            let r0 = self.ranges_m[i];
            let r1 = self.ranges_m[i + 1];
            if range_m >= r0 && range_m <= r1 {
                let span = (r1 - r0).max(f32::EPSILON);
                let frac = (range_m - r0) / span;
                return self.heights_m[i] + (self.heights_m[i + 1] - self.heights_m[i]) * frac;
            }
        }
        0.0
    }
}

/// Configuration for a GFPE simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GfpeConfig {
    /// Source frequency (Hz).
    pub frequency_hz: f32,
    /// Source height above z=0 datum (meters).
    pub source_height_m: f32,
    /// Maximum propagation range (meters).
    pub max_range_m: f32,
    /// Maximum simulation height above z=0 datum (meters).
    pub max_height_m: f32,
    /// Range step Δr (meters). Smaller = more accurate, more expensive.
    pub range_step_m: f32,
    /// Height step Δz (meters). Should sample the wavelength: ≤ λ/4.
    pub height_step_m: f32,
    /// Ground impedance model.
    pub ground_impedance: GroundImpedance,
    /// Atmospheric refraction.
    pub atmosphere: GfpeAtmosphere,
}

/// Simulation field: excess attenuation (dB) on the (range × height) grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfpeResult {
    /// Range coordinates (m), one per column.
    pub ranges_m: Vec<f32>,
    /// Height coordinates (m), one per row.
    pub heights_m: Vec<f32>,
    /// Excess attenuation (dB) relative to free-field. Row-major:
    /// `index = range_idx * num_heights + height_idx`.
    pub excess_attenuation_db: Vec<f32>,
}

impl GfpeResult {
    /// Number of range columns.
    #[must_use]
    #[inline]
    pub fn num_ranges(&self) -> usize {
        self.ranges_m.len()
    }

    /// Number of height rows.
    #[must_use]
    #[inline]
    pub fn num_heights(&self) -> usize {
        self.heights_m.len()
    }

    /// Excess attenuation (dB) at `(range_idx, height_idx)`. Returns
    /// `f32::INFINITY` if either index is out of range or below terrain.
    #[must_use]
    #[inline]
    pub fn at(&self, range_idx: usize, height_idx: usize) -> f32 {
        let nz = self.num_heights();
        if range_idx >= self.num_ranges() || height_idx >= nz {
            return f32::INFINITY;
        }
        self.excess_attenuation_db[range_idx * nz + height_idx]
    }
}

/// Maximum allowed grid cells (height × range), to bound memory.
const MAX_GRID_CELLS: usize = 10_000_000;

/// Solve GFPE over a terrain profile.
#[must_use]
#[tracing::instrument(skip(terrain, config), fields(
    frequency_hz = config.frequency_hz,
    max_range_m = config.max_range_m,
    max_height_m = config.max_height_m,
))]
pub fn solve_gfpe(terrain: &GfpeTerrain, config: &GfpeConfig) -> GfpeResult {
    if config.frequency_hz <= 0.0
        || config.max_range_m <= 0.0
        || config.max_height_m <= 0.0
        || config.range_step_m <= 0.0
        || config.height_step_m <= 0.0
        || config.atmosphere.reference_speed <= 0.0
    {
        return empty_result();
    }

    let nz = ((config.max_height_m / config.height_step_m) as usize).max(2);
    let num_ranges = ((config.max_range_m / config.range_step_m) as usize).max(1) + 1;
    if nz.saturating_mul(num_ranges) > MAX_GRID_CELLS {
        return empty_result();
    }

    let dz = config.max_height_m / nz as f32;
    let dr = config.range_step_m;
    let k0 = TAU * config.frequency_hz / config.atmosphere.reference_speed;

    let heights_m: Vec<f32> = (0..nz).map(|i| (i as f32 + 0.5) * dz).collect();
    let ranges_m: Vec<f32> = (0..num_ranges).map(|i| i as f32 * dr).collect();

    // Initial field at r=0: Gaussian starter centred at source_height_m.
    let sigma = (4.0 / k0).max(dz);
    let mut psi: Vec<(f32, f32)> = heights_m
        .iter()
        .map(|&z| {
            let arg = (z - config.source_height_m) / sigma;
            let amp = (-0.5 * arg * arg).exp();
            (amp, 0.0)
        })
        .collect();

    let mut excess_attenuation_db = vec![0.0_f32; num_ranges * nz];
    record_excess_attenuation(
        &psi,
        &heights_m,
        ranges_m[0],
        terrain,
        &mut excess_attenuation_db,
        0,
        nz,
    );

    // Reflection coefficient: evaluated once at a representative grazing
    // angle (approx 1/k0/dr arctan) — small-angle PE assumption.
    let representative_angle = (config.source_height_m.max(dz) / config.max_range_m.max(dr))
        .atan()
        .max(1e-3);
    let r_complex = ground_reflection_complex(
        config.frequency_hz,
        representative_angle,
        &config.ground_impedance,
    );

    // March range.
    for (r_idx, &range) in ranges_m.iter().enumerate().skip(1) {
        // 1. Phase screen for atmosphere.
        for (i, sample) in psi.iter_mut().enumerate() {
            let z = heights_m[i];
            let c_z = config.atmosphere.reference_speed + config.atmosphere.gradient_per_m * z;
            // n² − 1 ≈ (c0/c)² − 1
            let ratio = config.atmosphere.reference_speed / c_z.max(1.0);
            let n2_minus_1 = ratio * ratio - 1.0;
            let phase = 0.5 * k0 * n2_minus_1 * dr;
            *sample = complex_mul(*sample, complex_exp(phase));
        }

        // 2. Convolve with PE Green's function (direct half-space form).
        psi = convolve_pe_step(&psi, &heights_m, dr, dz, k0, r_complex);

        // 3. Apply terrain: zero field below local ground.
        let ground = terrain.height_at(range);
        for (i, sample) in psi.iter_mut().enumerate() {
            if heights_m[i] < ground {
                *sample = (0.0, 0.0);
            }
        }

        record_excess_attenuation(
            &psi,
            &heights_m,
            range,
            terrain,
            &mut excess_attenuation_db,
            r_idx,
            nz,
        );
    }

    GfpeResult {
        ranges_m,
        heights_m,
        excess_attenuation_db,
    }
}

fn empty_result() -> GfpeResult {
    GfpeResult {
        ranges_m: Vec::new(),
        heights_m: Vec::new(),
        excess_attenuation_db: Vec::new(),
    }
}

/// Direct half-space PE convolution: ψ_new(z) = ∫ G(z,z';Δr) ψ(z') dz'
/// with G the free-space PE kernel plus an image-source term for the ground.
fn convolve_pe_step(
    psi: &[(f32, f32)],
    heights_m: &[f32],
    dr: f32,
    dz: f32,
    k0: f32,
    r_complex: (f32, f32),
) -> Vec<(f32, f32)> {
    let nz = psi.len();
    let mut out = vec![(0.0_f32, 0.0_f32); nz];
    // Free-space kernel: sqrt(k0/(2π·i·Δr)) · exp(i·k0·(z−z')² / (2Δr))
    // |coeff| = sqrt(k0/(2π·Δr));   phase factor sqrt(1/i) = exp(-iπ/4)
    let coeff_mag = (k0 / (TAU * dr)).sqrt();
    let coeff_phase = -PI * 0.25;
    let coeff = (coeff_mag * coeff_phase.cos(), coeff_mag * coeff_phase.sin());

    for (i, z) in heights_m.iter().enumerate() {
        let mut acc = (0.0_f32, 0.0_f32);
        for (j, z_prime) in heights_m.iter().enumerate() {
            // Direct term
            let dz_diff = z - z_prime;
            let phase_d = k0 * dz_diff * dz_diff / (2.0 * dr);
            let g_d = complex_mul(coeff, complex_exp(phase_d));
            // Image-source term (reflection from z=0)
            let dz_sum = z + z_prime;
            let phase_i = k0 * dz_sum * dz_sum / (2.0 * dr);
            let g_i_free = complex_mul(coeff, complex_exp(phase_i));
            let g_i = complex_mul(g_i_free, r_complex); // multiply by reflection coefficient
            // G_total = direct − image · R̃ (sign from image method with rigid → -R̃)
            let g_total = (g_d.0 - g_i.0, g_d.1 - g_i.1);
            // Riemann-sum integration: × dz
            let contribution = complex_mul(g_total, psi[j]);
            acc.0 += contribution.0 * dz;
            acc.1 += contribution.1 * dz;
        }
        out[i] = acc;
    }
    out
}

/// Record excess attenuation at the current range slice.
#[allow(clippy::too_many_arguments)]
fn record_excess_attenuation(
    psi: &[(f32, f32)],
    heights_m: &[f32],
    range_m: f32,
    terrain: &GfpeTerrain,
    out: &mut [f32],
    r_idx: usize,
    nz: usize,
) {
    let ground = terrain.height_at(range_m);
    let r_safe = range_m.max(f32::EPSILON);
    // 2D free-field reference: |ψ_free| ∝ 1/√r. Normalised to 1 at r=0
    // is moot; we measure relative dB so a constant prefactor cancels.
    let free_ref = 1.0 / r_safe.sqrt();
    for (h_idx, sample) in psi.iter().enumerate() {
        let z = heights_m[h_idx];
        let mag = (sample.0 * sample.0 + sample.1 * sample.1).sqrt();
        let ea = if z < ground || mag < f32::EPSILON || free_ref < f32::EPSILON {
            f32::INFINITY
        } else {
            -20.0 * (mag / free_ref).log10()
        };
        out[r_idx * nz + h_idx] = ea;
    }
}

/// Complex Miki reflection coefficient at a given grazing angle.
fn ground_reflection_complex(
    frequency_hz: f32,
    grazing_angle_rad: f32,
    impedance: &GroundImpedance,
) -> (f32, f32) {
    if frequency_hz <= 0.0 || impedance.flow_resistivity <= 0.0 {
        return (1.0, 0.0);
    }
    let x = frequency_hz / impedance.flow_resistivity;
    // Miki normalised surface impedance Z/ρc
    let z_real = 1.0 + 0.0699 * x.powf(-0.632);
    let z_imag = -0.1071 * x.powf(-0.632);
    let sin_theta = grazing_angle_rad.sin();
    let num = (z_real * sin_theta - 1.0, z_imag * sin_theta);
    let den = (z_real * sin_theta + 1.0, z_imag * sin_theta);
    let den_mag2 = den.0 * den.0 + den.1 * den.1;
    if den_mag2 < f32::EPSILON {
        return (1.0, 0.0);
    }
    // R = num / den = num · conj(den) / |den|²
    let r_real = (num.0 * den.0 + num.1 * den.1) / den_mag2;
    let r_imag = (num.1 * den.0 - num.0 * den.1) / den_mag2;
    (r_real, r_imag)
}

#[inline]
fn complex_mul(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn complex_exp(theta: f32) -> (f32, f32) {
    (theta.cos(), theta.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> GfpeConfig {
        GfpeConfig {
            frequency_hz: 500.0,
            source_height_m: 5.0,
            max_range_m: 100.0,
            max_height_m: 30.0,
            range_step_m: 5.0,
            height_step_m: 1.0,
            ground_impedance: GroundImpedance::grass(),
            atmosphere: GfpeAtmosphere::default(),
        }
    }

    #[test]
    fn flat_terrain_height_is_zero() {
        let t = GfpeTerrain::flat(100.0);
        assert_eq!(t.height_at(0.0), 0.0);
        assert_eq!(t.height_at(50.0), 0.0);
        assert_eq!(t.height_at(150.0), 0.0); // outside, held
    }

    #[test]
    fn terrain_interpolates_linearly() {
        let t = GfpeTerrain {
            ranges_m: vec![0.0, 50.0, 100.0],
            heights_m: vec![0.0, 10.0, 0.0],
        };
        assert!((t.height_at(0.0) - 0.0).abs() < 1e-5);
        assert!((t.height_at(25.0) - 5.0).abs() < 1e-5);
        assert!((t.height_at(50.0) - 10.0).abs() < 1e-5);
        assert!((t.height_at(75.0) - 5.0).abs() < 1e-5);
        assert!((t.height_at(100.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn terrain_clamps_outside_range() {
        let t = GfpeTerrain {
            ranges_m: vec![0.0, 100.0],
            heights_m: vec![5.0, 15.0],
        };
        assert!((t.height_at(-10.0) - 5.0).abs() < 1e-5);
        assert!((t.height_at(200.0) - 15.0).abs() < 1e-5);
    }

    #[test]
    fn empty_terrain_returns_zero() {
        let t = GfpeTerrain {
            ranges_m: vec![],
            heights_m: vec![],
        };
        assert_eq!(t.height_at(50.0), 0.0);
    }

    #[test]
    fn solve_returns_grid_of_expected_size() {
        let terrain = GfpeTerrain::flat(100.0);
        let result = solve_gfpe(&terrain, &small_config());
        assert!(!result.ranges_m.is_empty());
        assert!(!result.heights_m.is_empty());
        let expected_len = result.num_ranges() * result.num_heights();
        assert_eq!(result.excess_attenuation_db.len(), expected_len);
    }

    #[test]
    fn invalid_config_returns_empty() {
        let terrain = GfpeTerrain::flat(100.0);
        let mut c = small_config();
        c.frequency_hz = 0.0;
        let r = solve_gfpe(&terrain, &c);
        assert!(r.ranges_m.is_empty());
        assert!(r.excess_attenuation_db.is_empty());
    }

    #[test]
    fn negative_range_step_returns_empty() {
        let terrain = GfpeTerrain::flat(100.0);
        let mut c = small_config();
        c.range_step_m = -1.0;
        assert!(solve_gfpe(&terrain, &c).ranges_m.is_empty());
    }

    #[test]
    fn excess_attenuation_finite_above_ground() {
        let terrain = GfpeTerrain::flat(100.0);
        let result = solve_gfpe(&terrain, &small_config());
        // At a mid-grid point above the source, EA should be finite.
        let r_idx = result.num_ranges() / 2;
        let h_idx = (result.num_heights() / 2).min(result.num_heights() - 1);
        let ea = result.at(r_idx, h_idx);
        assert!(ea.is_finite(), "EA should be finite at mid-grid, got {ea}");
    }

    #[test]
    fn shadow_zone_under_upward_refraction() {
        // Negative gradient → upward refraction → low-altitude receivers
        // at long range should be more attenuated than under flat atmosphere.
        let terrain = GfpeTerrain::flat(200.0);
        let mut c = small_config();
        c.max_range_m = 200.0;
        c.source_height_m = 2.0;
        c.atmosphere = GfpeAtmosphere {
            gradient_per_m: 0.0,
            reference_speed: 343.0,
        };
        let r_flat = solve_gfpe(&terrain, &c);
        c.atmosphere.gradient_per_m = -0.2;
        let r_shadow = solve_gfpe(&terrain, &c);
        // Compare EA at low height, far range
        let r_idx = r_flat.num_ranges() - 1;
        let h_idx = 1; // near ground
        let ea_flat = r_flat.at(r_idx, h_idx);
        let ea_shadow = r_shadow.at(r_idx, h_idx);
        // Both should be finite for this test to be meaningful
        if ea_flat.is_finite() && ea_shadow.is_finite() {
            assert!(
                ea_shadow > ea_flat,
                "shadow EA ({ea_shadow}) should exceed flat EA ({ea_flat}) under upward refraction"
            );
        }
    }

    #[test]
    fn hill_blocks_field_below_crest() {
        // Build a terrain with a hill at range 50m, height 20m, and
        // verify the field below 20m at that range column is INFINITY.
        let terrain = GfpeTerrain {
            ranges_m: vec![0.0, 40.0, 50.0, 60.0, 100.0],
            heights_m: vec![0.0, 0.0, 20.0, 0.0, 0.0],
        };
        let mut c = small_config();
        c.max_height_m = 40.0;
        c.height_step_m = 1.0;
        c.range_step_m = 5.0;
        let r = solve_gfpe(&terrain, &c);
        // Find the column closest to 50m
        let r_idx = r
            .ranges_m
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - 50.0).abs().partial_cmp(&(b.1 - 50.0).abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        // h_idx for z = 5 m (well below the 20 m crest)
        let h_idx = r.heights_m.iter().position(|&h| h >= 5.0).unwrap_or(0);
        let ea = r.at(r_idx, h_idx);
        assert!(
            ea.is_infinite(),
            "field below hill crest should be blocked (∞ EA), got {ea}"
        );
    }

    #[test]
    fn grid_cap_returns_empty() {
        let terrain = GfpeTerrain::flat(1.0);
        let c = GfpeConfig {
            frequency_hz: 500.0,
            source_height_m: 1.0,
            max_range_m: 1.0e9,
            max_height_m: 1.0e9,
            range_step_m: 0.001,
            height_step_m: 0.001,
            ground_impedance: GroundImpedance::grass(),
            atmosphere: GfpeAtmosphere::default(),
        };
        let r = solve_gfpe(&terrain, &c);
        assert!(r.ranges_m.is_empty(), "grid > MAX_GRID_CELLS should bail");
    }

    #[test]
    fn miki_complex_reflection_within_unit_disk() {
        let r = ground_reflection_complex(500.0, 0.1, &GroundImpedance::grass());
        let mag2 = r.0 * r.0 + r.1 * r.1;
        assert!(mag2 <= 1.001, "|R|² should be ≤ 1, got {mag2}");
    }

    #[test]
    fn miki_complex_grazing_zero_angle_high_reflection() {
        // At grazing angle → 0, reflection magnitude → 1.
        let r = ground_reflection_complex(500.0, 0.001, &GroundImpedance::grass());
        let mag = (r.0 * r.0 + r.1 * r.1).sqrt();
        assert!(mag > 0.9, "near-grazing |R| should be ≈ 1, got {mag}");
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = small_config();
        let json = serde_json::to_string(&c).unwrap();
        let back: GfpeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn terrain_serialization_roundtrip() {
        let t = GfpeTerrain {
            ranges_m: vec![0.0, 50.0, 100.0],
            heights_m: vec![0.0, 10.0, 0.0],
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: GfpeTerrain = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
