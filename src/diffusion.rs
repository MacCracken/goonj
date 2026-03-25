//! Acoustic diffusion equation (ADE) — energy density PDE solver.
//!
//! Models sound energy density as a diffusion process (Fick's law analogy).
//! Effective for high-frequency diffuse fields in complex spaces: long rooms,
//! coupled rooms, and industrial environments where ray tracing struggles.
//!
//! Reference: Valeau et al., "On the use of a diffusion equation for
//! room-acoustic prediction," JASA 119(3), 2006.

use serde::{Deserialize, Serialize};

/// Configuration for the acoustic diffusion equation solver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffusionConfig {
    /// Grid spacing in meters.
    pub dx: f32,
    /// Time step in seconds.
    pub dt: f32,
    /// Number of grid cells along x-axis.
    pub nx: usize,
    /// Number of grid cells along y-axis.
    pub ny: usize,
    /// Speed of sound in m/s.
    pub speed_of_sound: f32,
    /// Mean free path in meters (≈ 4V/S for a room).
    pub mean_free_path: f32,
    /// Maximum simulation time in seconds.
    pub max_time: f32,
}

/// Result of diffusion equation simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffusionResult {
    /// Energy density field at the final time step (flattened: `[y * nx + x]`).
    pub energy_density: Vec<f32>,
    /// Grid dimensions (nx, ny).
    pub dimensions: [usize; 2],
    /// Time steps simulated.
    pub time_steps: u32,
    /// Grid spacing in meters.
    pub dx: f32,
}

/// Solve the 2D acoustic diffusion equation on a regular grid.
///
/// The diffusion equation: ∂w/∂t = D × ∇²w − c/(λ_mfp) × α × w + S(x,t)
/// where D = c × λ_mfp / 3 (diffusion coefficient),
/// α = absorption coefficient, λ_mfp = mean free path.
///
/// Uses explicit finite differences (FTCS scheme).
#[must_use]
#[tracing::instrument(skip(absorption_field, source_field))]
pub fn solve_diffusion_2d(
    config: &DiffusionConfig,
    absorption_field: &[f32],
    source_field: &[f32],
) -> DiffusionResult {
    let nx = config.nx.max(3);
    let ny = config.ny.max(3);
    let n = nx * ny;

    if config.dx <= 0.0 || config.dt <= 0.0 || config.mean_free_path <= 0.0 {
        return DiffusionResult {
            energy_density: vec![0.0; n],
            dimensions: [nx, ny],
            time_steps: 0,
            dx: config.dx,
        };
    }

    let c = config.speed_of_sound;
    let mfp = config.mean_free_path;
    let d_coeff = c * mfp / 3.0; // diffusion coefficient
    let dx2 = config.dx * config.dx;

    // Stability criterion: dt < dx² / (4D) for 2D explicit scheme
    let dt_max = dx2 / (4.0 * d_coeff);
    let dt = config.dt.min(dt_max * 0.9);
    let num_steps = ((config.max_time / dt) as u32).min(1_000_000);

    let mut w = vec![0.0_f32; n]; // energy density
    let mut w_new = vec![0.0_f32; n];

    // Initialize source
    for (i, &s) in source_field.iter().enumerate().take(n) {
        w[i] = s;
    }

    let ratio = d_coeff * dt / dx2;

    for _step in 0..num_steps {
        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                let idx = y * nx + x;
                let laplacian = w[idx - 1] + w[idx + 1] + w[idx - nx] + w[idx + nx] - 4.0 * w[idx];

                // Absorption term
                let alpha = if idx < absorption_field.len() {
                    absorption_field[idx]
                } else {
                    0.0
                };
                let absorption = c / mfp * alpha * w[idx];

                w_new[idx] = w[idx] + ratio * laplacian - dt * absorption;
                w_new[idx] = w_new[idx].max(0.0); // energy can't go negative
            }
        }

        std::mem::swap(&mut w, &mut w_new);
    }

    DiffusionResult {
        energy_density: w,
        dimensions: [nx, ny],
        time_steps: num_steps,
        dx: config.dx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_config() -> DiffusionConfig {
        DiffusionConfig {
            dx: 0.5,
            dt: 0.001,
            nx: 20,
            ny: 16,
            speed_of_sound: crate::propagation::speed_of_sound(20.0),
            mean_free_path: 4.0,
            max_time: 0.1,
        }
    }

    #[test]
    fn diffusion_produces_output() {
        let config = simple_config();
        let n = config.nx * config.ny;
        let absorption = vec![0.1; n];
        let mut source = vec![0.0; n];
        source[config.ny / 2 * config.nx + config.nx / 2] = 1.0; // centre source

        let result = solve_diffusion_2d(&config, &absorption, &source);
        assert_eq!(result.energy_density.len(), n);
        assert!(result.time_steps > 0);
    }

    #[test]
    fn diffusion_energy_spreads() {
        let config = simple_config();
        let n = config.nx * config.ny;
        let absorption = vec![0.0; n]; // no absorption
        let mut source = vec![0.0; n];
        let centre = config.ny / 2 * config.nx + config.nx / 2;
        source[centre] = 1.0;

        let result = solve_diffusion_2d(&config, &absorption, &source);
        // Energy should spread beyond the centre
        let neighbours = [
            centre - 1,
            centre + 1,
            centre - config.nx,
            centre + config.nx,
        ];
        let has_spread = neighbours.iter().any(|&i| result.energy_density[i] > 0.0);
        assert!(has_spread, "energy should diffuse to neighbouring cells");
    }

    #[test]
    fn diffusion_energy_non_negative() {
        let config = simple_config();
        let n = config.nx * config.ny;
        let absorption = vec![0.5; n];
        let mut source = vec![0.0; n];
        source[config.ny / 2 * config.nx + config.nx / 2] = 1.0;

        let result = solve_diffusion_2d(&config, &absorption, &source);
        for &w in &result.energy_density {
            assert!(w >= 0.0, "energy density should be non-negative");
        }
    }

    #[test]
    fn diffusion_invalid_config() {
        let config = DiffusionConfig {
            dx: 0.0,
            dt: 0.001,
            nx: 10,
            ny: 10,
            speed_of_sound: 343.0,
            mean_free_path: 4.0,
            max_time: 0.1,
        };
        let result = solve_diffusion_2d(&config, &[], &[]);
        assert_eq!(result.time_steps, 0);
    }
}
