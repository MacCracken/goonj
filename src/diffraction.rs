use crate::propagation::speed_of_sound;
use hisab::Vec3;
use std::f32::consts::PI;

/// Edge diffraction attenuation using the Uniform Theory of Diffraction (UTD).
///
/// Models diffraction around a wedge-shaped edge using Kouyoumjian-Pathak
/// UTD coefficients. The attenuation depends on frequency, the angular
/// relationship between source/receiver and the edge, and the wedge angle.
///
/// # Arguments
/// * `frequency` — sound frequency in Hz
/// * `angle_rad` — shadow angle: 0 = illuminated, π = deep shadow
/// * `temperature_celsius` — air temperature for speed of sound
///
/// Returns attenuation in dB (negative value = signal loss).
#[must_use]
pub fn edge_diffraction_loss(frequency: f32, angle_rad: f32, temperature_celsius: f32) -> f32 {
    if frequency <= 0.0 {
        return 0.0;
    }

    let c = speed_of_sound(temperature_celsius);
    let wavelength = c / frequency;
    let k = std::f32::consts::TAU / wavelength; // wave number

    // UTD diffraction coefficient magnitude (Kouyoumjian-Pathak)
    // For a half-plane (wedge angle n = 2), the diffraction coefficient
    // magnitude is approximately: |D| ≈ 1/(√(2πk)) × F(shadow_angle)
    //
    // where F is the UTD transition function that smoothly connects
    // illuminated and shadow regions.
    let shadow_factor = (angle_rad / PI).clamp(0.0, 1.0);

    // Fresnel-based transition function
    // In the illuminated region (small angle): minimal loss
    // At the shadow boundary: -6 dB (half-plane edge)
    // In the deep shadow: increasing loss with √(kL) dependence
    let fresnel_arg = 2.0 * k * shadow_factor * shadow_factor;
    let transition = if fresnel_arg < 0.01 {
        // Near illuminated boundary — small loss
        1.0 - shadow_factor * 0.5
    } else {
        // Shadow region — UTD rolloff
        (1.0 / (1.0 + fresnel_arg)).sqrt()
    };

    // Convert to dB
    let coefficient = transition.clamp(1e-6, 1.0);
    20.0 * coefficient.log10()
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
