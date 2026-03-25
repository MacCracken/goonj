//! Room acoustics analysis metrics.
//!
//! Computes standard room acoustics quality metrics from impulse responses:
//! - **C50/C80 (Clarity)**: ratio of early to late energy
//! - **D50 (Definition)**: early-to-total energy ratio
//! - **STI (Speech Transmission Index)**: speech intelligibility estimate
//! - **Absorption placement**: suggestions for optimal acoustic treatment

use crate::impulse::{ImpulseResponse, sabine_rt60};
use crate::room::AcousticRoom;
use serde::{Deserialize, Serialize};

/// Compute clarity C50 — ratio of early energy (0–50 ms) to late energy (50 ms+).
///
/// C50 = 10 × log10(E_early / E_late) in dB.
/// Higher values indicate better speech intelligibility.
#[must_use]
#[inline]
pub fn clarity_c50(ir: &ImpulseResponse) -> f32 {
    clarity(ir, 0.050)
}

/// Compute clarity C80 — ratio of early energy (0–80 ms) to late energy (80 ms+).
///
/// C80 = 10 × log10(E_early / E_late) in dB.
/// Standard metric for music clarity.
#[must_use]
#[inline]
pub fn clarity_c80(ir: &ImpulseResponse) -> f32 {
    clarity(ir, 0.080)
}

/// Generic clarity metric at a given time boundary.
#[must_use]
#[inline]
fn clarity(ir: &ImpulseResponse, boundary_seconds: f32) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }

    let boundary_sample = (boundary_seconds * ir.sample_rate as f32) as usize;
    let boundary_sample = boundary_sample.min(ir.samples.len());

    let early_energy: f32 = ir.samples[..boundary_sample].iter().map(|&s| s * s).sum();
    let late_energy: f32 = ir.samples[boundary_sample..].iter().map(|&s| s * s).sum();

    if late_energy < f32::EPSILON {
        return f32::INFINITY;
    }
    10.0 * (early_energy / late_energy).log10()
}

/// Compute definition D50 — ratio of early energy (0–50 ms) to total energy.
///
/// D50 = E_early / E_total (linear, 0.0–1.0).
/// Higher values indicate better speech intelligibility.
#[must_use]
#[inline]
pub fn definition_d50(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }

    let boundary_sample = (0.050 * ir.sample_rate as f32) as usize;
    let boundary_sample = boundary_sample.min(ir.samples.len());

    let early_energy: f32 = ir.samples[..boundary_sample].iter().map(|&s| s * s).sum();
    let total_energy: f32 = ir.samples.iter().map(|&s| s * s).sum();

    if total_energy < f32::EPSILON {
        return 0.0;
    }
    early_energy / total_energy
}

/// Simplified Speech Transmission Index (STI) estimation from an impulse response.
///
/// Uses the modulation transfer function (MTF) approach:
/// 1. Compute the squared IR (energy envelope)
/// 2. For each modulation frequency, compute the modulation index
/// 3. Convert to apparent SNR and then to STI
///
/// Returns a value 0.0–1.0 where >0.75 is "excellent" and <0.45 is "poor".
#[must_use]
#[tracing::instrument(skip(ir), fields(samples = ir.samples.len(), sample_rate = ir.sample_rate))]
pub fn sti_estimate(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }

    // Standard STI modulation frequencies (Hz)
    let mod_freqs = [
        0.63, 0.8, 1.0, 1.25, 1.6, 2.0, 2.5, 3.15, 4.0, 5.0, 6.3, 8.0, 10.0, 12.5,
    ];

    // Compute total energy without allocation
    let total_energy: f32 = ir.samples.iter().map(|&s| s * s).sum();
    if total_energy < f32::EPSILON {
        return 0.0;
    }

    // Compute modulation transfer index for each modulation frequency
    let dt = 1.0 / ir.sample_rate as f32;
    let mut mti_sum = 0.0_f32;
    let mut count = 0;

    for &fm in &mod_freqs {
        // MTF(fm) = |sum(h² * exp(-j*2π*fm*t))| / sum(h²)
        let omega = std::f32::consts::TAU * fm;
        let mut real = 0.0_f32;
        let mut imag = 0.0_f32;
        for (i, &s) in ir.samples.iter().enumerate() {
            let h2_i = s * s;
            let t = i as f32 * dt;
            let angle = omega * t;
            real += h2_i * angle.cos();
            imag += h2_i * angle.sin();
        }
        let mtf = (real * real + imag * imag).sqrt() / total_energy;
        let mtf = mtf.clamp(0.0, 1.0);

        // Convert MTF to apparent SNR (dB), clamped to ±15 dB
        let snr = if mtf >= 1.0 {
            15.0
        } else if mtf <= 0.0 {
            -15.0
        } else {
            (mtf / (1.0 - mtf)).log10() * 10.0
        };
        let snr = snr.clamp(-15.0, 15.0);

        // Convert to TI (Transmission Index): TI = (SNR + 15) / 30
        let ti = (snr + 15.0) / 30.0;
        mti_sum += ti;
        count += 1;
    }

    if count == 0 {
        return 0.0;
    }
    (mti_sum / count as f32).clamp(0.0, 1.0)
}

/// A suggestion for acoustic absorption placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsorptionSuggestion {
    /// Index of the wall in the room geometry.
    pub wall_index: usize,
    /// How much RT60 changes (in seconds) if this wall's absorption is increased.
    /// Negative means RT60 decreases (desirable for reducing reverb).
    pub rt60_sensitivity: f32,
    /// Current average absorption of this wall.
    pub current_absorption: f32,
}

/// Suggest which walls would have the most impact on RT60 if treated.
///
/// Computes how much absorption increase each wall needs to approach `target_rt60`.
/// Returns suggestions sorted by sensitivity (most impactful first).
#[must_use]
#[tracing::instrument(skip(room), fields(target_rt60))]
pub fn suggest_absorption_placement(
    room: &AcousticRoom,
    target_rt60: f32,
) -> Vec<AbsorptionSuggestion> {
    let volume = room.geometry.volume_shoebox();
    let current_total_abs = room.geometry.total_absorption();
    let current_rt60 = sabine_rt60(volume, current_total_abs);

    // Scale the test absorption bump based on how far we are from target
    let rt60_ratio = if target_rt60 > f32::EPSILON && current_rt60 > f32::EPSILON {
        (current_rt60 / target_rt60).clamp(0.1, 10.0)
    } else {
        1.0
    };
    let bump = (0.2 * rt60_ratio).min(0.5);

    let mut suggestions: Vec<AbsorptionSuggestion> = room
        .geometry
        .walls
        .iter()
        .enumerate()
        .map(|(i, wall)| {
            let wall_area = wall.area();
            let current_abs = wall.material.average_absorption();

            // Simulate increasing this wall's absorption
            let new_abs = (current_abs + bump).min(1.0);
            let delta_abs = (new_abs - current_abs) * wall_area;
            let new_total_abs = current_total_abs + delta_abs;
            let new_rt60 = sabine_rt60(volume, new_total_abs);
            let sensitivity = new_rt60 - current_rt60;

            AbsorptionSuggestion {
                wall_index: i,
                current_absorption: current_abs,
                rt60_sensitivity: sensitivity,
            }
        })
        .collect();

    // Sort by magnitude of sensitivity (largest negative = most impact)
    suggestions.sort_by(|a, b| {
        a.rt60_sensitivity
            .partial_cmp(&b.rt60_sensitivity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;

    fn make_exponential_ir(rt60: f32, sample_rate: u32, duration: f32) -> ImpulseResponse {
        let decay_rate = 6.908 / rt60; // ln(1000) / RT60
        let num_samples = (duration * sample_rate as f32) as usize;
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (-decay_rate * t).exp()
            })
            .collect();
        ImpulseResponse {
            samples,
            sample_rate,
            rt60,
        }
    }

    #[test]
    fn clarity_c50_exponential() {
        // For short RT60, C50 should be clearly positive (early energy dominates)
        let ir = make_exponential_ir(0.3, 48000, 2.0);
        let c50 = clarity_c50(&ir);
        assert!(
            c50 > 0.0,
            "C50 should be positive for short RT60 decay, got {c50}"
        );
    }

    #[test]
    fn clarity_c80_greater_than_c50() {
        let ir = make_exponential_ir(1.0, 48000, 2.0);
        let c50 = clarity_c50(&ir);
        let c80 = clarity_c80(&ir);
        assert!(c80 > c50, "C80 ({c80}) should be greater than C50 ({c50})");
    }

    #[test]
    fn short_rt60_higher_clarity() {
        let short = make_exponential_ir(0.3, 48000, 2.0);
        let long = make_exponential_ir(2.0, 48000, 2.0);
        let c80_short = clarity_c80(&short);
        let c80_long = clarity_c80(&long);
        assert!(
            c80_short > c80_long,
            "short RT60 ({c80_short}) should have higher C80 than long ({c80_long})"
        );
    }

    #[test]
    fn definition_d50_in_range() {
        let ir = make_exponential_ir(1.0, 48000, 2.0);
        let d50 = definition_d50(&ir);
        assert!(
            (0.0..=1.0).contains(&d50),
            "D50 should be in [0,1], got {d50}"
        );
    }

    #[test]
    fn definition_d50_short_reverb_higher() {
        let short = make_exponential_ir(0.3, 48000, 2.0);
        let long = make_exponential_ir(2.0, 48000, 2.0);
        assert!(
            definition_d50(&short) > definition_d50(&long),
            "short reverb should have higher D50"
        );
    }

    #[test]
    fn sti_clear_ir_high() {
        // Very short impulse (clear speech) → high STI
        let ir = make_exponential_ir(0.2, 48000, 2.0);
        let sti = sti_estimate(&ir);
        assert!(
            sti > 0.5,
            "clear room (0.2s RT60) should have STI > 0.5, got {sti}"
        );
    }

    #[test]
    fn sti_reverberant_ir_lower() {
        let clear = make_exponential_ir(0.3, 48000, 2.0);
        let reverberant = make_exponential_ir(3.0, 48000, 4.0);
        let sti_clear = sti_estimate(&clear);
        let sti_rev = sti_estimate(&reverberant);
        assert!(
            sti_clear > sti_rev,
            "clear ({sti_clear}) should have higher STI than reverberant ({sti_rev})"
        );
    }

    #[test]
    fn sti_in_valid_range() {
        let ir = make_exponential_ir(1.0, 48000, 2.0);
        let sti = sti_estimate(&ir);
        assert!(
            (0.0..=1.0).contains(&sti),
            "STI should be in [0,1], got {sti}"
        );
    }

    #[test]
    fn empty_ir_returns_zero() {
        let ir = ImpulseResponse {
            samples: vec![],
            sample_rate: 48000,
            rt60: 1.0,
        };
        assert_eq!(clarity_c50(&ir), 0.0);
        assert_eq!(definition_d50(&ir), 0.0);
        assert_eq!(sti_estimate(&ir), 0.0);
    }

    #[test]
    fn absorption_suggestions_sorted_by_sensitivity() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let suggestions = suggest_absorption_placement(&room, 0.5);
        assert_eq!(suggestions.len(), 6);

        // All sensitivities should be negative (adding absorption reduces RT60)
        for s in &suggestions {
            assert!(
                s.rt60_sensitivity < 0.0,
                "wall {} sensitivity should be negative, got {}",
                s.wall_index,
                s.rt60_sensitivity
            );
        }

        // Should be sorted by sensitivity (most negative first)
        for window in suggestions.windows(2) {
            assert!(window[0].rt60_sensitivity <= window[1].rt60_sensitivity);
        }
    }

    #[test]
    fn larger_wall_more_sensitive() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let suggestions = suggest_absorption_placement(&room, 0.5);

        // Floor/ceiling (10×8=80 m²) should be more sensitive than side walls (8×3=24 m²)
        let floor_sens = suggestions
            .iter()
            .find(|s| s.wall_index == 0)
            .unwrap()
            .rt60_sensitivity;
        let side_sens = suggestions
            .iter()
            .find(|s| s.wall_index == 4)
            .unwrap()
            .rt60_sensitivity;

        assert!(
            floor_sens.abs() > side_sens.abs(),
            "floor ({floor_sens}) should be more sensitive than side wall ({side_sens})"
        );
    }

    // --- Audit edge-case tests ---

    #[test]
    fn clarity_zero_sample_rate() {
        let ir = ImpulseResponse {
            samples: vec![1.0; 100],
            sample_rate: 0,
            rt60: 1.0,
        };
        assert_eq!(clarity_c50(&ir), 0.0);
        assert_eq!(clarity_c80(&ir), 0.0);
    }

    #[test]
    fn clarity_all_energy_early() {
        // IR with energy only in first 10ms → C50 should be +∞
        let mut samples = vec![0.0_f32; 48000];
        samples[0] = 1.0;
        let ir = ImpulseResponse {
            samples,
            sample_rate: 48000,
            rt60: 0.01,
        };
        assert!(clarity_c50(&ir).is_infinite(), "all-early should be +∞");
    }

    #[test]
    fn definition_d50_all_early() {
        let mut samples = vec![0.0_f32; 48000];
        samples[0] = 1.0;
        let ir = ImpulseResponse {
            samples,
            sample_rate: 48000,
            rt60: 0.01,
        };
        assert!((definition_d50(&ir) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sti_silent_ir_returns_zero() {
        let ir = ImpulseResponse {
            samples: vec![0.0; 48000],
            sample_rate: 48000,
            rt60: 1.0,
        };
        assert_eq!(sti_estimate(&ir), 0.0);
    }

    #[test]
    fn sti_impulse_response_range() {
        // Any valid IR should produce STI in [0, 1]
        for rt60 in [0.1, 0.5, 1.0, 2.0, 5.0] {
            let ir = make_exponential_ir(rt60, 48000, 3.0);
            let sti = sti_estimate(&ir);
            assert!(
                (0.0..=1.0).contains(&sti),
                "STI {sti} out of range for RT60={rt60}"
            );
        }
    }

    #[test]
    fn suggest_absorption_already_at_target() {
        // Room already at target RT60 → suggestions should still work
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        let vol = room.geometry.volume_shoebox();
        let abs = room.geometry.total_absorption();
        let current_rt60 = crate::impulse::sabine_rt60(vol, abs);
        let suggestions = suggest_absorption_placement(&room, current_rt60);
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn clarity_zero_total_energy() {
        let ir = ImpulseResponse {
            samples: vec![0.0; 48000],
            sample_rate: 48000,
            rt60: 1.0,
        };
        // Both early and late energy are zero → late < EPSILON → returns infinity
        // but definition_d50 returns 0 for zero total
        assert_eq!(definition_d50(&ir), 0.0);
    }

    #[test]
    fn sti_mtf_saturation_branches() {
        // Delta impulse → MTF should saturate near 1.0 for all mod freqs
        let mut samples = vec![0.0_f32; 48000];
        samples[0] = 1.0;
        let ir = ImpulseResponse {
            samples,
            sample_rate: 48000,
            rt60: 0.001,
        };
        let sti = sti_estimate(&ir);
        assert!(
            sti > 0.9,
            "delta impulse should give near-perfect STI, got {sti}"
        );
    }

    #[test]
    fn suggest_absorption_zero_target() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let suggestions = suggest_absorption_placement(&room, 0.0);
        assert!(!suggestions.is_empty());
    }
}
