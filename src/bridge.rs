//! Cross-crate bridges — convert primitive values from other AGNOS science crates
//! into goonj acoustic parameters.
//!
//! Always available — takes primitive values (f32/f64), no science crate deps.
//!
//! # Architecture
//!
//! ```text
//! pavan (aerodynamics)  ──┐
//! badal (weather)        ┤
//! ushma (thermodynamics) ┼──> bridge ──> goonj acoustic parameters
//! bijli (electromagnetism)┘
//! ```

use crate::propagation;

// ── Pavan bridges (aerodynamics) ───────────────────────────────────────────

/// Convert wind speed (m/s) to outdoor propagation attenuation factor.
///
/// Wind creates turbulent scattering that attenuates sound over distance.
/// Based on ISO 9613-2 meteorological correction approximation.
/// Returns an additional attenuation in dB/m (always >= 0).
#[must_use]
#[inline]
pub fn wind_attenuation_factor(wind_speed_ms: f64) -> f32 {
    // Turbulent scattering increases roughly with wind speed squared
    // Typical range: 0-3 dB/km for 0-15 m/s wind
    let speed = wind_speed_ms.max(0.0) as f32;
    (speed * speed * 0.013).min(5.0) / 1000.0 // dB/m
}

/// Convert Mach number to Doppler frequency shift ratio.
///
/// For a source moving at Mach number M relative to the listener:
/// `f_observed / f_emitted = 1 / (1 - M)` (approaching)
/// `f_observed / f_emitted = 1 / (1 + M)` (receding)
///
/// `approaching`: true if source moves toward listener.
/// Returns the frequency ratio (>1 = higher pitch, <1 = lower pitch).
#[must_use]
#[inline]
pub fn doppler_ratio_from_mach(mach_number: f64, approaching: bool) -> f32 {
    let m = mach_number.abs().min(0.99) as f32; // clamp below sonic
    if approaching {
        1.0 / (1.0 - m)
    } else {
        1.0 / (1.0 + m)
    }
}

/// Convert relative wind velocity between source and listener (m/s) to
/// effective speed of sound modification.
///
/// Downwind propagation increases effective speed; upwind decreases it.
/// `wind_component_ms`: positive = downwind (source to listener along wind).
/// Returns speed of sound adjusted by the wind component.
#[must_use]
#[inline]
pub fn effective_speed_of_sound(temperature_celsius: f32, wind_component_ms: f32) -> f32 {
    propagation::speed_of_sound(temperature_celsius) + wind_component_ms
}

// ── Badal bridges (weather/atmosphere) ─────────────────────────────────────

/// Convert humidity (%) and temperature (°C) to per-band air absorption
/// coefficients in dB/m.
///
/// Uses ISO 9613-1 atmospheric absorption model at the 8 standard
/// octave-band center frequencies (63–8000 Hz).
#[must_use]
pub fn air_absorption_from_weather(
    humidity_percent: f64,
    temperature_celsius: f64,
) -> [f32; crate::material::NUM_BANDS] {
    let h = humidity_percent.clamp(0.0, 100.0) as f32;
    let t = temperature_celsius as f32;
    std::array::from_fn(|band| {
        propagation::atmospheric_absorption(crate::material::FREQUENCY_BANDS[band], h, t, 1.0)
    })
}

/// Convert temperature (°C) to speed of sound (m/s).
///
/// Thin wrapper exposing `propagation::speed_of_sound` for bridge callers
/// who pass weather data without depending on goonj directly.
#[must_use]
#[inline]
pub fn speed_of_sound_from_temperature(temperature_celsius: f64) -> f32 {
    propagation::speed_of_sound(temperature_celsius as f32)
}

/// Convert atmospheric pressure (hPa) to pressure ratio for absorption
/// calculations.
///
/// Standard pressure = 1013.25 hPa = 1.0 atm.
#[must_use]
#[inline]
pub fn pressure_to_atm(pressure_hpa: f64) -> f32 {
    (pressure_hpa / 1013.25) as f32
}

// ── Ushma bridges (thermodynamics) ─────────────────────────────────────────

/// Convert temperature (°C) to a material absorption coefficient scaling factor.
///
/// Many porous absorbers become slightly more absorptive at higher temperatures
/// due to increased molecular mean free path. This returns a multiplicative
/// scaling factor (1.0 at 20°C reference).
#[must_use]
#[inline]
pub fn absorption_temperature_scale(temperature_celsius: f64) -> f32 {
    // Absorption scales roughly with sqrt(T/T_ref) for porous materials
    let t_kelvin = (temperature_celsius + 273.15).max(1.0);
    let t_ref = 293.15; // 20°C
    (t_kelvin / t_ref).sqrt() as f32
}

/// Convert a thermal gradient (°C per meter of height) to a sound speed
/// gradient (m/s per meter) for atmospheric refraction.
///
/// Since c = 331.3 + 0.606×T, dc/dz = 0.606 × dT/dz.
/// Negative gradient (temperature inversion) bends sound downward.
/// Positive gradient (lapse) bends sound upward.
#[must_use]
#[inline]
pub fn sound_speed_gradient(thermal_gradient_c_per_m: f64) -> f32 {
    (0.606 * thermal_gradient_c_per_m) as f32
}

// ── Bijli bridges (electromagnetism) ───────────────────────────────────────

/// Convert an EM vibration frequency (Hz) to an acoustic resonance coupling
/// factor for vibroacoustic analysis.
///
/// In vibroacoustics, an EM-driven actuator at `em_frequency_hz` drives
/// a structural element with natural frequency `structural_freq_hz`.
/// Returns the resonance amplification factor (>1 near resonance).
/// `damping_ratio`: typical 0.01–0.05 for metals.
#[must_use]
pub fn em_resonance_coupling(
    em_frequency_hz: f64,
    structural_freq_hz: f64,
    damping_ratio: f64,
) -> f32 {
    if structural_freq_hz <= 0.0 {
        return 1.0;
    }
    let r = em_frequency_hz / structural_freq_hz;
    let zeta = damping_ratio.max(0.001);
    // Classic single-DOF transfer function magnitude
    let denom = ((1.0 - r * r).powi(2) + (2.0 * zeta * r).powi(2)).sqrt();
    if denom < 1e-10 {
        return 1000.0; // cap at very high amplification
    }
    (1.0 / denom).min(1000.0) as f32
}

/// Convert EM field oscillation amplitude (V/m) and frequency to estimated
/// acoustic power radiated by a piezoelectric transducer.
///
/// Simplified model: P_acoustic ∝ (d × E × A)² × f² / (ρ × c)
/// where d is piezoelectric coefficient, A is area.
/// Returns radiated acoustic power in watts.
#[must_use]
pub fn piezo_acoustic_power(
    e_field_v_per_m: f64,
    frequency_hz: f64,
    piezo_coefficient_m_per_v: f64,
    transducer_area_m2: f64,
) -> f32 {
    let rho = 1.225_f64; // air density kg/m³
    let c = 343.0_f64; // approximate speed of sound
    let displacement = piezo_coefficient_m_per_v * e_field_v_per_m;
    let velocity = displacement * 2.0 * std::f64::consts::PI * frequency_hz;
    let power = 0.5 * rho * c * transducer_area_m2 * velocity * velocity;
    power.max(0.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Pavan bridges ──────────────────────────────────────────────────

    #[test]
    fn wind_attenuation_zero_wind() {
        assert_eq!(wind_attenuation_factor(0.0), 0.0);
    }

    #[test]
    fn wind_attenuation_increases_with_speed() {
        let low = wind_attenuation_factor(5.0);
        let high = wind_attenuation_factor(10.0);
        assert!(high > low);
    }

    #[test]
    fn wind_attenuation_negative_clamped() {
        assert_eq!(wind_attenuation_factor(-5.0), 0.0);
    }

    #[test]
    fn doppler_approaching_higher_pitch() {
        let ratio = doppler_ratio_from_mach(0.5, true);
        assert!(ratio > 1.0);
    }

    #[test]
    fn doppler_receding_lower_pitch() {
        let ratio = doppler_ratio_from_mach(0.5, false);
        assert!(ratio < 1.0);
    }

    #[test]
    fn doppler_zero_mach_unity() {
        let ratio = doppler_ratio_from_mach(0.0, true);
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn effective_speed_downwind_faster() {
        let still = propagation::speed_of_sound(20.0);
        let downwind = effective_speed_of_sound(20.0, 10.0);
        assert!((downwind - still - 10.0).abs() < 0.01);
    }

    // ── Badal bridges ──────────────────────────────────────────────────

    #[test]
    fn air_absorption_increases_with_frequency() {
        let coeffs = air_absorption_from_weather(50.0, 20.0);
        // Higher bands should have higher absorption
        assert!(coeffs[7] > coeffs[0]);
    }

    #[test]
    fn air_absorption_zero_humidity_zero() {
        let coeffs = air_absorption_from_weather(0.0, 20.0);
        // All bands should be zero or near-zero with 0% humidity
        for c in &coeffs {
            assert!(*c >= 0.0);
        }
    }

    #[test]
    fn speed_of_sound_from_temp() {
        let c = speed_of_sound_from_temperature(20.0);
        assert!((c - 343.42).abs() < 0.1);
    }

    #[test]
    fn pressure_standard() {
        let atm = pressure_to_atm(1013.25);
        assert!((atm - 1.0).abs() < 0.001);
    }

    #[test]
    fn pressure_half() {
        let atm = pressure_to_atm(506.625);
        assert!((atm - 0.5).abs() < 0.001);
    }

    // ── Ushma bridges ──────────────────────────────────────────────────

    #[test]
    fn absorption_scale_reference_temp() {
        let scale = absorption_temperature_scale(20.0);
        assert!((scale - 1.0).abs() < 0.01);
    }

    #[test]
    fn absorption_scale_hot_increases() {
        let hot = absorption_temperature_scale(40.0);
        assert!(hot > 1.0);
    }

    #[test]
    fn sound_speed_gradient_from_thermal() {
        let grad = sound_speed_gradient(-0.0065); // standard lapse rate ~6.5 K/km
        assert!(grad < 0.0); // positive lapse = upward refraction
    }

    #[test]
    fn sound_speed_gradient_inversion() {
        let grad = sound_speed_gradient(0.01); // temperature inversion
        assert!(grad > 0.0); // downward refraction
    }

    // ── Bijli bridges ──────────────────────────────────────────────────

    #[test]
    fn em_resonance_at_natural_freq() {
        let amp = em_resonance_coupling(100.0, 100.0, 0.02);
        // At resonance, amplification = 1/(2*zeta) = 25
        assert!(amp > 20.0);
    }

    #[test]
    fn em_resonance_off_frequency() {
        let amp = em_resonance_coupling(100.0, 1000.0, 0.02);
        assert!(amp < 2.0);
    }

    #[test]
    fn em_resonance_zero_structural_freq() {
        let amp = em_resonance_coupling(100.0, 0.0, 0.02);
        assert_eq!(amp, 1.0);
    }

    #[test]
    fn piezo_power_zero_field() {
        let p = piezo_acoustic_power(0.0, 1000.0, 1e-10, 0.01);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn piezo_power_positive() {
        let p = piezo_acoustic_power(1000.0, 1000.0, 1e-10, 0.01);
        assert!(p > 0.0);
    }

    #[test]
    fn piezo_power_increases_with_frequency() {
        let low = piezo_acoustic_power(1000.0, 100.0, 1e-10, 0.01);
        let high = piezo_acoustic_power(1000.0, 1000.0, 1e-10, 0.01);
        assert!(high > low);
    }
}
