//! Hybrid frequency crossover — blending wave-based and geometric results.
//!
//! Provides an API for combining low-frequency wave-based simulation results
//! (FDTD, FEM, BEM) with high-frequency geometric results (ray tracing,
//! image-source, radiosity) at a crossover frequency near the Schroeder
//! frequency of the room.

use crate::material::NUM_BANDS;
use serde::{Deserialize, Serialize};

/// Configuration for hybrid frequency crossover blending.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossoverConfig {
    /// Crossover frequency in Hz (typically near the Schroeder frequency).
    pub crossover_hz: f32,
    /// Transition bandwidth in octaves (how gradually the blend occurs).
    /// 0.5 = half-octave transition, 1.0 = one-octave transition.
    pub transition_octaves: f32,
}

impl Default for CrossoverConfig {
    fn default() -> Self {
        Self {
            crossover_hz: 500.0,
            transition_octaves: 1.0,
        }
    }
}

/// Compute per-band blending weights for wave-based vs geometric results.
///
/// Returns an array of weights where 0.0 = fully wave-based and
/// 1.0 = fully geometric. The transition between them is a smooth
/// sigmoid centred on the crossover frequency.
#[must_use]
#[inline]
pub fn blend_weights(config: &CrossoverConfig) -> [f32; NUM_BANDS] {
    let fc = config.crossover_hz;
    let bw = config.transition_octaves.max(0.1);

    std::array::from_fn(|band| {
        let f = crate::material::FREQUENCY_BANDS[band];
        if fc <= 0.0 {
            return 1.0; // all geometric
        }
        // Sigmoid blend in log-frequency space
        let octaves_above = (f / fc).log2() / bw;
        let weight = 1.0 / (1.0 + (-4.0 * octaves_above).exp());
        weight.clamp(0.0, 1.0)
    })
}

/// Blend two per-band results using crossover weights.
///
/// `wave_result` is the wave-based simulation result (per-band values).
/// `geometric_result` is the geometric simulation result (per-band values).
/// Returns the blended result.
#[must_use]
#[inline]
pub fn blend_results(
    wave_result: &[f32; NUM_BANDS],
    geometric_result: &[f32; NUM_BANDS],
    config: &CrossoverConfig,
) -> [f32; NUM_BANDS] {
    let weights = blend_weights(config);
    std::array::from_fn(|band| {
        wave_result[band] * (1.0 - weights[band]) + geometric_result[band] * weights[band]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_freq_favours_wave() {
        let config = CrossoverConfig {
            crossover_hz: 500.0,
            transition_octaves: 0.5,
        };
        let weights = blend_weights(&config);
        // 63 Hz is well below 500 Hz → should favour wave (low weight)
        assert!(
            weights[0] < 0.3,
            "63 Hz should favour wave, got weight {}",
            weights[0]
        );
    }

    #[test]
    fn high_freq_favours_geometric() {
        let config = CrossoverConfig {
            crossover_hz: 500.0,
            transition_octaves: 0.5,
        };
        let weights = blend_weights(&config);
        // 8000 Hz is well above 500 Hz → should favour geometric (high weight)
        assert!(
            weights[7] > 0.9,
            "8 kHz should favour geometric, got weight {}",
            weights[7]
        );
    }

    #[test]
    fn crossover_frequency_is_midpoint() {
        let config = CrossoverConfig {
            crossover_hz: 500.0,
            transition_octaves: 1.0,
        };
        let weights = blend_weights(&config);
        // 500 Hz band (index 3) should be near 0.5
        assert!(
            (weights[3] - 0.5).abs() < 0.15,
            "crossover freq should be ~0.5, got {}",
            weights[3]
        );
    }

    #[test]
    fn blend_results_works() {
        let wave = [1.0; NUM_BANDS];
        let geom = [0.0; NUM_BANDS];
        let config = CrossoverConfig::default();
        let blended = blend_results(&wave, &geom, &config);
        // Low bands should be closer to 1.0 (wave), high bands closer to 0.0 (geometric)
        assert!(blended[0] > blended[7]);
    }

    #[test]
    fn weights_in_valid_range() {
        let config = CrossoverConfig::default();
        let weights = blend_weights(&config);
        for &w in &weights {
            assert!((0.0..=1.0).contains(&w), "weight {w} out of range");
        }
    }
}
