//! Vector-based scattering model for diffuse reflections.
//!
//! Replaces the simple normal-blending approach with proper cosine-weighted
//! hemisphere sampling (Lambert diffuse reflection). The scattered direction
//! is computed as `(1-s)*specular + s*random_hemisphere` using the ODEON-style
//! vector-based scattering model.

use hisab::Vec3;

/// Generate a cosine-weighted random direction on the hemisphere around `normal`.
///
/// Uses the Malley method: generate a uniform random point on a disk, then
/// project up to the hemisphere. The resulting distribution follows Lambert's
/// cosine law (probability proportional to cos θ).
///
/// `u1` and `u2` are uniform random values in [0, 1).
#[must_use]
#[inline]
pub fn cosine_hemisphere_sample(normal: Vec3, u1: f32, u2: f32) -> Vec3 {
    // Malley's method: uniform disk → hemisphere projection
    let r = u1.sqrt();
    let theta = std::f32::consts::TAU * u2;
    let x = r * theta.cos();
    let y = r * theta.sin();
    let z = (1.0 - u1).max(0.0).sqrt(); // cos(elevation)

    // Build orthonormal basis from normal
    let (tangent, bitangent) = orthonormal_basis(normal);

    let dir = tangent * x + bitangent * y + normal * z;
    let len = dir.length();
    if len > f32::EPSILON {
        dir / len
    } else {
        normal
    }
}

/// Compute the scattered reflection direction using vector-based scattering.
///
/// Blends between specular reflection and a cosine-weighted random direction
/// based on the scattering coefficient. This is the ODEON-style model:
/// `dir = (1 - s) * specular + s * random_hemisphere`
///
/// The result is normalized to unit length.
#[must_use]
#[inline]
pub fn scatter_direction(specular: Vec3, normal: Vec3, scattering: f32, u1: f32, u2: f32) -> Vec3 {
    if scattering < f32::EPSILON {
        return specular;
    }
    if scattering >= 1.0 - f32::EPSILON {
        return cosine_hemisphere_sample(normal, u1, u2);
    }

    let diffuse = cosine_hemisphere_sample(normal, u1, u2);
    let blended = specular * (1.0 - scattering) + diffuse * scattering;
    let len = blended.length();
    if len > f32::EPSILON {
        blended / len
    } else {
        specular
    }
}

/// Build an orthonormal basis from a single normal vector.
///
/// Returns (tangent, bitangent) perpendicular to the normal.
#[must_use]
#[inline]
fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    // Choose a vector not parallel to n
    let helper = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let tangent = n.cross(helper);
    let len = tangent.length();
    let tangent = if len > f32::EPSILON {
        tangent / len
    } else {
        Vec3::X
    };
    let bitangent = n.cross(tangent);
    (tangent, bitangent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_hemisphere_above_surface() {
        let normal = Vec3::Y;
        for i in 0..100 {
            let u1 = i as f32 / 100.0;
            let u2 = (i as f32 * 0.618) % 1.0;
            let dir = cosine_hemisphere_sample(normal, u1, u2);
            assert!(
                dir.dot(normal) >= -0.01,
                "sample should be above surface, dot={:.3}",
                dir.dot(normal)
            );
            assert!((dir.length() - 1.0).abs() < 0.01, "should be unit length");
        }
    }

    #[test]
    fn scatter_zero_returns_specular() {
        let specular = Vec3::new(0.5, 0.5, 0.707);
        let normal = Vec3::Y;
        let result = scatter_direction(specular, normal, 0.0, 0.5, 0.5);
        assert!(
            (result - specular).length() < 0.01,
            "zero scattering should return specular"
        );
    }

    #[test]
    fn scatter_one_ignores_specular() {
        let specular = -Vec3::Y; // pointing into surface
        let normal = Vec3::Y;
        let result = scatter_direction(specular, normal, 1.0, 0.5, 0.5);
        assert!(
            result.dot(normal) >= -0.01,
            "full scatter should produce hemisphere sample above surface"
        );
    }

    #[test]
    fn scatter_direction_normalized() {
        let specular = Vec3::new(1.0, 0.0, -1.0);
        let len = specular.length();
        let specular = specular / len;
        let normal = Vec3::Y;
        for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let dir = scatter_direction(specular, normal, s, 0.3, 0.7);
            assert!(
                (dir.length() - 1.0).abs() < 0.01,
                "scattered dir should be normalized at s={s}"
            );
        }
    }

    #[test]
    fn orthonormal_basis_perpendicular() {
        for n in [
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            Vec3::new(1.0, 1.0, 1.0).normalize(),
        ] {
            let (t, b) = orthonormal_basis(n);
            assert!(
                n.dot(t).abs() < 0.01,
                "tangent should be perpendicular to normal"
            );
            assert!(
                n.dot(b).abs() < 0.01,
                "bitangent should be perpendicular to normal"
            );
            assert!(
                t.dot(b).abs() < 0.01,
                "tangent and bitangent should be perpendicular"
            );
        }
    }
}
