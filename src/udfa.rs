//! Universal Diffraction Filter Approximation (UDFA).
//!
//! Filter-based diffraction model for real-time applications. Approximates
//! diffraction effects as minimum-phase IIR filters, enabling efficient
//! higher-order diffraction interactions around finite objects.
//!
//! Reference: Acta Acustica, 2024 — "Universal Diffraction Filter Approximation."

use crate::material::NUM_BANDS;
use crate::propagation::speed_of_sound;
use serde::{Deserialize, Serialize};

/// UDFA diffraction filter coefficients for a single edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffractionFilter {
    /// Per-band attenuation in dB.
    pub attenuation: [f32; NUM_BANDS],
    /// Phase delay in seconds (extra path length / speed of sound).
    pub delay_seconds: f32,
    /// Filter quality — higher means sharper transition.
    pub quality: f32,
}

/// Compute UDFA diffraction filter for a single edge.
///
/// The filter approximates the frequency-dependent diffraction loss
/// as a smooth spectral shape parameterised by the Fresnel number
/// at each frequency band.
///
/// # Arguments
/// * `path_difference` — extra path length around the edge vs direct path (m)
/// * `edge_length` — length of the diffracting edge (m), for finite-edge correction
/// * `temperature_celsius` — air temperature
#[must_use]
pub fn compute_diffraction_filter(
    path_difference: f32,
    edge_length: f32,
    temperature_celsius: f32,
) -> DiffractionFilter {
    if path_difference <= 0.0 {
        return DiffractionFilter {
            attenuation: [0.0; NUM_BANDS],
            delay_seconds: 0.0,
            quality: 1.0,
        };
    }

    let c = speed_of_sound(temperature_celsius);
    let delay = path_difference / c;

    // Finite-edge correction factor: reduces diffraction for short edges
    let finite_correction = if edge_length > 0.0 {
        (edge_length / (edge_length + path_difference)).sqrt()
    } else {
        1.0
    };

    let attenuation = std::array::from_fn(|band| {
        let freq = crate::material::FREQUENCY_BANDS[band];
        let wavelength = c / freq;

        // Fresnel number
        let fresnel_n = 2.0 * path_difference / wavelength;

        // UDFA spectral shape: smooth approximation of Maekawa + higher-order corrections
        let base_loss = if fresnel_n < 0.01 {
            0.0 // very low Fresnel number: no significant diffraction loss
        } else {
            // Approximate: IL ≈ 10 × log10(3 + 20N) (Maekawa)
            // with UDFA smooth transition and finite-edge correction
            10.0 * (3.0 + 20.0 * fresnel_n).log10() * finite_correction
        };

        -base_loss.max(0.0) // negative = attenuation
    });

    // Filter quality based on geometry
    let quality = (path_difference * 10.0).clamp(0.1, 10.0);

    DiffractionFilter {
        attenuation,
        delay_seconds: delay,
        quality,
    }
}

/// Apply a sequence of diffraction filters (higher-order diffraction).
///
/// Each filter represents one edge in the diffraction path. The total
/// attenuation is the sum (in dB) of individual edge contributions,
/// and the total delay is the sum of individual delays.
#[must_use]
pub fn chain_diffraction_filters(filters: &[DiffractionFilter]) -> DiffractionFilter {
    if filters.is_empty() {
        return DiffractionFilter {
            attenuation: [0.0; NUM_BANDS],
            delay_seconds: 0.0,
            quality: 1.0,
        };
    }

    let mut total_atten = [0.0_f32; NUM_BANDS];
    let mut total_delay = 0.0_f32;
    let mut min_quality = f32::MAX;

    for filter in filters {
        for (a, &fa) in total_atten.iter_mut().zip(filter.attenuation.iter()) {
            *a += fa;
        }
        total_delay += filter.delay_seconds;
        min_quality = min_quality.min(filter.quality);
    }

    DiffractionFilter {
        attenuation: total_atten,
        delay_seconds: total_delay,
        quality: min_quality,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_path_diff_no_attenuation() {
        let f = compute_diffraction_filter(0.0, 1.0, 20.0);
        for &a in &f.attenuation {
            assert_eq!(a, 0.0);
        }
        assert_eq!(f.delay_seconds, 0.0);
    }

    #[test]
    fn attenuation_increases_with_frequency() {
        let f = compute_diffraction_filter(0.5, 2.0, 20.0);
        // Higher bands should have more attenuation (more negative)
        assert!(
            f.attenuation[7] < f.attenuation[0],
            "8kHz ({}) should be more attenuated than 63Hz ({})",
            f.attenuation[7],
            f.attenuation[0]
        );
    }

    #[test]
    fn attenuation_increases_with_path_diff() {
        let small = compute_diffraction_filter(0.1, 2.0, 20.0);
        let large = compute_diffraction_filter(2.0, 2.0, 20.0);
        let small_sum: f32 = small.attenuation.iter().sum();
        let large_sum: f32 = large.attenuation.iter().sum();
        assert!(
            large_sum < small_sum,
            "larger path diff should have more total attenuation"
        );
    }

    #[test]
    fn chain_filters_accumulates() {
        let f1 = compute_diffraction_filter(0.5, 2.0, 20.0);
        let f2 = compute_diffraction_filter(0.3, 1.5, 20.0);
        let chained = chain_diffraction_filters(&[f1.clone(), f2.clone()]);

        for band in 0..NUM_BANDS {
            let expected = f1.attenuation[band] + f2.attenuation[band];
            assert!(
                (chained.attenuation[band] - expected).abs() < 0.01,
                "chained attenuation should sum"
            );
        }
        assert!((chained.delay_seconds - f1.delay_seconds - f2.delay_seconds).abs() < 0.001);
    }

    #[test]
    fn chain_empty_returns_zero() {
        let f = chain_diffraction_filters(&[]);
        for &a in &f.attenuation {
            assert_eq!(a, 0.0);
        }
    }

    #[test]
    fn finite_edge_reduces_loss() {
        let infinite = compute_diffraction_filter(0.5, 100.0, 20.0);
        let finite = compute_diffraction_filter(0.5, 0.5, 20.0);
        // Short edge should have less diffraction loss (attenuation closer to 0)
        let inf_sum: f32 = infinite.attenuation.iter().map(|a| a.abs()).sum();
        let fin_sum: f32 = finite.attenuation.iter().map(|a| a.abs()).sum();
        assert!(
            fin_sum < inf_sum,
            "finite edge ({fin_sum}) should have less loss than infinite ({inf_sum})"
        );
    }

    #[test]
    fn filter_serializes() {
        let f = compute_diffraction_filter(1.0, 2.0, 20.0);
        let json = serde_json::to_string(&f).unwrap();
        let back: DiffractionFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }
}
