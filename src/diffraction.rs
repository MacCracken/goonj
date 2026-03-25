use crate::propagation::speed_of_sound;
use hisab::Vec3;
use std::f32::consts::PI;

/// Simplified edge diffraction attenuation (based on Uniform Theory of Diffraction).
///
/// Returns attenuation in dB (negative value = signal loss).
/// Higher frequencies diffract less (more shadowing).
#[must_use]
pub fn edge_diffraction_loss(frequency: f32, angle_rad: f32, temperature_celsius: f32) -> f32 {
    let c = speed_of_sound(temperature_celsius);
    let wavelength = c / frequency;
    // Simplified: loss increases with frequency and shadow angle
    // At grazing (angle ~0), minimal loss. At deep shadow (angle ~π), maximum loss.
    let shadow_factor = (angle_rad / PI).clamp(0.0, 1.0);
    let freq_factor = (0.1 / wavelength).clamp(0.0, 10.0);
    -6.0 * shadow_factor * freq_factor
}

/// Check if a direct line-of-sight exists between source and listener.
///
/// Returns true if the path is occluded (blocked by at least one wall).
#[must_use]
#[tracing::instrument(skip(walls), fields(wall_count = walls.len()))]
pub fn is_occluded(source: Vec3, listener: Vec3, walls: &[crate::room::Wall]) -> bool {
    let direction = listener - source;
    let max_dist = direction.length();

    if max_dist < f32::EPSILON {
        return false;
    }

    let ray = crate::ray::AcousticRay::new(source, direction, 0.0);

    for wall in walls {
        if let Some(t) = crate::ray::ray_wall_intersection(&ray, wall)
            && t > f32::EPSILON
            && t < max_dist - f32::EPSILON
        {
            return true;
        }
    }

    false
}

/// Estimate diffraction path length around an edge.
///
/// Returns the extra path length (in meters) sound travels when diffracting
/// around an obstacle edge compared to the direct path.
#[must_use]
pub fn diffraction_path_extra(source: Vec3, edge_point: Vec3, listener: Vec3) -> f32 {
    let d_source_edge = source.distance(edge_point);
    let d_edge_listener = edge_point.distance(listener);
    let d_direct = source.distance(listener);
    (d_source_edge + d_edge_listener) - d_direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffraction_loss_increases_with_angle() {
        let loss_small = edge_diffraction_loss(1000.0, 0.1, 20.0);
        let loss_large = edge_diffraction_loss(1000.0, 2.0, 20.0);
        assert!(
            loss_large < loss_small,
            "deeper shadow should have more loss"
        );
    }

    #[test]
    fn diffraction_loss_increases_with_frequency() {
        let loss_low = edge_diffraction_loss(250.0, 1.0, 20.0);
        let loss_high = edge_diffraction_loss(4000.0, 1.0, 20.0);
        assert!(
            loss_high < loss_low,
            "higher frequency should have more loss (less diffraction)"
        );
    }

    #[test]
    fn diffraction_loss_is_negative() {
        let loss = edge_diffraction_loss(1000.0, 1.0, 20.0);
        assert!(loss <= 0.0, "loss should be negative dB");
    }

    #[test]
    fn no_occlusion_empty_room() {
        let source = Vec3::new(1.0, 1.0, 1.0);
        let listener = Vec3::new(5.0, 1.0, 1.0);
        assert!(!is_occluded(source, listener, &[]));
    }

    #[test]
    fn diffraction_path_extra_positive() {
        let source = Vec3::ZERO;
        let edge = Vec3::new(5.0, 3.0, 0.0);
        let listener = Vec3::new(10.0, 0.0, 0.0);
        let extra = diffraction_path_extra(source, edge, listener);
        assert!(extra > 0.0, "diffraction path should be longer than direct");
    }

    #[test]
    fn diffraction_path_zero_when_collinear() {
        let source = Vec3::ZERO;
        let edge = Vec3::new(5.0, 0.0, 0.0);
        let listener = Vec3::new(10.0, 0.0, 0.0);
        let extra = diffraction_path_extra(source, edge, listener);
        assert!(
            extra.abs() < 0.001,
            "collinear points should have ~0 extra path"
        );
    }

    #[test]
    fn is_occluded_collocated_source_listener() {
        let wall = crate::room::Wall {
            vertices: vec![
                Vec3::new(5.0, -5.0, 5.0),
                Vec3::new(5.0, 5.0, 5.0),
                Vec3::new(5.0, 5.0, -5.0),
                Vec3::new(5.0, -5.0, -5.0),
            ],
            material: crate::material::AcousticMaterial::concrete(),
            normal: Vec3::new(-1.0, 0.0, 0.0),
        };
        // Same position → max_dist ≈ 0 → not occluded
        assert!(!is_occluded(Vec3::ZERO, Vec3::ZERO, &[wall]));
    }
}
