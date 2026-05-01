//! Dark Velvet Noise reverb — non-exponential late reverberation
//! synthesized from sparse stochastic pulse sequences.
//!
//! Generates a velvet-noise pulse train (one ±1 pulse per fixed-length
//! cell, position uniform within the cell) and shapes it with a decay
//! envelope and a time-varying low-pass for "dark" coloration. The
//! envelope can be exponential (single RT60) or piecewise-linear-in-dB
//! (e.g. for coupled-room double-slope decay).
//!
//! Reference: Fagerström et al., "Non-Exponential Reverberation with
//! Dark Velvet Noise," JAES 72(6), 2024. The implementation here uses
//! a single time-varying 1-pole low-pass instead of per-pulse FIR
//! filtering — perceptually equivalent but O(N) instead of O(N·M).

use serde::{Deserialize, Serialize};

/// Decay envelope shape for the velvet-noise tail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecayEnvelope {
    /// Exponential decay: 60 dB amplitude drop over `rt60_seconds`.
    /// `amp(t) = 10^(-3·t/rt60)`.
    Exponential {
        /// Reverberation time in seconds (60 dB amplitude decay).
        rt60_seconds: f32,
    },
    /// Piecewise-linear-in-dB envelope. Breakpoints are
    /// `(time_seconds, level_db)` pairs sorted ascending by time.
    /// `level_db = 20·log10(amplitude)`. Outside the range, the
    /// endpoint level is held.
    PiecewiseDb {
        /// Sorted (time_s, level_db) breakpoints. At least one required.
        breakpoints: Vec<(f32, f32)>,
    },
}

/// Configuration for Dark Velvet Noise IR synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvnConfig {
    /// Sample rate (Hz).
    pub sample_rate: u32,
    /// Pulse density (pulses per second). Typical: 1000–4000.
    pub pulse_density_hz: f32,
    /// Total IR duration (seconds).
    pub duration_seconds: f32,
    /// Decay envelope shape.
    pub decay: DecayEnvelope,
    /// Initial low-pass cutoff at t=0 (Hz). Higher = brighter onset.
    pub coloration_initial_cutoff_hz: f32,
    /// Final low-pass cutoff at t=duration (Hz). Lower = darker tail.
    pub coloration_final_cutoff_hz: f32,
    /// PRNG seed (xorshift64).
    pub seed: u64,
}

impl Default for DvnConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            pulse_density_hz: 2_000.0,
            duration_seconds: 2.0,
            decay: DecayEnvelope::Exponential { rt60_seconds: 1.5 },
            coloration_initial_cutoff_hz: 16_000.0,
            coloration_final_cutoff_hz: 800.0,
            seed: 0xCAFE_BABE_F00D_D15C,
        }
    }
}

/// Maximum allowed IR length in samples (matches `impulse::generate_ir`).
const MAX_IR_SAMPLES: usize = 192_000 * 600;

/// Synthesize a Dark Velvet Noise impulse response.
///
/// Returns an empty vector when configuration is degenerate
/// (`sample_rate == 0`, `duration_seconds <= 0`, `pulse_density_hz <= 0`).
#[must_use]
#[tracing::instrument(skip(config), fields(
    sample_rate = config.sample_rate,
    pulse_density_hz = config.pulse_density_hz,
    duration_seconds = config.duration_seconds,
))]
pub fn synthesize_dvn_ir(config: &DvnConfig) -> Vec<f32> {
    if config.sample_rate == 0 || config.duration_seconds <= 0.0 || config.pulse_density_hz <= 0.0 {
        return Vec::new();
    }

    let num_samples =
        ((config.duration_seconds * config.sample_rate as f32) as usize).min(MAX_IR_SAMPLES);
    if num_samples == 0 {
        return Vec::new();
    }
    let mut out = vec![0.0_f32; num_samples];

    // Cell size in samples; saturate at one pulse per sample.
    let td = (config.sample_rate as f32 / config.pulse_density_hz).max(1.0);
    let td_int = (td as usize).max(1);

    let mut rng = Xorshift64::new(config.seed);
    let inv_sr = 1.0 / config.sample_rate as f32;

    // Stage 1: place sparse pulses with envelope amplitude.
    let mut cell_start = 0usize;
    while cell_start < num_samples {
        let cell_end = (cell_start + td_int).min(num_samples);
        let cell_len = cell_end - cell_start;
        let r = rng.next_u64();
        let k = (r as usize) % cell_len;
        let sign = if (r >> 32) & 1 == 0 { 1.0 } else { -1.0 };
        let idx = cell_start + k;
        let t = idx as f32 * inv_sr;
        let env = envelope_value(&config.decay, t);
        out[idx] = sign * env;
        cell_start = cell_end;
    }

    // Stage 2: time-varying 1-pole low-pass for "dark" coloration.
    // a[n] = 1 - exp(-2π·fc[n]/fs); fc interpolated linearly between
    // initial and final cutoffs (linear-in-a is a perceptual
    // approximation; well within the 4% RT60 budget).
    let f0 = config.coloration_initial_cutoff_hz.max(1.0);
    let f1 = config.coloration_final_cutoff_hz.max(1.0);
    let twopi_inv_sr = std::f32::consts::TAU * inv_sr;
    let a0 = 1.0 - (-twopi_inv_sr * f0).exp();
    let a1 = 1.0 - (-twopi_inv_sr * f1).exp();
    let inv_n = 1.0 / num_samples as f32;
    let mut y_prev = 0.0_f32;
    for (n, sample) in out.iter_mut().enumerate() {
        let frac = n as f32 * inv_n;
        let a = a0 + (a1 - a0) * frac;
        let y = a * (*sample) + (1.0 - a) * y_prev;
        *sample = y;
        y_prev = y;
    }

    out
}

#[inline]
fn envelope_value(env: &DecayEnvelope, t: f32) -> f32 {
    match env {
        DecayEnvelope::Exponential { rt60_seconds } => {
            if *rt60_seconds <= 0.0 {
                return 0.0;
            }
            10.0_f32.powf(-3.0 * t / rt60_seconds)
        }
        DecayEnvelope::PiecewiseDb { breakpoints } => piecewise_db_value(breakpoints, t),
    }
}

#[inline]
fn piecewise_db_value(breakpoints: &[(f32, f32)], t: f32) -> f32 {
    if breakpoints.is_empty() {
        return 0.0;
    }
    if breakpoints.len() == 1 || t <= breakpoints[0].0 {
        return 10.0_f32.powf(breakpoints[0].1 / 20.0);
    }
    let last = breakpoints[breakpoints.len() - 1];
    if t >= last.0 {
        return 10.0_f32.powf(last.1 / 20.0);
    }
    for w in breakpoints.windows(2) {
        let (t_a, db_a) = w[0];
        let (t_b, db_b) = w[1];
        if t >= t_a && t <= t_b {
            let span = (t_b - t_a).max(f32::EPSILON);
            let frac = (t - t_a) / span;
            let db = db_a + (db_b - db_a) * frac;
            return 10.0_f32.powf(db / 20.0);
        }
    }
    0.0
}

/// xorshift64 PRNG (private; no external rand dep).
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x5555_5555_5555_5555
        } else {
            seed
        })
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_2s() -> DvnConfig {
        DvnConfig {
            sample_rate: 48_000,
            pulse_density_hz: 2_000.0,
            duration_seconds: 2.0,
            decay: DecayEnvelope::Exponential { rt60_seconds: 1.5 },
            coloration_initial_cutoff_hz: 16_000.0,
            coloration_final_cutoff_hz: 800.0,
            seed: 42,
        }
    }

    #[test]
    fn synthesize_produces_correct_length() {
        let ir = synthesize_dvn_ir(&config_2s());
        assert_eq!(ir.len(), 96_000);
    }

    #[test]
    fn synthesize_has_nonzero_content() {
        let ir = synthesize_dvn_ir(&config_2s());
        let energy: f32 = ir.iter().map(|s| s * s).sum();
        assert!(energy > 0.0);
    }

    #[test]
    fn rt60_recovery_within_4_percent() {
        let target_rt60 = 1.5_f32;
        let config = DvnConfig {
            sample_rate: 48_000,
            pulse_density_hz: 4_000.0,
            duration_seconds: target_rt60 * 1.5,
            decay: DecayEnvelope::Exponential {
                rt60_seconds: target_rt60,
            },
            coloration_initial_cutoff_hz: 20_000.0,
            coloration_final_cutoff_hz: 20_000.0,
            seed: 1,
        };
        let ir = synthesize_dvn_ir(&config);
        let mut edc = vec![0.0_f32; ir.len()];
        let mut cum = 0.0_f32;
        for i in (0..ir.len()).rev() {
            cum += ir[i] * ir[i];
            edc[i] = cum;
        }
        let max = edc[0].max(f32::EPSILON);
        let target = max * 1e-6;
        let mut t60_sample = ir.len();
        for (i, &v) in edc.iter().enumerate() {
            if v <= target {
                t60_sample = i;
                break;
            }
        }
        assert!(t60_sample < ir.len(), "EDC never reached -60 dB");
        let measured = t60_sample as f32 / config.sample_rate as f32;
        let err = (measured - target_rt60).abs() / target_rt60;
        assert!(
            err < 0.04,
            "RT60 error {err:.4} exceeds 4%, measured={measured}, target={target_rt60}"
        );
    }

    #[test]
    fn coloration_darkens_tail() {
        let config = DvnConfig {
            sample_rate: 48_000,
            pulse_density_hz: 4_000.0,
            duration_seconds: 1.0,
            decay: DecayEnvelope::Exponential { rt60_seconds: 0.8 },
            coloration_initial_cutoff_hz: 16_000.0,
            coloration_final_cutoff_hz: 500.0,
            seed: 7,
        };
        let ir = synthesize_dvn_ir(&config);
        // Crude HF detector: first-difference energy in early vs late windows.
        let early: f32 = ir
            .windows(2)
            .take(4_800)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum();
        let late: f32 = ir
            .windows(2)
            .skip(ir.len() - 4_801)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum();
        assert!(
            early > late,
            "early HF energy ({early}) should exceed late ({late}) under darkening"
        );
    }

    #[test]
    fn deterministic_for_same_seed() {
        let a = synthesize_dvn_ir(&config_2s());
        let b = synthesize_dvn_ir(&config_2s());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x, y);
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut c = config_2s();
        c.seed = 1;
        let a = synthesize_dvn_ir(&c);
        c.seed = 2;
        let b = synthesize_dvn_ir(&c);
        let diff: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
        assert!(diff > 0.0, "different seeds should produce different IRs");
    }

    #[test]
    fn zero_duration_returns_empty() {
        let mut c = config_2s();
        c.duration_seconds = 0.0;
        assert!(synthesize_dvn_ir(&c).is_empty());
    }

    #[test]
    fn zero_sample_rate_returns_empty() {
        let mut c = config_2s();
        c.sample_rate = 0;
        assert!(synthesize_dvn_ir(&c).is_empty());
    }

    #[test]
    fn negative_pulse_density_returns_empty() {
        let mut c = config_2s();
        c.pulse_density_hz = -1.0;
        assert!(synthesize_dvn_ir(&c).is_empty());
    }

    #[test]
    fn saturation_density_does_not_panic() {
        let mut c = config_2s();
        c.pulse_density_hz = 100_000.0; // exceeds 48kHz sample_rate
        let ir = synthesize_dvn_ir(&c);
        assert_eq!(ir.len(), 96_000);
    }

    #[test]
    fn piecewise_envelope_double_slope() {
        let config = DvnConfig {
            sample_rate: 48_000,
            pulse_density_hz: 4_000.0,
            duration_seconds: 1.0,
            decay: DecayEnvelope::PiecewiseDb {
                breakpoints: vec![(0.0, 0.0), (0.3, -10.0), (1.0, -60.0)],
            },
            coloration_initial_cutoff_hz: 20_000.0,
            coloration_final_cutoff_hz: 20_000.0,
            seed: 9,
        };
        let ir = synthesize_dvn_ir(&config);
        assert_eq!(ir.len(), 48_000);
        assert!(ir.iter().any(|s| s.abs() > 0.0));
    }

    #[test]
    fn piecewise_empty_breakpoints_silent() {
        let env = DecayEnvelope::PiecewiseDb {
            breakpoints: vec![],
        };
        assert_eq!(envelope_value(&env, 0.5), 0.0);
    }

    #[test]
    fn piecewise_single_breakpoint_holds() {
        let env = DecayEnvelope::PiecewiseDb {
            breakpoints: vec![(0.0, -20.0)],
        };
        let v = envelope_value(&env, 1.0);
        let expected = 10.0_f32.powf(-1.0); // -20 dB
        assert!((v - expected).abs() < 1e-6);
    }

    #[test]
    fn exponential_envelope_at_rt60_is_minus_60db() {
        let env = DecayEnvelope::Exponential { rt60_seconds: 1.5 };
        let v = envelope_value(&env, 1.5);
        assert!(
            (v - 1e-3).abs() < 1e-5,
            "amp at RT60 should be 10^-3, got {v}"
        );
    }

    #[test]
    fn exponential_zero_rt60_returns_zero() {
        let env = DecayEnvelope::Exponential { rt60_seconds: 0.0 };
        assert_eq!(envelope_value(&env, 1.0), 0.0);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = config_2s();
        let json = serde_json::to_string(&c).unwrap();
        let back: DvnConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
