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

    // SN3D normalization for 1st order: W has √2 gain
    let w_gain = std::f32::consts::SQRT_2;

    ir.w[delay_samples] += amplitude * w_gain;
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

/// Compute real spherical harmonics up to given order (ACN ordering, SN3D normalization).
///
/// Returns coefficients for (order+1)² channels.
#[must_use]
fn spherical_harmonics(order: u32, theta: f32, phi: f32) -> Vec<f32> {
    let num = ((order + 1) * (order + 1)) as usize;
    let mut sh = Vec::with_capacity(num);

    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    // Order 0: Y_0^0 = 1
    sh.push(1.0);

    if order >= 1 {
        // Order 1 (ACN 1,2,3): Y_1^{-1}, Y_1^0, Y_1^1
        sh.push(sin_theta * phi.sin()); // ACN 1: Y_1^{-1}
        sh.push(cos_theta); // ACN 2: Y_1^0
        sh.push(sin_theta * phi.cos()); // ACN 3: Y_1^1
    }

    if order >= 2 {
        // Order 2 (ACN 4..8)
        let sin2 = sin_theta * sin_theta;
        let cos2 = cos_theta * cos_theta;
        sh.push(sin2 * (2.0 * phi).sin()); // ACN 4: Y_2^{-2}
        sh.push(sin_theta * cos_theta * phi.sin()); // ACN 5: Y_2^{-1}
        sh.push(1.5 * cos2 - 0.5); // ACN 6: Y_2^0
        sh.push(sin_theta * cos_theta * phi.cos()); // ACN 7: Y_2^1
        sh.push(sin2 * (2.0 * phi).cos()); // ACN 8: Y_2^2
    }

    if order >= 3 {
        // Order 3 (ACN 9..15)
        let sin2 = sin_theta * sin_theta;
        let sin3 = sin2 * sin_theta;
        let cos2 = cos_theta * cos_theta;
        sh.push(sin3 * (3.0 * phi).sin()); // ACN 9
        sh.push(sin2 * cos_theta * (2.0 * phi).sin()); // ACN 10
        sh.push(sin_theta * (5.0 * cos2 - 1.0) * phi.sin()); // ACN 11
        sh.push(2.5 * cos2 * cos_theta - 1.5 * cos_theta); // ACN 12
        sh.push(sin_theta * (5.0 * cos2 - 1.0) * phi.cos()); // ACN 13
        sh.push(sin2 * cos_theta * (2.0 * phi).cos()); // ACN 14
        sh.push(sin3 * (3.0 * phi).cos()); // ACN 15
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
        // W should have √2 gain, X should have 1.0, Y and Z should be 0
        assert!(ir.w[0] > 1.0, "W should have √2 scaling");
        assert!((ir.x[0] - 1.0).abs() < 0.01);
        assert!(ir.y[0].abs() < 0.01);
        assert!(ir.z[0].abs() < 0.01);
    }

    #[test]
    fn bformat_up_direction() {
        let mut ir = new_bformat_ir(100, 48000);
        encode_bformat(1.0, Vec3::Y, 0, &mut ir);
        assert!(ir.w[0] > 1.0);
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
        assert_eq!(sh.len(), 1);
        assert!((sh[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn spherical_harmonics_order_1_count() {
        let sh = spherical_harmonics(1, 0.5, 0.3);
        assert_eq!(sh.len(), 4);
    }

    #[test]
    fn spherical_harmonics_order_3_count() {
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
