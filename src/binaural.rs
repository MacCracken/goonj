//! Binaural impulse response generation using user-provided HRTF data.
//!
//! Combines room acoustics simulation with Head-Related Transfer Functions
//! (HRTFs) to produce stereo impulse responses suitable for headphone
//! spatialization.

use crate::image_source::compute_early_reflections;
use crate::impulse::IrConfig;
use crate::propagation::speed_of_sound;
use crate::room::AcousticRoom;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A pair of head-related impulse responses (left/right ear) for one direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrtfPair {
    /// Azimuth angle in radians (-π to π, 0 = front).
    pub azimuth: f32,
    /// Elevation angle in radians (-π/2 to π/2, 0 = horizontal).
    pub elevation: f32,
    /// Left ear impulse response.
    pub left: Vec<f32>,
    /// Right ear impulse response.
    pub right: Vec<f32>,
}

/// A dataset of HRTF pairs indexed by direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrtfDataset {
    /// Collection of HRTF pairs at measured directions.
    pub pairs: Vec<HrtfPair>,
    /// Sample rate of the HRIR data.
    pub sample_rate: u32,
}

impl HrtfDataset {
    /// Create a dataset from a collection of pre-loaded HRTF pairs.
    #[must_use]
    pub fn from_pairs(pairs: Vec<HrtfPair>, sample_rate: u32) -> Self {
        Self { pairs, sample_rate }
    }

    /// Find the nearest HRTF pair for a given direction (azimuth, elevation).
    #[must_use]
    pub fn nearest(&self, azimuth: f32, elevation: f32) -> Option<&HrtfPair> {
        self.pairs.iter().min_by(|a, b| {
            let da = angular_distance(a.azimuth, a.elevation, azimuth, elevation);
            let db = angular_distance(b.azimuth, b.elevation, azimuth, elevation);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Squared angular distance between two directions (for nearest-neighbor lookup).
#[must_use]
#[inline]
fn angular_distance(az1: f32, el1: f32, az2: f32, el2: f32) -> f32 {
    let daz = (az1 - az2).abs();
    let daz = daz.min(std::f32::consts::TAU - daz); // wrap around
    let del = (el1 - el2).abs();
    daz * daz + del * del
}

/// A binaural (stereo) impulse response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinauralIr {
    /// Left ear samples.
    pub left: Vec<f32>,
    /// Right ear samples.
    pub right: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

#[cfg(feature = "wav")]
impl BinauralIr {
    /// Write as a stereo WAV file.
    pub fn to_wav(&self, writer: &mut impl std::io::Write) -> crate::error::Result<()> {
        crate::wav::write_wav_stereo(&self.left, &self.right, self.sample_rate, writer)
    }
}

/// Convert a direction vector to azimuth and elevation angles.
///
/// Azimuth: angle in the XZ plane from +Z axis (0 = front, positive = right).
/// Elevation: angle from the horizontal plane (positive = up).
#[must_use]
#[inline]
fn direction_to_angles(dir: Vec3) -> (f32, f32) {
    let azimuth = dir.x.atan2(dir.z);
    let horizontal = (dir.x * dir.x + dir.z * dir.z).sqrt();
    let elevation = dir.y.atan2(horizontal);
    (azimuth, elevation)
}

/// Generate a binaural impulse response for a room.
///
/// Computes early reflections using the image-source method, spatializes each
/// reflection using the nearest HRTF pair from the dataset, and sums into
/// left/right channels.
#[must_use]
#[tracing::instrument(skip(room, hrtf, config), fields(
    sample_rate = config.sample_rate,
    max_order = config.max_order,
    hrtf_pairs = hrtf.pairs.len(),
))]
pub fn generate_binaural_ir(
    source: Vec3,
    listener: Vec3,
    room: &AcousticRoom,
    hrtf: &HrtfDataset,
    config: &IrConfig,
) -> BinauralIr {
    let c = speed_of_sound(room.temperature_celsius);
    let num_samples = (config.max_time_seconds * config.sample_rate as f32) as usize;
    let hrir_len = hrtf.pairs.first().map(|p| p.left.len()).unwrap_or(0);

    let mut left = vec![0.0_f32; num_samples + hrir_len];
    let mut right = vec![0.0_f32; num_samples + hrir_len];

    let reflections = compute_early_reflections(source, listener, room, config.max_order, c);

    for refl in &reflections {
        let sample_idx = (refl.delay_seconds * config.sample_rate as f32) as usize;
        if sample_idx >= num_samples {
            continue;
        }

        // Average amplitude across bands for broadband HRTF application
        let amp: f32 = refl.amplitude.iter().sum::<f32>() / 6.0;
        if amp < f32::EPSILON {
            continue;
        }

        // Find direction-matched HRTF
        let (az, el) = direction_to_angles(refl.direction);
        let Some(hrtf_pair) = hrtf.nearest(az, el) else {
            continue;
        };

        // Convolve (add scaled HRIR at the reflection's arrival time)
        for (i, (&hl, &hr)) in hrtf_pair
            .left
            .iter()
            .zip(hrtf_pair.right.iter())
            .enumerate()
        {
            let idx = sample_idx + i;
            if idx < left.len() {
                left[idx] += amp * hl;
                right[idx] += amp * hr;
            }
        }
    }

    // Trim to requested length
    left.truncate(num_samples);
    right.truncate(num_samples);

    BinauralIr {
        left,
        right,
        sample_rate: config.sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;

    fn simple_hrtf() -> HrtfDataset {
        // Minimal HRTF dataset: front and right
        let front = HrtfPair {
            azimuth: 0.0,
            elevation: 0.0,
            left: vec![1.0, 0.5, 0.2],
            right: vec![1.0, 0.5, 0.2],
        };
        let right = HrtfPair {
            azimuth: std::f32::consts::FRAC_PI_2,
            elevation: 0.0,
            left: vec![0.3, 0.1, 0.05], // quieter on left
            right: vec![1.0, 0.8, 0.4], // louder on right
        };
        let left_dir = HrtfPair {
            azimuth: -std::f32::consts::FRAC_PI_2,
            elevation: 0.0,
            left: vec![1.0, 0.8, 0.4],
            right: vec![0.3, 0.1, 0.05],
        };
        HrtfDataset::from_pairs(vec![front, right, left_dir], 48000)
    }

    #[test]
    fn binaural_ir_has_two_channels() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let hrtf = simple_hrtf();
        let config = IrConfig {
            max_time_seconds: 0.1,
            num_diffuse_rays: 0,
            ..IrConfig::default()
        };
        let ir = generate_binaural_ir(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            &hrtf,
            &config,
        );
        assert_eq!(ir.left.len(), ir.right.len());
        assert!(!ir.left.is_empty());
    }

    #[test]
    fn binaural_ir_has_content() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let hrtf = simple_hrtf();
        let config = IrConfig {
            max_time_seconds: 0.1,
            num_diffuse_rays: 0,
            ..IrConfig::default()
        };
        let ir = generate_binaural_ir(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            &hrtf,
            &config,
        );
        let left_energy: f32 = ir.left.iter().map(|&s| s * s).sum();
        let right_energy: f32 = ir.right.iter().map(|&s| s * s).sum();
        assert!(left_energy > 0.0, "left channel should have content");
        assert!(right_energy > 0.0, "right channel should have content");
    }

    #[test]
    fn hrtf_nearest_finds_closest() {
        let hrtf = simple_hrtf();
        let front = hrtf.nearest(0.1, 0.0).unwrap();
        assert!(
            front.azimuth.abs() < 0.01,
            "should find front HRTF for near-front direction"
        );
    }

    #[test]
    fn direction_to_angles_front() {
        let (az, el) = direction_to_angles(Vec3::Z);
        assert!(az.abs() < 0.01, "front should be azimuth ~0, got {az}");
        assert!(el.abs() < 0.01, "front should be elevation ~0, got {el}");
    }

    #[test]
    fn direction_to_angles_right() {
        let (az, _el) = direction_to_angles(Vec3::X);
        assert!(
            (az - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "right should be azimuth ~π/2, got {az}"
        );
    }

    #[test]
    fn direction_to_angles_up() {
        let (_az, el) = direction_to_angles(Vec3::Y);
        assert!(
            (el - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "up should be elevation ~π/2, got {el}"
        );
    }

    #[test]
    fn angular_distance_same_direction() {
        let d = angular_distance(0.0, 0.0, 0.0, 0.0);
        assert!(d.abs() < f32::EPSILON);
    }

    #[test]
    fn angular_distance_wrap_around() {
        // π and -π should be close (same direction in azimuth)
        let d = angular_distance(
            std::f32::consts::PI - 0.01,
            0.0,
            -std::f32::consts::PI + 0.01,
            0.0,
        );
        assert!(d < 0.01, "wrapped azimuth should be close, got {d}");
    }

    #[test]
    fn empty_hrtf_produces_silent_ir() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let hrtf = HrtfDataset::from_pairs(vec![], 48000);
        let config = IrConfig {
            max_time_seconds: 0.1,
            num_diffuse_rays: 0,
            ..IrConfig::default()
        };
        let ir = generate_binaural_ir(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            &hrtf,
            &config,
        );
        let total: f32 = ir
            .left
            .iter()
            .chain(ir.right.iter())
            .map(|&s| s.abs())
            .sum();
        assert!(
            total < f32::EPSILON,
            "empty HRTF should produce silent output"
        );
    }
}
