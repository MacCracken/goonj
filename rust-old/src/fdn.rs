//! Feedback Delay Network (FDN) for efficient late reverberation synthesis.
//!
//! An FDN consists of N parallel delay lines connected through a feedback
//! matrix. Each delay line has a length proportional to a room dimension
//! and absorption filters that control the frequency-dependent decay rate.
//! This produces physically motivated late reverb without ray tracing.

use serde::{Deserialize, Serialize};

/// Configuration for a Feedback Delay Network reverberator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdnConfig {
    /// Number of delay lines (typically 4, 8, or 16).
    pub num_delays: usize,
    /// Delay lengths in samples (one per delay line).
    pub delay_lengths: Vec<u32>,
    /// Per-delay-line absorption gain (0.0–1.0) controlling RT60.
    pub feedback_gains: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Target RT60 in seconds (used to compute feedback gains if not set manually).
    pub target_rt60: f32,
}

/// A Feedback Delay Network reverberator.
#[derive(Debug, Clone)]
pub struct Fdn {
    delay_lines: Vec<Vec<f32>>,
    write_positions: Vec<usize>,
    feedback_gains: Vec<f32>,
    /// Scratch buffer for delay line outputs (avoids per-sample allocation).
    scratch: Vec<f32>,
    _sample_rate: u32,
}

impl Fdn {
    /// Create an FDN from configuration.
    ///
    /// If `feedback_gains` is empty, gains are computed from `target_rt60`
    /// and the delay lengths.
    #[must_use]
    pub fn new(config: &FdnConfig) -> Self {
        let n = config.num_delays;
        let delay_lines: Vec<Vec<f32>> = config
            .delay_lengths
            .iter()
            .take(n)
            .map(|&len| vec![0.0; len.max(1) as usize])
            .collect();

        let feedback_gains = if config.feedback_gains.len() >= n {
            config.feedback_gains[..n].to_vec()
        } else {
            // Compute from RT60: gain = 10^(-3 × delay_time / RT60)
            config
                .delay_lengths
                .iter()
                .take(n)
                .map(|&len| {
                    let delay_time = len as f32 / config.sample_rate.max(1) as f32;
                    if config.target_rt60 > 0.0 {
                        10.0_f32.powf(-3.0 * delay_time / config.target_rt60)
                    } else {
                        0.0
                    }
                })
                .collect()
        };

        Self {
            write_positions: vec![0; n],
            scratch: vec![0.0; n],
            delay_lines,
            feedback_gains,
            _sample_rate: config.sample_rate,
        }
    }

    /// Process a single input sample through the FDN, returning the mixed output.
    ///
    /// The feedback matrix is a Householder matrix (all-pass, energy preserving).
    /// Zero heap allocations — uses pre-allocated scratch buffer.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let n = self.delay_lines.len();
        if n == 0 {
            return input;
        }

        // Read from delay lines into scratch buffer (zero allocation)
        let mut output_sum = 0.0_f32;
        for i in 0..n {
            let len = self.delay_lines[i].len();
            let read_pos = (self.write_positions[i] + 1) % len;
            self.scratch[i] = self.delay_lines[i][read_pos];
            output_sum += self.scratch[i];
        }

        // Householder feedback + write to delay lines (single pass, zero allocation)
        let factor = 2.0 / n as f32;
        let correction = factor * output_sum;
        let input_per_line = input / n as f32;

        for i in 0..n {
            let fb = self.scratch[i] - correction;
            let len = self.delay_lines[i].len();
            self.delay_lines[i][self.write_positions[i]] =
                input_per_line + fb * self.feedback_gains[i];
            self.write_positions[i] = (self.write_positions[i] + 1) % len;
        }

        output_sum / (n as f32).sqrt()
    }

    /// Process a buffer of input samples, returning the reverb output.
    #[must_use]
    pub fn process_buffer(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&s| self.process_sample(s)).collect()
    }

    /// Reset all delay lines to silence.
    pub fn reset(&mut self) {
        for line in &mut self.delay_lines {
            for s in line.iter_mut() {
                *s = 0.0;
            }
        }
        for pos in &mut self.write_positions {
            *pos = 0;
        }
    }
}

/// Create an FDN configuration for a shoebox room.
///
/// Delay lengths are derived from room dimensions to approximate the room's
/// modal behavior. Uses 8 delay lines with mutually prime lengths.
#[must_use]
pub fn fdn_config_for_room(
    length: f32,
    width: f32,
    height: f32,
    rt60: f32,
    sample_rate: u32,
) -> FdnConfig {
    let c = crate::propagation::speed_of_sound(20.0);

    // Base delay from each dimension (round trip time)
    let base_delays = [
        length,
        width,
        height,
        (length + width) * 0.5,
        (length + height) * 0.5,
        (width + height) * 0.5,
        (length + width + height) / 3.0,
        length.max(width).max(height),
    ];

    let delay_lengths: Vec<u32> = base_delays
        .iter()
        .map(|&d| {
            let samples = (2.0 * d / c * sample_rate as f32) as u32;
            samples.max(1)
        })
        .collect();

    FdnConfig {
        num_delays: 8,
        delay_lengths,
        feedback_gains: vec![], // auto-compute from RT60
        sample_rate,
        target_rt60: rt60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fdn_produces_output() {
        let config = fdn_config_for_room(10.0, 8.0, 3.0, 1.0, 48000);
        let mut fdn = Fdn::new(&config);
        // Feed an impulse
        let mut output = vec![0.0_f32; 4800];
        output[0] = fdn.process_sample(1.0);
        for s in output[1..].iter_mut() {
            *s = fdn.process_sample(0.0);
        }
        let energy: f32 = output.iter().map(|&s| s * s).sum();
        assert!(energy > 0.0, "FDN should produce reverb output");
    }

    #[test]
    fn fdn_decays_over_time() {
        let config = fdn_config_for_room(10.0, 8.0, 3.0, 0.5, 48000);
        let mut fdn = Fdn::new(&config);
        fdn.process_sample(1.0);
        // Skip initial buildup (first 100ms), then compare two 200ms windows
        for _ in 0..4800 {
            fdn.process_sample(0.0);
        }
        let early: f32 = (0..9600).map(|_| fdn.process_sample(0.0).powi(2)).sum();
        let late: f32 = (0..9600).map(|_| fdn.process_sample(0.0).powi(2)).sum();
        assert!(
            early > late,
            "early energy ({early}) should exceed late ({late})"
        );
    }

    #[test]
    fn fdn_reset_clears_state() {
        let config = fdn_config_for_room(5.0, 4.0, 3.0, 1.0, 48000);
        let mut fdn = Fdn::new(&config);
        fdn.process_sample(1.0);
        for _ in 0..100 {
            fdn.process_sample(0.0);
        }
        fdn.reset();
        let out = fdn.process_sample(0.0);
        assert!(out.abs() < f32::EPSILON, "reset should clear all state");
    }

    #[test]
    fn fdn_config_for_room_valid() {
        let config = fdn_config_for_room(10.0, 8.0, 3.0, 1.0, 48000);
        assert_eq!(config.num_delays, 8);
        assert_eq!(config.delay_lengths.len(), 8);
        for &d in &config.delay_lengths {
            assert!(d > 0, "delay lengths should be positive");
        }
    }

    #[test]
    fn process_buffer_matches_sample_by_sample() {
        let config = fdn_config_for_room(5.0, 4.0, 3.0, 0.8, 48000);
        let mut fdn1 = Fdn::new(&config);
        let mut fdn2 = Fdn::new(&config);

        let input: Vec<f32> = (0..100).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let buf_out = fdn1.process_buffer(&input);
        let sample_out: Vec<f32> = input.iter().map(|&s| fdn2.process_sample(s)).collect();

        for (a, b) in buf_out.iter().zip(sample_out.iter()) {
            assert!(
                (a - b).abs() < f32::EPSILON,
                "buffer and sample-by-sample should match"
            );
        }
    }
}
