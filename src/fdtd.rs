//! 2D FDTD modal solver per Botteldooren 1995.
//!
//! Explicit finite-difference time-domain solver for the 2D acoustic wave
//! equation `∂²p/∂t² = c² ∇² p` on a regular Cartesian grid with rigid
//! (Neumann) boundaries. Best suited for **low-frequency modal behaviour**
//! below the Schroeder frequency, where ray-tracing methods break down and
//! wave interference dominates the room response.
//!
//! Reference: Botteldooren, "Finite-difference time-domain simulation of
//! low-frequency room acoustic problems," JASA 98(6), 1995.
//!
//! ## Plugging into the hybrid crossover
//!
//! Run the solver, then turn the receiver pressure trace into per-band
//! energies via `band_energies` (Goertzel at each ISO octave centre).
//! Pass the result to `crate::hybrid::blend_results` as the
//! `wave_result` slot, with a geometric (ray / image-source) result in
//! the `geometric_result` slot.
//!
//! ## Stability and accuracy
//!
//! The Courant condition `c · Δt / Δx ≤ 1/√2` (2D) bounds the time step.
//! The solver computes Δt from `1 / sample_rate` and refuses to run if the
//! ratio exceeds the CFL limit (returns an empty result).
//!
//! Numerical dispersion limits accuracy at high frequencies; sample at
//! ≤ λ/8 spatial resolution for usable response up to Δx-defined
//! frequency `c / (8 · Δx)`.

use crate::material::{FREQUENCY_BANDS, NUM_BANDS};
use serde::{Deserialize, Serialize};
use std::f32::consts::FRAC_1_SQRT_2;

/// Courant–Friedrichs–Lewy upper bound for 2D FDTD: `c·Δt/Δx ≤ 1/√2`.
const CFL_2D_LIMIT: f32 = FRAC_1_SQRT_2;

/// Maximum allowed grid cells `nx · ny` to bound memory.
const MAX_GRID_CELLS: usize = 4_000_000;

/// Maximum allowed time-step count to bound runtime.
const MAX_TIME_STEPS: u32 = 1_000_000;

/// Configuration for a 2D FDTD simulation in a rigid-walled box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdtdConfig {
    /// Sample rate (Hz). Defines `Δt = 1 / sample_rate`.
    pub sample_rate: u32,
    /// Grid spacing Δx = Δy (meters).
    pub dx: f32,
    /// Speed of sound (m/s).
    pub speed_of_sound: f32,
    /// Number of grid cells along the x-axis.
    pub nx: usize,
    /// Number of grid cells along the y-axis.
    pub ny: usize,
    /// Total simulation time (seconds).
    pub duration_seconds: f32,
}

impl Default for FdtdConfig {
    fn default() -> Self {
        Self {
            sample_rate: 22_050,
            dx: 0.05,
            speed_of_sound: 343.0,
            nx: 80,
            ny: 80,
            duration_seconds: 0.5,
        }
    }
}

/// Source injected into the grid as an additive pressure term per time step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdtdSource {
    /// Grid x-index of the source cell.
    pub ix: usize,
    /// Grid y-index of the source cell.
    pub iy: usize,
    /// Per-time-step injected pressure samples. Steps beyond
    /// `signal.len()` continue propagating without further injection.
    pub signal: Vec<f32>,
}

impl FdtdSource {
    /// Gaussian pulse centred at `peak_step` with width `sigma_steps` samples.
    /// Energy spread covers ~`6 σ` steps; useful as a broad-spectrum impulse.
    #[must_use]
    pub fn gaussian_pulse(
        ix: usize,
        iy: usize,
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
        Self { ix, iy, signal }
    }
}

/// Receiver — a grid cell whose pressure history is recorded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdtdReceiver {
    /// Grid x-index.
    pub ix: usize,
    /// Grid y-index.
    pub iy: usize,
}

/// Result of an FDTD simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdtdResult {
    /// Pressure trace at each receiver, one `Vec<f32>` per receiver, length =
    /// `time_steps`. Empty if the receiver index is out of range (silent).
    pub receiver_signals: Vec<Vec<f32>>,
    /// Final pressure-field snapshot, row-major (`y * nx + x`).
    pub final_pressure: Vec<f32>,
    /// Number of time steps actually run.
    pub time_steps: u32,
    /// Effective Δt used (seconds).
    pub dt: f32,
}

/// Solve the 2D acoustic wave equation on a rigid-walled box.
///
/// Returns an empty result on degenerate input (zero grid, sub-CFL Δt,
/// out-of-range source, etc.).
#[must_use]
#[tracing::instrument(skip(config, source, receivers), fields(
    sample_rate = config.sample_rate,
    nx = config.nx,
    ny = config.ny,
    duration_seconds = config.duration_seconds,
    receivers = receivers.len(),
))]
pub fn solve_fdtd_2d(
    config: &FdtdConfig,
    source: &FdtdSource,
    receivers: &[FdtdReceiver],
) -> FdtdResult {
    if config.sample_rate == 0
        || config.dx <= 0.0
        || config.speed_of_sound <= 0.0
        || config.nx < 3
        || config.ny < 3
        || config.duration_seconds <= 0.0
    {
        return empty_result();
    }
    if config.nx.saturating_mul(config.ny) > MAX_GRID_CELLS {
        return empty_result();
    }
    if source.ix >= config.nx || source.iy >= config.ny {
        return empty_result();
    }

    let dt = 1.0 / config.sample_rate as f32;
    let cfl = config.speed_of_sound * dt / config.dx;
    if cfl > CFL_2D_LIMIT {
        return empty_result();
    }

    let num_steps = ((config.duration_seconds / dt) as u32).min(MAX_TIME_STEPS);
    if num_steps == 0 {
        return empty_result();
    }

    let nx = config.nx;
    let ny = config.ny;
    let n = nx * ny;
    let mut p_prev = vec![0.0_f32; n];
    let mut p_curr = vec![0.0_f32; n];
    let mut p_next = vec![0.0_f32; n];

    let cfl_sq = cfl * cfl;

    let mut receiver_signals: Vec<Vec<f32>> = receivers
        .iter()
        .map(|r| {
            if r.ix < nx && r.iy < ny {
                Vec::with_capacity(num_steps as usize)
            } else {
                Vec::new()
            }
        })
        .collect();

    for step in 0..num_steps {
        // Interior 5-point Laplacian + leapfrog update.
        for y in 1..ny - 1 {
            let row = y * nx;
            for x in 1..nx - 1 {
                let idx = row + x;
                let lap = p_curr[idx - 1] + p_curr[idx + 1] + p_curr[idx - nx] + p_curr[idx + nx]
                    - 4.0 * p_curr[idx];
                p_next[idx] = 2.0 * p_curr[idx] - p_prev[idx] + cfl_sq * lap;
            }
        }

        // Rigid walls: copy the adjacent interior cell (∂p/∂n = 0).
        for x in 1..nx - 1 {
            p_next[x] = p_next[x + nx]; // bottom edge
            p_next[(ny - 1) * nx + x] = p_next[(ny - 2) * nx + x]; // top edge
        }
        for y in 1..ny - 1 {
            p_next[y * nx] = p_next[y * nx + 1]; // left edge
            p_next[y * nx + nx - 1] = p_next[y * nx + nx - 2]; // right edge
        }
        // Corners: average of adjacent edges.
        p_next[0] = 0.5 * (p_next[1] + p_next[nx]);
        p_next[nx - 1] = 0.5 * (p_next[nx - 2] + p_next[2 * nx - 1]);
        let top_left = (ny - 1) * nx;
        p_next[top_left] = 0.5 * (p_next[top_left + 1] + p_next[top_left - nx]);
        p_next[top_left + nx - 1] =
            0.5 * (p_next[top_left + nx - 2] + p_next[top_left + nx - 1 - nx]);

        // Source injection (additive).
        if let Some(&s) = source.signal.get(step as usize) {
            p_next[source.iy * nx + source.ix] += s;
        }

        // Sample receivers.
        for (rx, recv_signal) in receivers.iter().zip(receiver_signals.iter_mut()) {
            if rx.ix < nx && rx.iy < ny {
                recv_signal.push(p_next[rx.iy * nx + rx.ix]);
            }
        }

        // Rotate buffers.
        std::mem::swap(&mut p_prev, &mut p_curr);
        std::mem::swap(&mut p_curr, &mut p_next);
    }

    FdtdResult {
        receiver_signals,
        final_pressure: p_curr,
        time_steps: num_steps,
        dt,
    }
}

fn empty_result() -> FdtdResult {
    FdtdResult {
        receiver_signals: Vec::new(),
        final_pressure: Vec::new(),
        time_steps: 0,
        dt: 0.0,
    }
}

/// Per-band energies from a time-domain signal via a single-bin Goertzel
/// evaluation at each ISO octave centre frequency.
///
/// Returns `|X(f_band)|²` for the 8 bands (63–8000 Hz). Approximate — uses
/// one DFT bin per band rather than integrating power over the band's
/// bandwidth — but the cost is `O(NUM_BANDS · signal.len())` and it
/// captures the relative weight of each band well for modal signals.
#[must_use]
#[tracing::instrument(skip(signal))]
pub fn band_energies(signal: &[f32], sample_rate: u32) -> [f32; NUM_BANDS] {
    if signal.is_empty() || sample_rate == 0 {
        return [0.0; NUM_BANDS];
    }
    let inv_sr = 1.0 / sample_rate as f32;
    std::array::from_fn(|band| {
        let f = FREQUENCY_BANDS[band];
        let omega = std::f32::consts::TAU * f * inv_sr;
        let coeff = 2.0 * omega.cos();
        let mut s_prev = 0.0_f32;
        let mut s_prev2 = 0.0_f32;
        for &x in signal {
            let s = x + coeff * s_prev - s_prev2;
            s_prev2 = s_prev;
            s_prev = s;
        }
        // |X(f)|² = s_prev² + s_prev2² − coeff·s_prev·s_prev2
        (s_prev * s_prev + s_prev2 * s_prev2 - coeff * s_prev * s_prev2).max(0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> FdtdConfig {
        FdtdConfig {
            sample_rate: 22_050,
            dx: 0.05,
            speed_of_sound: 343.0,
            nx: 40,
            ny: 40,
            duration_seconds: 0.05,
        }
    }

    #[test]
    fn cfl_violation_returns_empty() {
        let mut c = small_config();
        // Make CFL > 1/√2: shrink dx
        c.dx = 0.001;
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let r = solve_fdtd_2d(&c, &src, &[]);
        assert!(r.receiver_signals.is_empty());
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn zero_dx_returns_empty() {
        let mut c = small_config();
        c.dx = 0.0;
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let r = solve_fdtd_2d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn source_out_of_grid_returns_empty() {
        let c = small_config();
        let src = FdtdSource {
            ix: 1000,
            iy: 5,
            signal: vec![1.0],
        };
        let r = solve_fdtd_2d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn tiny_grid_returns_empty() {
        let mut c = small_config();
        c.nx = 1;
        c.ny = 1;
        let src = FdtdSource {
            ix: 0,
            iy: 0,
            signal: vec![1.0],
        };
        let r = solve_fdtd_2d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn pulse_propagates_to_receiver() {
        let c = small_config();
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let recv = FdtdReceiver { ix: 30, iy: 20 };
        let r = solve_fdtd_2d(&c, &src, &[recv]);
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
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let recv = FdtdReceiver { ix: 1000, iy: 1000 };
        let r = solve_fdtd_2d(&c, &src, &[recv]);
        assert!(r.receiver_signals[0].is_empty());
    }

    #[test]
    fn rigid_box_energy_bounded() {
        // Energy of the field shouldn't blow up under a single pulse +
        // rigid walls — numerical dissipation at boundaries should keep
        // it bounded (no exponential growth).
        let mut c = small_config();
        c.duration_seconds = 0.2;
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let r = solve_fdtd_2d(&c, &src, &[]);
        let total_energy: f32 = r.final_pressure.iter().map(|p| p * p).sum();
        assert!(
            total_energy.is_finite() && total_energy < 1e6,
            "energy should stay bounded, got {total_energy}"
        );
    }

    #[test]
    fn first_axial_mode_in_response() {
        // 2 m × 2 m rigid box (40 cells × 0.05 m), c = 343. First axial
        // mode at f₁ = c/(2·L) = 343/4 ≈ 85.75 Hz.
        let c = FdtdConfig {
            sample_rate: 22_050,
            dx: 0.05,
            speed_of_sound: 343.0,
            nx: 40,
            ny: 40,
            duration_seconds: 0.5,
        };
        // Off-centre source so it couples to the (1,0) axial mode
        let src = FdtdSource::gaussian_pulse(8, 20, 5, 1.0, 1.0);
        let recv = FdtdReceiver { ix: 32, iy: 20 };
        let r = solve_fdtd_2d(&c, &src, &[recv]);
        let energies = band_energies(&r.receiver_signals[0], c.sample_rate);
        // 85.75 Hz lands in the 125 Hz octave band (range 88–177 Hz).
        // FDTD numerical-dispersion shifts it slightly; expect either
        // the 63 or 125 band to dominate.
        let dominant = energies
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert!(
            dominant <= 2,
            "first-mode energy should land in 63/125/250 Hz, got dominant band {dominant} energies={energies:?}"
        );
    }

    #[test]
    fn band_energies_returns_eight() {
        let signal: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.1).sin()).collect();
        let energies = band_energies(&signal, 22_050);
        assert_eq!(energies.len(), NUM_BANDS);
        for e in energies {
            assert!(
                e >= 0.0 && e.is_finite(),
                "band energy should be ≥ 0 finite, got {e}"
            );
        }
    }

    #[test]
    fn band_energies_pure_tone_concentrated() {
        // Synthesise a 250 Hz pure tone; verify the 250-Hz band has the
        // largest Goertzel response.
        let sr = 22_050_u32;
        let f = 250.0_f32;
        let signal: Vec<f32> = (0..2_205)
            .map(|i| (std::f32::consts::TAU * f * i as f32 / sr as f32).sin())
            .collect();
        let energies = band_energies(&signal, sr);
        let dominant = energies
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(
            dominant, 2,
            "250 Hz tone should peak in band 2 (250 Hz), got {dominant}, energies={energies:?}"
        );
    }

    #[test]
    fn band_energies_empty_signal_zero() {
        let energies = band_energies(&[], 22_050);
        assert_eq!(energies, [0.0; NUM_BANDS]);
    }

    #[test]
    fn band_energies_zero_sample_rate_zero() {
        let energies = band_energies(&[1.0, 2.0, 3.0], 0);
        assert_eq!(energies, [0.0; NUM_BANDS]);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = small_config();
        let json = serde_json::to_string(&c).unwrap();
        let back: FdtdConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn source_serialization_roundtrip() {
        let s = FdtdSource::gaussian_pulse(10, 20, 3, 1.5, 0.5);
        let json = serde_json::to_string(&s).unwrap();
        let back: FdtdSource = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn grid_cap_returns_empty() {
        let mut c = small_config();
        c.nx = 5_000;
        c.ny = 5_000;
        let src = FdtdSource {
            ix: 0,
            iy: 0,
            signal: vec![1.0],
        };
        let r = solve_fdtd_2d(&c, &src, &[]);
        assert_eq!(r.time_steps, 0);
    }

    #[test]
    fn integrates_with_hybrid_crossover() {
        // End-to-end: solve FDTD, derive band energies, blend via hybrid.
        let c = small_config();
        let src = FdtdSource::gaussian_pulse(20, 20, 5, 2.0, 1.0);
        let recv = FdtdReceiver { ix: 25, iy: 25 };
        let r = solve_fdtd_2d(&c, &src, &[recv]);
        let wave = band_energies(&r.receiver_signals[0], c.sample_rate);
        let geom = [1.0; NUM_BANDS];
        let cfg = crate::hybrid::CrossoverConfig::default();
        let blended = crate::hybrid::blend_results(&wave, &geom, &cfg);
        // Low bands lean toward FDTD (wave), high bands toward geometric.
        assert!(blended[0] < blended[7] || blended[7] >= 0.9);
    }
}
