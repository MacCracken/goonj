//! dhvani integration — IR handoff for convolution reverb.
//!
//! Provides [`DhvaniIr`], a wrapper around [`ImpulseResponse`](crate::impulse::ImpulseResponse)
//! with additional metadata that dhvani needs for convolution reverb processing.

use crate::impulse::{ImpulseResponse, IrConfig, MultibandIr, generate_ir, sabine_rt60};
use crate::room::AcousticRoom;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// An impulse response packaged for dhvani's convolution reverb engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DhvaniIr {
    /// The broadband impulse response.
    pub ir: ImpulseResponse,
    /// Per-band impulse responses (optional, for frequency-dependent reverb).
    pub multiband: Option<MultibandIr>,
    /// Room volume in m³.
    pub room_volume: f32,
    /// Per-band RT60 estimates in seconds.
    pub rt60_bands: [f32; 6],
}

/// Generate an IR ready for dhvani consumption.
#[must_use]
#[tracing::instrument(skip(room, config), fields(
    sample_rate = config.sample_rate,
))]
pub fn generate_dhvani_ir(
    source: Vec3,
    listener: Vec3,
    room: &AcousticRoom,
    config: &IrConfig,
) -> DhvaniIr {
    let multiband = generate_ir(source, listener, room, config);
    let broadband = multiband.to_broadband();
    let volume = room.geometry.volume_shoebox();

    // Estimate per-band RT60 using Sabine with per-band absorption
    let _surface_area = room.geometry.surface_area();
    let rt60_bands = std::array::from_fn(|band| {
        let total_abs: f32 = room
            .geometry
            .walls
            .iter()
            .map(|w| w.area() * w.material.absorption[band])
            .sum();
        if total_abs > 0.0 {
            sabine_rt60(volume, total_abs)
        } else {
            f32::INFINITY
        }
    });

    DhvaniIr {
        ir: broadband,
        multiband: Some(multiband),
        room_volume: volume,
        rt60_bands,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;

    #[test]
    fn dhvani_ir_has_valid_data() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let config = IrConfig {
            num_diffuse_rays: 100,
            max_time_seconds: 0.2,
            ..IrConfig::default()
        };
        let ir = generate_dhvani_ir(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            &config,
        );
        assert!(!ir.ir.samples.is_empty());
        assert!(ir.room_volume > 0.0);
        assert!(ir.multiband.is_some());
        for &rt in &ir.rt60_bands {
            assert!(rt > 0.0, "RT60 should be positive");
        }
    }

    #[test]
    fn dhvani_ir_serializes() {
        let room = AcousticRoom::shoebox(5.0, 4.0, 3.0, AcousticMaterial::concrete());
        let config = IrConfig {
            num_diffuse_rays: 10,
            max_time_seconds: 0.05,
            ..IrConfig::default()
        };
        let ir = generate_dhvani_ir(
            Vec3::new(2.5, 1.5, 2.0),
            Vec3::new(2.5, 1.5, 2.0 + 0.5),
            &room,
            &config,
        );
        let json = serde_json::to_string(&ir);
        assert!(json.is_ok(), "should serialize to JSON");
    }
}
