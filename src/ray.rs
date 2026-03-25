use crate::material::FREQUENCY_BANDS;
use crate::room::{AcceleratedRoom, Wall};
use hisab::Vec3;
use hisab::geo::{Bvh, Ray};
use serde::{Deserialize, Serialize};

/// Minimum energy threshold — a ray (or band) below this is considered dead.
const ENERGY_THRESHOLD: f32 = 0.001;

/// An acoustic ray traveling through space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticRay {
    pub origin: Vec3,
    pub direction: Vec3,
    /// Remaining energy (0.0–1.0).
    pub energy: f32,
    /// Frequency in Hz carried by this ray.
    pub frequency_hz: f32,
    /// Total distance traveled by this ray.
    pub distance_traveled: f32,
}

/// A multiband acoustic ray carrying per-frequency-band energy.
///
/// Each element of `energy` corresponds to a standard frequency band
/// (125, 250, 500, 1000, 2000, 4000 Hz).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultibandRay {
    pub origin: Vec3,
    pub direction: Vec3,
    /// Per-band remaining energy (0.0–1.0 each), indexed by [`FREQUENCY_BANDS`].
    pub energy: [f32; 6],
    /// Total distance traveled.
    pub distance_traveled: f32,
}

/// A single bounce in a traced ray path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayBounce {
    /// Point of intersection on the wall.
    pub point: Vec3,
    /// Surface normal at the intersection.
    pub normal: Vec3,
    /// Index of the wall that was hit.
    pub wall_index: usize,
    /// Distance from the previous origin (or ray source for the first bounce).
    pub distance_from_previous: f32,
    /// Per-band energy remaining after this bounce's absorption.
    pub energy_after: [f32; 6],
}

/// Complete path of a traced multiband ray through a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayPath {
    /// Ordered list of bounces.
    pub bounces: Vec<RayBounce>,
    /// Total distance traveled across all bounces.
    pub total_distance: f32,
    /// Per-band energy remaining at end of trace.
    pub final_energy: [f32; 6],
}

/// Result of a ray hitting a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub wall_index: usize,
}

impl AcousticRay {
    /// Create a new ray with full energy at a given frequency.
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3, frequency_hz: f32) -> Self {
        let len = direction.length();
        let norm = if len > f32::EPSILON {
            direction / len
        } else {
            Vec3::Z
        };
        Self {
            origin,
            direction: norm,
            energy: 1.0,
            frequency_hz,
            distance_traveled: 0.0,
        }
    }

    /// Is this ray still carrying significant energy?
    #[must_use]
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.energy > ENERGY_THRESHOLD
    }
}

impl MultibandRay {
    /// Create a new multiband ray with full energy in all bands.
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let len = direction.length();
        let norm = if len > f32::EPSILON {
            direction / len
        } else {
            Vec3::Z
        };
        Self {
            origin,
            direction: norm,
            energy: [1.0; 6],
            distance_traveled: 0.0,
        }
    }

    /// Is any frequency band still carrying significant energy?
    #[must_use]
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.energy.iter().any(|&e| e > ENERGY_THRESHOLD)
    }

    /// Maximum energy across all bands.
    #[must_use]
    #[inline]
    pub fn max_energy(&self) -> f32 {
        self.energy.iter().copied().fold(0.0_f32, f32::max)
    }

    /// The frequency bands corresponding to each energy index.
    #[must_use]
    #[inline]
    pub fn frequency_bands() -> &'static [f32; 6] {
        &FREQUENCY_BANDS
    }
}

/// Test if a ray intersects a planar wall (Möller–Trumbore-style for quad/tri fan).
///
/// Returns the closest intersection distance, or None.
#[must_use]
#[inline]
pub fn ray_wall_intersection(ray: &AcousticRay, wall: &Wall) -> Option<f32> {
    if wall.vertices.len() < 3 {
        return None;
    }

    // Ray-plane intersection
    let n = wall.normal;
    let d_dot_n = ray.direction.dot(n);

    // Ray parallel to plane
    if d_dot_n.abs() < f32::EPSILON {
        return None;
    }

    let p0 = wall.vertices[0];
    let diff = p0 - ray.origin;
    let t = diff.dot(n) / d_dot_n;

    // Intersection behind ray
    if t < f32::EPSILON {
        return None;
    }

    // Check if hit point is inside polygon (simplified: works for convex polygons)
    let hit = ray.origin + ray.direction * t;

    if point_in_convex_polygon(hit, &wall.vertices, n) {
        Some(t)
    } else {
        None
    }
}

/// Reflect a ray off a surface, reducing energy based on material absorption.
///
/// Uses the scattering coefficient to blend between specular and diffuse reflection.
/// The diffuse component is approximated by perturbing the specular direction toward
/// the surface normal.
#[must_use]
#[inline]
pub fn reflect_ray(
    ray: &AcousticRay,
    hit: &RayHit,
    absorption: f32,
    scattering: f32,
) -> AcousticRay {
    let n = hit.normal;
    let d = ray.direction;
    let d_dot_n = d.dot(n);

    // Specular reflection: r = d - 2(d·n)n
    let specular = d - 2.0 * d_dot_n * n;

    // Diffuse approximation: reflect toward normal hemisphere.
    // Use the normal itself as the diffuse direction (Lambert cosine lobe peak).
    let direction = if scattering > f32::EPSILON {
        let blended = specular * (1.0 - scattering) + n * scattering;
        let len = blended.length();
        if len > f32::EPSILON {
            blended / len
        } else {
            specular
        }
    } else {
        specular
    };

    AcousticRay {
        origin: hit.point,
        direction,
        energy: ray.energy * (1.0 - absorption),
        frequency_hz: ray.frequency_hz,
        distance_traveled: ray.distance_traveled + hit.distance,
    }
}

/// Reflect a multiband ray off a surface, applying per-band absorption from the material.
///
/// Each frequency band's energy is independently reduced by that band's absorption coefficient.
/// Scattering blends between specular and diffuse reflection direction (same as [`reflect_ray`]).
#[must_use]
#[inline]
pub fn reflect_ray_multiband(
    ray: &MultibandRay,
    hit: &RayHit,
    absorption: &[f32; 6],
    scattering: f32,
) -> MultibandRay {
    let n = hit.normal;
    let d = ray.direction;
    let d_dot_n = d.dot(n);

    // Specular reflection: r = d - 2(d·n)n
    let specular = d - 2.0 * d_dot_n * n;

    let direction = if scattering > f32::EPSILON {
        let blended = specular * (1.0 - scattering) + n * scattering;
        let len = blended.length();
        if len > f32::EPSILON {
            blended / len
        } else {
            specular
        }
    } else {
        specular
    };

    let mut energy = [0.0_f32; 6];
    for i in 0..6 {
        energy[i] = ray.energy[i] * (1.0 - absorption[i]);
    }

    MultibandRay {
        origin: hit.point,
        direction,
        energy,
        distance_traveled: ray.distance_traveled + hit.distance,
    }
}

/// Find the nearest wall intersection for a multiband ray, skipping a given wall index.
///
/// Returns `(distance, wall_index)` of the closest intersection, or `None`.
#[must_use]
#[inline]
fn find_nearest_wall(
    origin: Vec3,
    direction: Vec3,
    walls: &[Wall],
    skip_wall: Option<usize>,
) -> Option<(f32, usize)> {
    // Build a temporary AcousticRay for intersection testing
    let probe = AcousticRay {
        origin,
        direction,
        energy: 1.0,
        frequency_hz: 1000.0,
        distance_traveled: 0.0,
    };

    let mut closest: Option<(f32, usize)> = None;
    for (i, wall) in walls.iter().enumerate() {
        if skip_wall == Some(i) {
            continue;
        }
        if let Some(t) = ray_wall_intersection(&probe, wall)
            && closest.is_none_or(|(best, _)| t < best)
        {
            closest = Some((t, i));
        }
    }
    closest
}

/// Trace a multiband ray through a scene of walls, recording each bounce.
///
/// The ray is reflected with per-band absorption at each wall hit. Tracing stops when
/// all bands fall below the energy threshold or `max_bounces` is reached.
#[must_use]
#[tracing::instrument(skip(walls), fields(wall_count = walls.len(), max_bounces))]
pub fn trace_ray(ray: &MultibandRay, walls: &[Wall], max_bounces: u32) -> RayPath {
    let mut current = ray.clone();
    let mut bounces = Vec::with_capacity(max_bounces.min(64) as usize);
    let mut last_wall: Option<usize> = None;

    for _ in 0..max_bounces {
        if !current.is_alive() {
            break;
        }

        let Some((t, idx)) = find_nearest_wall(current.origin, current.direction, walls, last_wall)
        else {
            break;
        };

        let wall = &walls[idx];
        let hit = RayHit {
            point: current.origin + current.direction * t,
            normal: wall.normal,
            distance: t,
            wall_index: idx,
        };

        current = reflect_ray_multiband(
            &current,
            &hit,
            &wall.material.absorption,
            wall.material.scattering,
        );

        bounces.push(RayBounce {
            point: hit.point,
            normal: hit.normal,
            wall_index: idx,
            distance_from_previous: t,
            energy_after: current.energy,
        });

        last_wall = Some(idx);
    }

    RayPath {
        total_distance: current.distance_traveled,
        final_energy: current.energy,
        bounces,
    }
}

/// Find the nearest wall intersection using a BVH for broadphase culling.
///
/// Returns `(distance, wall_index)` of the closest intersection, or `None`.
#[must_use]
#[inline]
fn find_nearest_wall_bvh(
    origin: Vec3,
    direction: Vec3,
    walls: &[Wall],
    bvh: &Bvh,
    skip_wall: Option<usize>,
) -> Option<(f32, usize)> {
    let Ok(hisab_ray) = Ray::new(origin, direction) else {
        return None;
    };

    let candidates = bvh.query_ray(&hisab_ray);

    let probe = AcousticRay {
        origin,
        direction,
        energy: 1.0,
        frequency_hz: 1000.0,
        distance_traveled: 0.0,
    };

    let mut closest: Option<(f32, usize)> = None;
    for idx in candidates {
        if skip_wall == Some(idx) {
            continue;
        }
        if let Some(t) = ray_wall_intersection(&probe, &walls[idx])
            && closest.is_none_or(|(best, _)| t < best)
        {
            closest = Some((t, idx));
        }
    }
    closest
}

/// Trace a multiband ray through an [`AcceleratedRoom`] using BVH acceleration.
///
/// Functionally identical to [`trace_ray`] but uses BVH broadphase to skip
/// walls whose bounding boxes are not hit by the ray. Most beneficial for
/// rooms with many walls (>20).
#[must_use]
#[tracing::instrument(skip(accel_room), fields(max_bounces))]
pub fn trace_ray_bvh(
    ray: &MultibandRay,
    accel_room: &AcceleratedRoom,
    max_bounces: u32,
) -> RayPath {
    let walls = &accel_room.room.geometry.walls;
    let bvh = &accel_room.bvh;
    let mut current = ray.clone();
    let mut bounces = Vec::with_capacity(max_bounces.min(64) as usize);
    let mut last_wall: Option<usize> = None;

    for _ in 0..max_bounces {
        if !current.is_alive() {
            break;
        }

        let Some((t, idx)) =
            find_nearest_wall_bvh(current.origin, current.direction, walls, bvh, last_wall)
        else {
            break;
        };

        let wall = &walls[idx];
        let hit = RayHit {
            point: current.origin + current.direction * t,
            normal: wall.normal,
            distance: t,
            wall_index: idx,
        };

        current = reflect_ray_multiband(
            &current,
            &hit,
            &wall.material.absorption,
            wall.material.scattering,
        );

        bounces.push(RayBounce {
            point: hit.point,
            normal: hit.normal,
            wall_index: idx,
            distance_from_previous: t,
            energy_after: current.energy,
        });

        last_wall = Some(idx);
    }

    RayPath {
        total_distance: current.distance_traveled,
        final_energy: current.energy,
        bounces,
    }
}

fn point_in_convex_polygon(point: Vec3, vertices: &[Vec3], normal: Vec3) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    for i in 0..n {
        let v0 = vertices[i];
        let v1 = vertices[(i + 1) % n];
        let edge = v1 - v0;
        let to_point = point - v0;
        let c = edge.cross(to_point);
        if c.dot(normal) < -f32::EPSILON {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::room::{AcceleratedRoom, AcousticRoom, RoomGeometry};

    #[test]
    fn ray_new_normalizes_direction() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), 1000.0);
        assert!((ray.direction.x - 1.0).abs() < 0.001);
    }

    #[test]
    fn ray_starts_alive() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::X, 1000.0);
        assert!(ray.is_alive());
    }

    #[test]
    fn ray_dies_at_low_energy() {
        let mut ray = AcousticRay::new(Vec3::ZERO, Vec3::X, 1000.0);
        ray.energy = 0.0001;
        assert!(!ray.is_alive());
    }

    #[test]
    fn ray_preserves_frequency() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::X, 440.0);
        assert!((ray.frequency_hz - 440.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ray_hits_front_wall() {
        // Ray pointing in +z, wall at z=5, normal facing -z (toward ray)
        let ray = AcousticRay::new(Vec3::new(2.5, 1.5, 0.0), Vec3::Z, 1000.0);
        let wall = Wall {
            vertices: vec![
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::new(0.0, 3.0, 5.0),
                Vec3::new(5.0, 3.0, 5.0),
                Vec3::new(5.0, 0.0, 5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(0.0, 0.0, -1.0),
        };
        let t = ray_wall_intersection(&ray, &wall);
        assert!(t.is_some(), "ray should hit wall");
        assert!(
            (t.unwrap() - 5.0).abs() < 0.01,
            "distance should be ~5.0, got {:?}",
            t
        );
    }

    #[test]
    fn ray_misses_wall() {
        // Ray pointing away from wall
        let ray = AcousticRay::new(Vec3::new(2.5, 1.5, 0.0), Vec3::new(0.0, 0.0, -1.0), 1000.0);
        let wall = Wall {
            vertices: vec![
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::new(5.0, 0.0, 5.0),
                Vec3::new(5.0, 3.0, 5.0),
                Vec3::new(0.0, 3.0, 5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(ray_wall_intersection(&ray, &wall).is_none());
    }

    #[test]
    fn reflection_reduces_energy() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::Z, 1000.0);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray(&ray, &hit, 0.3, 0.0);
        assert!((reflected.energy - 0.7).abs() < 0.01);
        assert!(reflected.distance_traveled > 0.0);
    }

    #[test]
    fn specular_reflection_reverses_normal_component() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::Z, 1000.0);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray(&ray, &hit, 0.0, 0.0);
        // Should bounce back: direction [0, 0, -1]
        assert!(
            (reflected.direction.z - (-1.0)).abs() < 0.01,
            "reflected z should be -1, got {}",
            reflected.direction.z
        );
    }

    #[test]
    fn scattering_deflects_toward_normal() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::Z, 1000.0);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let specular = reflect_ray(&ray, &hit, 0.0, 0.0);
        let scattered = reflect_ray(&ray, &hit, 0.0, 0.5);
        // Both should reflect backward, but scattered direction should still be normalized
        let len = scattered.direction.length();
        assert!(
            (len - 1.0).abs() < 0.01,
            "scattered direction should be normalized, got length {len}"
        );
        // Specular and scattered should both point in -z for this head-on case
        assert!(specular.direction.z < 0.0);
        assert!(scattered.direction.z < 0.0);
    }

    #[test]
    fn reflection_preserves_frequency() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::Z, 2000.0);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray(&ray, &hit, 0.1, 0.0);
        assert!(
            (reflected.frequency_hz - 2000.0).abs() < f32::EPSILON,
            "reflection should preserve frequency"
        );
    }

    #[test]
    fn point_in_polygon_inside() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
        ];
        let normal = Vec3::new(0.0, 0.0, 1.0);
        assert!(point_in_convex_polygon(
            Vec3::new(2.5, 2.5, 0.0),
            &vertices,
            normal
        ));
    }

    #[test]
    fn point_in_polygon_outside() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
        ];
        let normal = Vec3::new(0.0, 0.0, 1.0);
        assert!(!point_in_convex_polygon(
            Vec3::new(10.0, 10.0, 0.0),
            &vertices,
            normal
        ));
    }

    // --- MultibandRay tests ---

    #[test]
    fn multiband_ray_starts_alive() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::X);
        assert!(ray.is_alive());
        assert!((ray.max_energy() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multiband_ray_normalizes_direction() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0));
        assert!((ray.direction.x - 1.0).abs() < 0.001);
    }

    #[test]
    fn multiband_ray_dies_when_all_bands_low() {
        let mut ray = MultibandRay::new(Vec3::ZERO, Vec3::X);
        ray.energy = [0.0001; 6];
        assert!(!ray.is_alive());
    }

    #[test]
    fn multiband_ray_alive_if_any_band_high() {
        let mut ray = MultibandRay::new(Vec3::ZERO, Vec3::X);
        ray.energy = [0.0001, 0.0001, 0.0001, 0.5, 0.0001, 0.0001];
        assert!(ray.is_alive());
    }

    #[test]
    fn multiband_ray_frequency_bands() {
        let bands = MultibandRay::frequency_bands();
        assert_eq!(bands.len(), 6);
        assert!((bands[0] - 125.0).abs() < f32::EPSILON);
        assert!((bands[5] - 4000.0).abs() < f32::EPSILON);
    }

    // --- reflect_ray_multiband tests ---

    #[test]
    fn multiband_reflection_applies_per_band_absorption() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        // High absorption at low freq, low at high
        let absorption = [0.8, 0.6, 0.4, 0.2, 0.1, 0.05];
        let reflected = reflect_ray_multiband(&ray, &hit, &absorption, 0.0);

        assert!((reflected.energy[0] - 0.2).abs() < 0.001);
        assert!((reflected.energy[1] - 0.4).abs() < 0.001);
        assert!((reflected.energy[2] - 0.6).abs() < 0.001);
        assert!((reflected.energy[3] - 0.8).abs() < 0.001);
        assert!((reflected.energy[4] - 0.9).abs() < 0.001);
        assert!((reflected.energy[5] - 0.95).abs() < 0.001);
    }

    #[test]
    fn multiband_reflection_tracks_distance() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray_multiband(&ray, &hit, &[0.1; 6], 0.0);
        assert!((reflected.distance_traveled - 5.0).abs() < 0.01);
    }

    #[test]
    fn multiband_reflection_reverses_direction() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray_multiband(&ray, &hit, &[0.0; 6], 0.0);
        assert!((reflected.direction.z - (-1.0)).abs() < 0.01);
    }

    // --- trace_ray tests ---

    fn concrete_shoebox() -> RoomGeometry {
        RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete())
    }

    fn carpet_shoebox() -> RoomGeometry {
        RoomGeometry::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet())
    }

    #[test]
    fn trace_ray_records_bounces_in_shoebox() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 50);

        assert!(
            path.bounces.len() > 5,
            "should bounce many times in concrete, got {}",
            path.bounces.len()
        );
    }

    #[test]
    fn trace_ray_energy_decreases_per_band() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 20);

        for &e in &path.final_energy {
            assert!(e < 1.0, "energy should decrease after bounces");
        }
    }

    #[test]
    fn trace_ray_total_distance_accumulates() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 10);

        let sum: f32 = path.bounces.iter().map(|b| b.distance_from_previous).sum();
        assert!(
            (path.total_distance - sum).abs() < 0.1,
            "total distance {} should match sum of segments {}",
            path.total_distance,
            sum
        );
    }

    #[test]
    fn trace_ray_carpet_absorbs_faster() {
        let concrete = concrete_shoebox();
        let carpet = carpet_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);

        let path_concrete = trace_ray(&ray, &concrete.walls, 50);
        let path_carpet = trace_ray(&ray, &carpet.walls, 50);

        // Carpet should kill the ray faster → fewer bounces or lower final energy
        let concrete_max = path_concrete
            .final_energy
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);
        let carpet_max = path_carpet
            .final_energy
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);
        assert!(
            carpet_max < concrete_max,
            "carpet ({carpet_max}) should absorb more than concrete ({concrete_max})"
        );
    }

    #[test]
    fn trace_ray_per_band_absorption_differs_for_carpet() {
        let carpet = carpet_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &carpet.walls, 10);

        // Carpet absorbs high frequencies more than low → energy[0] > energy[5]
        assert!(
            path.final_energy[0] > path.final_energy[5],
            "low freq energy ({}) should exceed high freq ({}) for carpet",
            path.final_energy[0],
            path.final_energy[5]
        );
    }

    #[test]
    fn trace_ray_stops_at_max_bounces() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 3);

        assert!(
            path.bounces.len() <= 3,
            "should not exceed max_bounces, got {}",
            path.bounces.len()
        );
    }

    #[test]
    fn trace_ray_empty_scene_no_bounces() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::X);
        let path = trace_ray(&ray, &[], 50);

        assert!(path.bounces.is_empty());
        assert!((path.total_distance).abs() < f32::EPSILON);
    }

    #[test]
    fn trace_ray_bounce_wall_indices_valid() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 10);

        for bounce in &path.bounces {
            assert!(
                bounce.wall_index < geom.walls.len(),
                "wall_index {} out of range",
                bounce.wall_index
            );
        }
    }

    #[test]
    fn trace_ray_monotonic_energy_decay() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 20);

        // Energy should never increase between bounces
        let mut prev = [1.0_f32; 6];
        for bounce in &path.bounces {
            for (band, &prev_e) in prev.iter().enumerate() {
                assert!(
                    bounce.energy_after[band] <= prev_e + f32::EPSILON,
                    "energy should not increase: band {band}, prev {prev_e}, now {}",
                    bounce.energy_after[band]
                );
            }
            prev = bounce.energy_after;
        }
    }

    // --- BVH acceleration tests ---

    #[test]
    fn bvh_trace_matches_linear_trace() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let accel = AcceleratedRoom::new(room.clone());
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);

        let linear = trace_ray(&ray, &room.geometry.walls, 20);
        let bvh_result = trace_ray_bvh(&ray, &accel, 20);

        assert_eq!(
            linear.bounces.len(),
            bvh_result.bounces.len(),
            "BVH and linear should produce same bounce count"
        );
        assert!(
            (linear.total_distance - bvh_result.total_distance).abs() < 0.1,
            "total distances should match"
        );
        for (a, b) in linear.bounces.iter().zip(bvh_result.bounces.iter()) {
            assert_eq!(a.wall_index, b.wall_index, "wall indices should match");
        }
    }

    #[test]
    fn bvh_trace_energy_matches_linear() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        let accel = AcceleratedRoom::new(room.clone());
        let ray = MultibandRay::new(Vec3::new(3.0, 1.0, 2.0), Vec3::new(1.0, 0.5, 0.7));

        let linear = trace_ray(&ray, &room.geometry.walls, 30);
        let bvh_result = trace_ray_bvh(&ray, &accel, 30);

        for band in 0..6 {
            assert!(
                (linear.final_energy[band] - bvh_result.final_energy[band]).abs() < 0.001,
                "band {band} energy mismatch"
            );
        }
    }

    #[test]
    fn bvh_trace_empty_room() {
        let room = AcousticRoom {
            geometry: RoomGeometry { walls: vec![] },
            temperature_celsius: 20.0,
            humidity_percent: 50.0,
        };
        let accel = AcceleratedRoom::new(room);
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::X);
        let path = trace_ray_bvh(&ray, &accel, 50);
        assert!(path.bounces.is_empty());
    }

    #[test]
    fn wall_aabb_contains_vertices() {
        let wall = Wall {
            vertices: vec![
                Vec3::new(1.0, 2.0, 3.0),
                Vec3::new(4.0, 5.0, 3.0),
                Vec3::new(4.0, 2.0, 3.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::Z,
        };
        let aabb = wall.aabb();
        for &v in &wall.vertices {
            assert!(aabb.contains(v), "AABB should contain vertex {v:?}");
        }
    }

    // --- Audit edge-case tests ---

    #[test]
    fn multiband_ray_zero_direction_falls_back_to_z() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::ZERO);
        assert!((ray.direction.z - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn acoustic_ray_zero_direction_falls_back_to_z() {
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::ZERO, 1000.0);
        assert!((ray.direction.z - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trace_ray_single_wall_room() {
        // Degenerate: only one wall, ray should bounce off it once then escape
        // Vertices wound CW from +Z so winding matches -Z normal
        let wall = Wall {
            vertices: vec![
                Vec3::new(-5.0, -5.0, 5.0),
                Vec3::new(-5.0, 5.0, 5.0),
                Vec3::new(5.0, 5.0, 5.0),
                Vec3::new(5.0, -5.0, 5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(0.0, 0.0, -1.0),
        };
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let path = trace_ray(&ray, &[wall], 50);
        assert_eq!(path.bounces.len(), 1, "should bounce once off single wall");
    }

    #[test]
    fn trace_ray_max_bounces_zero() {
        let geom = concrete_shoebox();
        let ray = MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
        let path = trace_ray(&ray, &geom.walls, 0);
        assert!(
            path.bounces.is_empty(),
            "zero max_bounces should produce no bounces"
        );
    }

    #[test]
    fn ray_wall_intersection_degenerate_wall() {
        // Wall with fewer than 3 vertices
        let wall = Wall {
            vertices: vec![Vec3::ZERO, Vec3::X],
            material: AcousticMaterial::concrete(),
            normal: Vec3::Z,
        };
        let ray = AcousticRay::new(Vec3::ZERO, Vec3::Z, 1000.0);
        assert!(ray_wall_intersection(&ray, &wall).is_none());
    }

    #[test]
    fn ray_parallel_to_wall_misses() {
        let wall = Wall {
            vertices: vec![
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::new(5.0, 0.0, 5.0),
                Vec3::new(5.0, 5.0, 5.0),
                Vec3::new(0.0, 5.0, 5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(0.0, 0.0, -1.0),
        };
        // Ray parallel to wall (in XY plane)
        let ray = AcousticRay::new(Vec3::new(0.0, 0.0, 5.0), Vec3::X, 1000.0);
        assert!(ray_wall_intersection(&ray, &wall).is_none());
    }

    #[test]
    fn reflect_ray_multiband_zero_absorption_preserves_energy() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray_multiband(&ray, &hit, &[0.0; 6], 0.0);
        for &e in &reflected.energy {
            assert!(
                (e - 1.0).abs() < f32::EPSILON,
                "zero absorption should preserve energy"
            );
        }
    }

    #[test]
    fn reflect_ray_multiband_full_absorption_kills_energy() {
        let ray = MultibandRay::new(Vec3::ZERO, Vec3::Z);
        let hit = RayHit {
            point: Vec3::new(0.0, 0.0, 5.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            distance: 5.0,
            wall_index: 0,
        };
        let reflected = reflect_ray_multiband(&ray, &hit, &[1.0; 6], 0.0);
        for &e in &reflected.energy {
            assert!(e.abs() < f32::EPSILON, "full absorption should kill energy");
        }
    }
}
