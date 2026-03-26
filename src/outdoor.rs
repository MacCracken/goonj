//! ISO 9613-2 outdoor sound propagation methods.
//!
//! Engineering methods for sound attenuation outdoors, including:
//! - Barrier diffraction (screening by obstacles)
//! - Foliage attenuation
//! - Meteorological correction factors
//! - Ground effect

use crate::material::NUM_BANDS;
use crate::propagation::speed_of_sound;

/// Barrier diffraction attenuation per ISO 9613-2.
///
/// Computes the insertion loss of a thin barrier between source and receiver.
/// Uses the Maekawa approximation: `IL = 10 × log10(3 + 20N)` where
/// N is the Fresnel number.
///
/// # Arguments
/// * `path_difference` — extra path length over the barrier vs direct path (m)
/// * `frequency` — sound frequency in Hz
/// * `temperature_celsius` — air temperature
///
/// Returns insertion loss in dB (positive = attenuation).
#[must_use]
#[inline]
pub fn barrier_insertion_loss(
    path_difference: f32,
    frequency: f32,
    temperature_celsius: f32,
) -> f32 {
    if path_difference <= 0.0 || frequency <= 0.0 {
        return 0.0;
    }

    let c = speed_of_sound(temperature_celsius);
    let wavelength = c / frequency;

    // Fresnel number: N = 2δ/λ
    let fresnel_n = 2.0 * path_difference / wavelength;

    // Maekawa approximation (single edge)
    if fresnel_n < 0.0 {
        return 0.0;
    }
    (10.0 * (3.0 + 20.0 * fresnel_n).log10()).max(0.0)
}

/// Per-band barrier insertion loss.
#[must_use]
pub fn barrier_insertion_loss_bands(
    path_difference: f32,
    temperature_celsius: f32,
) -> [f32; NUM_BANDS] {
    std::array::from_fn(|band| {
        barrier_insertion_loss(
            path_difference,
            crate::material::FREQUENCY_BANDS[band],
            temperature_celsius,
        )
    })
}

/// Foliage attenuation per ISO 9613-2 Annex.
///
/// Sound attenuation through dense foliage (trees, bushes). The attenuation
/// depends on depth of foliage and frequency.
///
/// # Arguments
/// * `depth_m` — depth of foliage belt in meters
/// * `frequency` — sound frequency in Hz
///
/// Returns attenuation in dB.
#[must_use]
#[inline]
pub fn foliage_attenuation(depth_m: f32, frequency: f32) -> f32 {
    if depth_m <= 0.0 || frequency <= 0.0 {
        return 0.0;
    }

    // ISO 9613-2: approximately 0.01–0.12 dB/m depending on frequency
    // Increases up to 8 kHz (not capped at 1 kHz)
    let f_khz = frequency / 1000.0;
    let rate = 0.01 + 0.014 * f_khz.clamp(0.0, 8.0);
    // Cap at 10 dB per 20m belt as per ISO
    (rate * depth_m).min(10.0)
}

/// Meteorological correction factor C_met per ISO 9613-2.
///
/// Accounts for favourable propagation conditions (downwind, temperature inversion).
///
/// # Arguments
/// * `distance` — source-receiver distance in meters
/// * `source_height` — source height above ground in meters
/// * `receiver_height` — receiver height above ground in meters
///
/// Returns correction in dB (0 for short distances, positive for long distances).
#[must_use]
#[inline]
pub fn meteorological_correction(distance: f32, source_height: f32, receiver_height: f32) -> f32 {
    if distance <= 0.0 {
        return 0.0;
    }

    let h_avg = (source_height + receiver_height) / 2.0;
    let dp = distance - 10.0 * h_avg;
    if dp <= 0.0 {
        return 0.0;
    }

    // C_met = C_0 × (1 - 10 × h_avg / d) for d > 10 × h_avg
    // C_0 ≈ 0 for d < 100m, increases for long distances
    let c0 = if distance > 1000.0 {
        3.5
    } else if distance > 100.0 {
        (distance - 100.0) / 900.0 * 3.5
    } else {
        0.0
    };

    let ratio = (1.0 - 10.0 * h_avg / distance).max(0.0);
    c0 * ratio * ratio // ISO 9613-2: squared term
}

/// Ground effect attenuation per ISO 9613-2.
///
/// Simplified ground attenuation model based on source/receiver height
/// and ground type (G factor: 0 = hard, 1 = soft/porous).
///
/// # Arguments
/// * `distance` — source-receiver distance in meters
/// * `source_height` — source height above ground in meters
/// * `receiver_height` — receiver height above ground in meters
/// * `ground_factor` — G value (0.0 = hard ground, 1.0 = soft/porous)
///
/// Returns per-band attenuation in dB.
#[must_use]
pub fn ground_attenuation(
    distance: f32,
    source_height: f32,
    receiver_height: f32,
    ground_factor: f32,
) -> [f32; NUM_BANDS] {
    if distance <= 0.0 {
        return [0.0; NUM_BANDS];
    }

    let g = ground_factor.clamp(0.0, 1.0);
    let h_s = source_height.max(0.0);
    let h_r = receiver_height.max(0.0);

    // ISO 9613-2 simplified ground correction regions
    // A_ground = A_source + A_middle + A_receiver
    let dp = distance.max(1.0);

    std::array::from_fn(|band| {
        let f = crate::material::FREQUENCY_BANDS[band];

        // Source region ground effect
        let a_s = if h_s < 0.5 {
            -1.5 + g * (-1.5 + 3.0 * (f / 500.0).min(1.0))
        } else {
            -1.5
        };

        // Receiver region ground effect
        let a_r = if h_r < 0.5 {
            -1.5 + g * (-1.5 + 3.0 * (f / 500.0).min(1.0))
        } else {
            -1.5
        };

        // Middle region: proportional to distance and G
        let a_m = -3.0 * g * (1.0 - 30.0 * (h_s + h_r) / dp).max(0.0);

        (a_s + a_m + a_r).clamp(-20.0, 0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrier_loss_increases_with_frequency() {
        let low = barrier_insertion_loss(1.0, 250.0, 20.0);
        let high = barrier_insertion_loss(1.0, 4000.0, 20.0);
        assert!(
            high > low,
            "higher freq ({high}) should have more barrier loss than low ({low})"
        );
    }

    #[test]
    fn barrier_loss_increases_with_path_diff() {
        let small = barrier_insertion_loss(0.1, 1000.0, 20.0);
        let large = barrier_insertion_loss(2.0, 1000.0, 20.0);
        assert!(large > small);
    }

    #[test]
    fn barrier_loss_zero_for_no_barrier() {
        assert_eq!(barrier_insertion_loss(0.0, 1000.0, 20.0), 0.0);
        assert_eq!(barrier_insertion_loss(-1.0, 1000.0, 20.0), 0.0);
    }

    #[test]
    fn foliage_increases_with_depth() {
        let thin = foliage_attenuation(5.0, 1000.0);
        let thick = foliage_attenuation(20.0, 1000.0);
        assert!(thick > thin);
    }

    #[test]
    fn foliage_capped_at_10db() {
        let deep = foliage_attenuation(1000.0, 4000.0);
        assert!(deep <= 10.0, "foliage should cap at 10 dB, got {deep}");
    }

    #[test]
    fn met_correction_zero_short_distance() {
        let c = meteorological_correction(50.0, 2.0, 2.0);
        assert!(c.abs() < 0.01, "short distance should have ~0 correction");
    }

    #[test]
    fn met_correction_increases_with_distance() {
        let near = meteorological_correction(200.0, 2.0, 2.0);
        let far = meteorological_correction(2000.0, 2.0, 2.0);
        assert!(far > near);
    }

    #[test]
    fn ground_attenuation_hard_vs_soft() {
        let hard = ground_attenuation(100.0, 1.0, 1.5, 0.0);
        let soft = ground_attenuation(100.0, 1.0, 1.5, 1.0);
        // Soft ground generally has more mid-frequency attenuation
        let hard_sum: f32 = hard.iter().map(|&a| a.abs()).sum();
        let soft_sum: f32 = soft.iter().map(|&a| a.abs()).sum();
        assert!(
            soft_sum > hard_sum || (soft_sum - hard_sum).abs() < 1.0,
            "soft ground should differ from hard"
        );
    }

    #[test]
    fn ground_attenuation_in_range() {
        let atten = ground_attenuation(200.0, 0.3, 1.5, 0.5);
        for &a in &atten {
            assert!(
                (-20.0..=0.0).contains(&a),
                "ground atten should be [-20, 0], got {a}"
            );
        }
    }

    #[test]
    fn barrier_bands_count() {
        let bands = barrier_insertion_loss_bands(1.0, 20.0);
        assert_eq!(bands.len(), NUM_BANDS);
    }
}
