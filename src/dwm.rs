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
//! variant with per-wall materials, per-band IIR boundary filtering, and
//! a coarse first-order dispersion correction; non-rectilinear topologies
//! remain demand-gated (see `docs/development/roadmap.md`).
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

/// Per-face wall materials for a 3D DWM box. Each field corresponds to
/// the boundary face on its named side of the grid:
///
/// - `x_neg` — left face at `x = 0`
/// - `x_pos` — right face at `x = nx − 1`
/// - `y_neg` — bottom face at `y = 0` (floor)
/// - `y_pos` — top face at `y = ny − 1` (ceiling)
/// - `z_neg` — back face at `z = 0`
/// - `z_pos` — front face at `z = nz − 1`
///
/// Each face's outgoing wave is filtered through a per-face 1-pole IIR
/// reflection filter (see `BoundaryFilter`) fitted to that material's
/// low-band (63 Hz) and high-band (8 kHz) absorption coefficients so the
/// filter's DC and Nyquist gains match the corresponding `R = √(1 − α)`
/// values; intermediate bands are interpolated by the IIR's natural
/// frequency response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallMaterials {
    /// Material at the left face (x = 0).
    pub x_neg: AcousticMaterial,
    /// Material at the right face (x = nx − 1).
    pub x_pos: AcousticMaterial,
    /// Material at the floor (y = 0).
    pub y_neg: AcousticMaterial,
    /// Material at the ceiling (y = ny − 1).
    pub y_pos: AcousticMaterial,
    /// Material at the back face (z = 0).
    pub z_neg: AcousticMaterial,
    /// Material at the front face (z = nz − 1).
    pub z_pos: AcousticMaterial,
}

impl WallMaterials {
    /// All six faces share the same material.
    #[must_use]
    pub fn uniform(material: AcousticMaterial) -> Self {
        Self {
            x_neg: material.clone(),
            x_pos: material.clone(),
            y_neg: material.clone(),
            y_pos: material.clone(),
            z_neg: material.clone(),
            z_pos: material,
        }
    }

    /// Fully rigid walls (zero absorption everywhere, `R = 1`). Useful
    /// for pristine modal analysis where any boundary loss disrupts the
    /// mode-buildup test. Constructed directly via the public fields of
    /// `AcousticMaterial` since `AcousticMaterial::new` validates
    /// against `[0, 1]` (which 0.0 satisfies trivially) but allocates a
    /// name string regardless.
    #[must_use]
    pub fn rigid() -> Self {
        let rigid_mat = AcousticMaterial {
            name: "rigid".into(),
            absorption: [0.0; NUM_BANDS],
            scattering: 0.0,
        };
        Self::uniform(rigid_mat)
    }
}

impl Default for WallMaterials {
    fn default() -> Self {
        Self::rigid()
    }
}

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
    /// Per-face wall materials. Each face is given a 1-pole IIR
    /// reflection filter fitted to that material's low-band and
    /// high-band absorption coefficients (see `BoundaryFilter`), with
    /// per-cell filter state on the face's 2D extent. Defaults to
    /// fully-rigid (zero absorption everywhere).
    pub wall_materials: WallMaterials,
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
            wall_materials: WallMaterials::rigid(),
        }
    }
}

impl DwmConfig {
    /// Set every wall to the same material. Convenience over
    /// `with_wall_materials(WallMaterials::uniform(mat.clone()))`.
    /// Consumes and returns `self` for builder-style chaining.
    #[must_use]
    pub fn with_acoustic_material(self, material: &AcousticMaterial) -> Self {
        self.with_wall_materials(WallMaterials::uniform(material.clone()))
    }

    /// Set per-face materials directly. Consumes and returns `self`.
    #[must_use]
    pub fn with_wall_materials(mut self, walls: WallMaterials) -> Self {
        self.wall_materials = walls;
        self
    }
}

/// 1-pole IIR reflection filter for a single wall face:
/// `H(z) = b0 / (1 − a1·z⁻¹)`, applied to the outgoing wave at every
/// boundary cell of that face to produce the incoming wave at the next
/// time step. Coefficients are fitted so that |H(0)| matches the
/// material's reflection at the lowest octave band (63 Hz) and |H(π)|
/// matches it at the highest band (8 kHz):
///
/// ```text
/// |H(0)|  = b0 / (1 − a1) = √(1 − α[63 Hz])  = R_low
/// |H(π)|  = b0 / (1 + a1) = √(1 − α[8 kHz]) = R_high
/// ```
///
/// Solving:
/// ```text
/// a1 = (R_low − R_high) / (R_low + R_high)
/// b0 = R_low · (1 − a1)
/// ```
///
/// `a1 > 0` ⇒ low-pass (more high-frequency absorption — typical fabric
/// or carpet). `a1 < 0` ⇒ high-pass (more low-frequency absorption —
/// typical glass or thin panels). `a1 = 0` ⇒ frequency-flat reflection.
/// Clamped to `(−0.99, 0.99)` for numerical stability.
///
/// Intermediate octave bands are interpolated by the IIR's natural
/// frequency response — not least-squares fit across all 8, but the
/// 2-parameter fit captures the dominant low-vs-high tilt that
/// AcousticMaterial coefficients encode and is robust at endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BoundaryFilter {
    b0: f32,
    a1: f32,
}

impl BoundaryFilter {
    fn from_material(material: &AcousticMaterial) -> Self {
        let alpha_low = material.absorption[0].clamp(0.0, 1.0);
        let alpha_high = material.absorption[NUM_BANDS - 1].clamp(0.0, 1.0);
        let r_low = (1.0 - alpha_low).max(0.0).sqrt();
        let r_high = (1.0 - alpha_high).max(0.0).sqrt();
        let denom = r_low + r_high;
        let a1 = if denom > f32::EPSILON {
            ((r_low - r_high) / denom).clamp(-0.99, 0.99)
        } else {
            0.0
        };
        let b0 = r_low * (1.0 - a1);
        Self { b0, a1 }
    }

    #[inline]
    fn process(&self, x: f32, prev_y: f32) -> f32 {
        self.b0 * x + self.a1 * prev_y
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

    // Per-face 1-pole IIR reflection filters fitted to each material's
    // low / high band absorption coefficients.
    let f_xneg = BoundaryFilter::from_material(&config.wall_materials.x_neg);
    let f_xpos = BoundaryFilter::from_material(&config.wall_materials.x_pos);
    let f_yneg = BoundaryFilter::from_material(&config.wall_materials.y_neg);
    let f_ypos = BoundaryFilter::from_material(&config.wall_materials.y_pos);
    let f_zneg = BoundaryFilter::from_material(&config.wall_materials.z_neg);
    let f_zpos = BoundaryFilter::from_material(&config.wall_materials.z_pos);

    // Per-cell filter state on each face, one f32 (the filter's previous
    // output `y[n−1]`). Indexed by the face's 2D coordinate.
    let mut s_xneg = vec![0.0_f32; ny * nz];
    let mut s_xpos = vec![0.0_f32; ny * nz];
    let mut s_yneg = vec![0.0_f32; nx * nz];
    let mut s_ypos = vec![0.0_f32; nx * nz];
    let mut s_zneg = vec![0.0_f32; nx * ny];
    let mut s_zpos = vec![0.0_f32; nx * ny];

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
                    // back through that face's 1-pole IIR filter, with one
                    // filter state per boundary cell.
                    let in_xpos = if x + 1 < nx {
                        out_curr[idx(x + 1, y, z, nx, ny) * K + 1] // neighbour's −x outgoing
                    } else {
                        let s_idx = z * ny + y;
                        let y_n = f_xpos.process(out_curr[base], s_xpos[s_idx]);
                        s_xpos[s_idx] = y_n;
                        y_n
                    };
                    let in_xneg = if x > 0 {
                        out_curr[idx(x - 1, y, z, nx, ny) * K]
                    } else {
                        let s_idx = z * ny + y;
                        let y_n = f_xneg.process(out_curr[base + 1], s_xneg[s_idx]);
                        s_xneg[s_idx] = y_n;
                        y_n
                    };
                    let in_ypos = if y + 1 < ny {
                        out_curr[idx(x, y + 1, z, nx, ny) * K + 3]
                    } else {
                        let s_idx = z * nx + x;
                        let y_n = f_ypos.process(out_curr[base + 2], s_ypos[s_idx]);
                        s_ypos[s_idx] = y_n;
                        y_n
                    };
                    let in_yneg = if y > 0 {
                        out_curr[idx(x, y - 1, z, nx, ny) * K + 2]
                    } else {
                        let s_idx = z * nx + x;
                        let y_n = f_yneg.process(out_curr[base + 3], s_yneg[s_idx]);
                        s_yneg[s_idx] = y_n;
                        y_n
                    };
                    let in_zpos = if z + 1 < nz {
                        out_curr[idx(x, y, z + 1, nx, ny) * K + 5]
                    } else {
                        let s_idx = y * nx + x;
                        let y_n = f_zpos.process(out_curr[base + 4], s_zpos[s_idx]);
                        s_zpos[s_idx] = y_n;
                        y_n
                    };
                    let in_zneg = if z > 0 {
                        out_curr[idx(x, y, z - 1, nx, ny) * K + 4]
                    } else {
                        let s_idx = y * nx + x;
                        let y_n = f_zneg.process(out_curr[base + 5], s_zneg[s_idx]);
                        s_zneg[s_idx] = y_n;
                        y_n
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

// ---------------------------------------------------------------------------
// Dispersion characterization and correction (Savioja-IDWM-style, post-process)
// ---------------------------------------------------------------------------
//
// The 3D rectilinear DWM has a known direction-dependent phase-velocity error:
// waves at high frequencies propagate slightly slow on axis-aligned paths, with
// the deviation growing toward the mesh frequency `f_mesh = c/(2·Δx)`. The
// helpers below let callers (a) characterize the error, and (b) apply a coarse
// 2-tap FIR post-process correction to the receiver signal that boosts the
// upper octaves by ~5% at the half-mesh point with no DC gain change.
//
// This is a *first-order* correction. A paper-faithful Savioja IDWM phase-
// equalizer would use a longer all-pass IIR — out of scope for v1.4.3 unless a
// downstream consumer needs it. The characterization functions below are
// sufficient for callers who want to design their own corrections.

/// Mesh frequency `f_mesh = c / (2·Δx)` for the given DWM configuration —
/// the upper limit of usable simulation bandwidth. Returns `0.0` if `dx` is
/// non-positive.
#[must_use]
#[inline]
pub fn mesh_frequency(config: &DwmConfig) -> f32 {
    if config.dx <= 0.0 {
        return 0.0;
    }
    config.speed_of_sound / (2.0 * config.dx)
}

/// Dispersion factor `f_sim / f_true` for axis-aligned propagation in the
/// 3D rectilinear DWM. Returns `1.0` at DC, drops below 1.0 as `f` approaches
/// the mesh frequency, and `0.0` above it. Returns `1.0` for invalid input
/// (degenerate configs, non-positive frequency).
///
/// Derivation: the on-axis dispersion relation
/// `sin(ω_sim·Δt/2) = sin(ω_true·Δt·√3/2) / √3`
/// inverts to `ω_sim = (2/Δt)·arcsin(sin(ω_true·Δt·√3/2)/√3)`. Above the
/// mesh limit the inner sine exceeds √3 and the relation has no real
/// solution.
#[must_use]
pub fn dispersion_factor(config: &DwmConfig, frequency_hz: f32) -> f32 {
    if frequency_hz <= 0.0 || config.sample_rate == 0 || config.speed_of_sound <= 0.0 {
        return 1.0;
    }
    // Above the mesh frequency, the lattice can't represent the wavenumber
    // (axis-aligned k > π/Δx) — report 0 as "out of range".
    if frequency_hz >= mesh_frequency(config) {
        return 0.0;
    }
    let dt = 1.0 / config.sample_rate as f32;
    let omega_true = std::f32::consts::TAU * frequency_hz;
    let arg = omega_true * dt * SQRT_3 / 2.0;
    let sin_arg = (arg.sin() / SQRT_3).clamp(-1.0, 1.0);
    let omega_sim = 2.0 * sin_arg.asin() / dt;
    if omega_true.abs() < f32::EPSILON {
        return 1.0;
    }
    omega_sim / omega_true
}

/// Coarse first-order dispersion-correction post-process: a 2-tap FIR
/// `y[n] = b0·x[n] + b1·x[n−1]` calibrated to keep DC gain at 1.0 and boost
/// magnitude by `target_boost` at the half-mesh frequency (`f_mesh / 2`).
/// Above the half-mesh point the boost grows monotonically, but DWM doesn't
/// resolve those frequencies cleanly anyway.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DispersionCorrection {
    /// Feedforward coefficient on `x[n]`.
    pub b0: f32,
    /// Feedforward coefficient on `x[n−1]`.
    pub b1: f32,
}

impl DispersionCorrection {
    /// Default boost target — `1.05` at the half-mesh frequency.
    pub const DEFAULT_BOOST: f32 = 1.05;

    /// Calibrate the FIR for the given `DwmConfig` so that the boost at
    /// `f_mesh / 2` matches `boost`. Use `boost = 1.0` to disable correction.
    #[must_use]
    pub fn calibrated(config: &DwmConfig, boost: f32) -> Self {
        let f_mesh = mesh_frequency(config);
        if f_mesh <= 0.0 || config.sample_rate == 0 {
            return Self::passthrough();
        }
        // Target angular frequency in normalized rad/sample.
        let omega_target = std::f32::consts::TAU * (0.5 * f_mesh) / config.sample_rate as f32;
        // Constraint 1: |H(0)|² = 1 ⇒ b0 + b1 = 1 (taking real coeffs and
        // requiring positive-DC gain).
        // Constraint 2: |H(ω_t)|² = boost² where
        //   |H(ω)|² = b0² + 2·b0·b1·cos(ω) + b1² = 1 + 2·b0·b1·(cos(ω) − 1).
        // Substituting b1 = 1 − b0 and solving for b0:
        //   b0·(1 − b0) = (boost² − 1) / (2·(cos(ω_t) − 1))
        // gives a quadratic with two real roots; pick the one closer to 1
        // (the trivial passthrough root is b0 = 1, b1 = 0).
        let cos_t = omega_target.cos();
        let denom = 2.0 * (cos_t - 1.0); // negative for ω_t in (0, π)
        if denom.abs() < f32::EPSILON {
            return Self::passthrough();
        }
        let target_sq = boost * boost;
        let rhs = (target_sq - 1.0) / denom;
        // Solve b0² − b0 + rhs = 0 ⇒ b0 = (1 ± √(1 − 4·rhs)) / 2.
        let disc = 1.0 - 4.0 * rhs;
        if disc < 0.0 {
            return Self::passthrough();
        }
        let root = disc.sqrt();
        // Larger root gives the meaningful (non-trivial) filter.
        let b0 = (1.0 + root) * 0.5;
        let b1 = 1.0 - b0;
        Self { b0, b1 }
    }

    /// Calibrate at the default `1.05` boost.
    #[must_use]
    #[inline]
    pub fn for_config(config: &DwmConfig) -> Self {
        Self::calibrated(config, Self::DEFAULT_BOOST)
    }

    /// Identity filter — leaves the signal unchanged.
    #[must_use]
    #[inline]
    pub fn passthrough() -> Self {
        Self { b0: 1.0, b1: 0.0 }
    }

    /// Apply the FIR in-place. Iterates backwards so the input sample
    /// `x[i−1]` isn't clobbered by the just-computed `y[i]`.
    pub fn apply(&self, signal: &mut [f32]) {
        if signal.is_empty() {
            return;
        }
        for i in (1..signal.len()).rev() {
            signal[i] = self.b0 * signal[i] + self.b1 * signal[i - 1];
        }
        signal[0] *= self.b0;
    }
}

impl Default for DispersionCorrection {
    fn default() -> Self {
        Self::passthrough()
    }
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
            wall_materials: WallMaterials::rigid(),
        }
    }

    fn uniform_alpha(alpha: f32) -> WallMaterials {
        let mat = AcousticMaterial {
            name: format!("alpha={alpha}"),
            absorption: [alpha; NUM_BANDS],
            scattering: 0.0,
        };
        WallMaterials::uniform(mat)
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
            wall_materials: WallMaterials::rigid(),
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
    fn default_uses_rigid_walls() {
        let c = DwmConfig::default();
        assert_eq!(c.wall_materials, WallMaterials::rigid());
    }

    #[test]
    fn wall_materials_rigid_has_zero_absorption() {
        let walls = WallMaterials::rigid();
        for face in [
            &walls.x_neg,
            &walls.x_pos,
            &walls.y_neg,
            &walls.y_pos,
            &walls.z_neg,
            &walls.z_pos,
        ] {
            assert!(face.absorption.iter().all(|&a| a == 0.0));
        }
    }

    #[test]
    fn wall_materials_uniform_clones_to_all_faces() {
        let walls = WallMaterials::uniform(AcousticMaterial::carpet());
        let avg = AcousticMaterial::carpet().average_absorption();
        for face in [
            &walls.x_neg,
            &walls.x_pos,
            &walls.y_neg,
            &walls.y_pos,
            &walls.z_neg,
            &walls.z_pos,
        ] {
            assert!((face.average_absorption() - avg).abs() < 1e-6);
        }
    }

    #[test]
    fn absorption_decreases_field_energy() {
        // Run the same source with rigid vs. half-absorbing walls; the
        // absorbing case should retain less field energy at simulation end.
        let mut c = small_config();
        c.duration_seconds = 0.1;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);

        c.wall_materials = WallMaterials::rigid();
        let rigid = solve_dwm_3d(&c, &src, &[]);
        let energy_rigid: f32 = rigid.final_pressure.iter().map(|p| p * p).sum();

        c.wall_materials = uniform_alpha(0.5);
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
        c.wall_materials = uniform_alpha(1.0);
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

        c.wall_materials = WallMaterials::rigid();
        let rigid = solve_dwm_3d(&c, &src, std::slice::from_ref(&recv));
        c.wall_materials = uniform_alpha(0.5);
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
    fn with_acoustic_material_uniform_walls_match_average_absorption() {
        let carpet = AcousticMaterial::carpet();
        let c = DwmConfig::default().with_acoustic_material(&carpet);
        let avg = carpet.average_absorption();
        for face in [
            &c.wall_materials.x_neg,
            &c.wall_materials.x_pos,
            &c.wall_materials.y_neg,
            &c.wall_materials.y_pos,
            &c.wall_materials.z_neg,
            &c.wall_materials.z_pos,
        ] {
            assert!((face.average_absorption() - avg).abs() < 1e-6);
        }
    }

    #[test]
    fn out_of_range_absorption_clamped_silently() {
        // Direct field-construct an AcousticMaterial with absorption > 1
        // (bypasses AcousticMaterial::new validation). Boundary
        // computation should clamp to α = 1 (fully absorbing) rather
        // than produce NaN or panic.
        let bogus = AcousticMaterial {
            name: "bogus".into(),
            absorption: [5.0; NUM_BANDS],
            scattering: 0.0,
        };
        let mut c = small_config();
        c.duration_seconds = 0.02;
        c.wall_materials = WallMaterials::uniform(bogus);
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 1.0, 1.0);
        let r = solve_dwm_3d(&c, &src, &[]);
        assert!(r.time_steps > 0);
        for &p in &r.final_pressure {
            assert!(p.is_finite(), "pressure should stay finite under clamped α");
        }
    }

    #[test]
    fn asymmetric_walls_drain_more_than_all_concrete() {
        // All-concrete vs. concrete + carpet floor: the carpet floor
        // should pull more energy out of the box. Tests per-face
        // material assignment routes through the solver correctly.
        let mut c = small_config();
        c.duration_seconds = 0.05;
        let src = DwmSource::gaussian_pulse(15, 12, 10, 5, 2.0, 1.0);

        c.wall_materials = WallMaterials::uniform(AcousticMaterial::concrete());
        let all_concrete = solve_dwm_3d(&c, &src, &[]);
        let e_all_concrete: f32 = all_concrete.final_pressure.iter().map(|p| p * p).sum();

        c.wall_materials = WallMaterials {
            y_neg: AcousticMaterial::carpet(), // floor only
            ..WallMaterials::uniform(AcousticMaterial::concrete())
        };
        let carpet_floor = solve_dwm_3d(&c, &src, &[]);
        let e_carpet_floor: f32 = carpet_floor.final_pressure.iter().map(|p| p * p).sum();

        assert!(
            e_carpet_floor < e_all_concrete,
            "carpet floor + concrete elsewhere should retain less energy than all concrete: \
             carpet_floor={e_carpet_floor}, all_concrete={e_all_concrete}"
        );
    }

    /// Magnitude response of a 1-pole IIR `H(z) = b0 / (1 − a1·z⁻¹)` at
    /// normalised frequency `omega = 2π·f/fs`. Internal test helper.
    fn iir_magnitude(filter: &BoundaryFilter, omega: f32) -> f32 {
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let denom = ((1.0 - filter.a1 * cos_w).powi(2) + (filter.a1 * sin_w).powi(2)).sqrt();
        filter.b0.abs() / denom.max(f32::EPSILON)
    }

    #[test]
    fn boundary_filter_rigid_is_identity() {
        let rigid = AcousticMaterial {
            name: "rigid".into(),
            absorption: [0.0; NUM_BANDS],
            scattering: 0.0,
        };
        let f = BoundaryFilter::from_material(&rigid);
        // α = 0 ⇒ R_low = R_high = 1, b0 = 1, a1 = 0 ⇒ y[n] = x[n].
        assert!((f.b0 - 1.0).abs() < 1e-6);
        assert!(f.a1.abs() < 1e-6);
        assert!((f.process(0.7, 999.0) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn boundary_filter_fully_absorbing_zeros_signal() {
        let absorbing = AcousticMaterial {
            name: "absorbing".into(),
            absorption: [1.0; NUM_BANDS],
            scattering: 0.0,
        };
        let f = BoundaryFilter::from_material(&absorbing);
        // α = 1 ⇒ R_low = R_high = 0 ⇒ b0 = 0, a1 = 0.
        assert!(f.b0.abs() < 1e-6);
        assert!(f.a1.abs() < 1e-6);
        assert!(f.process(1.0, 0.0).abs() < 1e-6);
    }

    #[test]
    fn boundary_filter_dc_matches_low_band_reflection() {
        let mat = AcousticMaterial::carpet();
        let f = BoundaryFilter::from_material(&mat);
        let r_low = (1.0 - mat.absorption[0]).sqrt();
        // |H(0)| = b0 / (1 − a1)
        let h_dc = iir_magnitude(&f, 0.0);
        assert!(
            (h_dc - r_low).abs() < 1e-4,
            "|H(0)|={h_dc} should match R_low={r_low}"
        );
    }

    #[test]
    fn boundary_filter_nyquist_matches_high_band_reflection() {
        let mat = AcousticMaterial::carpet();
        let f = BoundaryFilter::from_material(&mat);
        let r_high = (1.0 - mat.absorption[NUM_BANDS - 1]).sqrt();
        // |H(π)| = b0 / (1 + a1)
        let h_ny = iir_magnitude(&f, std::f32::consts::PI);
        assert!(
            (h_ny - r_high).abs() < 1e-4,
            "|H(π)|={h_ny} should match R_high={r_high}"
        );
    }

    #[test]
    fn boundary_filter_carpet_attenuates_high_freq_more() {
        // Carpet: low α at 63 Hz, high α at 8 kHz ⇒ low-pass behavior
        // (a1 > 0). |H| at low freq should exceed |H| at high freq.
        let f = BoundaryFilter::from_material(&AcousticMaterial::carpet());
        let h_low = iir_magnitude(&f, 0.05); // ~low frequency
        let h_high = iir_magnitude(&f, std::f32::consts::PI - 0.05);
        assert!(
            h_low > h_high,
            "carpet should reflect low freq more than high; |H_low|={h_low}, |H_high|={h_high}"
        );
        assert!(
            f.a1 > 0.0,
            "carpet IIR pole should be positive (low-pass), got {}",
            f.a1
        );
    }

    #[test]
    fn boundary_filter_glass_attenuates_low_freq_more() {
        // Glass: high α at 63 Hz, low α at 8 kHz ⇒ high-pass behavior
        // (a1 < 0). |H| at high freq should exceed |H| at low freq.
        let f = BoundaryFilter::from_material(&AcousticMaterial::glass());
        let h_low = iir_magnitude(&f, 0.05);
        let h_high = iir_magnitude(&f, std::f32::consts::PI - 0.05);
        assert!(
            h_high > h_low,
            "glass should reflect high freq more than low; |H_low|={h_low}, |H_high|={h_high}"
        );
        assert!(
            f.a1 < 0.0,
            "glass IIR pole should be negative (high-pass), got {}",
            f.a1
        );
    }

    #[test]
    fn boundary_filter_pole_within_stable_range() {
        for mat in [
            AcousticMaterial::concrete(),
            AcousticMaterial::carpet(),
            AcousticMaterial::glass(),
            AcousticMaterial::wood(),
            AcousticMaterial::curtain(),
            AcousticMaterial::drywall(),
        ] {
            let f = BoundaryFilter::from_material(&mat);
            assert!(
                f.a1.abs() <= 0.99,
                "pole for {} = {} outside [-0.99, 0.99]",
                mat.name,
                f.a1
            );
        }
    }

    #[test]
    fn wall_materials_serialization_roundtrip() {
        let walls = WallMaterials::uniform(AcousticMaterial::carpet());
        let json = serde_json::to_string(&walls).unwrap();
        let back: WallMaterials = serde_json::from_str(&json).unwrap();
        assert_eq!(walls, back);
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

    // --- Dispersion correction (v1.4.3) ---

    #[test]
    fn mesh_frequency_at_standard_params() {
        // c=343, dx ≈ 0.02694 ⇒ f_mesh ≈ 6364 Hz
        let c = small_config();
        let f = mesh_frequency(&c);
        assert!(
            (f - 6364.0).abs() < 50.0,
            "mesh frequency should be ~6364 Hz at standard params, got {f}"
        );
    }

    #[test]
    fn mesh_frequency_zero_dx_returns_zero() {
        let mut c = small_config();
        c.dx = 0.0;
        assert_eq!(mesh_frequency(&c), 0.0);
    }

    #[test]
    fn dispersion_factor_at_dc_is_one() {
        let c = small_config();
        let factor = dispersion_factor(&c, 0.0);
        assert!((factor - 1.0).abs() < 1e-6);
    }

    #[test]
    fn dispersion_factor_decreases_with_frequency() {
        let c = small_config();
        let f_mesh = mesh_frequency(&c);
        let mid = dispersion_factor(&c, 0.25 * f_mesh);
        let near = dispersion_factor(&c, 0.75 * f_mesh);
        assert!(
            mid > near,
            "dispersion factor should decrease toward mesh frequency: \
             quarter-mesh={mid}, three-quarter-mesh={near}"
        );
        assert!(
            mid < 1.0,
            "should be slightly below 1.0 at f > 0, got {mid}"
        );
        assert!(
            mid > 0.9,
            "shouldn't drop below 90% at quarter-mesh, got {mid}"
        );
    }

    #[test]
    fn dispersion_factor_above_mesh_is_zero() {
        let c = small_config();
        let f_mesh = mesh_frequency(&c);
        // Just above the mesh frequency the dispersion relation has no
        // real solution — we report 0 to signal "out of range".
        assert_eq!(dispersion_factor(&c, 1.5 * f_mesh), 0.0);
    }

    #[test]
    fn dispersion_correction_passthrough_is_identity() {
        let dc = DispersionCorrection::passthrough();
        let mut sig = vec![1.0_f32, 2.0, 3.0, 4.0];
        let original = sig.clone();
        dc.apply(&mut sig);
        for (a, b) in sig.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn dispersion_correction_dc_gain_is_unity() {
        let c = small_config();
        let dc = DispersionCorrection::for_config(&c);
        // |H(0)| = b0 + b1 should be 1.0
        assert!((dc.b0 + dc.b1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dispersion_correction_boosts_high_freq() {
        let c = small_config();
        let dc = DispersionCorrection::calibrated(&c, 1.10);
        // Generate a tone at f_mesh/2 and verify amplitude grows.
        let f_mesh = mesh_frequency(&c);
        let f_target = 0.5 * f_mesh;
        let sample_rate = c.sample_rate as f32;
        let n = (sample_rate * 0.5) as usize; // 0.5 s buffer
        let mut sig: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * f_target * i as f32 / sample_rate).sin())
            .collect();
        let energy_before: f32 = sig.iter().map(|s| s * s).sum();
        dc.apply(&mut sig);
        let energy_after: f32 = sig.iter().map(|s| s * s).sum();
        let ratio = (energy_after / energy_before).sqrt();
        assert!(
            (ratio - 1.10).abs() < 0.05,
            "amplitude ratio at f_mesh/2 should match boost (1.10), got {ratio}"
        );
    }

    #[test]
    fn dispersion_correction_preserves_dc_signal() {
        let c = small_config();
        let dc = DispersionCorrection::for_config(&c);
        let mut sig = vec![5.0_f32; 100];
        dc.apply(&mut sig);
        // After the first sample (which has implicit x[-1]=0), the DC
        // response is steady-state |H(0)| = 1.0.
        for &s in sig.iter().skip(1) {
            assert!(
                (s - 5.0).abs() < 0.01,
                "DC signal should pass through unchanged after warm-up, got {s}"
            );
        }
    }

    #[test]
    fn dispersion_correction_empty_signal_no_panic() {
        let dc = DispersionCorrection::for_config(&small_config());
        let mut sig: Vec<f32> = vec![];
        dc.apply(&mut sig); // should not panic
    }

    #[test]
    fn dispersion_correction_single_sample() {
        let dc = DispersionCorrection { b0: 1.2, b1: -0.2 };
        let mut sig = vec![3.0_f32];
        dc.apply(&mut sig);
        // Single sample: y[0] = b0·x[0] (x[-1] = 0 implicit)
        assert!((sig[0] - 3.6).abs() < 1e-6);
    }

    #[test]
    fn dispersion_correction_serialization_roundtrip() {
        let dc = DispersionCorrection::calibrated(&small_config(), 1.08);
        let json = serde_json::to_string(&dc).unwrap();
        let back: DispersionCorrection = serde_json::from_str(&json).unwrap();
        assert_eq!(dc, back);
    }
}
