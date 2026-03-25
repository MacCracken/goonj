use serde::{Deserialize, Serialize};
use crate::material::AcousticMaterial;

/// A wall in a room, defined by vertices with an associated material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    /// Vertices of the wall polygon (minimum 3).
    pub vertices: Vec<[f32; 3]>,
    /// Acoustic material of this wall surface.
    pub material: AcousticMaterial,
    /// Outward-facing normal vector.
    pub normal: [f32; 3],
}

impl Wall {
    /// Compute the area of this wall (assumes convex polygon, fan triangulation).
    #[must_use]
    pub fn area(&self) -> f32 {
        if self.vertices.len() < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let v0 = self.vertices[0];
        for i in 1..self.vertices.len() - 1 {
            let v1 = self.vertices[i];
            let v2 = self.vertices[i + 1];
            let a = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let b = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            total += (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() * 0.5;
        }
        total
    }

    /// Absorption area (Sabine) for this wall: area × average absorption coefficient.
    #[must_use]
    pub fn absorption_area(&self) -> f32 {
        self.area() * self.material.average_absorption()
    }
}

/// Room geometry defined by a collection of walls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomGeometry {
    pub walls: Vec<Wall>,
}

impl RoomGeometry {
    /// Total surface area of all walls.
    #[must_use]
    pub fn surface_area(&self) -> f32 {
        self.walls.iter().map(|w| w.area()).sum()
    }

    /// Total absorption area (sum of wall area × absorption for each wall).
    #[must_use]
    pub fn total_absorption(&self) -> f32 {
        self.walls.iter().map(|w| w.absorption_area()).sum()
    }

    /// Create a shoebox (rectangular) room with uniform material on all surfaces.
    #[must_use]
    pub fn shoebox(length: f32, width: f32, height: f32, material: AcousticMaterial) -> Self {
        let walls = vec![
            // Floor (y = 0)
            Wall {
                vertices: vec![[0.0, 0.0, 0.0], [length, 0.0, 0.0], [length, 0.0, width], [0.0, 0.0, width]],
                material: material.clone(), normal: [0.0, -1.0, 0.0],
            },
            // Ceiling (y = height)
            Wall {
                vertices: vec![[0.0, height, 0.0], [length, height, 0.0], [length, height, width], [0.0, height, width]],
                material: material.clone(), normal: [0.0, 1.0, 0.0],
            },
            // Front wall (z = 0)
            Wall {
                vertices: vec![[0.0, 0.0, 0.0], [length, 0.0, 0.0], [length, height, 0.0], [0.0, height, 0.0]],
                material: material.clone(), normal: [0.0, 0.0, -1.0],
            },
            // Back wall (z = width)
            Wall {
                vertices: vec![[0.0, 0.0, width], [length, 0.0, width], [length, height, width], [0.0, height, width]],
                material: material.clone(), normal: [0.0, 0.0, 1.0],
            },
            // Left wall (x = 0)
            Wall {
                vertices: vec![[0.0, 0.0, 0.0], [0.0, 0.0, width], [0.0, height, width], [0.0, height, 0.0]],
                material: material.clone(), normal: [-1.0, 0.0, 0.0],
            },
            // Right wall (x = length)
            Wall {
                vertices: vec![[length, 0.0, 0.0], [length, 0.0, width], [length, height, width], [length, height, 0.0]],
                material, normal: [1.0, 0.0, 0.0],
            },
        ];
        Self { walls }
    }

    /// Volume of a shoebox room (only accurate for rectangular rooms).
    /// For complex geometry, use a proper volume algorithm.
    #[must_use]
    pub fn volume_shoebox(&self) -> f32 {
        // Estimate from floor dimensions × height
        // Floor is walls[0], height from walls[0] to walls[1]
        if self.walls.len() < 2 {
            return 0.0;
        }
        let floor = &self.walls[0];
        let ceil = &self.walls[1];
        let floor_area = floor.area();
        // Height = distance between floor and ceiling y-coordinates
        let floor_y = floor.vertices.first().map(|v| v[1]).unwrap_or(0.0);
        let ceil_y = ceil.vertices.first().map(|v| v[1]).unwrap_or(0.0);
        floor_area * (ceil_y - floor_y).abs()
    }
}

/// An acoustic room: geometry + environmental conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticRoom {
    pub geometry: RoomGeometry,
    /// Air temperature in Celsius (affects speed of sound).
    pub temperature_celsius: f32,
    /// Relative humidity percentage (affects atmospheric absorption).
    pub humidity_percent: f32,
}

impl AcousticRoom {
    /// Create a shoebox acoustic room with default conditions (20°C, 50% humidity).
    #[must_use]
    pub fn shoebox(length: f32, width: f32, height: f32, material: AcousticMaterial) -> Self {
        Self {
            geometry: RoomGeometry::shoebox(length, width, height, material),
            temperature_celsius: 20.0,
            humidity_percent: 50.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_room() -> AcousticRoom {
        AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete())
    }

    #[test]
    fn shoebox_has_6_walls() {
        let room = test_room();
        assert_eq!(room.geometry.walls.len(), 6);
    }

    #[test]
    fn shoebox_surface_area() {
        let room = test_room();
        // 2*(10*8) + 2*(10*3) + 2*(8*3) = 160 + 60 + 48 = 268
        let area = room.geometry.surface_area();
        assert!((area - 268.0).abs() < 1.0, "surface area should be ~268 m², got {area}");
    }

    #[test]
    fn shoebox_volume() {
        let room = test_room();
        let vol = room.geometry.volume_shoebox();
        assert!((vol - 240.0).abs() < 1.0, "volume should be ~240 m³, got {vol}");
    }

    #[test]
    fn wall_area_rectangle() {
        let wall = Wall {
            vertices: vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 3.0, 0.0], [0.0, 3.0, 0.0]],
            material: AcousticMaterial::concrete(),
            normal: [0.0, 0.0, 1.0],
        };
        assert!((wall.area() - 15.0).abs() < 0.01);
    }

    #[test]
    fn absorption_area_depends_on_material() {
        let concrete_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let carpet_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        assert!(carpet_room.geometry.total_absorption() > concrete_room.geometry.total_absorption());
    }

    #[test]
    fn default_conditions() {
        let room = test_room();
        assert!((room.temperature_celsius - 20.0).abs() < f32::EPSILON);
        assert!((room.humidity_percent - 50.0).abs() < f32::EPSILON);
    }
}
