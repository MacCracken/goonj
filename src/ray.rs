use crate::room::Wall;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

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
        self.energy > 0.001
    }
}

/// Test if a ray intersects a planar wall (Möller–Trumbore-style for quad/tri fan).
///
/// Returns the closest intersection distance, or None.
#[must_use]
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
}
