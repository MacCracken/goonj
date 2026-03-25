//! kiran integration — real-time occlusion queries for game audio.
//!
//! Provides [`OcclusionEngine`], a cached acceleration structure for fast
//! source-listener occlusion queries with per-band attenuation output.

use crate::diffraction::{edge_diffraction_loss, is_occluded};
use crate::material::FREQUENCY_BANDS;
use crate::room::{AcceleratedRoom, AcousticRoom};
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// Result of an occlusion query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcclusionResult {
    /// Whether direct line-of-sight is blocked.
    pub is_occluded: bool,
    /// Overall attenuation in dB (0.0 = no attenuation, negative = attenuated).
    pub attenuation_db: f32,
    /// Per-frequency-band attenuation in dB.
    pub frequency_dependent: [f32; crate::material::NUM_BANDS],
}

/// Pre-built occlusion query engine with cached BVH.
#[derive(Debug, Clone)]
pub struct OcclusionEngine {
    accel: AcceleratedRoom,
}

impl OcclusionEngine {
    /// Build an occlusion engine from a room.
    #[must_use]
    pub fn new(room: AcousticRoom) -> Self {
        Self {
            accel: AcceleratedRoom::new(room),
        }
    }

    /// Query occlusion between a source and listener.
    ///
    /// Returns occlusion status and per-band attenuation. If the path is
    /// occluded, estimates diffraction loss at each frequency band.
    #[must_use]
    #[tracing::instrument(skip(self))]
    pub fn query(&self, source: Vec3, listener: Vec3) -> OcclusionResult {
        let walls = &self.accel.room.geometry.walls;
        let occluded = is_occluded(source, listener, walls);

        if !occluded {
            return OcclusionResult {
                is_occluded: false,
                attenuation_db: 0.0,
                frequency_dependent: [0.0; crate::material::NUM_BANDS],
            };
        }

        // Estimate diffraction loss per band
        // Use a representative edge angle of π/4 for the simplified model
        let edge_angle = std::f32::consts::FRAC_PI_4;
        let frequency_dependent = std::array::from_fn(|band| {
            edge_diffraction_loss(
                FREQUENCY_BANDS[band],
                edge_angle,
                self.accel.room.temperature_celsius,
            )
        });

        let avg_atten = frequency_dependent.iter().sum::<f32>() / frequency_dependent.len() as f32;

        OcclusionResult {
            is_occluded: true,
            attenuation_db: avg_atten,
            frequency_dependent,
        }
    }

    /// Access the underlying room.
    #[must_use]
    pub fn room(&self) -> &AcousticRoom {
        &self.accel.room
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::room::Wall;

    #[test]
    fn unoccluded_path_zero_attenuation() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let engine = OcclusionEngine::new(room);

        // Source and listener inside the room, no internal walls
        let result = engine.query(Vec3::new(3.0, 1.5, 4.0), Vec3::new(7.0, 1.5, 4.0));
        assert!(!result.is_occluded);
        assert!((result.attenuation_db).abs() < f32::EPSILON);
    }

    #[test]
    fn occluded_path_has_attenuation() {
        // Create a room with an internal wall blocking the path
        let mut room = AcousticRoom::shoebox(20.0, 20.0, 3.0, AcousticMaterial::concrete());
        room.geometry.walls.push(Wall {
            vertices: vec![
                Vec3::new(10.0, -5.0, 5.0),
                Vec3::new(10.0, 5.0, 5.0),
                Vec3::new(10.0, 5.0, -5.0),
                Vec3::new(10.0, -5.0, -5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(-1.0, 0.0, 0.0),
        });

        let engine = OcclusionEngine::new(room);
        let result = engine.query(Vec3::new(5.0, 0.0, 0.0), Vec3::new(15.0, 0.0, 0.0));

        assert!(result.is_occluded);
        assert!(
            result.attenuation_db < 0.0,
            "occlusion should produce negative attenuation"
        );
    }

    #[test]
    fn frequency_dependent_attenuation() {
        let mut room = AcousticRoom::shoebox(20.0, 20.0, 3.0, AcousticMaterial::concrete());
        room.geometry.walls.push(Wall {
            vertices: vec![
                Vec3::new(10.0, -5.0, 5.0),
                Vec3::new(10.0, 5.0, 5.0),
                Vec3::new(10.0, 5.0, -5.0),
                Vec3::new(10.0, -5.0, -5.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(-1.0, 0.0, 0.0),
        });

        let engine = OcclusionEngine::new(room);
        let result = engine.query(Vec3::new(5.0, 0.0, 0.0), Vec3::new(15.0, 0.0, 0.0));

        // High frequencies should be more attenuated (more negative)
        assert!(
            result.frequency_dependent[5] < result.frequency_dependent[0],
            "high freq ({}) should be more attenuated than low ({})",
            result.frequency_dependent[5],
            result.frequency_dependent[0]
        );
    }

    #[test]
    fn occlusion_result_serializes() {
        let result = OcclusionResult {
            is_occluded: true,
            attenuation_db: -6.0,
            frequency_dependent: [-2.0, -3.0, -4.0, -5.0, -6.0, -7.0, -8.0, -9.0],
        };
        let json = serde_json::to_string(&result);
        assert!(json.is_ok());
    }

    #[test]
    fn occlusion_engine_room_accessor() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let engine = OcclusionEngine::new(room);
        assert_eq!(engine.room().geometry.walls.len(), 6);
    }
}
