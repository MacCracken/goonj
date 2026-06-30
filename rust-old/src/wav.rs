//! WAV file export — writes impulse responses as 16-bit PCM WAV files.
//!
//! No external dependency — implements the minimal 44-byte RIFF/WAVE header
//! directly. Supports mono and stereo output.

use crate::error::{GoonjError, Result};
use crate::impulse::ImpulseResponse;
use std::io::Write;

/// Write a mono impulse response as a 16-bit PCM WAV file.
pub fn write_wav_mono(samples: &[f32], sample_rate: u32, writer: &mut impl Write) -> Result<()> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = u32::try_from(samples.len())
        .ok()
        .and_then(|n| n.checked_mul(u32::from(block_align)))
        .ok_or_else(|| {
            GoonjError::ComputationError("WAV data too large for 32-bit header".into())
        })?;
    let file_size = 36 + data_size;

    write_wav_header(
        writer,
        &WavHeader {
            file_size,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            data_size,
        },
    )?;

    // Write PCM data
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let pcm = (clamped * 32767.0) as i16;
        writer
            .write_all(&pcm.to_le_bytes())
            .map_err(|e| GoonjError::ComputationError(e.to_string()))?;
    }

    Ok(())
}

/// Write a stereo WAV file from left and right channel samples.
///
/// Both channels must have the same length.
pub fn write_wav_stereo(
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    writer: &mut impl Write,
) -> Result<()> {
    if left.len() != right.len() {
        return Err(GoonjError::ComputationError(
            "left and right channels must have equal length".into(),
        ));
    }

    let num_channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = u32::try_from(left.len())
        .ok()
        .and_then(|n| n.checked_mul(u32::from(block_align)))
        .ok_or_else(|| {
            GoonjError::ComputationError("WAV data too large for 32-bit header".into())
        })?;
    let file_size = 36 + data_size;

    write_wav_header(
        writer,
        &WavHeader {
            file_size,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            data_size,
        },
    )?;

    // Interleave left/right samples
    for (&l, &r) in left.iter().zip(right.iter()) {
        let l_pcm = (l.clamp(-1.0, 1.0) * 32767.0) as i16;
        let r_pcm = (r.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer
            .write_all(&l_pcm.to_le_bytes())
            .map_err(|e| GoonjError::ComputationError(e.to_string()))?;
        writer
            .write_all(&r_pcm.to_le_bytes())
            .map_err(|e| GoonjError::ComputationError(e.to_string()))?;
    }

    Ok(())
}

struct WavHeader {
    file_size: u32,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data_size: u32,
}

/// Write the 44-byte RIFF/WAVE header.
fn write_wav_header(writer: &mut impl Write, h: &WavHeader) -> Result<()> {
    let write_err = |e: std::io::Error| GoonjError::ComputationError(e.to_string());

    // RIFF header
    writer.write_all(b"RIFF").map_err(write_err)?;
    writer
        .write_all(&h.file_size.to_le_bytes())
        .map_err(write_err)?;
    writer.write_all(b"WAVE").map_err(write_err)?;

    // fmt sub-chunk
    writer.write_all(b"fmt ").map_err(write_err)?;
    writer.write_all(&16_u32.to_le_bytes()).map_err(write_err)?;
    writer.write_all(&1_u16.to_le_bytes()).map_err(write_err)?;
    writer
        .write_all(&h.num_channels.to_le_bytes())
        .map_err(write_err)?;
    writer
        .write_all(&h.sample_rate.to_le_bytes())
        .map_err(write_err)?;
    writer
        .write_all(&h.byte_rate.to_le_bytes())
        .map_err(write_err)?;
    writer
        .write_all(&h.block_align.to_le_bytes())
        .map_err(write_err)?;
    writer
        .write_all(&h.bits_per_sample.to_le_bytes())
        .map_err(write_err)?;

    // data sub-chunk
    writer.write_all(b"data").map_err(write_err)?;
    writer
        .write_all(&h.data_size.to_le_bytes())
        .map_err(write_err)?;

    Ok(())
}

impl ImpulseResponse {
    /// Write this impulse response as a mono WAV file.
    #[cfg(feature = "wav")]
    pub fn to_wav(&self, writer: &mut impl Write) -> Result<()> {
        write_wav_mono(&self.samples, self.sample_rate, writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_wav_header_valid() {
        let samples = vec![0.0_f32; 100];
        let mut buf = Vec::new();
        write_wav_mono(&samples, 48000, &mut buf).unwrap();

        // Check RIFF header
        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        assert_eq!(&buf[36..40], b"data");

        // Total size: 44 header + 100 samples * 2 bytes = 244
        assert_eq!(buf.len(), 244);
    }

    #[test]
    fn stereo_wav_header_valid() {
        let left = vec![0.5_f32; 50];
        let right = vec![-0.5_f32; 50];
        let mut buf = Vec::new();
        write_wav_stereo(&left, &right, 44100, &mut buf).unwrap();

        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(&buf[8..12], b"WAVE");
        // 44 header + 50 samples * 2 channels * 2 bytes = 244
        assert_eq!(buf.len(), 244);
    }

    #[test]
    fn stereo_mismatched_lengths_errors() {
        let left = vec![0.0_f32; 50];
        let right = vec![0.0_f32; 30];
        let mut buf = Vec::new();
        assert!(write_wav_stereo(&left, &right, 48000, &mut buf).is_err());
    }

    #[test]
    fn mono_wav_sample_clamping() {
        // Values outside [-1, 1] should be clamped
        let samples = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let mut buf = Vec::new();
        write_wav_mono(&samples, 48000, &mut buf).unwrap();

        // Read back PCM values
        let pcm_start = 44;
        let s0 = i16::from_le_bytes([buf[pcm_start], buf[pcm_start + 1]]);
        let s4 = i16::from_le_bytes([buf[pcm_start + 8], buf[pcm_start + 9]]);

        // -2.0 clamped to -1.0 → -32767, 2.0 clamped to 1.0 → 32767
        assert_eq!(s0, -32767);
        assert_eq!(s4, 32767);
    }

    #[test]
    fn mono_wav_silence() {
        let samples = vec![0.0_f32; 480];
        let mut buf = Vec::new();
        write_wav_mono(&samples, 48000, &mut buf).unwrap();

        // All PCM values should be 0
        for i in (44..buf.len()).step_by(2) {
            let pcm = i16::from_le_bytes([buf[i], buf[i + 1]]);
            assert_eq!(pcm, 0, "silence should produce zero PCM values");
        }
    }

    #[test]
    fn wav_roundtrip_header_fields() {
        let samples = vec![0.1_f32; 1000];
        let sample_rate: u32 = 44100;
        let mut buf = Vec::new();
        write_wav_mono(&samples, sample_rate, &mut buf).unwrap();

        // Read back header fields
        let sr = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        assert_eq!(sr, sample_rate);

        let channels = u16::from_le_bytes([buf[22], buf[23]]);
        assert_eq!(channels, 1);

        let bps = u16::from_le_bytes([buf[34], buf[35]]);
        assert_eq!(bps, 16);
    }

    #[test]
    fn empty_wav_valid() {
        let mut buf = Vec::new();
        write_wav_mono(&[], 48000, &mut buf).unwrap();
        assert_eq!(buf.len(), 44); // header only, no data
    }
}
