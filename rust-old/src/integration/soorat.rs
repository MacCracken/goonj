//! soorat integration — visualization data structures for acoustic analysis.
//!
//! Provides data types that soorat can render: ray paths, pressure maps,
//! and mode pattern visualizations.

use crate::ray::RayPath;
use crate::resonance;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A collection of ray paths for visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayVisualization {
    /// Source position.
    pub source: Vec3,
    /// Traced ray paths with bounce points.
    pub paths: Vec<RayPath>,
}

/// A 3D pressure map on a regular grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PressureMap {
    /// Pressure values at grid points (flattened: `values[z * ny * nx + y * nx + x]`).
    pub values: Vec<f32>,
    /// Grid dimensions (nx, ny, nz).
    pub dimensions: [usize; 3],
    /// World-space origin of the grid (min corner).
    pub origin: Vec3,
    /// Spacing between grid points in meters.
    pub spacing: f32,
    /// Frequency in Hz for this pressure map.
    pub frequency_hz: f32,
}

/// Room mode pattern visualization data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeVisualization {
    /// Mode frequency in Hz.
    pub frequency_hz: f32,
    /// Mode indices (nx, ny, nz) for axial/tangential/oblique modes.
    pub mode_indices: [u32; 3],
    /// Pressure pattern values on a 2D slice (flattened).
    pub pattern: Vec<f32>,
    /// Pattern grid dimensions (width, height).
    pub pattern_dimensions: [usize; 2],
}

impl ModeVisualization {
    /// Generate a mode pattern for a shoebox room on the XZ plane at a given height.
    ///
    /// Computes the standing wave pattern `cos(nπx/Lx) × cos(nπz/Lz)`.
    #[must_use]
    pub fn for_shoebox(
        nx: u32,
        nz: u32,
        length: f32,
        width: f32,
        speed_of_sound: f32,
        grid_resolution: usize,
    ) -> Self {
        let freq_x = if length > 0.0 {
            resonance::room_mode(length, nx, speed_of_sound)
        } else {
            0.0
        };
        let freq_z = if width > 0.0 {
            resonance::room_mode(width, nz, speed_of_sound)
        } else {
            0.0
        };
        let frequency_hz = (freq_x * freq_x + freq_z * freq_z).sqrt();

        let mut pattern = Vec::with_capacity(grid_resolution * grid_resolution);
        for iz in 0..grid_resolution {
            for ix in 0..grid_resolution {
                let x = ix as f32 / (grid_resolution - 1).max(1) as f32 * length;
                let z = iz as f32 / (grid_resolution - 1).max(1) as f32 * width;

                let px = (nx as f32 * std::f32::consts::PI * x / length).cos();
                let pz = (nz as f32 * std::f32::consts::PI * z / width).cos();
                pattern.push(px * pz);
            }
        }

        Self {
            frequency_hz,
            mode_indices: [nx, 0, nz],
            pattern,
            pattern_dimensions: [grid_resolution, grid_resolution],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_visualization_serializes() {
        let viz = RayVisualization {
            source: Vec3::ZERO,
            paths: vec![],
        };
        let json = serde_json::to_string(&viz);
        assert!(json.is_ok());
    }

    #[test]
    fn pressure_map_serializes() {
        let map = PressureMap {
            values: vec![0.0; 8],
            dimensions: [2, 2, 2],
            origin: Vec3::ZERO,
            spacing: 1.0,
            frequency_hz: 1000.0,
        };
        let json = serde_json::to_string(&map);
        assert!(json.is_ok());
    }

    #[test]
    fn mode_visualization_shoebox() {
        let mode = ModeVisualization::for_shoebox(1, 0, 10.0, 8.0, 343.0, 10);
        assert_eq!(mode.pattern.len(), 100); // 10×10
        assert!(mode.frequency_hz > 0.0);
        assert_eq!(mode.mode_indices, [1, 0, 0]);
    }

    #[test]
    fn mode_pattern_corners_and_center() {
        let mode = ModeVisualization::for_shoebox(1, 0, 10.0, 8.0, 343.0, 11);
        // For mode (1,0): pattern = cos(πx/L)
        // At x=0: cos(0) = 1.0
        // At x=L/2: cos(π/2) = 0.0
        // At x=L: cos(π) = -1.0

        // First row, first point (x=0, z=0)
        assert!(
            (mode.pattern[0] - 1.0).abs() < 0.01,
            "corner should be ~1.0, got {}",
            mode.pattern[0]
        );

        // First row, middle point (x=L/2, z=0)
        let mid_idx = 5; // 11/2 = 5 (center column)
        assert!(
            mode.pattern[mid_idx].abs() < 0.01,
            "center should be ~0.0, got {}",
            mode.pattern[mid_idx]
        );
    }

    #[test]
    fn mode_visualization_serializes() {
        let mode = ModeVisualization::for_shoebox(1, 1, 10.0, 8.0, 343.0, 5);
        let json = serde_json::to_string(&mode);
        assert!(json.is_ok());
    }

    #[test]
    fn mode_visualization_zero_dimensions() {
        let mode = ModeVisualization::for_shoebox(1, 1, 0.0, 0.0, 343.0, 5);
        assert_eq!(mode.frequency_hz, 0.0);
    }
}
