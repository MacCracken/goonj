//! Beam tracing — volumetric sound propagation without sampling artifacts.
//!
//! A beam is a pyramidal frustum originating at a source. When a beam
//! intersects a wall, it is clipped to the wall boundary and a reflected
//! child beam is spawned. Unlike ray tracing, beam tracing finds *all*
//! specular reflection paths within the beam's solid angle, eliminating
//! the aliasing inherent in discrete ray sampling.
//!
//! Reference: Funkhouser et al., "A Beam Tracing Method for Interactive
//! Architectural Acoustics," JASA 115(2), 2004.

use crate::material::NUM_BANDS;
use crate::room::Wall;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// An acoustic beam — a pyramidal frustum representing a bundle of ray paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticBeam {
    /// Apex of the beam (source or image source position).
    pub apex: Vec3,
    /// Beam axis direction (normalized, centre of the frustum).
    pub direction: Vec3,
    /// Half-angle of the beam cone in radians.
    pub half_angle: f32,
    /// Per-band energy carried by this beam.
    pub energy: [f32; NUM_BANDS],
    /// Total path length from the original source.
    pub path_length: f32,
    /// Number of reflections this beam has undergone.
    pub order: u32,
}

/// A beam-wall intersection result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeamWallHit {
    /// Centre point of the beam footprint on the wall.
    pub centre: Vec3,
    /// Distance from beam apex to wall along beam axis.
    pub distance: f32,
    /// Index of the wall that was hit.
    pub wall_index: usize,
    /// Fraction of beam solid angle intercepted by this wall (0.0–1.0).
    pub coverage: f32,
}

/// Result of tracing a beam through a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeamPath {
    /// Sequence of wall hits along this beam's propagation path.
    pub hits: Vec<BeamWallHit>,
    /// Per-band energy at the end of the path.
    pub final_energy: [f32; NUM_BANDS],
    /// Total path length.
    pub total_distance: f32,
    /// Reflection order.
    pub order: u32,
}

/// Trace a beam through room geometry, recording wall intersections.
///
/// The beam is intersected with each wall. At the nearest wall, the beam
/// is reflected and tracing continues up to `max_order` reflections.
/// The beam's energy is reduced per-band at each reflection.
#[must_use]
#[tracing::instrument(skip(walls), fields(wall_count = walls.len(), max_order))]
pub fn trace_beam(beam: &AcousticBeam, walls: &[Wall], max_order: u32) -> BeamPath {
    let mut current = beam.clone();
    let mut hits = Vec::with_capacity(max_order.min(32) as usize);
    let mut last_wall: Option<usize> = None;

    for _ in 0..max_order {
        // Find nearest wall intersection along beam axis
        let mut nearest: Option<(f32, usize)> = None;
        for (i, wall) in walls.iter().enumerate() {
            if last_wall == Some(i) {
                continue;
            }
            if let Some(t) = beam_wall_distance(&current, wall)
                && nearest.is_none_or(|(best, _)| t < best)
            {
                nearest = Some((t, i));
            }
        }

        let Some((t, idx)) = nearest else { break };
        let wall = &walls[idx];

        // Coverage = fraction of beam solid angle intercepted by the wall.
        // If wall is larger than beam footprint → coverage ≈ 1.0
        // If wall is smaller → coverage = wall_area / beam_footprint_area
        let footprint_radius = t * current.half_angle.tan();
        let wall_area = wall.area();
        let beam_area = std::f32::consts::PI * footprint_radius * footprint_radius;
        let coverage = if beam_area > f32::EPSILON {
            (wall_area / beam_area).clamp(0.0, 1.0)
        } else {
            1.0 // infinitely narrow beam — any wall fully intercepts it
        };

        let hit_point = current.apex + current.direction * t;

        hits.push(BeamWallHit {
            centre: hit_point,
            distance: t,
            wall_index: idx,
            coverage,
        });

        // Reflect beam: specular reflection of axis direction
        let n = wall.normal;
        let d_dot_n = current.direction.dot(n);
        let reflected_dir = current.direction - 2.0 * d_dot_n * n;
        let len = reflected_dir.length();
        let reflected_dir = if len > f32::EPSILON {
            reflected_dir / len
        } else {
            current.direction
        };

        // Apply per-band absorption
        let mut new_energy = [0.0_f32; NUM_BANDS];
        for (band, new_e) in new_energy.iter_mut().enumerate() {
            *new_e = current.energy[band] * (1.0 - wall.material.absorption[band]);
        }

        // Check if beam is dead
        let max_e = new_energy.iter().copied().fold(0.0_f32, f32::max);
        if max_e < 0.001 {
            break;
        }

        current = AcousticBeam {
            apex: hit_point,
            direction: reflected_dir,
            half_angle: current.half_angle,
            energy: new_energy,
            path_length: current.path_length + t,
            order: current.order + 1,
        };
        last_wall = Some(idx);
    }

    BeamPath {
        total_distance: current.path_length,
        final_energy: current.energy,
        order: current.order,
        hits,
    }
}

/// Generate a set of beams covering the full sphere from a source.
///
/// Uses an icosahedral subdivision to create uniformly distributed beams.
/// `subdivisions` controls the resolution: 0 → 20 beams, 1 → 80, 2 → 320.
#[must_use]
pub fn generate_beam_set(source: Vec3, subdivisions: u32) -> Vec<AcousticBeam> {
    // Fibonacci sphere for uniform direction distribution
    let n = 20 * 4_u32.pow(subdivisions.min(3));
    let half_angle = std::f32::consts::PI / (n as f32).sqrt();

    crate::diffuse::fibonacci_sphere(n)
        .into_iter()
        .map(|dir| AcousticBeam {
            apex: source,
            direction: dir,
            half_angle,
            energy: [1.0; NUM_BANDS],
            path_length: 0.0,
            order: 0,
        })
        .collect()
}

/// Test if a beam's axis intersects a wall plane, returning distance.
#[must_use]
#[inline]
fn beam_wall_distance(beam: &AcousticBeam, wall: &Wall) -> Option<f32> {
    if wall.vertices.len() < 3 {
        return None;
    }

    let n = wall.normal;
    let d_dot_n = beam.direction.dot(n);
    if d_dot_n.abs() < f32::EPSILON {
        return None;
    }

    let p0 = wall.vertices[0];
    let t = (p0 - beam.apex).dot(n) / d_dot_n;
    if t < f32::EPSILON {
        return None;
    }

    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::room::RoomGeometry;

    #[test]
    fn beam_traces_through_shoebox() {
        let geom = RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let beam = AcousticBeam {
            apex: Vec3::new(5.0, 1.5, 4.0),
            direction: Vec3::Z,
            half_angle: 0.1,
            energy: [1.0; NUM_BANDS],
            path_length: 0.0,
            order: 0,
        };
        let path = trace_beam(&beam, &geom.walls, 10);
        assert!(!path.hits.is_empty(), "beam should hit walls in shoebox");
        assert!(path.total_distance > 0.0);
    }

    #[test]
    fn beam_energy_decreases() {
        let geom = RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        let beam = AcousticBeam {
            apex: Vec3::new(5.0, 1.5, 4.0),
            direction: Vec3::Z,
            half_angle: 0.1,
            energy: [1.0; NUM_BANDS],
            path_length: 0.0,
            order: 0,
        };
        let path = trace_beam(&beam, &geom.walls, 20);
        for &e in &path.final_energy {
            assert!(e < 1.0, "energy should decrease");
        }
    }

    #[test]
    fn beam_set_covers_sphere() {
        let beams = generate_beam_set(Vec3::ZERO, 0);
        assert_eq!(beams.len(), 20);
        // All directions should be unit length
        for b in &beams {
            assert!((b.direction.length() - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn beam_set_subdivision_increases_count() {
        let b0 = generate_beam_set(Vec3::ZERO, 0);
        let b1 = generate_beam_set(Vec3::ZERO, 1);
        assert!(b1.len() > b0.len());
    }

    #[test]
    fn beam_empty_scene() {
        let beam = AcousticBeam {
            apex: Vec3::ZERO,
            direction: Vec3::X,
            half_angle: 0.1,
            energy: [1.0; NUM_BANDS],
            path_length: 0.0,
            order: 0,
        };
        let path = trace_beam(&beam, &[], 10);
        assert!(path.hits.is_empty());
    }

    #[test]
    fn beam_wall_hit_coverage_in_range() {
        let geom = RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let beam = AcousticBeam {
            apex: Vec3::new(5.0, 1.5, 4.0),
            direction: Vec3::Z,
            half_angle: 0.05,
            energy: [1.0; NUM_BANDS],
            path_length: 0.0,
            order: 0,
        };
        let path = trace_beam(&beam, &geom.walls, 5);
        for hit in &path.hits {
            assert!((0.0..=1.0).contains(&hit.coverage));
        }
    }

    #[test]
    fn beam_path_serializes() {
        let path = BeamPath {
            hits: vec![],
            final_energy: [0.5; NUM_BANDS],
            total_distance: 10.0,
            order: 3,
        };
        let json = serde_json::to_string(&path).unwrap();
        let back: BeamPath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, back);
    }
}
