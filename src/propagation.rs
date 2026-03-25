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

/// Simplified atmospheric absorption coefficient (dB/m) at a given frequency and humidity.
///
/// Loosely based on ISO 9613-1 but heavily simplified: models absorption as
/// proportional to f² with an inverse humidity factor. Accuracy is within an
/// order of magnitude for 125 Hz–4 kHz at 20–80% RH. For precision work
/// (outdoor propagation >100 m, ultrasonic frequencies), use a full ISO 9613-1
/// implementation instead.
///
/// Returns absorption in dB per meter.
#[must_use]
pub fn atmospheric_absorption(frequency_hz: f32, humidity_percent: f32) -> f32 {
    // Simplified model: absorption increases with f² and decreases with humidity
    let f_khz = frequency_hz / 1000.0;
    let humidity_factor = if humidity_percent > 0.0 {
        50.0 / humidity_percent
    } else {
        10.0
    };
    // Approximate: ~0.001 dB/m at 1kHz, 50% humidity
    0.001 * f_khz * f_khz * humidity_factor / 50.0
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
        let a1k = atmospheric_absorption(1000.0, 50.0);
        let a4k = atmospheric_absorption(4000.0, 50.0);
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
        let a = atmospheric_absorption(1000.0, 0.0);
        assert!(a > 0.0, "should still produce absorption at zero humidity");
    }

    #[test]
    fn atmospheric_absorption_negative_frequency() {
        let a = atmospheric_absorption(-100.0, 50.0);
        // f² is always positive, so result is positive regardless of sign
        assert!(a >= 0.0);
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
}
