//! Image-source method for computing exact early specular reflections.
//!
//! Implements the Allen & Berkeley image-source method for shoebox rooms
//! and a general recursive method for arbitrary convex rooms. Image sources
//! represent virtual source positions obtained by mirroring the real source
//! across room walls, producing exact specular reflection paths with precise
//! arrival times and per-band attenuation.

use crate::material::AcousticMaterial;
use crate::room::{AcousticRoom, Wall};
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// A virtual image source produced by reflecting the real source across walls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    /// Position of the image source in space.
    pub position: Vec3,
    /// Reflection order (0 = direct path, 1 = first reflection, etc.).
    pub order: u32,
    /// Per-band attenuation factor (product of `(1 - absorption)` for each reflection).
    pub attenuation: [f32; crate::material::NUM_BANDS],
}

/// An early reflection computed from an image source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarlyReflection {
    /// Delay from source to listener via this reflection path, in seconds.
    pub delay_seconds: f32,
    /// Per-band amplitude (attenuation × inverse distance law).
    pub amplitude: [f32; crate::material::NUM_BANDS],
    /// Direction of arrival at the listener (normalized).
    pub direction: Vec3,
    /// Reflection order (0 = direct).
    pub order: u32,
    /// Distance from image source to listener.
    pub distance: f32,
}

/// Reflect a point across a plane defined by a point on the plane and its normal.
#[must_use]
#[inline]
fn reflect_point(point: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Vec3 {
    let d = (point - plane_point).dot(plane_normal);
    point - 2.0 * d * plane_normal
}

/// Compute image sources for a shoebox room using the analytic mirror method.
///
/// For a shoebox room with dimensions (length, width, height), image sources
/// at order N are placed at mirrored positions for each combination of
/// reflections across the 6 walls. The `materials` array corresponds to
/// \[floor, ceiling, front, back, left, right\] wall materials.
#[must_use]
#[tracing::instrument(skip(materials), fields(max_order))]
pub fn compute_image_sources_shoebox(
    source: Vec3,
    length: f32,
    width: f32,
    height: f32,
    materials: &[AcousticMaterial; 6],
    max_order: u32,
) -> Vec<ImageSource> {
    // Cap max_order to prevent O(n³) explosion (order 20 → 68k sources)
    let max_order = max_order.min(20);
    let max_n = max_order as i32;
    let side = (2 * max_n + 1) as usize;
    let mut sources = Vec::with_capacity(side * side * side);

    // Order 0 = direct path (the source itself)
    sources.push(ImageSource {
        position: source,
        order: 0,
        attenuation: [1.0; crate::material::NUM_BANDS],
    });

    // For each combination of reflection counts (nx, ny, nz) where |nx|+|ny|+|nz| <= max_order
    for nx in -(max_n)..=max_n {
        for ny in -(max_n)..=max_n {
            for nz in -(max_n)..=max_n {
                let order = nx.unsigned_abs() + ny.unsigned_abs() + nz.unsigned_abs();
                if order == 0 || order > max_order {
                    continue;
                }

                // Image position along each axis
                let x = image_coordinate(source.x, length, nx);
                let y = image_coordinate(source.y, height, ny);
                let z = image_coordinate(source.z, width, nz);

                // Per-band attenuation: product of (1 - absorption) for each wall bounce
                let mut atten = [1.0_f32; crate::material::NUM_BANDS];
                apply_axis_attenuation(&mut atten, materials, 4, 5, nx); // left(4)/right(5) = x-axis
                apply_axis_attenuation(&mut atten, materials, 0, 1, ny); // floor(0)/ceiling(1) = y-axis
                apply_axis_attenuation(&mut atten, materials, 2, 3, nz); // front(2)/back(3) = z-axis

                sources.push(ImageSource {
                    position: Vec3::new(x, y, z),
                    order,
                    attenuation: atten,
                });
            }
        }
    }

    sources
}

/// Compute the image coordinate along one axis for reflection count `n`.
///
/// For even `n`: image is at `n * dim + source_coord` (same-parity mirror).
/// For odd `n`: image is at `n * dim + (dim - source_coord)` (opposite-parity mirror).
#[must_use]
#[inline]
fn image_coordinate(source_coord: f32, dimension: f32, n: i32) -> f32 {
    let nd = n as f32 * dimension;
    if n % 2 == 0 {
        nd + source_coord
    } else if n > 0 {
        nd + (dimension - source_coord)
    } else {
        nd - (dimension - source_coord)
    }
}

/// Apply per-band attenuation for reflections along one axis.
///
/// `neg_wall` is the wall at coordinate 0, `pos_wall` is the wall at coordinate = dimension.
/// `n` reflection bounces alternate between the two walls.
#[inline]
fn apply_axis_attenuation(
    atten: &mut [f32; crate::material::NUM_BANDS],
    materials: &[AcousticMaterial; 6],
    neg_wall: usize,
    pos_wall: usize,
    n: i32,
) {
    let abs_n = n.unsigned_abs();
    // Number of times each wall is hit
    let (neg_hits, pos_hits) = if n > 0 {
        // Positive direction: hits pos wall ceil(n/2) times, neg wall floor(n/2) times
        (abs_n / 2, abs_n.div_ceil(2))
    } else if n < 0 {
        // Negative direction: hits neg wall ceil(|n|/2) times, pos wall floor(|n|/2) times
        (abs_n.div_ceil(2), abs_n / 2)
    } else {
        return;
    };

    for (band, a) in atten.iter_mut().enumerate() {
        let neg_factor = (1.0 - materials[neg_wall].absorption[band]).powi(neg_hits as i32);
        let pos_factor = (1.0 - materials[pos_wall].absorption[band]).powi(pos_hits as i32);
        *a *= neg_factor * pos_factor;
    }
}

/// Compute image sources for an arbitrary convex room by recursive wall reflection.
///
/// This is more expensive than the shoebox method and should be limited to
/// low orders (2-3) for practical use.
#[must_use]
#[tracing::instrument(skip(walls), fields(wall_count = walls.len(), max_order))]
pub fn compute_image_sources_general(
    source: Vec3,
    walls: &[Wall],
    max_order: u32,
) -> Vec<ImageSource> {
    // Cap max_order to prevent O(W^N) explosion
    let max_order = max_order.min(6);
    let mut sources = vec![ImageSource {
        position: source,
        order: 0,
        attenuation: [1.0; crate::material::NUM_BANDS],
    }];

    // Working set: sources at the current order to be reflected
    let mut current_order = vec![sources[0].clone()];

    for _order in 1..=max_order {
        let mut next_order = Vec::new();
        for parent in &current_order {
            for wall in walls {
                let reflected_pos = reflect_point(parent.position, wall.vertices[0], wall.normal);

                let mut atten = parent.attenuation;
                for (band, a) in atten.iter_mut().enumerate() {
                    *a *= 1.0 - wall.material.absorption[band];
                }

                let img = ImageSource {
                    position: reflected_pos,
                    order: parent.order + 1,
                    attenuation: atten,
                };
                next_order.push(img);
            }
        }
        sources.extend(next_order.iter().cloned());
        current_order = next_order;
    }

    sources
}

/// Compute early reflections from image sources for a shoebox room.
///
/// Generates image sources up to `max_order`, computes arrival time and per-band
/// amplitude for each, and returns them sorted by arrival time.
#[must_use]
#[tracing::instrument(skip(room), fields(max_order, speed_of_sound))]
pub fn compute_early_reflections(
    source: Vec3,
    listener: Vec3,
    room: &AcousticRoom,
    max_order: u32,
    speed_of_sound: f32,
) -> Vec<EarlyReflection> {
    let geom = &room.geometry;

    if geom.walls.is_empty() || speed_of_sound <= 0.0 {
        return Vec::new();
    }

    // Determine room dimensions from bounding box
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for wall in &geom.walls {
        for &v in &wall.vertices {
            min = min.min(v);
            max = max.max(v);
        }
    }
    let dims = max - min;

    // Build materials array [floor, ceiling, front, back, left, right]
    // Assumes shoebox wall ordering from RoomGeometry::shoebox
    let materials: [AcousticMaterial; 6] = if geom.walls.len() >= 6 {
        std::array::from_fn(|i| geom.walls[i].material.clone())
    } else {
        std::array::from_fn(|_| geom.walls[0].material.clone())
    };

    let image_sources =
        compute_image_sources_shoebox(source, dims.x, dims.z, dims.y, &materials, max_order);

    let mut reflections: Vec<EarlyReflection> = image_sources
        .iter()
        .filter_map(|img| {
            let diff = listener - img.position;
            let distance = diff.length();
            if distance < f32::EPSILON {
                return None;
            }

            let delay = distance / speed_of_sound;

            // Per-band amplitude: attenuation / distance (inverse distance law)
            let inv_dist = 1.0 / distance;
            let mut amplitude = [0.0_f32; crate::material::NUM_BANDS];
            for (band, amp) in amplitude.iter_mut().enumerate() {
                *amp = img.attenuation[band] * inv_dist;
            }

            let direction = diff / distance;

            Some(EarlyReflection {
                delay_seconds: delay,
                amplitude,
                direction,
                order: img.order,
                distance,
            })
        })
        .collect();

    reflections.sort_by(|a, b| {
        a.delay_seconds
            .partial_cmp(&b.delay_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    reflections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::propagation::speed_of_sound;

    fn test_room() -> AcousticRoom {
        AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete())
    }

    #[test]
    fn direct_path_is_order_zero() {
        let room = test_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let c = speed_of_sound(20.0);

        let reflections = compute_early_reflections(source, listener, &room, 3, c);
        assert!(!reflections.is_empty());

        let direct = reflections.iter().find(|r| r.order == 0);
        assert!(direct.is_some(), "should have direct path (order 0)");

        let direct = direct.unwrap();
        let expected_dist = (listener - source).length();
        assert!(
            (direct.distance - expected_dist).abs() < 0.1,
            "direct distance should be ~{expected_dist}, got {}",
            direct.distance
        );
    }

    #[test]
    fn direct_path_arrives_first() {
        let room = test_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let c = speed_of_sound(20.0);

        let reflections = compute_early_reflections(source, listener, &room, 3, c);
        assert!(reflections.len() > 1);

        // First reflection should be direct path (smallest delay)
        assert_eq!(reflections[0].order, 0, "direct path should arrive first");
    }

    #[test]
    fn first_order_arrives_after_direct() {
        let room = test_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let c = speed_of_sound(20.0);

        let reflections = compute_early_reflections(source, listener, &room, 1, c);
        let direct_delay = reflections[0].delay_seconds;

        for r in reflections.iter().skip(1) {
            assert!(
                r.delay_seconds >= direct_delay,
                "order {} reflection at {:.4}s should arrive after direct at {:.4}s",
                r.order,
                r.delay_seconds,
                direct_delay
            );
        }
    }

    #[test]
    fn amplitude_decreases_with_order() {
        let room = test_room();
        let source = Vec3::new(5.0, 1.5, 4.0);
        let listener = Vec3::new(5.0, 1.5, 4.0 + 0.01); // nearly collocated
        let c = speed_of_sound(20.0);

        let reflections = compute_early_reflections(source, listener, &room, 3, c);

        // Average amplitude should decrease with order
        let avg_amp = |order: u32| -> f32 {
            let refs: Vec<_> = reflections.iter().filter(|r| r.order == order).collect();
            if refs.is_empty() {
                return 0.0;
            }
            let total: f32 = refs
                .iter()
                .map(|r| r.amplitude.iter().sum::<f32>() / r.amplitude.len() as f32)
                .sum();
            total / refs.len() as f32
        };

        let amp_1 = avg_amp(1);
        let amp_2 = avg_amp(2);
        let amp_3 = avg_amp(3);

        assert!(
            amp_1 > amp_2,
            "order 1 avg amp ({amp_1}) should exceed order 2 ({amp_2})"
        );
        assert!(
            amp_2 > amp_3,
            "order 2 avg amp ({amp_2}) should exceed order 3 ({amp_3})"
        );
    }

    #[test]
    fn image_source_count_shoebox_order_1() {
        let materials = std::array::from_fn(|_| AcousticMaterial::concrete());
        let source = Vec3::new(5.0, 1.5, 4.0);
        let sources = compute_image_sources_shoebox(source, 10.0, 8.0, 3.0, &materials, 1);

        // Order 0: 1 (direct)
        // Order 1: 6 (one per wall)
        assert_eq!(
            sources.len(),
            7,
            "order 1 should have 7 image sources (1 direct + 6 first-order)"
        );
    }

    #[test]
    fn image_coordinate_even_n() {
        // n=0: should return source_coord itself
        assert!((image_coordinate(3.0, 10.0, 0) - 3.0).abs() < f32::EPSILON);
        // n=2: 2*10 + 3 = 23
        assert!((image_coordinate(3.0, 10.0, 2) - 23.0).abs() < f32::EPSILON);
        // n=-2: -2*10 + 3 = -17
        assert!((image_coordinate(3.0, 10.0, -2) - (-17.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn image_coordinate_odd_n() {
        // n=1: 1*10 + (10-3) = 17
        assert!((image_coordinate(3.0, 10.0, 1) - 17.0).abs() < f32::EPSILON);
        // n=-1: -1*10 - (10-3) = -17
        assert!((image_coordinate(3.0, 10.0, -1) - (-17.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn reflections_sorted_by_delay() {
        let room = test_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let c = speed_of_sound(20.0);

        let reflections = compute_early_reflections(source, listener, &room, 3, c);
        for window in reflections.windows(2) {
            assert!(
                window[0].delay_seconds <= window[1].delay_seconds,
                "reflections should be sorted by delay"
            );
        }
    }

    #[test]
    fn general_method_produces_correct_order_count() {
        let room = test_room();
        let source = Vec3::new(5.0, 1.5, 4.0);
        let sources = compute_image_sources_general(source, &room.geometry.walls, 1);

        // Order 0: 1, Order 1: 6 walls → 7 total
        assert_eq!(sources.len(), 7);
    }

    #[test]
    fn reflect_point_across_plane() {
        // Reflect (1, 0, 0) across the YZ plane at origin
        let reflected = reflect_point(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::X);
        assert!((reflected.x - (-1.0)).abs() < f32::EPSILON);
        assert!(reflected.y.abs() < f32::EPSILON);
        assert!(reflected.z.abs() < f32::EPSILON);
    }

    #[test]
    fn carpet_room_attenuates_more() {
        let concrete_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let carpet_room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let c = speed_of_sound(20.0);

        let concrete_refs = compute_early_reflections(source, listener, &concrete_room, 2, c);
        let carpet_refs = compute_early_reflections(source, listener, &carpet_room, 2, c);

        // Compare first-order reflection amplitudes
        let concrete_first: f32 = concrete_refs
            .iter()
            .filter(|r| r.order == 1)
            .map(|r| r.amplitude.iter().sum::<f32>())
            .sum();
        let carpet_first: f32 = carpet_refs
            .iter()
            .filter(|r| r.order == 1)
            .map(|r| r.amplitude.iter().sum::<f32>())
            .sum();

        assert!(
            concrete_first > carpet_first,
            "concrete reflections ({concrete_first}) should be stronger than carpet ({carpet_first})"
        );
    }

    #[test]
    fn direct_path_attenuation_is_one() {
        let materials = std::array::from_fn(|_| AcousticMaterial::concrete());
        let source = Vec3::new(5.0, 1.5, 4.0);
        let sources = compute_image_sources_shoebox(source, 10.0, 8.0, 3.0, &materials, 1);

        let direct = sources.iter().find(|s| s.order == 0).unwrap();
        for &a in &direct.attenuation {
            assert!(
                (a - 1.0).abs() < f32::EPSILON,
                "direct path should have unit attenuation"
            );
        }
    }

    // --- Audit edge-case tests ---

    #[test]
    fn empty_geometry_returns_empty() {
        let room = AcousticRoom {
            geometry: crate::room::RoomGeometry { walls: vec![] },
            temperature_celsius: 20.0,
            humidity_percent: 50.0,
        };
        let reflections = compute_early_reflections(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 1.0, 1.0),
            &room,
            3,
            343.0,
        );
        assert!(
            reflections.is_empty(),
            "empty geometry should produce no reflections"
        );
    }

    #[test]
    fn zero_speed_of_sound_returns_empty() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let reflections = compute_early_reflections(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            3,
            0.0,
        );
        assert!(reflections.is_empty());
    }

    #[test]
    fn max_order_zero_direct_only() {
        let room = test_room();
        let c = speed_of_sound(20.0);
        let reflections = compute_early_reflections(
            Vec3::new(3.0, 1.5, 4.0),
            Vec3::new(7.0, 1.5, 4.0),
            &room,
            0,
            c,
        );
        assert_eq!(reflections.len(), 1, "order 0 should only have direct path");
        assert_eq!(reflections[0].order, 0);
    }

    #[test]
    fn collocated_source_listener() {
        let room = test_room();
        let c = speed_of_sound(20.0);
        let pos = Vec3::new(5.0, 1.5, 4.0);
        let reflections = compute_early_reflections(pos, pos, &room, 2, c);
        // Direct path distance ≈ 0, should be filtered out (distance < EPSILON)
        // But reflections should still exist
        let has_higher_order = reflections.iter().any(|r| r.order > 0);
        assert!(
            has_higher_order,
            "collocated should still produce reflections"
        );
    }

    #[test]
    fn general_method_empty_walls() {
        let sources = compute_image_sources_general(Vec3::ZERO, &[], 3);
        assert_eq!(sources.len(), 1, "should have only direct source");
        assert_eq!(sources[0].order, 0);
    }

    #[test]
    fn early_reflections_non_shoebox_room() {
        // Room with 4 walls (not 6) → falls back to cloning first wall material
        let mat = AcousticMaterial::concrete();
        let room = AcousticRoom {
            geometry: crate::room::RoomGeometry {
                walls: vec![
                    crate::room::Wall {
                        vertices: vec![
                            Vec3::new(0.0, 0.0, 0.0),
                            Vec3::new(10.0, 0.0, 0.0),
                            Vec3::new(10.0, 3.0, 0.0),
                            Vec3::new(0.0, 3.0, 0.0),
                        ],
                        material: mat.clone(),
                        normal: Vec3::new(0.0, 0.0, -1.0),
                    },
                    crate::room::Wall {
                        vertices: vec![
                            Vec3::new(0.0, 0.0, 8.0),
                            Vec3::new(10.0, 0.0, 8.0),
                            Vec3::new(10.0, 3.0, 8.0),
                            Vec3::new(0.0, 3.0, 8.0),
                        ],
                        material: mat.clone(),
                        normal: Vec3::new(0.0, 0.0, 1.0),
                    },
                    crate::room::Wall {
                        vertices: vec![
                            Vec3::new(0.0, 0.0, 0.0),
                            Vec3::new(0.0, 0.0, 8.0),
                            Vec3::new(0.0, 3.0, 8.0),
                            Vec3::new(0.0, 3.0, 0.0),
                        ],
                        material: mat.clone(),
                        normal: Vec3::new(-1.0, 0.0, 0.0),
                    },
                    crate::room::Wall {
                        vertices: vec![
                            Vec3::new(10.0, 0.0, 0.0),
                            Vec3::new(10.0, 0.0, 8.0),
                            Vec3::new(10.0, 3.0, 8.0),
                            Vec3::new(10.0, 3.0, 0.0),
                        ],
                        material: mat,
                        normal: Vec3::new(1.0, 0.0, 0.0),
                    },
                ],
            },
            temperature_celsius: 20.0,
            humidity_percent: 50.0,
        };
        let c = speed_of_sound(20.0);
        let reflections = compute_early_reflections(
            Vec3::new(5.0, 1.5, 4.0),
            Vec3::new(5.0, 1.5, 4.0 + 0.5),
            &room,
            2,
            c,
        );
        // Should produce reflections even with non-shoebox geometry
        assert!(!reflections.is_empty());
    }
}
