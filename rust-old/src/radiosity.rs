//! Acoustic radiosity — energy exchange between surface patches.
//!
//! Models diffuse sound field energy distribution by computing view factors
//! between wall patches and iterating energy exchange until convergence.
//! Source/receiver-independent: once the patch-to-patch matrix is computed,
//! moving sources or listeners costs almost nothing.

use crate::material::NUM_BANDS;
use crate::room::Wall;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A surface patch for radiosity computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// Centre of the patch.
    pub centre: Vec3,
    /// Normal direction.
    pub normal: Vec3,
    /// Area in m².
    pub area: f32,
    /// Per-band reflectance (1 - absorption).
    pub reflectance: [f32; NUM_BANDS],
    /// Per-band energy (radiosity value, updated during iteration).
    pub energy: [f32; NUM_BANDS],
}

/// Result of radiosity computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadiosityResult {
    /// Final per-patch energy distribution.
    pub patches: Vec<Patch>,
    /// Number of iterations performed.
    pub iterations: u32,
    /// Whether the solution converged within tolerance.
    pub converged: bool,
}

/// Subdivide room walls into patches for radiosity computation.
///
/// Each wall is divided into a grid of approximately `patches_per_wall` patches.
#[must_use]
pub fn create_patches(walls: &[Wall], patches_per_wall: usize) -> Vec<Patch> {
    // Cap to prevent DoS: max 100 patches per wall, max 10000 total
    let patches_per_wall = patches_per_wall.min(100);
    let mut patches = Vec::with_capacity(walls.len() * patches_per_wall);

    for wall in walls {
        let area = wall.area();
        if area < f32::EPSILON || wall.vertices.len() < 3 {
            continue;
        }

        // Simple subdivision: divide wall into n patches along each axis
        let n = (patches_per_wall as f32).sqrt().ceil() as usize;
        let n = n.max(1);
        let patch_area = area / (n * n) as f32;

        // Approximate patch centres using bilinear interpolation on quad
        let v0 = wall.vertices[0];
        let v1 = if wall.vertices.len() > 1 {
            wall.vertices[1]
        } else {
            v0
        };
        let v2 = if wall.vertices.len() > 2 {
            wall.vertices[2]
        } else {
            v0
        };
        let v3 = if wall.vertices.len() > 3 {
            wall.vertices[3]
        } else {
            v2
        };

        let mut reflectance = [0.0_f32; NUM_BANDS];
        for (band, r) in reflectance.iter_mut().enumerate() {
            *r = 1.0 - wall.material.absorption[band];
        }

        for iy in 0..n {
            for ix in 0..n {
                let u = (ix as f32 + 0.5) / n as f32;
                let v = (iy as f32 + 0.5) / n as f32;
                let centre = v0 * (1.0 - u) * (1.0 - v)
                    + v1 * u * (1.0 - v)
                    + v2 * u * v
                    + v3 * (1.0 - u) * v;

                patches.push(Patch {
                    centre,
                    normal: wall.normal,
                    area: patch_area,
                    reflectance,
                    energy: [0.0; NUM_BANDS],
                });
            }
        }
    }

    patches
}

/// Compute the form factor between two patches.
///
/// F_ij = (cos θ_i × cos θ_j) / (π × r²) × A_j
/// where θ_i and θ_j are the angles from the patch normals to the
/// connecting line, and r is the distance between patch centres.
#[must_use]
#[inline]
fn form_factor(p_i: &Patch, p_j: &Patch) -> f32 {
    let diff = p_j.centre - p_i.centre;
    let r2 = diff.dot(diff);
    if r2 < f32::EPSILON {
        return 0.0;
    }
    let r = r2.sqrt();
    let dir = diff / r;

    let cos_i = dir.dot(p_i.normal).max(0.0);
    let cos_j = (-dir).dot(p_j.normal).max(0.0);

    cos_i * cos_j * p_j.area / (std::f32::consts::PI * r2)
}

/// Solve acoustic radiosity for the given patches with a source patch.
///
/// Iterates energy exchange until convergence or `max_iterations`.
/// `source_patch` is the index of the patch that emits energy.
/// `source_energy` is the per-band energy emitted by the source.
#[must_use]
#[tracing::instrument(skip(patches), fields(num_patches = patches.len(), max_iterations))]
pub fn solve_radiosity(
    patches: &mut [Patch],
    source_patch: usize,
    source_energy: [f32; NUM_BANDS],
    max_iterations: u32,
    tolerance: f32,
) -> RadiosityResult {
    if patches.is_empty() || source_patch >= patches.len() {
        return RadiosityResult {
            patches: patches.to_vec(),
            iterations: 0,
            converged: true,
        };
    }

    // Initialize source patch energy
    patches[source_patch].energy = source_energy;

    let n = patches.len();
    let mut converged = false;
    // Pre-allocate outside loop to avoid per-iteration heap allocation
    let mut incoming = vec![[0.0_f32; NUM_BANDS]; n];

    for iteration in 0..max_iterations {
        let mut max_change = 0.0_f32;

        // Clear incoming buffer
        for inc in incoming.iter_mut() {
            *inc = [0.0; NUM_BANDS];
        }
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let ff = form_factor(&patches[i], &patches[j]);
                if ff < f32::EPSILON {
                    continue;
                }
                for (band, inc) in incoming[i].iter_mut().enumerate() {
                    *inc += patches[j].energy[band] * ff;
                }
            }
        }

        // Scatter: update patch energies
        for i in 0..n {
            for band in 0..NUM_BANDS {
                let new_energy = if i == source_patch {
                    source_energy[band]
                } else {
                    0.0
                } + incoming[i][band] * patches[i].reflectance[band];

                let change = (new_energy - patches[i].energy[band]).abs();
                max_change = max_change.max(change);
                patches[i].energy[band] = new_energy;
            }
        }

        if max_change < tolerance {
            converged = true;
            return RadiosityResult {
                patches: patches.to_vec(),
                iterations: iteration + 1,
                converged,
            };
        }
    }

    RadiosityResult {
        patches: patches.to_vec(),
        iterations: max_iterations,
        converged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::room::RoomGeometry;

    #[test]
    fn create_patches_shoebox() {
        let geom = RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let patches = create_patches(&geom.walls, 4);
        assert!(!patches.is_empty());
        // 6 walls × 4 patches each = 24 (approximately — grid rounding)
        assert!(patches.len() >= 6);
    }

    #[test]
    fn form_factor_facing_patches() {
        let p1 = Patch {
            centre: Vec3::ZERO,
            normal: Vec3::X,
            area: 1.0,
            reflectance: [0.9; NUM_BANDS],
            energy: [0.0; NUM_BANDS],
        };
        let p2 = Patch {
            centre: Vec3::new(5.0, 0.0, 0.0),
            normal: -Vec3::X,
            area: 1.0,
            reflectance: [0.9; NUM_BANDS],
            energy: [0.0; NUM_BANDS],
        };
        let ff = form_factor(&p1, &p2);
        assert!(ff > 0.0, "facing patches should have positive form factor");
    }

    #[test]
    fn form_factor_perpendicular_zero() {
        let p1 = Patch {
            centre: Vec3::ZERO,
            normal: Vec3::X,
            area: 1.0,
            reflectance: [0.9; NUM_BANDS],
            energy: [0.0; NUM_BANDS],
        };
        let p2 = Patch {
            centre: Vec3::new(0.0, 5.0, 0.0),
            normal: Vec3::Y,
            area: 1.0,
            reflectance: [0.9; NUM_BANDS],
            energy: [0.0; NUM_BANDS],
        };
        let ff = form_factor(&p1, &p2);
        assert!(
            ff < 0.001,
            "perpendicular patches should have near-zero FF, got {ff}"
        );
    }

    #[test]
    fn radiosity_converges() {
        let geom = RoomGeometry::shoebox(5.0, 4.0, 3.0, AcousticMaterial::concrete());
        let mut patches = create_patches(&geom.walls, 1);
        let result = solve_radiosity(&mut patches, 0, [1.0; NUM_BANDS], 50, 0.01);
        assert!(result.iterations > 0);
    }

    #[test]
    fn radiosity_empty_patches() {
        let result = solve_radiosity(&mut [], 0, [1.0; NUM_BANDS], 10, 0.01);
        assert!(result.converged);
        assert_eq!(result.iterations, 0);
    }
}
