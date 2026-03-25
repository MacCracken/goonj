use serde::{Deserialize, Serialize};

/// An impulse response — the acoustic fingerprint of a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpulseResponse {
    /// Audio samples of the impulse response.
    pub samples: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Estimated RT60 (reverberation time) in seconds.
    pub rt60: f32,
}

impl ImpulseResponse {
    /// Compute the energy decay curve (Schroeder backward integration).
    #[must_use]
    pub fn energy_decay_curve(&self) -> Vec<f32> {
        let mut edc = vec![0.0_f32; self.samples.len()];
        let mut cumulative = 0.0;

        // Backward integration of squared samples
        for i in (0..self.samples.len()).rev() {
            cumulative += self.samples[i] * self.samples[i];
            edc[i] = cumulative;
        }

        // Normalize to 0 dB at start
        let max = edc.first().copied().unwrap_or(1.0).max(f32::EPSILON);
        for v in &mut edc {
            *v = 10.0 * (*v / max).log10();
        }

        edc
    }

    /// Duration of the impulse response in seconds.
    #[must_use]
    #[inline]
    pub fn duration_seconds(&self) -> f32 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / self.sample_rate as f32
    }
}

/// Sabine reverberation time (RT60).
///
/// T = 0.161 × V / A
///
/// Where V = room volume (m³), A = total absorption area (m²·Sabins).
#[must_use]
#[inline]
pub fn sabine_rt60(volume: f32, total_absorption: f32) -> f32 {
    if total_absorption <= 0.0 {
        return f32::INFINITY;
    }
    0.161 * volume / total_absorption
}

/// Eyring reverberation time (RT60), more accurate for rooms with higher absorption.
///
/// T = 0.161 × V / (-S × ln(1 - ā))
///
/// Where S = total surface area, ā = average absorption coefficient.
#[must_use]
pub fn eyring_rt60(volume: f32, surface_area: f32, average_absorption: f32) -> f32 {
    if average_absorption <= 0.0 || average_absorption >= 1.0 || surface_area <= 0.0 {
        return f32::INFINITY;
    }
    let denominator = -surface_area * (1.0 - average_absorption).ln();
    if denominator <= 0.0 {
        return f32::INFINITY;
    }
    0.161 * volume / denominator
}

/// Estimate RT60 for a shoebox room from dimensions and material.
#[must_use]
pub fn estimate_rt60_shoebox(length: f32, width: f32, height: f32, avg_absorption: f32) -> f32 {
    let volume = length * width * height;
    let surface_area = 2.0 * (length * width + length * height + width * height);
    let total_absorption = surface_area * avg_absorption;
    sabine_rt60(volume, total_absorption)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sabine_basic() {
        // 240 m³ room with 50 m² absorption → RT60 ≈ 0.773 s
        let rt60 = sabine_rt60(240.0, 50.0);
        assert!((rt60 - 0.773).abs() < 0.01, "Sabine RT60 should be ~0.773s, got {rt60}");
    }

    #[test]
    fn sabine_zero_absorption() {
        assert!(sabine_rt60(100.0, 0.0).is_infinite());
    }

    #[test]
    fn sabine_higher_absorption_shorter_rt60() {
        let rt60_low = sabine_rt60(100.0, 10.0);
        let rt60_high = sabine_rt60(100.0, 50.0);
        assert!(rt60_high < rt60_low);
    }

    #[test]
    fn sabine_larger_room_longer_rt60() {
        let small = sabine_rt60(50.0, 20.0);
        let large = sabine_rt60(200.0, 20.0);
        assert!(large > small);
    }

    #[test]
    fn eyring_converges_to_sabine_low_absorption() {
        // For low absorption, Eyring ≈ Sabine
        let volume = 240.0;
        let surface = 268.0;
        let avg_abs = 0.02; // low absorption (concrete)
        let sabine = sabine_rt60(volume, surface * avg_abs);
        let eyring = eyring_rt60(volume, surface, avg_abs);
        let diff = (sabine - eyring).abs() / sabine;
        assert!(diff < 0.05, "Eyring should be within 5% of Sabine at low absorption, diff={diff:.3}");
    }

    #[test]
    fn eyring_shorter_than_sabine_high_absorption() {
        let volume = 100.0;
        let surface = 150.0;
        let avg_abs = 0.5;
        let sabine = sabine_rt60(volume, surface * avg_abs);
        let eyring = eyring_rt60(volume, surface, avg_abs);
        assert!(eyring < sabine, "Eyring should be shorter than Sabine at high absorption");
    }

    #[test]
    fn shoebox_estimate_10x8x3_concrete() {
        let rt60 = estimate_rt60_shoebox(10.0, 8.0, 3.0, 0.02);
        // Volume=240, Surface=268, A=5.36, RT60 ≈ 7.2s (very reverberant, concrete)
        assert!(rt60 > 5.0 && rt60 < 10.0, "Concrete room should be very reverberant, got {rt60}");
    }

    #[test]
    fn impulse_response_duration() {
        let ir = ImpulseResponse {
            samples: vec![0.0; 48000],
            sample_rate: 48000,
            rt60: 1.0,
        };
        assert!((ir.duration_seconds() - 1.0).abs() < 0.001);
    }

    #[test]
    fn energy_decay_curve_monotonic() {
        let ir = ImpulseResponse {
            samples: (0..1000).map(|i| (-0.005 * i as f32).exp() * 0.5).collect(),
            sample_rate: 48000,
            rt60: 1.0,
        };
        let edc = ir.energy_decay_curve();
        // EDC should be monotonically decreasing
        for window in edc.windows(2) {
            assert!(window[0] >= window[1], "EDC should decrease monotonically");
        }
    }
}
