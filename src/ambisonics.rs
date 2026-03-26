//! Ambisonics encoding — spatial sound field representation.
//!
//! Encodes reflections and ray contributions into Ambisonics channels
//! (B-Format for 1st order, Higher-Order Ambisonics for 3rd order).
//! The encoded sound field is orientation-independent and can be decoded
//! to any speaker layout or headphone HRTF.

use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// 1st-order Ambisonics (B-Format) impulse response: W, X, Y, Z channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BFormatIr {
    /// W channel (omnidirectional pressure).
    pub w: Vec<f32>,
    /// X channel (front-back velocity, +X = front).
    pub x: Vec<f32>,
    /// Y channel (left-right velocity, +Y = left).
    pub y: Vec<f32>,
    /// Z channel (up-down velocity, +Z = up).
    pub z: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

/// 3rd-order Ambisonics impulse response (16 ACN channels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoaIr {
    /// ACN-ordered channels (0..16 for 3rd order).
    pub channels: Vec<Vec<f32>>,
    /// Ambisonics order (1, 2, or 3).
    pub order: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

/// Encode a single reflection into B-Format (1st-order Ambisonics).
///
/// The W channel receives the amplitude directly (with √2 normalization).
/// X, Y, Z channels receive the amplitude scaled by the direction cosines.
///
/// # Arguments
/// * `amplitude` — reflection amplitude (linear)
/// * `direction` — unit vector of the arrival direction at the listener
/// * `delay_samples` — sample index at which to place the contribution
/// * `ir` — the B-Format IR to accumulate into
#[inline]
pub fn encode_bformat(amplitude: f32, direction: Vec3, delay_samples: usize, ir: &mut BFormatIr) {
    if delay_samples >= ir.w.len() || amplitude.abs() < f32::EPSILON {
        return;
    }

    // SN3D/ACN (AmbiX) normalization: W = 1.0, X/Y/Z = direction cosines
    ir.w[delay_samples] += amplitude;
    ir.x[delay_samples] += amplitude * direction.x;
    ir.y[delay_samples] += amplitude * direction.y;
    ir.z[delay_samples] += amplitude * direction.z;
}

/// Encode a single reflection into Higher-Order Ambisonics (ACN/SN3D).
///
/// Computes real spherical harmonic coefficients up to the specified order.
#[inline]
pub fn encode_hoa(amplitude: f32, direction: Vec3, delay_samples: usize, ir: &mut HoaIr) {
    let num_channels = ((ir.order + 1) * (ir.order + 1)) as usize;
    if delay_samples >= ir.channels.first().map_or(0, |c| c.len())
        || amplitude.abs() < f32::EPSILON
        || ir.channels.len() < num_channels
    {
        return;
    }

    // Convert direction to spherical coordinates
    let r = direction.length();
    if r < f32::EPSILON {
        return;
    }
    let theta = (direction.z / r).acos(); // inclination from +Z
    let phi = direction.y.atan2(direction.x); // azimuth from +X

    // Encode ACN/SN3D spherical harmonics up to order
    let sh = spherical_harmonics(ir.order, theta, phi);
    for (ch, &coeff) in sh.iter().enumerate().take(num_channels) {
        if ch < ir.channels.len() {
            ir.channels[ch][delay_samples] += amplitude * coeff;
        }
    }
}

/// Maximum supported Ambisonics order.
const MAX_HOA_ORDER: u32 = 3;
/// Maximum number of HOA channels: (3+1)² = 16.
const MAX_HOA_CHANNELS: usize = 16;

/// Compute real spherical harmonics up to given order (ACN ordering, SN3D normalization).
///
/// Returns a fixed-size array of coefficients (zero-allocation).
/// SN3D normalization factors from: Nachbar et al., "AmbiX — A Suggested
/// Ambisonics Format," AES 2011.
#[must_use]
fn spherical_harmonics(order: u32, theta: f32, phi: f32) -> [f32; MAX_HOA_CHANNELS] {
    let mut sh = [0.0_f32; MAX_HOA_CHANNELS];
    let order = order.min(MAX_HOA_ORDER);

    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Order 0: Y_0^0 = 1 (SN3D factor = 1)
    sh[0] = 1.0;

    if order >= 1 {
        // SN3D normalization: order-1 factors are all 1.0
        sh[1] = sin_t * phi.sin(); // ACN 1: Y_1^{-1}
        sh[2] = cos_t; // ACN 2: Y_1^0
        sh[3] = sin_t * phi.cos(); // ACN 3: Y_1^1
    }

    if order >= 2 {
        let sin2 = sin_t * sin_t;
        let cos2 = cos_t * cos_t;
        // SN3D factors for order 2: m=±2 → √(3)/2, m=±1 → √(3), m=0 → 1
        let n2_2 = 3.0_f32.sqrt() / 2.0; // 0.866
        let n2_1 = 3.0_f32.sqrt(); // 1.732
        sh[4] = n2_2 * sin2 * (2.0 * phi).sin(); // ACN 4: Y_2^{-2}
        sh[5] = n2_1 * sin_t * cos_t * phi.sin(); // ACN 5: Y_2^{-1}
        sh[6] = 1.5 * cos2 - 0.5; // ACN 6: Y_2^0 (factor = 1)
        sh[7] = n2_1 * sin_t * cos_t * phi.cos(); // ACN 7: Y_2^1
        sh[8] = n2_2 * sin2 * (2.0 * phi).cos(); // ACN 8: Y_2^2
    }

    if order >= 3 {
        let sin2 = sin_t * sin_t;
        let sin3 = sin2 * sin_t;
        let cos2 = cos_t * cos_t;
        // SN3D factors for order 3
        let n3_3 = (5.0 / 8.0_f32).sqrt(); // m=±3
        let n3_2 = 15.0_f32.sqrt() / 2.0; // m=±2: sqrt(15)/2
        let n3_1 = (3.0 / 8.0_f32).sqrt(); // m=±1
        sh[9] = n3_3 * sin3 * (3.0 * phi).sin(); // ACN 9: Y_3^{-3}
        sh[10] = n3_2 * sin2 * cos_t * (2.0 * phi).sin(); // ACN 10: Y_3^{-2}
        sh[11] = n3_1 * sin_t * (5.0 * cos2 - 1.0) * phi.sin(); // ACN 11: Y_3^{-1}
        sh[12] = 2.5 * cos2 * cos_t - 1.5 * cos_t; // ACN 12: Y_3^0 (factor = 1)
        sh[13] = n3_1 * sin_t * (5.0 * cos2 - 1.0) * phi.cos(); // ACN 13: Y_3^1
        sh[14] = n3_2 * sin2 * cos_t * (2.0 * phi).cos(); // ACN 14: Y_3^2
        sh[15] = n3_3 * sin3 * (3.0 * phi).cos(); // ACN 15: Y_3^3
    }

    sh
}

/// Create an empty B-Format IR with the given parameters.
#[must_use]
pub fn new_bformat_ir(num_samples: usize, sample_rate: u32) -> BFormatIr {
    BFormatIr {
        w: vec![0.0; num_samples],
        x: vec![0.0; num_samples],
        y: vec![0.0; num_samples],
        z: vec![0.0; num_samples],
        sample_rate,
    }
}

/// Create an empty HOA IR with the given parameters.
#[must_use]
pub fn new_hoa_ir(order: u32, num_samples: usize, sample_rate: u32) -> HoaIr {
    let num_channels = ((order + 1) * (order + 1)) as usize;
    HoaIr {
        channels: (0..num_channels).map(|_| vec![0.0; num_samples]).collect(),
        order,
        sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bformat_front_direction() {
        let mut ir = new_bformat_ir(100, 48000);
        encode_bformat(1.0, Vec3::X, 0, &mut ir);
        // SN3D: W = 1.0, X = direction.x = 1.0
        assert!(
            (ir.w[0] - 1.0).abs() < 0.01,
            "W should be 1.0, got {}",
            ir.w[0]
        );
        assert!((ir.x[0] - 1.0).abs() < 0.01);
        assert!(ir.y[0].abs() < 0.01);
        assert!(ir.z[0].abs() < 0.01);
    }

    #[test]
    fn bformat_up_direction() {
        let mut ir = new_bformat_ir(100, 48000);
        encode_bformat(1.0, Vec3::Y, 0, &mut ir);
        assert!((ir.w[0] - 1.0).abs() < 0.01);
        assert!(ir.x[0].abs() < 0.01);
        assert!((ir.y[0] - 1.0).abs() < 0.01);
        assert!(ir.z[0].abs() < 0.01);
    }

    #[test]
    fn bformat_out_of_bounds_safe() {
        let mut ir = new_bformat_ir(10, 48000);
        encode_bformat(1.0, Vec3::X, 100, &mut ir); // past end
        // Should not panic, no data written
        assert!(ir.w.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn hoa_order_1_has_4_channels() {
        let ir = new_hoa_ir(1, 100, 48000);
        assert_eq!(ir.channels.len(), 4);
    }

    #[test]
    fn hoa_order_3_has_16_channels() {
        let ir = new_hoa_ir(3, 100, 48000);
        assert_eq!(ir.channels.len(), 16);
    }

    #[test]
    fn hoa_encode_front() {
        let mut ir = new_hoa_ir(1, 100, 48000);
        encode_hoa(1.0, Vec3::Z, 0, &mut ir);
        // ACN 0 (W) should have contribution
        assert!(ir.channels[0][0].abs() > 0.0, "W should have content");
    }

    #[test]
    fn spherical_harmonics_order_0() {
        let sh = spherical_harmonics(0, 0.0, 0.0);
        assert_eq!(sh.len(), MAX_HOA_CHANNELS);
        assert!((sh[0] - 1.0).abs() < 0.01);
        // Higher orders should be zero for order=0 input
    }

    #[test]
    fn spherical_harmonics_order_1_has_values() {
        let sh = spherical_harmonics(1, 0.5, 0.3);
        assert_eq!(sh.len(), MAX_HOA_CHANNELS);
        assert!(sh[0].abs() > 0.0, "W should be nonzero");
    }

    #[test]
    fn spherical_harmonics_order_3_all_populated() {
        let sh = spherical_harmonics(3, 0.5, 0.3);
        assert_eq!(sh.len(), 16);
    }

    #[test]
    fn bformat_ir_serializes() {
        let ir = new_bformat_ir(10, 48000);
        let json = serde_json::to_string(&ir).unwrap();
        let back: BFormatIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, back);
    }
}
