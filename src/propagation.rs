use hisab::Vec3;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Speed of sound in air as a function of temperature.
///
/// Formula: c = 331.3 + 0.606 × T (°C)
#[must_use]
#[inline]
pub fn speed_of_sound(temperature_celsius: f32) -> f32 {
    331.3 + 0.606 * temperature_celsius
}

/// Sound intensity at distance via inverse square law.
///
/// I = P / (4π × r²)
#[must_use]
#[inline]
pub fn inverse_square_law(power: f32, distance: f32) -> f32 {
    if distance <= 0.0 {
        return 0.0;
    }
    power / (4.0 * PI * distance * distance)
}

/// Sound pressure level drop with distance (dB).
///
/// ΔL = 20 × log10(r2 / r1)
#[must_use]
#[inline]
pub fn spl_drop_with_distance(distance_ref: f32, distance: f32) -> f32 {
    if distance_ref <= 0.0 || distance <= 0.0 {
        return 0.0;
    }
    20.0 * (distance / distance_ref).log10()
}

/// Atmospheric absorption coefficient per ISO 9613-1 / ANSI S1.26.
///
/// Computes the pure-tone absorption coefficient in dB/m as a function of
/// frequency, temperature, humidity, and atmospheric pressure. Models the
/// molecular relaxation of O₂ and N₂ that causes frequency-dependent
/// absorption peaks.
///
/// Valid for: 50 Hz – 10 kHz, −20°C – +50°C, 10% – 100% RH.
///
/// # Arguments
/// * `frequency_hz` — pure-tone frequency in Hz
/// * `humidity_percent` — relative humidity (0–100%)
/// * `temperature_celsius` — air temperature in °C
/// * `pressure_atm` — atmospheric pressure in atmospheres (1.0 = standard)
#[must_use]
pub fn atmospheric_absorption(
    frequency_hz: f32,
    humidity_percent: f32,
    temperature_celsius: f32,
    pressure_atm: f32,
) -> f32 {
    if frequency_hz <= 0.0 || humidity_percent <= 0.0 || pressure_atm <= 0.0 {
        return 0.0;
    }

    let t_kelvin = temperature_celsius + 273.15;
    let t_ref = 293.15_f32; // reference temperature (20°C)
    let t_01 = 273.16_f32; // triple point of water

    let f = frequency_hz;
    let p_rel = pressure_atm; // p / p_ref where p_ref = 1 atm

    // Molar concentration of water vapour (from relative humidity)
    let c_sat = -6.8346 * (t_01 / t_kelvin).powf(1.261) + 4.6151;
    let h = humidity_percent * 10.0_f32.powf(c_sat) / p_rel;

    // Oxygen relaxation frequency (Hz)
    let fr_o2 = p_rel * (24.0 + 4.04e4 * h * (0.02 + h) / (0.391 + h));

    // Nitrogen relaxation frequency (Hz)
    let fr_n2 = p_rel
        * (t_ref / t_kelvin).sqrt()
        * (9.0 + 280.0 * h * (-4.17 * ((t_ref / t_kelvin).powf(1.0 / 3.0) - 1.0)).exp());

    let f2 = f * f;
    let t_ratio = t_kelvin / t_ref;

    // Absorption coefficient in Nepers/m, then convert to dB/m
    let alpha = f2
        * (1.84e-11 * p_rel.recip() * t_ratio.sqrt()
            + t_ratio.powf(-2.5)
                * (0.01275 * (-2239.1 / t_kelvin).exp() * fr_o2 / (fr_o2 * fr_o2 + f2)
                    + 0.10680 * (-3352.0 / t_kelvin).exp() * fr_n2 / (fr_n2 * fr_n2 + f2)));

    // Convert from Nepers/m to dB/m: 1 Np = 8.686 dB
    (alpha * 8.686).max(0.0)
}

/// Doppler-shifted frequency.
///
/// f' = f × (c + v_listener) / (c + v_source)
///
/// Positive velocity = moving toward each other.
/// v_source positive = source moving away from listener.
/// v_listener positive = listener moving toward source.
#[must_use]
#[inline]
pub fn doppler_shift(
    source_frequency: f32,
    source_velocity: f32,
    listener_velocity: f32,
    speed_of_sound: f32,
) -> f32 {
    let denominator = speed_of_sound + source_velocity;
    if denominator.abs() < f32::EPSILON {
        return source_frequency;
    }
    source_frequency * (speed_of_sound + listener_velocity) / denominator
}

/// Convert sound pressure level (dB SPL) to pressure in Pascals.
///
/// p = p_ref × 10^(dB/20), where p_ref = 20 µPa.
#[must_use]
#[inline]
pub fn db_spl_to_pressure(db_spl: f32) -> f32 {
    let p_ref = 20.0e-6; // 20 µPa reference
    p_ref * 10.0_f32.powf(db_spl / 20.0)
}

/// Convert pressure in Pascals to dB SPL.
///
/// dB = 20 × log10(p / p_ref)
#[must_use]
#[inline]
pub fn pressure_to_db_spl(pressure_pa: f32) -> f32 {
    let p_ref = 20.0e-6;
    if pressure_pa <= 0.0 {
        return f32::NEG_INFINITY;
    }
    20.0 * (pressure_pa / p_ref).log10()
}

/// Wind profile for atmospheric propagation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindProfile {
    /// Wind direction (normalized).
    pub direction: Vec3,
    /// Wind speed at ground level in m/s.
    pub speed_ground: f32,
    /// Speed increase per meter of height (m/s per m).
    pub gradient: f32,
}

/// Temperature profile for atmospheric propagation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureProfile {
    /// Temperature at ground level in Celsius.
    pub ground_temp_celsius: f32,
    /// Lapse rate in °C per meter (negative = temperature inversion).
    pub lapse_rate: f32,
}

/// Ground impedance model for surface reflection (Delany-Bazley).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundImpedance {
    /// Flow resistivity in Pa·s/m² (rayls/m).
    /// Typical values: grass ~200_000, hard soil ~2_000_000, asphalt ~20_000_000.
    pub flow_resistivity: f32,
}

impl GroundImpedance {
    /// Grass ground.
    #[must_use]
    pub fn grass() -> Self {
        Self {
            flow_resistivity: 200_000.0,
        }
    }

    /// Hard soil.
    #[must_use]
    pub fn hard_soil() -> Self {
        Self {
            flow_resistivity: 2_000_000.0,
        }
    }

    /// Asphalt / concrete.
    #[must_use]
    pub fn asphalt() -> Self {
        Self {
            flow_resistivity: 20_000_000.0,
        }
    }
}

/// Effective speed of sound at a given height, accounting for wind.
///
/// `c_eff = c(T) + wind_speed(height) × cos(angle)` where angle is between
/// the wind direction and ray direction.
#[must_use]
#[inline]
pub fn refracted_speed(
    base_speed: f32,
    wind: &WindProfile,
    ray_direction: Vec3,
    height: f32,
) -> f32 {
    let wind_speed = wind.speed_ground + wind.gradient * height;
    let cos_angle = wind.direction.dot(ray_direction);
    base_speed + wind_speed * cos_angle
}

/// Speed of sound at a given height, accounting for temperature gradient.
#[must_use]
#[inline]
pub fn speed_at_height(profile: &TemperatureProfile, height: f32) -> f32 {
    let temp = profile.ground_temp_celsius + profile.lapse_rate * height;
    speed_of_sound(temp)
}

/// Single step of ray refraction through a stratified atmosphere (Snell's law).
///
/// Given a ray at a certain position, updates the direction based on the speed
/// gradient at that height. Returns the new origin and direction after one step.
#[must_use]
pub fn refract_ray_step(
    origin: Vec3,
    direction: Vec3,
    speed_fn: impl Fn(f32) -> f32,
    step_size: f32,
) -> (Vec3, Vec3) {
    let height = origin.y;
    let c_current = speed_fn(height);

    // Advance one step
    let new_origin = origin + direction * step_size;
    let new_height = new_origin.y;
    let c_new = speed_fn(new_height);

    if c_current.abs() < f32::EPSILON {
        return (new_origin, direction);
    }

    // Apply Snell's law: sin(θ₁)/c₁ = sin(θ₂)/c₂
    // For a horizontal stratified medium, the horizontal component of the
    // "slowness vector" is conserved. We adjust the vertical component.
    let horizontal = Vec3::new(direction.x, 0.0, direction.z);
    let h_len = horizontal.length();

    if h_len < f32::EPSILON {
        // Purely vertical ray — no refraction
        return (new_origin, direction);
    }

    // Conserved quantity: sin(θ)/c = h_len/c_current
    let sin_ratio = h_len / c_current;
    let new_sin = sin_ratio * c_new;

    if new_sin >= 1.0 {
        // Total internal reflection — ray turns horizontal
        let new_dir = horizontal / h_len;
        return (new_origin, new_dir);
    }

    let new_cos = (1.0 - new_sin * new_sin).sqrt();
    let vert_sign = if direction.y >= 0.0 { 1.0 } else { -1.0 };
    let new_dir = Vec3::new(
        direction.x / h_len * new_sin,
        vert_sign * new_cos,
        direction.z / h_len * new_sin,
    );
    let len = new_dir.length();
    let new_dir = if len > f32::EPSILON {
        new_dir / len
    } else {
        direction
    };

    (new_origin, new_dir)
}

/// Trace a ray through a stratified atmosphere with wind and temperature gradients.
///
/// Returns the sequence of ray positions (path), useful for visualization and
/// computing curved propagation distances.
#[must_use]
#[tracing::instrument(skip(wind, temp), fields(max_distance, step_size))]
pub fn trace_ray_atmospheric(
    source: Vec3,
    direction: Vec3,
    wind: &WindProfile,
    temp: &TemperatureProfile,
    max_distance: f32,
    step_size: f32,
) -> Vec<Vec3> {
    let len = direction.length();
    let mut dir = if len > f32::EPSILON {
        direction / len
    } else {
        return vec![source];
    };
    let mut pos = source;
    let max_iterations = ((max_distance / step_size.max(0.001)) as u32).min(1_000_000);
    let mut path = Vec::with_capacity((max_iterations.min(10_000) + 1) as usize);
    path.push(pos);
    let mut total_distance = 0.0_f32;
    let mut iteration = 0_u32;
    while total_distance < max_distance && iteration < max_iterations {
        iteration += 1;
        let step = step_size.min(max_distance - total_distance);
        let current_dir = dir;
        let speed_fn = |h: f32| -> f32 {
            let base = speed_at_height(temp, h);
            refracted_speed(base, wind, current_dir, h)
        };
        let (new_pos, new_dir) = refract_ray_step(pos, dir, speed_fn, step);

        // Stop if ray hits ground
        if new_pos.y < 0.0 {
            // Interpolate to ground level
            if dir.y.abs() > f32::EPSILON {
                let t = -pos.y / dir.y;
                let ground_pos = pos + dir * t.max(0.0);
                path.push(ground_pos);
            }
            break;
        }

        pos = new_pos;
        dir = new_dir;
        total_distance += step;
        path.push(pos);
    }

    path
}

/// Ground reflection coefficient magnitude using the Miki model (1990).
///
/// Corrects the Delany-Bazley (1970) model to ensure positive real impedance
/// at low frequencies. Uses the same single parameter (flow resistivity) but
/// with corrected power-law coefficients.
///
/// Returns the magnitude of the reflection coefficient (0.0–1.0).
#[must_use]
#[inline]
pub fn ground_reflection_coefficient(
    frequency_hz: f32,
    grazing_angle_rad: f32,
    impedance: &GroundImpedance,
) -> f32 {
    if frequency_hz <= 0.0 || impedance.flow_resistivity <= 0.0 {
        return 1.0;
    }

    // Miki (1990) corrected impedance model
    let x = frequency_hz / impedance.flow_resistivity;

    // Normalized specific acoustic impedance Z/ρc (Miki coefficients)
    let z_real = 1.0 + 5.50 * x.powf(-0.632);
    let z_imag = -8.43 * x.powf(-0.632);

    // Reflection coefficient for grazing angle θ (angle from surface)
    // R = (Z sinθ - 1) / (Z sinθ + 1)
    let sin_theta = grazing_angle_rad.sin();
    let num_real = z_real * sin_theta - 1.0;
    let num_imag = z_imag * sin_theta;
    let den_real = z_real * sin_theta + 1.0;
    let den_imag = z_imag * sin_theta;

    let num_mag2 = num_real * num_real + num_imag * num_imag;
    let den_mag2 = den_real * den_real + den_imag * den_imag;

    if den_mag2 < f32::EPSILON {
        return 1.0;
    }

    (num_mag2 / den_mag2).sqrt().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_at_20c() {
        let c = speed_of_sound(20.0);
        assert!(
            (c - 343.42).abs() < 0.1,
            "speed at 20°C should be ~343.4 m/s, got {c}"
        );
    }

    #[test]
    fn speed_at_0c() {
        let c = speed_of_sound(0.0);
        assert!(
            (c - 331.3).abs() < 0.1,
            "speed at 0°C should be ~331.3 m/s, got {c}"
        );
    }

    #[test]
    fn inverse_square_halves_per_doubling() {
        let i1 = inverse_square_law(100.0, 1.0);
        let i2 = inverse_square_law(100.0, 2.0);
        let ratio = i1 / i2;
        assert!(
            (ratio - 4.0).abs() < 0.01,
            "intensity should drop 4x at 2x distance, got {ratio}"
        );
    }

    #[test]
    fn inverse_square_zero_distance() {
        assert_eq!(inverse_square_law(100.0, 0.0), 0.0);
    }

    #[test]
    fn spl_drop_doubling_distance() {
        let drop = spl_drop_with_distance(1.0, 2.0);
        assert!(
            (drop - 6.02).abs() < 0.1,
            "SPL drops ~6 dB per distance doubling, got {drop}"
        );
    }

    #[test]
    fn atmospheric_absorption_increases_with_frequency() {
        let a1k = atmospheric_absorption(1000.0, 50.0, 20.0, 1.0);
        let a4k = atmospheric_absorption(4000.0, 50.0, 20.0, 1.0);
        assert!(a4k > a1k, "absorption should increase with frequency");
    }

    #[test]
    fn doppler_approaching_increases_frequency() {
        let c = speed_of_sound(20.0);
        let shifted = doppler_shift(440.0, -30.0, 0.0, c);
        assert!(
            shifted > 440.0,
            "approaching source should increase frequency, got {shifted}"
        );
    }

    #[test]
    fn doppler_receding_decreases_frequency() {
        let c = speed_of_sound(20.0);
        let shifted = doppler_shift(440.0, 30.0, 0.0, c);
        assert!(
            shifted < 440.0,
            "receding source should decrease frequency, got {shifted}"
        );
    }

    #[test]
    fn doppler_stationary_no_shift() {
        let c = speed_of_sound(20.0);
        let shifted = doppler_shift(440.0, 0.0, 0.0, c);
        assert!((shifted - 440.0).abs() < 0.01);
    }

    #[test]
    fn db_spl_roundtrip() {
        let db = 94.0; // 1 Pa
        let pressure = db_spl_to_pressure(db);
        assert!(
            (pressure - 1.0).abs() < 0.01,
            "94 dB SPL should be ~1 Pa, got {pressure}"
        );
        let back = pressure_to_db_spl(pressure);
        assert!((back - db).abs() < 0.1);
    }

    #[test]
    fn spl_drop_equal_distance_is_zero() {
        let drop = spl_drop_with_distance(5.0, 5.0);
        assert!(
            drop.abs() < 0.001,
            "equal distances should give 0 dB drop, got {drop}"
        );
    }

    #[test]
    fn atmospheric_absorption_zero_humidity() {
        let a = atmospheric_absorption(1000.0, 0.0, 20.0, 1.0);
        assert_eq!(a, 0.0, "zero humidity should return 0");
    }

    #[test]
    fn atmospheric_absorption_negative_frequency() {
        let a = atmospheric_absorption(-100.0, 50.0, 20.0, 1.0);
        assert_eq!(a, 0.0, "negative frequency should return 0");
    }

    #[test]
    fn atmospheric_absorption_iso9613_1khz_20c_50rh() {
        // At 1kHz, 20°C, 50% RH, 1 atm → approximately 0.005 dB/m (ISO 9613-1 reference)
        let a = atmospheric_absorption(1000.0, 50.0, 20.0, 1.0);
        assert!(
            a > 0.001 && a < 0.02,
            "1kHz absorption should be ~0.005 dB/m, got {a}"
        );
    }

    #[test]
    fn atmospheric_absorption_8khz_much_higher() {
        let a1k = atmospheric_absorption(1000.0, 50.0, 20.0, 1.0);
        let a8k = atmospheric_absorption(8000.0, 50.0, 20.0, 1.0);
        assert!(
            a8k > a1k * 5.0,
            "8kHz should be much higher than 1kHz: {a8k} vs {a1k}"
        );
    }

    #[test]
    fn pressure_to_db_spl_zero_pressure() {
        let db = pressure_to_db_spl(0.0);
        assert!(db.is_infinite() && db < 0.0);
    }

    #[test]
    fn inverse_square_negative_distance() {
        assert_eq!(inverse_square_law(100.0, -5.0), 0.0);
    }

    // --- Advanced propagation tests ---

    #[test]
    fn wind_downwind_increases_effective_speed() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 10.0,
            gradient: 0.0,
        };
        let base = speed_of_sound(20.0);
        let eff = refracted_speed(base, &wind, Vec3::X, 0.0);
        assert!(
            eff > base,
            "downwind effective speed ({eff}) should exceed base ({base})"
        );
    }

    #[test]
    fn wind_upwind_decreases_effective_speed() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 10.0,
            gradient: 0.0,
        };
        let base = speed_of_sound(20.0);
        let eff = refracted_speed(base, &wind, -Vec3::X, 0.0);
        assert!(
            eff < base,
            "upwind effective speed ({eff}) should be less than base ({base})"
        );
    }

    #[test]
    fn wind_gradient_increases_with_height() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 5.0,
            gradient: 0.5,
        };
        let base = speed_of_sound(20.0);
        let eff_ground = refracted_speed(base, &wind, Vec3::X, 0.0);
        let eff_high = refracted_speed(base, &wind, Vec3::X, 100.0);
        assert!(
            eff_high > eff_ground,
            "wind speed should increase with height"
        );
    }

    #[test]
    fn speed_at_height_normal_lapse() {
        let profile = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: -0.0065, // standard atmosphere
        };
        let c_ground = speed_at_height(&profile, 0.0);
        let c_high = speed_at_height(&profile, 1000.0);
        assert!(
            c_ground > c_high,
            "speed should decrease with height under normal lapse"
        );
    }

    #[test]
    fn speed_at_height_inversion() {
        let profile = TemperatureProfile {
            ground_temp_celsius: 10.0,
            lapse_rate: 0.01, // temperature inversion
        };
        let c_ground = speed_at_height(&profile, 0.0);
        let c_high = speed_at_height(&profile, 100.0);
        assert!(
            c_high > c_ground,
            "speed should increase with height under inversion"
        );
    }

    #[test]
    fn atmospheric_trace_produces_path() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 0.0,
            gradient: 0.0,
        };
        let temp = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: 0.0,
        };
        let path =
            trace_ray_atmospheric(Vec3::new(0.0, 10.0, 0.0), Vec3::X, &wind, &temp, 100.0, 1.0);
        assert!(path.len() > 1, "should produce multiple path points");
    }

    #[test]
    fn atmospheric_trace_hits_ground() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 0.0,
            gradient: 0.0,
        };
        let temp = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: 0.0,
        };
        // Launch ray downward
        let dir = Vec3::new(1.0, -0.5, 0.0);
        let path = trace_ray_atmospheric(Vec3::new(0.0, 5.0, 0.0), dir, &wind, &temp, 1000.0, 0.5);
        let last = path.last().unwrap();
        assert!(last.y <= 0.01, "ray should hit ground, last y = {}", last.y);
    }

    #[test]
    fn ground_reflection_hard_surface_high() {
        let asphalt = GroundImpedance::asphalt();
        // At grazing angle, hard surface should reflect most energy
        let r = ground_reflection_coefficient(1000.0, 0.1, &asphalt);
        assert!(
            r > 0.8,
            "asphalt at grazing angle should have high reflection, got {r}"
        );
    }

    #[test]
    fn ground_reflection_grass_lower_than_asphalt() {
        let grass = GroundImpedance::grass();
        let asphalt = GroundImpedance::asphalt();
        let angle = PI / 6.0; // 30 degrees
        let r_grass = ground_reflection_coefficient(1000.0, angle, &grass);
        let r_asphalt = ground_reflection_coefficient(1000.0, angle, &asphalt);
        assert!(
            r_asphalt > r_grass,
            "asphalt ({r_asphalt}) should reflect more than grass ({r_grass})"
        );
    }

    #[test]
    fn ground_reflection_in_valid_range() {
        let soil = GroundImpedance::hard_soil();
        for freq in [125.0, 500.0, 1000.0, 4000.0] {
            for angle in [0.1, 0.5, 1.0, 1.5] {
                let r = ground_reflection_coefficient(freq, angle, &soil);
                assert!(
                    (0.0..=1.0).contains(&r),
                    "coefficient {r} out of range for freq={freq}, angle={angle}"
                );
            }
        }
    }

    #[test]
    fn refract_ray_step_preserves_direction_length() {
        let origin = Vec3::new(0.0, 100.0, 0.0);
        let dir = Vec3::new(0.6, -0.8, 0.0);
        let (_, new_dir) =
            refract_ray_step(origin, dir, |h| speed_of_sound(20.0 - 0.0065 * h), 10.0);
        let len = new_dir.length();
        assert!(
            (len - 1.0).abs() < 0.01,
            "direction should stay normalized, got length {len}"
        );
    }

    // --- Audit edge-case tests ---

    #[test]
    fn ground_reflection_zero_frequency() {
        let soil = GroundImpedance::hard_soil();
        let r = ground_reflection_coefficient(0.0, 0.5, &soil);
        assert!((r - 1.0).abs() < f32::EPSILON, "zero freq should give R=1");
    }

    #[test]
    fn ground_reflection_zero_resistivity() {
        let g = GroundImpedance {
            flow_resistivity: 0.0,
        };
        let r = ground_reflection_coefficient(1000.0, 0.5, &g);
        assert!(
            (r - 1.0).abs() < f32::EPSILON,
            "zero resistivity should give R=1"
        );
    }

    #[test]
    fn refract_ray_step_vertical_ray() {
        // Purely vertical ray should not refract
        let origin = Vec3::new(0.0, 100.0, 0.0);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let (new_pos, new_dir) = refract_ray_step(origin, dir, |_| 343.0, 10.0);
        assert!(
            (new_dir.y - (-1.0)).abs() < 0.01,
            "vertical ray should stay vertical"
        );
        assert!((new_pos.y - 90.0).abs() < 0.1);
    }

    #[test]
    fn atmospheric_trace_zero_max_distance() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 0.0,
            gradient: 0.0,
        };
        let temp = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: 0.0,
        };
        let path =
            trace_ray_atmospheric(Vec3::new(0.0, 10.0, 0.0), Vec3::X, &wind, &temp, 0.0, 1.0);
        assert_eq!(path.len(), 1, "zero distance should produce source only");
    }

    #[test]
    fn refracted_speed_no_wind() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 0.0,
            gradient: 0.0,
        };
        let base = speed_of_sound(20.0);
        let eff = refracted_speed(base, &wind, Vec3::X, 0.0);
        assert!(
            (eff - base).abs() < f32::EPSILON,
            "no wind should give base speed"
        );
    }

    #[test]
    fn speed_at_height_zero() {
        let profile = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: -0.01,
        };
        let c = speed_at_height(&profile, 0.0);
        assert!((c - speed_of_sound(20.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn spl_drop_zero_distance_ref() {
        assert_eq!(spl_drop_with_distance(0.0, 5.0), 0.0);
    }

    #[test]
    fn spl_drop_zero_distance() {
        assert_eq!(spl_drop_with_distance(5.0, 0.0), 0.0);
    }

    #[test]
    fn doppler_denominator_near_zero() {
        // source velocity = -speed_of_sound → denominator ≈ 0
        let c = speed_of_sound(20.0);
        let f = doppler_shift(440.0, -c, 0.0, c);
        assert!((f - 440.0).abs() < 0.01, "should return source frequency");
    }

    #[test]
    fn refract_ray_step_zero_speed() {
        let origin = Vec3::new(0.0, 100.0, 0.0);
        let dir = Vec3::new(0.6, -0.8, 0.0);
        let (_, new_dir) = refract_ray_step(origin, dir, |_| 0.0, 10.0);
        // Should return original direction when c_current ≈ 0
        assert!((new_dir.length() - 1.0).abs() < 0.1 || new_dir == dir);
    }

    #[test]
    fn refract_ray_step_zero_length_result() {
        // Extreme speed ratio causing near-zero refracted direction
        let origin = Vec3::new(0.0, 100.0, 0.0);
        let dir = Vec3::new(0.99, -0.01, 0.0);
        let (_, new_dir) =
            refract_ray_step(origin, dir, |h| if h > 99.0 { 1000.0 } else { 1.0 }, 10.0);
        let len = new_dir.length();
        assert!(len > 0.5, "direction should remain valid, got length {len}");
    }

    #[test]
    fn atmospheric_trace_zero_direction() {
        let wind = WindProfile {
            direction: Vec3::X,
            speed_ground: 0.0,
            gradient: 0.0,
        };
        let temp = TemperatureProfile {
            ground_temp_celsius: 20.0,
            lapse_rate: 0.0,
        };
        let path = trace_ray_atmospheric(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::ZERO,
            &wind,
            &temp,
            100.0,
            1.0,
        );
        assert_eq!(path.len(), 1, "zero direction should return source only");
    }

    #[test]
    fn ground_reflection_denominator_near_zero() {
        // Very low frequency + high impedance → denominator can be small
        let g = GroundImpedance {
            flow_resistivity: 1e12,
        };
        let r = ground_reflection_coefficient(0.001, 0.01, &g);
        assert!((0.0..=1.0).contains(&r));
    }
}
