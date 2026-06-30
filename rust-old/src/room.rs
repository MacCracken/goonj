use crate::material::AcousticMaterial;
use hisab::Vec3;
use hisab::geo::{Aabb, Bvh};
use serde::{Deserialize, Serialize};

/// A wall in a room, defined by vertices with an associated material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    /// Vertices of the wall polygon (minimum 3).
    pub vertices: Vec<Vec3>,
    /// Acoustic material of this wall surface.
    pub material: AcousticMaterial,
    /// Outward-facing normal vector.
    pub normal: Vec3,
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
            let a = self.vertices[i] - v0;
            let b = self.vertices[i + 1] - v0;
            total += a.cross(b).length() * 0.5;
        }
        total
    }

    /// Absorption area (Sabine) for this wall: area × average absorption coefficient.
    #[must_use]
    pub fn absorption_area(&self) -> f32 {
        self.area() * self.material.average_absorption()
    }

    /// Compute the axis-aligned bounding box for this wall's vertices.
    #[must_use]
    pub fn aabb(&self) -> Aabb {
        if self.vertices.is_empty() {
            return Aabb::new(Vec3::ZERO, Vec3::ZERO);
        }
        let mut min = self.vertices[0];
        let mut max = self.vertices[0];
        for &v in &self.vertices[1..] {
            min = min.min(v);
            max = max.max(v);
        }
        // Slightly pad flat AABBs to avoid zero-thickness slabs
        let pad = Vec3::splat(0.001);
        Aabb::new(min - pad, max + pad)
    }
}

/// Room geometry defined by a collection of walls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomGeometry {
    /// The walls that define this room's boundary.
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

    /// Build a BVH acceleration structure from this geometry's walls.
    pub fn build_bvh(&self) -> Bvh {
        let mut items: Vec<(Aabb, usize)> = self
            .walls
            .iter()
            .enumerate()
            .map(|(i, wall)| (wall.aabb(), i))
            .collect();
        Bvh::build(&mut items)
    }

    /// Create a shoebox (rectangular) room with uniform material on all surfaces.
    ///
    /// Wall normals point **outward** from the room (away from the interior).
    /// Wall order: floor, ceiling, front (z=0), back (z=width), left (x=0), right (x=length).
    #[must_use]
    #[tracing::instrument(skip(material), fields(material = %material.name))]
    pub fn shoebox(length: f32, width: f32, height: f32, material: AcousticMaterial) -> Self {
        let walls = vec![
            // Floor (y = 0)
            Wall {
                vertices: vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(length, 0.0, 0.0),
                    Vec3::new(length, 0.0, width),
                    Vec3::new(0.0, 0.0, width),
                ],
                material: material.clone(),
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
            // Ceiling (y = height)
            Wall {
                vertices: vec![
                    Vec3::new(0.0, height, width),
                    Vec3::new(length, height, width),
                    Vec3::new(length, height, 0.0),
                    Vec3::new(0.0, height, 0.0),
                ],
                material: material.clone(),
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            // Front wall (z = 0)
            Wall {
                vertices: vec![
                    Vec3::new(0.0, height, 0.0),
                    Vec3::new(length, height, 0.0),
                    Vec3::new(length, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 0.0),
                ],
                material: material.clone(),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            // Back wall (z = width)
            Wall {
                vertices: vec![
                    Vec3::new(0.0, 0.0, width),
                    Vec3::new(length, 0.0, width),
                    Vec3::new(length, height, width),
                    Vec3::new(0.0, height, width),
                ],
                material: material.clone(),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            // Left wall (x = 0)
            Wall {
                vertices: vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, width),
                    Vec3::new(0.0, height, width),
                    Vec3::new(0.0, height, 0.0),
                ],
                material: material.clone(),
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            // Right wall (x = length)
            Wall {
                vertices: vec![
                    Vec3::new(length, height, 0.0),
                    Vec3::new(length, height, width),
                    Vec3::new(length, 0.0, width),
                    Vec3::new(length, 0.0, 0.0),
                ],
                material,
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
        ];
        Self { walls }
    }

    /// Volume of a shoebox room computed from axis-aligned bounding box of all vertices.
    ///
    /// Scans all wall vertices to find min/max extents rather than relying on
    /// wall ordering. Only accurate for rectangular (shoebox) rooms.
    #[must_use]
    pub fn volume_shoebox(&self) -> f32 {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for wall in &self.walls {
            for &v in &wall.vertices {
                min = min.min(v);
                max = max.max(v);
            }
        }
        let extents = max - min;
        if extents.x <= 0.0 || extents.y <= 0.0 || extents.z <= 0.0 {
            return 0.0;
        }
        extents.x * extents.y * extents.z
    }
}

/// Pre-computed acceleration structure for fast ray-wall queries.
///
/// Wraps an [`AcousticRoom`] with a BVH built from wall bounding boxes.
/// Use this for rooms with many walls (>20); for shoebox rooms (6 walls)
/// the linear scan is faster due to BVH overhead.
#[derive(Debug, Clone)]
pub struct AcceleratedRoom {
    /// The underlying acoustic room.
    pub room: AcousticRoom,
    /// BVH built from wall AABBs.
    pub bvh: Bvh,
}

impl AcceleratedRoom {
    /// Build an accelerated room from an existing acoustic room.
    #[must_use]
    #[tracing::instrument(skip(room), fields(wall_count = room.geometry.walls.len()))]
    pub fn new(room: AcousticRoom) -> Self {
        let bvh = room.geometry.build_bvh();
        Self { room, bvh }
    }
}

/// An acoustic room: geometry + environmental conditions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticRoom {
    /// Room geometry (walls, surfaces).
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
        assert!(
            (area - 268.0).abs() < 1.0,
            "surface area should be ~268 m², got {area}"
        );
    }

    #[test]
    fn shoebox_volume() {
        let room = test_room();
        let vol = room.geometry.volume_shoebox();
        assert!(
            (vol - 240.0).abs() < 1.0,
            "volume should be ~240 m³, got {vol}"
        );
    }

    #[test]
    fn wall_area_rectangle() {
        let wall = Wall {
            vertices: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(5.0, 3.0, 0.0),
                Vec3::new(0.0, 3.0, 0.0),
            ],
            material: AcousticMaterial::concrete(),
            normal: Vec3::new(0.0, 0.0, 1.0),
        };
        assert!((wall.area() - 15.0).abs() < 0.01);
    }

    #[test]
    fn absorption_area_depends_on_material() {
        let concrete_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let carpet_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        assert!(
            carpet_room.geometry.total_absorption() > concrete_room.geometry.total_absorption()
        );
    }

    #[test]
    fn default_conditions() {
        let room = test_room();
        assert!((room.temperature_celsius - 20.0).abs() < f32::EPSILON);
        assert!((room.humidity_percent - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn volume_shoebox_no_walls() {
        let geom = RoomGeometry { walls: vec![] };
        assert_eq!(geom.volume_shoebox(), 0.0);
    }

    #[test]
    fn volume_shoebox_independent_of_wall_order() {
        let room = test_room();
        let vol1 = room.geometry.volume_shoebox();
        // Reverse wall order
        let mut reversed = room.geometry.clone();
        reversed.walls.reverse();
        let vol2 = reversed.volume_shoebox();
        assert!(
            (vol1 - vol2).abs() < f32::EPSILON,
            "volume should not depend on wall ordering"
        );
    }
}
