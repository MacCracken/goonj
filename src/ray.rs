use serde::{Deserialize, Serialize};
use crate::room::Wall;

/// An acoustic ray traveling through space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticRay {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    /// Remaining energy (0.0–1.0).
    pub energy: f32,
    /// Total distance traveled by this ray.
    pub distance_traveled: f32,
}

/// Result of a ray hitting a surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RayHit {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub distance: f32,
    pub wall_index: usize,
}

impl AcousticRay {
    /// Create a new ray with full energy.
    #[must_use]
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Self {
        let len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
        let norm = if len > f32::EPSILON {
            [direction[0] / len, direction[1] / len, direction[2] / len]
        } else {
            [0.0, 0.0, 1.0]
        };
        Self { origin, direction: norm, energy: 1.0, distance_traveled: 0.0 }
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
    let d_dot_n = dot(ray.direction, n);

    // Ray parallel to plane
    if d_dot_n.abs() < f32::EPSILON {
        return None;
    }

    let p0 = wall.vertices[0];
    let diff = [p0[0] - ray.origin[0], p0[1] - ray.origin[1], p0[2] - ray.origin[2]];
    let t = dot(diff, n) / d_dot_n;

    // Intersection behind ray
    if t < f32::EPSILON {
        return None;
    }

    // Check if hit point is inside polygon (simplified: works for convex polygons)
    let hit = [
        ray.origin[0] + ray.direction[0] * t,
        ray.origin[1] + ray.direction[1] * t,
        ray.origin[2] + ray.direction[2] * t,
    ];

    if point_in_convex_polygon(&hit, &wall.vertices, &n) {
        Some(t)
    } else {
        None
    }
}

/// Reflect a ray off a surface, reducing energy based on material absorption.
#[must_use]
pub fn reflect_ray(ray: &AcousticRay, hit: &RayHit, absorption: f32) -> AcousticRay {
    let n = hit.normal;
    let d = ray.direction;
    let d_dot_n = dot(d, n);

    // Specular reflection: r = d - 2(d·n)n
    let reflected = [
        d[0] - 2.0 * d_dot_n * n[0],
        d[1] - 2.0 * d_dot_n * n[1],
        d[2] - 2.0 * d_dot_n * n[2],
    ];

    AcousticRay {
        origin: hit.point,
        direction: reflected,
        energy: ray.energy * (1.0 - absorption),
        distance_traveled: ray.distance_traveled + hit.distance,
    }
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn point_in_convex_polygon(point: &[f32; 3], vertices: &[[f32; 3]], normal: &[f32; 3]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    for i in 0..n {
        let v0 = vertices[i];
        let v1 = vertices[(i + 1) % n];
        let edge = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let to_point = [point[0] - v0[0], point[1] - v0[1], point[2] - v0[2]];
        let c = cross(edge, to_point);
        if dot(c, *normal) < -f32::EPSILON {
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
        let ray = AcousticRay::new([0.0; 3], [3.0, 0.0, 0.0]);
        assert!((ray.direction[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn ray_starts_alive() {
        let ray = AcousticRay::new([0.0; 3], [1.0, 0.0, 0.0]);
        assert!(ray.is_alive());
    }

    #[test]
    fn ray_dies_at_low_energy() {
        let mut ray = AcousticRay::new([0.0; 3], [1.0, 0.0, 0.0]);
        ray.energy = 0.0001;
        assert!(!ray.is_alive());
    }

    #[test]
    fn ray_hits_front_wall() {
        // Ray pointing in +z, wall at z=5, normal facing -z (toward ray)
        let ray = AcousticRay::new([2.5, 1.5, 0.0], [0.0, 0.0, 1.0]);
        let wall = Wall {
            vertices: vec![[0.0, 0.0, 5.0], [0.0, 3.0, 5.0], [5.0, 3.0, 5.0], [5.0, 0.0, 5.0]],
            material: AcousticMaterial::concrete(),
            normal: [0.0, 0.0, -1.0],
        };
        let t = ray_wall_intersection(&ray, &wall);
        assert!(t.is_some(), "ray should hit wall");
        assert!((t.unwrap() - 5.0).abs() < 0.01, "distance should be ~5.0, got {:?}", t);
    }

    #[test]
    fn ray_misses_wall() {
        // Ray pointing away from wall
        let ray = AcousticRay::new([2.5, 1.5, 0.0], [0.0, 0.0, -1.0]);
        let wall = Wall {
            vertices: vec![[0.0, 0.0, 5.0], [5.0, 0.0, 5.0], [5.0, 3.0, 5.0], [0.0, 3.0, 5.0]],
            material: AcousticMaterial::concrete(),
            normal: [0.0, 0.0, -1.0],
        };
        assert!(ray_wall_intersection(&ray, &wall).is_none());
    }

    #[test]
    fn reflection_reduces_energy() {
        let ray = AcousticRay::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let hit = RayHit { point: [0.0, 0.0, 5.0], normal: [0.0, 0.0, -1.0], distance: 5.0, wall_index: 0 };
        let reflected = reflect_ray(&ray, &hit, 0.3);
        assert!((reflected.energy - 0.7).abs() < 0.01);
        assert!(reflected.distance_traveled > 0.0);
    }

    #[test]
    fn specular_reflection_reverses_normal_component() {
        let ray = AcousticRay::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let hit = RayHit { point: [0.0, 0.0, 5.0], normal: [0.0, 0.0, -1.0], distance: 5.0, wall_index: 0 };
        let reflected = reflect_ray(&ray, &hit, 0.0);
        // Should bounce back: direction [0, 0, -1]
        assert!((reflected.direction[2] - (-1.0)).abs() < 0.01, "reflected z should be -1, got {}", reflected.direction[2]);
    }
}
