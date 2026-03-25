use crate::diffuse::{DiffuseRainConfig, generate_diffuse_rain};
use crate::image_source::compute_early_reflections;
use crate::propagation::speed_of_sound;
use crate::room::AcousticRoom;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// An impulse response — the acoustic fingerprint of a room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[tracing::instrument(skip(self), fields(samples = self.samples.len(), sample_rate = self.sample_rate))]
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
#[tracing::instrument]
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
#[tracing::instrument]
pub fn estimate_rt60_shoebox(length: f32, width: f32, height: f32, avg_absorption: f32) -> f32 {
    let volume = length * width * height;
    let surface_area = 2.0 * (length * width + length * height + width * height);
    let total_absorption = surface_area * avg_absorption;
    sabine_rt60(volume, total_absorption)
}

/// Configuration for full impulse response generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrConfig {
    /// Sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// Maximum image-source reflection order for early reflections.
    pub max_order: u32,
    /// Number of diffuse rain rays for late reverb.
    pub num_diffuse_rays: u32,
    /// Maximum diffuse rain bounces per ray.
    pub max_bounces: u32,
    /// Maximum IR length in seconds.
    pub max_time_seconds: f32,
    /// Random seed for diffuse rain reproducibility.
    pub seed: u64,
}

impl Default for IrConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            max_order: 3,
            num_diffuse_rays: 5000,
            max_bounces: 50,
            max_time_seconds: 2.0,
            seed: 42,
        }
    }
}

/// A multiband impulse response with one IR per frequency band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultibandIr {
    /// One IR buffer per frequency band (125, 250, 500, 1000, 2000, 4000 Hz).
    pub bands: [Vec<f32>; 6],
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Estimated RT60 in seconds.
    pub rt60: f32,
}

impl MultibandIr {
    /// Convert to a broadband (mono) impulse response by summing all bands.
    #[must_use]
    pub fn to_broadband(&self) -> ImpulseResponse {
        let len = self.bands[0].len();
        let mut samples = vec![0.0_f32; len];
        for band in &self.bands {
            for (i, &s) in band.iter().enumerate() {
                samples[i] += s;
            }
        }
        // Normalize
        let max_abs = samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        if max_abs > f32::EPSILON {
            for s in &mut samples {
                *s /= max_abs;
            }
        }
        ImpulseResponse {
            samples,
            sample_rate: self.sample_rate,
            rt60: self.rt60,
        }
    }
}

/// Generate a full impulse response for a room by combining image-source
/// early reflections with diffuse rain late reverb.
///
/// Returns a multiband IR with per-frequency-band data.
#[must_use]
#[tracing::instrument(skip(room, config), fields(
    sample_rate = config.sample_rate,
    max_order = config.max_order,
    num_diffuse_rays = config.num_diffuse_rays,
))]
pub fn generate_ir(
    source: Vec3,
    listener: Vec3,
    room: &AcousticRoom,
    config: &IrConfig,
) -> MultibandIr {
    let c = speed_of_sound(room.temperature_celsius);
    let num_samples = (config.max_time_seconds * config.sample_rate as f32) as usize;
    let mut bands = std::array::from_fn(|_| vec![0.0_f32; num_samples]);

    // --- Early reflections (image-source method) ---
    let early = compute_early_reflections(source, listener, room, config.max_order, c);
    for refl in &early {
        let sample_idx = (refl.delay_seconds * config.sample_rate as f32) as usize;
        if sample_idx < num_samples {
            for (band, buf) in bands.iter_mut().enumerate() {
                buf[sample_idx] += refl.amplitude[band];
            }
        }
    }

    // --- Late reverb (diffuse rain) ---
    let diffuse_config = DiffuseRainConfig {
        num_rays: config.num_diffuse_rays,
        max_bounces: config.max_bounces,
        max_time_seconds: config.max_time_seconds,
        collection_radius: 0.0, // auto
        speed_of_sound: c,
        seed: config.seed,
    };
    let diffuse = generate_diffuse_rain(source, listener, room, &diffuse_config);
    for contrib in &diffuse.contributions {
        let sample_idx = (contrib.time_seconds * config.sample_rate as f32) as usize;
        if sample_idx < num_samples {
            for (band, buf) in bands.iter_mut().enumerate() {
                buf[sample_idx] += contrib.energy[band];
            }
        }
    }

    // Estimate RT60 from broadband energy
    let volume = room.geometry.volume_shoebox();
    let absorption = room.geometry.total_absorption();
    let rt60 = sabine_rt60(volume, absorption);

    MultibandIr {
        bands,
        sample_rate: config.sample_rate,
        rt60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;

    #[test]
    fn generate_ir_produces_nonzero_samples() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = IrConfig {
            num_diffuse_rays: 500,
            max_time_seconds: 0.5,
            ..IrConfig::default()
        };

        let ir = generate_ir(source, listener, &room, &config);
        assert_eq!(ir.bands[0].len(), (0.5 * 48000.0) as usize);

        // At least some bands should have nonzero content (from early reflections)
        let has_content = ir
            .bands
            .iter()
            .any(|b| b.iter().any(|&s| s.abs() > f32::EPSILON));
        assert!(has_content, "generated IR should have nonzero content");
    }

    #[test]
    fn generate_ir_broadband_has_content() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = IrConfig {
            num_diffuse_rays: 500,
            max_time_seconds: 0.5,
            ..IrConfig::default()
        };

        let ir = generate_ir(source, listener, &room, &config);
        let broadband = ir.to_broadband();
        assert!(!broadband.samples.is_empty());
        let max_abs = broadband
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        assert!(
            (max_abs - 1.0).abs() < f32::EPSILON || max_abs < f32::EPSILON,
            "broadband should be normalized"
        );
    }

    #[test]
    fn multiband_ir_band_count() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let config = IrConfig {
            num_diffuse_rays: 100,
            max_time_seconds: 0.2,
            ..IrConfig::default()
        };
        let ir = generate_ir(
            Vec3::new(5.0, 1.5, 4.0),
            Vec3::new(5.0, 1.5, 4.0 + 0.5),
            &room,
            &config,
        );
        assert_eq!(ir.bands.len(), 6);
    }

    #[test]
    fn ir_config_default() {
        let config = IrConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.max_order, 3);
        assert_eq!(config.num_diffuse_rays, 5000);
    }

    #[test]
    fn sabine_basic() {
        // 240 m³ room with 50 m² absorption → RT60 ≈ 0.773 s
        let rt60 = sabine_rt60(240.0, 50.0);
        assert!(
            (rt60 - 0.773).abs() < 0.01,
            "Sabine RT60 should be ~0.773s, got {rt60}"
        );
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
        assert!(
            diff < 0.05,
            "Eyring should be within 5% of Sabine at low absorption, diff={diff:.3}"
        );
    }

    #[test]
    fn eyring_shorter_than_sabine_high_absorption() {
        let volume = 100.0;
        let surface = 150.0;
        let avg_abs = 0.5;
        let sabine = sabine_rt60(volume, surface * avg_abs);
        let eyring = eyring_rt60(volume, surface, avg_abs);
        assert!(
            eyring < sabine,
            "Eyring should be shorter than Sabine at high absorption"
        );
    }

    #[test]
    fn shoebox_estimate_10x8x3_concrete() {
        let rt60 = estimate_rt60_shoebox(10.0, 8.0, 3.0, 0.02);
        // Volume=240, Surface=268, A=5.36, RT60 ≈ 7.2s (very reverberant, concrete)
        assert!(
            rt60 > 5.0 && rt60 < 10.0,
            "Concrete room should be very reverberant, got {rt60}"
        );
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

    #[test]
    fn energy_decay_curve_empty_samples() {
        let ir = ImpulseResponse {
            samples: vec![],
            sample_rate: 48000,
            rt60: 1.0,
        };
        let edc = ir.energy_decay_curve();
        assert!(edc.is_empty());
    }

    #[test]
    fn duration_zero_sample_rate() {
        let ir = ImpulseResponse {
            samples: vec![1.0; 100],
            sample_rate: 0,
            rt60: 1.0,
        };
        assert_eq!(ir.duration_seconds(), 0.0);
    }
}
