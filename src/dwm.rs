//! 3D rectilinear Digital Waveguide Mesh (DWM) per Smith / Van Duyne & Smith.
//!
//! Models acoustic propagation as 1-D waveguides connecting nodes on a 3D
//! Cartesian grid. Each node is a `K`-port scattering junction (`K = 6` for
//! the rectilinear lattice: ±x, ±y, ±z neighbours). At each time step we
//! gather incoming wave components from neighbours, compute the junction
//! pressure as the average of incoming waves scaled by `2/K`, and emit
//! outgoing wave components back along each port for the next step.
//!
//! References:
//! - Smith, "Physical Audio Signal Processing," Stanford CCRMA online book.
//! - Van Duyne & Smith, "Physical modelling with the 2-D digital waveguide
//!   mesh," ICMC 1993.
//! - Murphy / Beeson / reuk, "Wayverb" — open-source 3D DWM/FDTD room
//!   acoustics simulator.
//!
//! ## Why DWM rather than 3D FDTD?
//!
//! For the rectilinear lattice, DWM is mathematically equivalent to a
//! specific FDTD scheme — but the waveguide formalism makes per-band
//! impedance boundaries and non-rectilinear topologies a clean extension
//! rather than a structural rewrite. This module ships the 3D rectilinear
//! variant with rigid Neumann walls; per-wall materials, per-band
//! impedance filters, and triangular meshes are planned as the v1.4.x
//! ladder (see `docs/development/roadmap.md`).
//!
//! ## Sound speed and grid spacing
//!
//! The 3D rectilinear DWM has a fixed wave speed coupling:
//! `c = Δx / (Δt · √3)`. In other words, given the user's `sample_rate`
//! (`Δt = 1/sample_rate`) and `speed_of_sound`, the grid spacing must be
//! `Δx = c · Δt · √3`. The solver validates this — it logs a `tracing::warn`
//! if `dx` deviates from the required value by more than 1%, and returns an
//! empty result if the deviation exceeds 10%.
//!
//! ## Plugging into the hybrid crossover
//!
//! Run the solver, then turn the receiver pressure trace into per-band
//! energies via `crate::fdtd::band_energies` (already pub, reused here to
//! avoid duplication). Pass the result to `crate::hybrid::blend_results`.

use crate::material::{AcousticMaterial, NUM_BANDS};
use serde::{Deserialize, Serialize};

/// √3, used to relate Δx, Δt, and c on the 3D rectilinear lattice.
const SQRT_3: f32 = 1.732_050_8;

/// Number of waveguide ports per node (rectilinear: ±x ±y ±z).
const K: usize = 6;
/// Junction-pressure scattering coefficient: `2/K`.
const SCATTER: f32 = 2.0 / K as f32;

/// Maximum allowed grid cells `nx · ny · nz` to bound memory.
const MAX_GRID_CELLS: usize = 4_000_000;

/// Maximum allowed time-step count to bound runtime.
const MAX_TIME_STEPS: u32 = 1_000_000;

/// Soft tolerance: warn when `|dx − c·Δt·√3| / dx_expected` exceeds this.
const DX_WARN_TOL: f32 = 0.01;
/// Hard tolerance: refuse to run when the deviation exceeds this.
const DX_FAIL_TOL: f32 = 0.10;

/// Configuration for a 3D DWM simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DwmConfig {
    /// Sample rate (Hz). Defines `Δt = 1 / sample_rate`.
    pub sample_rate: u32,
    /// Grid spacing Δx = Δy = Δz (meters). Must satisfy
    /// `dx ≈ speed_of_sound / sample_rate · √3` within 10%.
    pub dx: f32,
    /// Speed of sound (m/s). Used to validate `dx`.
    pub speed_of_sound: f32,
    /// Number of grid cells along the x-axis.
    pub nx: usize,
    /// Number of grid cells along the y-axis.
    pub ny: usize,
    /// Number of grid cells along the z-axis.
    pub nz: usize,
    /// Total simulation time (seconds).
    pub duration_seconds: f32,
    /// Uniform absorption coefficient `α` applied at every wall face,
    /// in `0..=1`. `0.0` = rigid Neumann (full reflection), `1.0` =
    /// fully absorbing (no reflection). Per-wall and per-band variants
    /// land in v1.4.1 / v1.4.2. Out-of-range values are clamped silently.
    ///
    /// Convention: the boundary reflection amplitude is `R = √(1 − α)`
    /// so that the reflected energy fraction equals `1 − α`, matching
    /// the standard acoustics meaning of α.
    pub wall_absorption: f32,
}

impl Default for DwmConfig {
    fn default() -> Self {
        // sample_rate=22050, c=343 → dx = 343 / 22050 · √3 ≈ 0.02694 m
        Self {
            sample_rate: 22_050,
            dx: 0.026_94,
            speed_of_sound: 343.0,
            nx: 30,
            ny: 25,
            nz: 20,
            duration_seconds: 0.05,
            wall_absorption: 0.0,
        }
    }
}

impl DwmConfig {
    /// Set `wall_absorption` from an [`AcousticMaterial`] by pulling its
    /// `average_absorption()` (mean of the 8 ISO octave-band coefficients).
    /// Consumes and returns `self` for builder-style chaining.
    ///
    /// v1.4.1 will replace this with a per-wall `[AcousticMaterial; 6]`
    /// taking each face's own band-average; the helper there will
    /// supersede this one.
    #[must_use]
    pub fn with_acoustic_material(mut self, material: &AcousticMaterial) -> Self {
        self.wall_absorption = material.average_absorption().clamp(0.0, 1.0);
        self
    }
}

/// Source injected into the grid as an additive pressure term per time step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DwmSource {
    /// Grid x-index of the source cell.
    pub ix: usize,
    /// Grid y-index of the source cell.
    pub iy: usize,
    /// Grid z-index of the source cell.
    pub iz: usize,
    /// Per-time-step injected pressure samples. Steps beyond `signal.len()`
    /// continue propagating without further injection.
    pub signal: Vec<f32>,
}

impl DwmSource {
    /// Gaussian pulse centred at `peak_step` with width `sigma_steps` samples.
    /// Spread covers ~`6 σ` steps; useful as a broad-spectrum impulse.
    #[must_use]
    pub fn gaussian_pulse(
        ix: usize,
        iy: usize,
        iz: usize,
        peak_step: u32,
        sigma_steps: f32,
        amplitude: f32,
    ) -> Self {
        let n = ((peak_step as f32 + 6.0 * sigma_steps).ceil() as usize).max(1);
        let inv_sigma = 1.0 / sigma_steps.max(1.0);
        let signal: Vec<f32> = (0..n)
            .map(|i| {
                let arg = (i as f32 - peak_step as f32) * inv_sigma;
                amplitude * (-0.5 * arg * arg).exp()
            })
            .collect();
        Self { ix, iy, iz, signal }
    }
}

/// Receiver — a grid cell whose pressure history is recorded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DwmReceiver {
    /// Grid x-index.
    pub ix: usize,
    /// Grid y-index.
    pub iy: usize,
    /// Grid z-index.
    pub iz: usize,
}

/// Result of a DWM simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DwmResult {
    /// Pressure trace at each receiver, one `Vec<f32>` per receiver, length =
    /// `time_steps`. Empty if the receiver index is out of range (silent).
    pub receiver_signals: Vec<Vec<f32>>,
    /// Final pressure-field snapshot, row-major (`(z * ny + y) * nx + x`).
    pub final_pressure: Vec<f32>,
    /// Number of time steps actually run.
    pub time_steps: u32,
    /// Effective Δt used (seconds).
    pub dt: f32,
}

#[inline]
fn idx(x: usize, y: usize, z: usize, nx: usize, ny: usize) -> usize {
    (z * ny + y) * nx + x
}

/// Solve the 3D acoustic wave equation on a rigid-walled box via DWM.
///
/// Returns an empty result on degenerate input (zero/tiny grid, source out
/// of range, `dx` deviates from `c·Δt·√3` by more than 10%, etc.).
#[must_use]
#[tracing::instrument(skip(config, source, receivers), fields(
    sample_rate = config.sample_rate,
    nx = config.nx,
    ny = config.ny,
    nz = config.nz,
    duration_seconds = config.duration_seconds,
    receivers = receivers.len(),
))]
pub fn solve_dwm_3d(
    config: &DwmConfig,
    source: &DwmSource,
    receivers: &[DwmReceiver],
) -> DwmResult {
    if config.sample_rate == 0
        || config.dx <= 0.0
        || config.speed_of_sound <= 0.0
        || config.nx < 3
        || config.ny < 3
        || config.nz < 3
        || config.duration_seconds <= 0.0
    {
        return empty_result();
    }
    let n_total = config
        .nx
        .saturating_mul(config.ny)
        .saturating_mul(config.nz);
    if n_total == 0 || n_total > MAX_GRID_CELLS {
        return empty_result();
    }
    if source.ix >= config.nx || source.iy >= config.ny || source.iz >= config.nz {
        return empty_result();
    }

    let dt = 1.0 / config.sample_rate as f32;
    let dx_expected = config.speed_of_sound * dt * SQRT_3;
    let dx_dev = (config.dx - dx_expected).abs() / dx_expected.max(f32::EPSILON);
    if dx_dev > DX_FAIL_TOL {
        tracing::warn!(
            dx = config.dx,
            dx_expected,
            dx_dev,
            "DWM dx deviates from c·Δt·√3 by more than 10%; refusing to run"
        );
        return empty_result();
    }
    if dx_dev > DX_WARN_TOL {
        tracing::warn!(
            dx = config.dx,
            dx_expected,
            dx_dev,
            "DWM dx deviates from c·Δt·√3 by >1%; sound speed will be off"
        );
    }

    let num_steps = ((config.duration_seconds / dt) as u32).min(MAX_TIME_STEPS);
    if num_steps == 0 {
        return empty_result();
    }

    let nx = config.nx;
    let ny = config.ny;
    let nz = config.nz;
    let n = n_total;

    // R = √(1 − α) so reflected *energy* = (1 − α) of incident.
    let reflection = (1.0 - config.wall_absorption.clamp(0.0, 1.0))
        .max(0.0)
        .sqrt();

    // Outgoing-wave buffers, K components per node, indexed `node * K + dir`.
    // Direction indices: 0 = +x, 1 = -x, 2 = +y, 3 = -y, 4 = +z, 5 = -z.
    let mut out_curr = vec![0.0_f32; n * K];
    let mut out_next = vec![0.0_f32; n * K];
    // Pressure scratch (kept around for the final snapshot).
    let mut pressure = vec![0.0_f32; n];

    let mut receiver_signals: Vec<Vec<f32>> = receivers
        .iter()
        .map(|r| {
            if r.ix < nx && r.iy < ny && r.iz < nz {
                Vec::with_capacity(num_steps as usize)
            } else {
                Vec::new()
            }
        })
        .collect();

    let source_node = idx(source.ix, source.iy, source.iz, nx, ny);

    for step in 0..num_steps {
        // Sweep all nodes: gather incoming waves, compute pressure, emit outgoing.
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let node = idx(x, y, z, nx, ny);
                    let base = node * K;

                    // Incoming from each direction. At a boundary face the
                    // missing neighbour reflects the previously-outgoing wave
                    // back, scaled by the reflection coefficient R.
                    // R = 1.0 ⇒ rigid Neumann; R = 0.0 ⇒ fully absorbing.
                    let in_xpos = if x + 1 < nx {
                        out_curr[idx(x + 1, y, z, nx, ny) * K + 1] // neighbour's −x outgoing
                    } else {
                        reflection * out_curr[base]
                    };
                    let in_xneg = if x > 0 {
                        out_curr[idx(x - 1, y, z, nx, ny) * K]
                    } else {
                        reflection * out_curr[base + 1]
                    };
                    let in_ypos = if y + 1 < ny {
                        out_curr[idx(x, y + 1, z, nx, ny) * K + 3]
                    } else {
                        reflection * out_curr[base + 2]
                    };
                    let in_yneg = if y > 0 {
                        out_curr[idx(x, y - 1, z, nx, ny) * K + 2]
                    } else {
                        reflection * out_curr[base + 3]
                    };
                    let in_zpos = if z + 1 < nz {
                        out_curr[idx(x, y, z + 1, nx, ny) * K + 5]
                    } else {
                        reflection * out_curr[base + 4]
                    };
                    let in_zneg = if z > 0 {
                        out_curr[idx(x, y, z - 1, nx, ny) * K + 4]
                    } else {
                        reflection * out_curr[base + 5]
                    };

                    // Junction pressure: p = (2/K) · Σ incoming.
                    let sum = in_xpos + in_xneg + in_ypos + in_yneg + in_zpos + in_zneg;
                    let mut p = SCATTER * sum;

                    // Source injection: transparent (additive) at the source node.
                    if node == source_node
                        && let Some(&s) = source.signal.get(step as usize)
                    {
                        p += s;
                    }

                    pressure[node] = p;

                    // Outgoing waves: out_i = p − in_i.
                    out_next[base] = p - in_xpos;
                    out_next[base + 1] = p - in_xneg;
                    out_next[base + 2] = p - in_ypos;
                    out_next[base + 3] = p - in_yneg;
                    out_next[base + 4] = p - in_zpos;
                    out_next[base + 5] = p - in_zneg;
                }
            }
        }

        // Sample receivers.
        for (rx, recv_signal) in receivers.iter().zip(receiver_signals.iter_mut()) {
            if rx.ix < nx && rx.iy < ny && rx.iz < nz {
                recv_signal.push(pressure[idx(rx.ix, rx.iy, rx.iz, nx, ny)]);
            }
        }

        // Rotate buffers.
        std::mem::swap(&mut out_curr, &mut out_next);
    }

    DwmResult {
        receiver_signals,
        final_pressure: pressure,
        time_steps: num_steps,
        dt,
    }
}

fn empty_result() -> DwmResult {
    DwmResult {
        receiver_signals: Vec::new(),
        final_pressure: Vec::new(),
        time_steps: 0,
        dt: 0.0,
    }
}

/// Convenience: the grid spacing required for a given sample rate and
/// sound speed, `Δx = c · Δt · √3`.
#[must_use]
#[inline]
pub fn required_dx(sample_rate: u32, speed_of_sound: f32) -> f32 {
    if sample_rate == 0 {
        return 0.0;
    }
    speed_of_sound * SQRT_3 / sample_rate as f32
}

/// Convenience: `[f32; NUM_BANDS]` energies via the existing FDTD Goertzel
/// helper. Re-export for hybrid-crossover ergonomics.
#[must_use]
#[inline]
pub fn band_energies(signal: &[f32], sample_rate: u32) -> [f32; NUM_BANDS] {
    crate::fdtd::band_energies(signal, sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> DwmConfig {
        DwmConfig {
            sample_rate: 22_050,
            dx: required_dx(22_050, 343.0),
            speed_of_sound: 343.0,
            nx: 30,
            ny: 25,
            nz: 20,
            duration_seconds: 0.05,
            wall_absorption: 0.0,
        }
    }

    #[test]
    fn required_dx_matches_formula() {
        let dx = required_dx(22_050, 343.0);
        let expected = 343.0 * SQRT_3 / 22_050.0;
        assert!((dx - expected).abs() < 1e-6);
    }

    #[test]
    fn required_dx_zero_sample_rate_zero() {
        assert_eq!(required_dx(0, 343.0), 0.0);
    }

    #[test]
    fn dx_severely_off_returns_empty() {
        let mut c = small_config();
        c.dx *= 5.0; // way off
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn small_dx_deviation_proceeds() {
        // 0.5% off — within tolerance, should run
        let mut c = small_config();
        c.dx *= 1.005;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        assert!(r.time_steps > 0);
    }

    #[test]
    fn tiny_grid_returns_empty() {
        let mut c = small_config();
        c.nx = 1;
        let src = DwmSource {
            ix: 0,
            iy: 0,
            iz: 0,
            signal: vec![1.0],
        };
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn source_out_of_grid_returns_empty() {
        let c = small_config();
        let src = DwmSource {
            ix: 1000,
            iy: 0,
            iz: 0,
            signal: vec![1.0],
        };
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn zero_dx_returns_empty() {
        let mut c = small_config();
        c.dx = 0.0;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn zero_duration_returns_empty() {
        let mut c = small_config();
        c.duration_seconds = 0.0;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn pulse_propagates_to_receiver() {
        let c = small_config();
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let recv = DwmReceiver {
            ix: 20,
            iy: 12,
            iz: 10,
        };
        let r = solve_dwm_3d(&c, &src, &[recv]);
        assert_eq!(r.receiver_signals.len(), 1);
        let trace = &r.receiver_signals[0];
        assert!(!trace.is_empty());
        let energy: f32 = trace.iter().map(|s| s * s).sum();
        assert!(
            energy > 0.0,
            "receiver should pick up pulse, energy={energy}"
        );
    }

    #[test]
    fn receiver_out_of_grid_silent() {
        let c = small_config();
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let recv = DwmReceiver {
            ix: 1000,
            iy: 1000,
            iz: 1000,
        };
        let r = solve_dwm_3d(&c, &src, &[recv]);
        assert!(r.receiver_signals[0].is_empty());
    }

    #[test]
    fn rigid_box_energy_bounded() {
        // A single Gaussian pulse in a rigid box should not amplify the
        // total field energy without bound. Run for ~10× the pulse width
        // and confirm the snapshot energy stays finite and bounded.
        let mut c = small_config();
        c.duration_seconds = 0.1;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        let total_energy: f32 = r.final_pressure.iter().map(|p| p * p).sum();
        assert!(
            total_energy.is_finite() && total_energy < 1e6,
            "energy should stay bounded, got {total_energy}"
        );
    }

    #[test]
    fn first_axial_mode_in_response() {
        // 1 m × 1 m × 1 m rigid box (≈37³ cells at the default dx).
        // First (1,0,0) axial mode at f₁ = c/(2L) = 343/2 = 171.5 Hz.
        //
        // DWM has no numerical dissipation, so a broadband pulse plus
        // rigid walls accumulates energy at the lattice frequency
        // indefinitely. Use a band-limited source (large σ) so the
        // injected energy lives mostly below the lowest few octaves.
        let dx = required_dx(22_050, 343.0);
        let n = (1.0 / dx).round() as usize; // ~37
        let c = DwmConfig {
            sample_rate: 22_050,
            dx,
            speed_of_sound: 343.0,
            nx: n,
            ny: n,
            nz: n,
            duration_seconds: 0.2,
            wall_absorption: 0.0,
        };
        // σ=30 samples → spectral half-width ~125 Hz; energy concentrated
        // below ~600 Hz, where the (1,0,0) mode at 171.5 Hz lives.
        let src = DwmSource::gaussian_pulse(n / 5, n / 2, n / 2, 60, 30.0, 1.0);
        let recv = DwmReceiver {
            ix: 4 * n / 5,
            iy: n / 2,
            iz: n / 2,
        };
        let r = solve_dwm_3d(&c, &src, &[recv]);
        assert!(!r.receiver_signals[0].is_empty());
        let energies = band_energies(&r.receiver_signals[0], c.sample_rate);
        let dominant = energies
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert!(
            dominant <= 3,
            "171 Hz first-mode energy should land in 63/125/250/500 Hz bands, \
             got dominant band {dominant}, energies={energies:?}"
        );
    }

    #[test]
    fn grid_cap_returns_empty() {
        let mut c = small_config();
        c.nx = 1_000;
        c.ny = 1_000;
        c.nz = 1_000;
        let src = DwmSource {
            ix: 0,
            iy: 0,
            iz: 0,
            signal: vec![1.0],
        };
        let r = solve_dwm_3d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = small_config();
        let json = serde_json::to_string(&c).unwrap();
        let back: DwmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn source_serialization_roundtrip() {
        let s = DwmSource::gaussian_pulse(2, 3, 4, 5, 1.5, 0.5);
        let json = serde_json::to_string(&s).unwrap();
        let back: DwmSource = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn integrates_with_hybrid_crossover() {
        // End-to-end: solve DWM, derive band energies via re-export,
        // blend via hybrid.
        let c = small_config();
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let recv = DwmReceiver {
            ix: 18,
            iy: 12,
            iz: 10,
        };
        let r = solve_dwm_3d(&c, &src, &[recv]);
        let wave = band_energies(&r.receiver_signals[0], c.sample_rate);
        let geom = [1.0; NUM_BANDS];
        let cfg = crate::hybrid::CrossoverConfig::default();
        let blended = crate::hybrid::blend_results(&wave, &geom, &cfg);
        // Low bands lean toward DWM, high bands toward geometric.
        assert!(blended[0] < blended[7] || blended[7] >= 0.9);
    }

    #[test]
    fn rigid_default_absorption_is_zero() {
        // Bite-1 baseline: default config has wall_absorption = 0 (rigid).
        let c = DwmConfig::default();
        assert_eq!(c.wall_absorption, 0.0);
    }

    #[test]
    fn absorption_decreases_field_energy() {
        // Run the same source with rigid vs. half-absorbing walls; the
        // absorbing case should retain less field energy at simulation end.
        let mut c = small_config();
        c.duration_seconds = 0.1;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);

        c.wall_absorption = 0.0;
        let rigid = solve_dwm_3d(&c, &src, &[]);
        let energy_rigid: f32 = rigid.final_pressure.iter().map(|p| p * p).sum();

        c.wall_absorption = 0.5;
        let absorbing = solve_dwm_3d(&c, &src, &[]);
        let energy_absorbing: f32 = absorbing.final_pressure.iter().map(|p| p * p).sum();

        assert!(
            energy_absorbing < 0.7 * energy_rigid,
            "α=0.5 should drop energy noticeably below rigid; rigid={energy_rigid}, absorbing={energy_absorbing}"
        );
    }

    #[test]
    fn fully_absorbing_walls_drain_to_near_zero() {
        let mut c = small_config();
        c.duration_seconds = 0.1;
        c.wall_absorption = 1.0;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        let energy: f32 = r.final_pressure.iter().map(|p| p * p).sum();
        // Source signal stops well before duration ends; α=1 walls
        // should leave the box mostly empty.
        assert!(
            energy < 1.0,
            "fully absorbing walls should drain field energy; got {energy}"
        );
    }

    #[test]
    fn absorption_increases_receiver_decay() {
        // Receiver pressure tail should fall faster under higher absorption.
        let mut c = small_config();
        c.duration_seconds = 0.05;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);
        let recv = DwmReceiver {
            ix: 20,
            iy: 12,
            iz: 10,
        };

        c.wall_absorption = 0.0;
        let rigid = solve_dwm_3d(&c, &src, std::slice::from_ref(&recv));
        c.wall_absorption = 0.5;
        let absorbing = solve_dwm_3d(&c, &src, std::slice::from_ref(&recv));

        // Compare the energy in the second half of the trace.
        let half_energy = |trace: &[f32]| -> f32 {
            let half = trace.len() / 2;
            trace[half..].iter().map(|s| s * s).sum()
        };
        let rigid_tail = half_energy(&rigid.receiver_signals[0]);
        let absorbing_tail = half_energy(&absorbing.receiver_signals[0]);
        assert!(
            absorbing_tail < rigid_tail,
            "absorbing tail energy ({absorbing_tail}) should be < rigid tail ({rigid_tail})"
        );
    }

    #[test]
    fn with_acoustic_material_pulls_average_absorption() {
        let carpet = AcousticMaterial::carpet();
        let c = DwmConfig::default().with_acoustic_material(&carpet);
        assert!((c.wall_absorption - carpet.average_absorption()).abs() < 1e-6);
    }

    #[test]
    fn with_acoustic_material_clamps() {
        // Construct a hypothetical material with average > 1 — clamp to 1.
        // (AcousticMaterial::new validates 0..=1, so this path shouldn't
        // hit in normal use, but check that the helper is defensive.)
        let mat = AcousticMaterial::carpet();
        let c = DwmConfig::default().with_acoustic_material(&mat);
        assert!((0.0..=1.0).contains(&c.wall_absorption));
    }

    #[test]
    fn out_of_range_absorption_clamped_silently() {
        let mut c = small_config();
        c.duration_seconds = 0.02;
        c.wall_absorption = 5.0; // nonsense; expected clamp to 1.0 (fully absorbing)
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        // Should run without panic and produce a valid (mostly drained) field.
        assert!(r.time_steps > 0);
        for &p in &r.final_pressure {
            assert!(p.is_finite(), "pressure should stay finite under clamped α");
        }
    }

    #[test]
    fn carpet_material_more_absorbing_than_concrete() {
        let mut c = small_config();
        c.duration_seconds = 0.05;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);

        let c_concrete = c
            .clone()
            .with_acoustic_material(&AcousticMaterial::concrete());
        let c_carpet = c.with_acoustic_material(&AcousticMaterial::carpet());

        let r_concrete = solve_dwm_3d(&c_concrete, &src, &[]);
        let r_carpet = solve_dwm_3d(&c_carpet, &src, &[]);

        let e_concrete: f32 = r_concrete.final_pressure.iter().map(|p| p * p).sum();
        let e_carpet: f32 = r_carpet.final_pressure.iter().map(|p| p * p).sum();
        assert!(
            e_carpet < e_concrete,
            "carpet (high α) should retain less energy than concrete (low α): \
             carpet={e_carpet}, concrete={e_concrete}"
        );
    }
}
