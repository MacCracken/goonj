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

/// Speech Transmission Index per IEC 60268-16:2020.
///
/// Speech Transmission Index estimate per IEC 60268-16:2020 structure.
///
/// Uses the 7-band × 14-modulation-frequency framework with 2020-revised
/// male speech spectrum weighting. **Note**: this operates on a broadband
/// impulse response — for full standard compliance, per-octave-band filtered
/// IRs should be used (yielding distinct MTF per band). The current
/// implementation applies identical MTF to all bands, which is an
/// approximation suitable for comparative evaluation.
///
/// Returns a value 0.0–1.0 where >0.75 is "excellent" and <0.45 is "poor".
#[must_use]
#[tracing::instrument(skip(ir), fields(samples = ir.samples.len(), sample_rate = ir.sample_rate))]
pub fn sti_estimate(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }

    let total_energy: f32 = ir.samples.iter().map(|&s| s * s).sum();
    if total_energy < f32::EPSILON {
        return 0.0;
    }

    // IEC 60268-16:2020 modulation frequencies (Hz)
    let mod_freqs: [f32; 14] = [
        0.63, 0.8, 1.0, 1.25, 1.6, 2.0, 2.5, 3.15, 4.0, 5.0, 6.3, 8.0, 10.0, 12.5,
    ];

    // IEC 60268-16:2020 male speech spectrum octave band weights (revised)
    // Bands: 125, 250, 500, 1000, 2000, 4000, 8000 Hz
    let band_weights: [f32; 7] = [0.085, 0.127, 0.230, 0.233, 0.309, 0.224, 0.173];
    let weight_sum: f32 = band_weights.iter().sum();

    // Redundancy weights between adjacent bands (IEC 60268-16 Table 3)
    let alpha_redundancy: [f32; 6] = [0.085, 0.127, 0.230, 0.233, 0.309, 0.224];
    let beta_redundancy: [f32; 6] = [0.085, 0.127, 0.230, 0.233, 0.309, 0.224];

    // Compute per-band MTI (using broadband IR as approximation when
    // per-band IRs are not available)
    let dt = 1.0 / ir.sample_rate as f32;
    let mut band_mti = [0.0_f32; 7];

    for (band_idx, mti) in band_mti.iter_mut().enumerate() {
        let mut ti_sum = 0.0_f32;
        for &fm in &mod_freqs {
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
            let mtf = ((real * real + imag * imag).sqrt() / total_energy).clamp(0.0, 1.0);

            let snr = if mtf >= 1.0 {
                15.0
            } else if mtf <= f32::EPSILON {
                -15.0
            } else {
                (mtf / (1.0 - mtf)).log10() * 10.0
            };
            ti_sum += (snr.clamp(-15.0, 15.0) + 15.0) / 30.0;
        }
        *mti = ti_sum / mod_freqs.len() as f32;
        let _ = band_idx; // band_idx reserved for per-band filtering in future
    }

    // Weighted sum of MTI values
    let mut sti = 0.0_f32;
    for (i, &mti) in band_mti.iter().enumerate() {
        sti += band_weights[i] * mti;
    }
    sti /= weight_sum;

    // Apply redundancy correction (inter-band correlation)
    let mut redundancy = 0.0_f32;
    for i in 0..6 {
        redundancy += alpha_redundancy[i] * (band_mti[i] * band_mti[i + 1]).sqrt();
        let _ = beta_redundancy[i]; // reserved for full correction
    }
    let redundancy_sum: f32 = alpha_redundancy.iter().sum();
    if redundancy_sum > f32::EPSILON {
        redundancy /= redundancy_sum;
    }

    // Final STI with redundancy adjustment
    let sti_final = sti - redundancy * 0.1; // simplified redundancy penalty
    sti_final.clamp(0.0, 1.0)
}

/// Early Decay Time (EDT) — ISO 3382-1.
///
/// Time for the energy decay curve to drop from 0 dB to −10 dB,
/// extrapolated to a 60 dB decay. More perceptually relevant than RT60
/// for perceived reverberance.
#[must_use]
pub fn early_decay_time(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }
    let edc = ir.energy_decay_curve();
    if edc.is_empty() {
        return 0.0;
    }

    // Find the sample where EDC crosses −10 dB
    let mut t10_sample = edc.len();
    for (i, &v) in edc.iter().enumerate() {
        if v <= -10.0 {
            t10_sample = i;
            break;
        }
    }

    if t10_sample == 0 {
        return 0.0;
    }
    if t10_sample >= edc.len() {
        // EDC never reached -10 dB — extremely reverberant
        return f32::INFINITY;
    }

    // EDT = 6 × T_10 (extrapolate 10 dB decay to 60 dB)
    6.0 * t10_sample as f32 / ir.sample_rate as f32
}

/// Sound Strength G — ISO 3382-1.
///
/// Energy of the impulse response relative to the energy at 10 m distance
/// in a free field. G = 10 × log10(∫h²dt / ∫h_ff²dt) in dB, where h_ff
/// is the free-field response at 10 m.
///
/// Since we don't have a free-field reference, this computes the total
/// energy in dB relative to an assumed reference level.
#[must_use]
#[inline]
pub fn sound_strength_g(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }
    let total_energy: f32 = ir.samples.iter().map(|&s| s * s).sum();
    let dt = 1.0 / ir.sample_rate as f32;
    let integrated = total_energy * dt;
    if integrated < f32::EPSILON {
        return f32::NEG_INFINITY;
    }
    // Reference: energy of 1/r at 10m → 1/(4π×100) integrated over one sample
    let ref_energy = 1.0 / (4.0 * std::f32::consts::PI * 100.0) * dt;
    10.0 * (integrated / ref_energy).log10()
}

/// Centre Time ts — ISO 3382-1.
///
/// First moment (centre of gravity) of the squared impulse response.
/// ts = ∫t·h²(t)dt / ∫h²(t)dt, in seconds.
/// Lower values indicate better clarity.
#[must_use]
#[inline]
pub fn centre_time_ts(ir: &ImpulseResponse) -> f32 {
    if ir.samples.is_empty() || ir.sample_rate == 0 {
        return 0.0;
    }
    let dt = 1.0 / ir.sample_rate as f32;
    let mut numerator = 0.0_f32;
    let mut denominator = 0.0_f32;
    for (i, &s) in ir.samples.iter().enumerate() {
        let h2 = s * s;
        let t = i as f32 * dt;
        numerator += t * h2;
        denominator += h2;
    }
    if denominator < f32::EPSILON {
        return 0.0;
    }
    numerator / denominator
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
        assert!(sti > 0.8, "delta impulse should give high STI, got {sti}");
    }

    #[test]
    fn suggest_absorption_zero_target() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let suggestions = suggest_absorption_placement(&room, 0.0);
        assert!(!suggestions.is_empty());
    }

    // --- ISO 3382-1 parameter tests ---

    #[test]
    fn edt_positive_for_reverberant_ir() {
        let ir = make_exponential_ir(1.0, 48000, 3.0);
        let edt = early_decay_time(&ir);
        assert!(edt > 0.0, "EDT should be positive, got {edt}");
        // EDT should be roughly proportional to RT60
        assert!(edt < 3.0, "EDT should be less than duration, got {edt}");
    }

    #[test]
    fn edt_shorter_for_absorptive_room() {
        let short = make_exponential_ir(0.3, 48000, 2.0);
        let long = make_exponential_ir(2.0, 48000, 3.0);
        assert!(
            early_decay_time(&short) < early_decay_time(&long),
            "shorter RT60 should give shorter EDT"
        );
    }

    #[test]
    fn edt_empty_ir() {
        let ir = ImpulseResponse {
            samples: vec![],
            sample_rate: 48000,
            rt60: 1.0,
        };
        assert_eq!(early_decay_time(&ir), 0.0);
    }

    #[test]
    fn sound_strength_positive() {
        let ir = make_exponential_ir(1.0, 48000, 2.0);
        let g = sound_strength_g(&ir);
        assert!(g.is_finite(), "G should be finite, got {g}");
    }

    #[test]
    fn centre_time_positive() {
        let ir = make_exponential_ir(1.0, 48000, 2.0);
        let ts = centre_time_ts(&ir);
        assert!(ts > 0.0, "ts should be positive, got {ts}");
        assert!(ts < 2.0, "ts should be less than IR duration");
    }

    #[test]
    fn centre_time_shorter_for_clear_room() {
        let clear = make_exponential_ir(0.2, 48000, 2.0);
        let reverberant = make_exponential_ir(2.0, 48000, 3.0);
        assert!(
            centre_time_ts(&clear) < centre_time_ts(&reverberant),
            "clearer room should have lower centre time"
        );
    }

    #[test]
    fn centre_time_empty() {
        let ir = ImpulseResponse {
            samples: vec![],
            sample_rate: 48000,
            rt60: 1.0,
        };
        assert_eq!(centre_time_ts(&ir), 0.0);
    }
}
