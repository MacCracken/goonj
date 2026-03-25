//! Diffuse rain — stochastic ray tracing for late reverberation tails.
//!
//! Launches many rays from a source in uniformly distributed directions,
//! traces them through the room geometry, and collects energy contributions
//! at a listener position. The resulting time-energy histogram represents
//! the late portion of the impulse response.
//!
//! Uses a Fibonacci sphere for deterministic ray distribution and an
//! inline xorshift64 PRNG for stochastic perturbation — no external
//! `rand` dependency.

use crate::ray::{self, MultibandRay, RayPath};
use crate::room::AcousticRoom;
use hisab::Vec3;
use serde::{Deserialize, Serialize};

/// Configuration for diffuse rain computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffuseRainConfig {
    /// Number of rays to launch from the source.
    pub num_rays: u32,
    /// Maximum bounces per ray.
    pub max_bounces: u32,
    /// Maximum simulation time in seconds.
    pub max_time_seconds: f32,
    /// Collection sphere radius around the listener.
    /// If 0.0, automatically computed from room volume and ray count.
    pub collection_radius: f32,
    /// Speed of sound in m/s.
    pub speed_of_sound: f32,
    /// Random seed for reproducibility.
    pub seed: u64,
}

/// A single late reverb energy contribution collected at the listener.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LateReverbContribution {
    /// Time of arrival in seconds (from source emission).
    pub time_seconds: f32,
    /// Per-band energy at this arrival.
    pub energy: [f32; 6],
    /// Direction of arrival at the listener (normalized).
    pub direction: Vec3,
}

/// Result of a diffuse rain computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffuseRainResult {
    /// Individual energy contributions, sorted by time.
    pub contributions: Vec<LateReverbContribution>,
    /// Total number of rays traced.
    pub rays_traced: u32,
    /// Total number of bounces across all rays.
    pub total_bounces: u32,
}

/// Simple xorshift64 PRNG — no external dependency.
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        // Ensure non-zero seed
        Self(if seed == 0 {
            0x5555_5555_5555_5555
        } else {
            seed
        })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Returns a float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Generate uniformly distributed directions on a sphere using the Fibonacci spiral.
#[must_use]
pub fn fibonacci_sphere(n: u32) -> Vec<Vec3> {
    let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let angle_increment = std::f32::consts::TAU / golden_ratio;

    (0..n)
        .map(|i| {
            let t = (i as f32 + 0.5) / n as f32;
            let phi = (1.0 - 2.0 * t).acos();
            let theta = angle_increment * i as f32;

            Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos())
        })
        .collect()
}

/// Auto-compute collection radius from room volume and ray count.
///
/// Uses a generous radius to ensure statistical convergence: `r = 2 × (V / N)^(1/3)`.
/// For a 240 m³ room with 1000 rays this gives ~1.2 m, which captures enough
/// bounce points for meaningful late-reverb histograms.
#[must_use]
#[inline]
fn auto_collection_radius(volume: f32, num_rays: u32) -> f32 {
    2.0 * (volume / num_rays as f32).cbrt()
}

/// Generate diffuse rain contributions for a room.
///
/// Launches `config.num_rays` from `source`, traces each through the room geometry,
/// and collects energy at each bounce point that falls within the collection sphere
/// around `listener`.
#[must_use]
#[tracing::instrument(skip(room), fields(num_rays = config.num_rays, max_bounces = config.max_bounces))]
pub fn generate_diffuse_rain(
    source: Vec3,
    listener: Vec3,
    room: &AcousticRoom,
    config: &DiffuseRainConfig,
) -> DiffuseRainResult {
    let volume = room.geometry.volume_shoebox();
    let collection_r = if config.collection_radius > f32::EPSILON {
        config.collection_radius
    } else {
        auto_collection_radius(volume, config.num_rays)
    };
    let collection_r2 = collection_r * collection_r;

    let directions = fibonacci_sphere(config.num_rays);
    let mut rng = Xorshift64::new(config.seed);
    let max_time = config.max_time_seconds;

    let mut contributions = Vec::new();
    let mut total_bounces = 0_u32;

    for base_dir in &directions {
        // Slight random perturbation for stochastic diversity
        let jitter = Vec3::new(
            (rng.next_f32() - 0.5) * 0.05,
            (rng.next_f32() - 0.5) * 0.05,
            (rng.next_f32() - 0.5) * 0.05,
        );
        let dir = *base_dir + jitter;
        let len = dir.length();
        let dir = if len > f32::EPSILON {
            dir / len
        } else {
            *base_dir
        };

        let ray = MultibandRay::new(source, dir);
        let path: RayPath = ray::trace_ray(&ray, &room.geometry.walls, config.max_bounces);

        total_bounces += path.bounces.len() as u32;

        // Walk through bounce points, accumulating distance for time computation
        let mut cumulative_distance = 0.0_f32;
        for bounce in &path.bounces {
            cumulative_distance += bounce.distance_from_previous;
            let time = cumulative_distance / config.speed_of_sound;
            if time > max_time {
                break;
            }

            // Check if bounce point is within collection sphere
            let diff = bounce.point - listener;
            let dist2 = diff.dot(diff);
            if dist2 <= collection_r2 {
                let dist = dist2.sqrt();
                let direction = if dist > f32::EPSILON {
                    diff / dist
                } else {
                    dir
                };

                contributions.push(LateReverbContribution {
                    time_seconds: time,
                    energy: bounce.energy_after,
                    direction,
                });
            }
        }
    }

    contributions.sort_by(|a, b| {
        a.time_seconds
            .partial_cmp(&b.time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    DiffuseRainResult {
        contributions,
        rays_traced: config.num_rays,
        total_bounces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::AcousticMaterial;
    use crate::propagation::speed_of_sound;

    fn test_config(num_rays: u32) -> DiffuseRainConfig {
        DiffuseRainConfig {
            num_rays,
            max_bounces: 50,
            max_time_seconds: 2.0,
            collection_radius: 2.0, // generous radius for test reliability
            speed_of_sound: speed_of_sound(20.0),
            seed: 42,
        }
    }

    fn concrete_room() -> AcousticRoom {
        AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete())
    }

    fn carpet_room() -> AcousticRoom {
        AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet())
    }

    #[test]
    fn fibonacci_sphere_unit_vectors() {
        let dirs = fibonacci_sphere(100);
        assert_eq!(dirs.len(), 100);
        for d in &dirs {
            let len = d.length();
            assert!(
                (len - 1.0).abs() < 0.01,
                "direction should be unit length, got {len}"
            );
        }
    }

    #[test]
    fn fibonacci_sphere_covers_hemisphere() {
        let dirs = fibonacci_sphere(1000);
        // Check that we have directions in all octants
        let mut octants = [false; 8];
        for d in &dirs {
            let idx =
                ((d.x > 0.0) as usize) << 2 | ((d.y > 0.0) as usize) << 1 | (d.z > 0.0) as usize;
            octants[idx] = true;
        }
        assert!(
            octants.iter().all(|&o| o),
            "fibonacci sphere should cover all octants"
        );
    }

    #[test]
    fn diffuse_rain_produces_contributions() {
        let room = concrete_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = test_config(500);

        let result = generate_diffuse_rain(source, listener, &room, &config);
        assert!(
            !result.contributions.is_empty(),
            "should produce contributions in concrete room"
        );
        assert_eq!(result.rays_traced, 500);
        assert!(result.total_bounces > 0);
    }

    #[test]
    fn contributions_sorted_by_time() {
        let room = concrete_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = test_config(200);

        let result = generate_diffuse_rain(source, listener, &room, &config);
        for window in result.contributions.windows(2) {
            assert!(
                window[0].time_seconds <= window[1].time_seconds,
                "contributions should be sorted by time"
            );
        }
    }

    #[test]
    fn seed_reproducibility() {
        let room = concrete_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = test_config(100);

        let r1 = generate_diffuse_rain(source, listener, &room, &config);
        let r2 = generate_diffuse_rain(source, listener, &room, &config);

        assert_eq!(
            r1.contributions.len(),
            r2.contributions.len(),
            "same seed should produce same contribution count"
        );
        for (a, b) in r1.contributions.iter().zip(r2.contributions.iter()) {
            assert!(
                (a.time_seconds - b.time_seconds).abs() < f32::EPSILON,
                "same seed should produce identical results"
            );
        }
    }

    #[test]
    fn different_seed_different_results() {
        let room = concrete_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config1 = test_config(100);
        let mut config2 = test_config(100);
        config2.seed = 999;

        let r1 = generate_diffuse_rain(source, listener, &room, &config1);
        let r2 = generate_diffuse_rain(source, listener, &room, &config2);

        // Different seeds should likely produce different contribution counts or times
        // (not guaranteed but highly probable)
        let same = r1.contributions.len() == r2.contributions.len()
            && r1
                .contributions
                .iter()
                .zip(r2.contributions.iter())
                .all(|(a, b)| (a.time_seconds - b.time_seconds).abs() < f32::EPSILON);
        assert!(!same, "different seeds should produce different results");
    }

    #[test]
    fn carpet_fewer_contributions_than_concrete() {
        let concrete = concrete_room();
        let carpet = carpet_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let config = test_config(500);

        let r_concrete = generate_diffuse_rain(source, listener, &concrete, &config);
        let r_carpet = generate_diffuse_rain(source, listener, &carpet, &config);

        // Carpet absorbs more → rays die faster → fewer late contributions
        let concrete_energy: f32 = r_concrete
            .contributions
            .iter()
            .map(|c| c.energy.iter().sum::<f32>())
            .sum();
        let carpet_energy: f32 = r_carpet
            .contributions
            .iter()
            .map(|c| c.energy.iter().sum::<f32>())
            .sum();

        assert!(
            concrete_energy > carpet_energy,
            "concrete total energy ({concrete_energy}) should exceed carpet ({carpet_energy})"
        );
    }

    #[test]
    fn contributions_within_time_limit() {
        let room = concrete_room();
        let source = Vec3::new(3.0, 1.5, 4.0);
        let listener = Vec3::new(7.0, 1.5, 4.0);
        let mut config = test_config(200);
        config.max_time_seconds = 0.5;

        let result = generate_diffuse_rain(source, listener, &room, &config);
        for c in &result.contributions {
            assert!(
                c.time_seconds <= 0.5 + f32::EPSILON,
                "contribution at {:.3}s exceeds max time 0.5s",
                c.time_seconds
            );
        }
    }

    #[test]
    fn auto_collection_radius_scales_with_volume() {
        let r1 = auto_collection_radius(100.0, 1000);
        let r2 = auto_collection_radius(1000.0, 1000);
        assert!(r2 > r1, "larger room should have larger collection radius");
    }

    #[test]
    fn xorshift64_produces_values_in_range() {
        let mut rng = Xorshift64::new(42);
        for _ in 0..1000 {
            let v = rng.next_f32();
            assert!((0.0..1.0).contains(&v), "value {v} out of [0, 1) range");
        }
    }

    #[test]
    fn energy_decays_over_time() {
        let room = concrete_room();
        let source = Vec3::new(5.0, 1.5, 4.0);
        let listener = Vec3::new(5.0, 1.5, 4.0 + 0.5);
        let mut config = test_config(1000);
        config.collection_radius = 2.0; // large radius to catch more contributions

        let result = generate_diffuse_rain(source, listener, &room, &config);
        if result.contributions.len() < 4 {
            return; // not enough data to test trend
        }

        // Split into early half and late half, compare average energy
        let mid = result.contributions.len() / 2;
        let early_avg: f32 = result.contributions[..mid]
            .iter()
            .map(|c| c.energy.iter().sum::<f32>())
            .sum::<f32>()
            / mid as f32;
        let late_avg: f32 = result.contributions[mid..]
            .iter()
            .map(|c| c.energy.iter().sum::<f32>())
            .sum::<f32>()
            / (result.contributions.len() - mid) as f32;

        assert!(
            early_avg >= late_avg,
            "early energy ({early_avg}) should be >= late energy ({late_avg})"
        );
    }
}
