use std::f32::consts::PI;
use crate::propagation::speed_of_sound;

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
    let freq_factor = (0.1 / wavelength).max(0.0).min(10.0);
    -6.0 * shadow_factor * freq_factor
}

/// Check if a direct line-of-sight exists between source and listener.
///
/// Returns true if the path is occluded (blocked by at least one wall).
#[must_use]
pub fn is_occluded(
    source: [f32; 3],
    listener: [f32; 3],
    walls: &[crate::room::Wall],
) -> bool {
    let direction = [
        listener[0] - source[0],
        listener[1] - source[1],
        listener[2] - source[2],
    ];
    let max_dist = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();

    if max_dist < f32::EPSILON {
        return false;
    }

    let ray = crate::ray::AcousticRay::new(source, direction);

    for wall in walls {
        if let Some(t) = crate::ray::ray_wall_intersection(&ray, wall) {
            if t > f32::EPSILON && t < max_dist - f32::EPSILON {
                return true;
            }
        }
    }

    false
}

/// Estimate diffraction path length around an edge.
///
/// Returns the extra path length (in meters) sound travels when diffracting
/// around an obstacle edge compared to the direct path.
#[must_use]
pub fn diffraction_path_extra(
    source: [f32; 3],
    edge_point: [f32; 3],
    listener: [f32; 3],
) -> f32 {
    let d_source_edge = distance(source, edge_point);
    let d_edge_listener = distance(edge_point, listener);
    let d_direct = distance(source, listener);
    (d_source_edge + d_edge_listener) - d_direct
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffraction_loss_increases_with_angle() {
        let loss_small = edge_diffraction_loss(1000.0, 0.1, 20.0);
        let loss_large = edge_diffraction_loss(1000.0, 2.0, 20.0);
        assert!(loss_large < loss_small, "deeper shadow should have more loss");
    }

    #[test]
    fn diffraction_loss_increases_with_frequency() {
        let loss_low = edge_diffraction_loss(250.0, 1.0, 20.0);
        let loss_high = edge_diffraction_loss(4000.0, 1.0, 20.0);
        assert!(loss_high < loss_low, "higher frequency should have more loss (less diffraction)");
    }

    #[test]
    fn diffraction_loss_is_negative() {
        let loss = edge_diffraction_loss(1000.0, 1.0, 20.0);
        assert!(loss <= 0.0, "loss should be negative dB");
    }

    #[test]
    fn no_occlusion_empty_room() {
        let source = [1.0, 1.0, 1.0];
        let listener = [5.0, 1.0, 1.0];
        assert!(!is_occluded(source, listener, &[]));
    }

    #[test]
    fn diffraction_path_extra_positive() {
        let source = [0.0, 0.0, 0.0];
        let edge = [5.0, 3.0, 0.0];
        let listener = [10.0, 0.0, 0.0];
        let extra = diffraction_path_extra(source, edge, listener);
        assert!(extra > 0.0, "diffraction path should be longer than direct");
    }

    #[test]
    fn diffraction_path_zero_when_collinear() {
        let source = [0.0, 0.0, 0.0];
        let edge = [5.0, 0.0, 0.0];
        let listener = [10.0, 0.0, 0.0];
        let extra = diffraction_path_extra(source, edge, listener);
        assert!(extra.abs() < 0.001, "collinear points should have ~0 extra path");
    }
}
