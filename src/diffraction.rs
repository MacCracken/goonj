use crate::propagation::speed_of_sound;
use hisab::Vec3;
use std::f32::consts::PI;

/// Edge diffraction attenuation (UTD-inspired approximation).
///
/// Approximates diffraction around a half-plane edge using a Fresnel-like
/// transition function. Captures the key physics: low frequencies diffract
/// freely (0 dB loss), high frequencies are shadowed, and the transition
/// depends on the shadow angle and wave number.
///
/// For full Kouyoumjian-Pathak UTD with proper wedge geometry parameters,
/// see the Tier 2 roadmap item.
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

/// Kouyoumjian-Pathak UTD wedge diffraction with geometry parameters.
///
/// Computes diffraction loss around a wedge of exterior angle `n × π` (where
/// `n = 2` is a half-plane, `n = 1.5` is a 270° exterior angle, etc.).
/// The source and receiver angles are measured from the illuminated face.
///
/// # Arguments
/// * `frequency` — sound frequency in Hz
/// * `wedge_n` — wedge parameter: exterior angle = n × π (2.0 = half-plane, 1.5 = 90° corner)
/// * `source_angle` — angle from source to edge, measured from illuminated face (radians)
/// * `receiver_angle` — angle from edge to receiver, measured from illuminated face (radians)
/// * `distance_source` — distance from source to edge (m)
/// * `distance_receiver` — distance from edge to receiver (m)
/// * `temperature_celsius` — air temperature
///
/// Returns attenuation in dB (negative = loss).
#[must_use]
pub fn utd_wedge_diffraction(
    frequency: f32,
    wedge_n: f32,
    source_angle: f32,
    receiver_angle: f32,
    distance_source: f32,
    distance_receiver: f32,
    temperature_celsius: f32,
) -> f32 {
    if frequency <= 0.0 || wedge_n <= 0.0 || distance_source <= 0.0 || distance_receiver <= 0.0 {
        return 0.0;
    }

    let c = speed_of_sound(temperature_celsius);
    let k = std::f32::consts::TAU * frequency / c;

    // UTD distance factor: L = (rs × rr) / (rs + rr)
    let l = (distance_source * distance_receiver) / (distance_source + distance_receiver);

    // Diffraction coefficient components (Kouyoumjian-Pathak, 1974)
    // D = -exp(-jπ/4) / (n × √(2πk)) × Σ cot((π±β)/2n) × F(kLa±)
    // where β = source_angle ± receiver_angle
    let inv_n = 1.0 / wedge_n;
    let beta_plus = source_angle + receiver_angle;
    let beta_minus = source_angle - receiver_angle;

    // Cotangent terms and transition function arguments
    let cot_term = |beta: f32, sign: f32| -> f32 {
        let arg = (PI + sign * beta) * inv_n * 0.5;
        let sin_val = arg.sin();
        if sin_val.abs() < 1e-6 {
            return 0.0;
        }
        let cot = arg.cos() / sin_val;

        // Transition function argument: a = 2 × k × L × cos²((2nNπ - beta) / 2)
        // Use N that minimizes |2nNπ - beta|
        let n_best = ((sign * beta) / (std::f32::consts::TAU * wedge_n)).round();
        let a_arg = std::f32::consts::TAU * wedge_n * n_best - sign * beta;
        let a = 2.0 * k * l * (a_arg * 0.5).cos().powi(2);

        // Approximate Fresnel transition function F(x)
        // F(x) ≈ √(x / (1 + x)) for x > 0 (smooth approximation)
        let f_val = if a < 0.01 {
            a.sqrt()
        } else {
            (a / (1.0 + a)).sqrt()
        };

        cot * f_val
    };

    let d_magnitude = (cot_term(beta_plus, 1.0).abs()
        + cot_term(beta_minus, 1.0).abs()
        + cot_term(beta_plus, -1.0).abs()
        + cot_term(beta_minus, -1.0).abs())
        / (wedge_n * (std::f32::consts::TAU * k).sqrt());

    // Distance spreading factor
    let spread = (l / (distance_source + distance_receiver)).sqrt();
    let total = (d_magnitude * spread).clamp(1e-10, 1.0);

    20.0 * total.log10()
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

    // --- UTD wedge diffraction tests ---

    #[test]
    fn utd_wedge_half_plane_shadow() {
        // Half-plane (n=2), source and receiver in shadow
        let loss = utd_wedge_diffraction(1000.0, 2.0, 0.5, 2.5, 5.0, 5.0, 20.0);
        assert!(
            loss < 0.0,
            "shadow region should have negative dB, got {loss}"
        );
    }

    #[test]
    fn utd_wedge_produces_negative_loss() {
        // Various geometries should all produce non-positive attenuation
        for &(sa, ra) in &[(0.3, 0.3), (0.5, 2.5), (1.0, 1.0), (PI * 0.5, PI * 1.5)] {
            let loss = utd_wedge_diffraction(1000.0, 2.0, sa, ra, 5.0, 5.0, 20.0);
            assert!(
                loss <= 0.0,
                "UTD should produce non-positive loss for sa={sa}, ra={ra}, got {loss}"
            );
        }
    }

    #[test]
    fn utd_wedge_higher_freq_more_loss() {
        let loss_1k = utd_wedge_diffraction(1000.0, 2.0, 1.0, 2.0, 5.0, 5.0, 20.0);
        let loss_4k = utd_wedge_diffraction(4000.0, 2.0, 1.0, 2.0, 5.0, 5.0, 20.0);
        assert!(
            loss_4k < loss_1k,
            "4kHz ({loss_4k}) should have more loss than 1kHz ({loss_1k})"
        );
    }

    #[test]
    fn utd_wedge_zero_frequency() {
        let loss = utd_wedge_diffraction(0.0, 2.0, 1.0, 2.0, 5.0, 5.0, 20.0);
        assert_eq!(loss, 0.0);
    }

    #[test]
    fn utd_wedge_in_valid_range() {
        let loss = utd_wedge_diffraction(500.0, 1.5, 0.8, 1.5, 3.0, 4.0, 20.0);
        assert!(
            loss <= 0.0,
            "diffraction loss should be non-positive, got {loss}"
        );
        assert!(
            loss > -100.0,
            "diffraction loss should be reasonable, got {loss}"
        );
    }

    #[test]
    fn edge_diffraction_zero_frequency() {
        let loss = edge_diffraction_loss(0.0, 1.0, 20.0);
        assert_eq!(loss, 0.0);
    }
}
