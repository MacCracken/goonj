//! Underwater acoustics — ocean sound propagation models.
//!
//! Provides ocean-specific acoustic models that replace the atmospheric
//! equivalents in [`crate::propagation`] for underwater applications:
//!
//! - **Sound speed**: Mackenzie equation (temperature, salinity, depth)
//! - **Absorption**: Francois-Garrison model (chemical relaxation in seawater)
//! - **Seabed reflection**: Hamilton sediment impedance model
//! - **Surface scattering**: Sea surface roughness loss
//!
//! These work with goonj's existing ray tracing and refraction infrastructure
//! — the physics is the same (Snell's law), only the medium parameters differ.

use crate::material::NUM_BANDS;
use serde::{Deserialize, Serialize};

/// Ocean sound speed profile parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OceanProfile {
    /// Water temperature in °C (typical: -2 to +30).
    pub temperature_celsius: f32,
    /// Salinity in parts per thousand (typical: 33–37 ppt, open ocean ~35).
    pub salinity_ppt: f32,
    /// Depth in meters (positive downward).
    pub depth_m: f32,
}

/// Sound speed in seawater using the Mackenzie equation (1981).
///
/// Valid for: T = 2–30°C, S = 25–40 ppt, D = 0–8000 m.
/// Accuracy: ±0.07 m/s over the valid range.
///
/// Reference: K.V. Mackenzie, "Nine-term equation for sound speed in the
/// oceans," JASA 70(3), 1981.
#[must_use]
#[inline]
pub fn ocean_sound_speed(profile: &OceanProfile) -> f32 {
    let t = profile.temperature_celsius;
    let s = profile.salinity_ppt;
    let d = profile.depth_m;

    // Mackenzie (1981) nine-term equation
    1448.96 + 4.591 * t - 0.05304 * t * t
        + 0.0002374 * t * t * t
        + 1.340 * (s - 35.0)
        + 0.01630 * d
        + 1.675e-7 * d * d
        - 0.01025 * t * (s - 35.0)
        - 7.139e-13 * t * d * d * d
}

/// Sound speed at a given depth using a linear thermocline model.
///
/// Models a surface mixed layer with constant temperature transitioning
/// to a thermocline with a linear temperature gradient.
///
/// # Arguments
/// * `surface` — ocean profile at the surface
/// * `thermocline_gradient` — temperature change per meter of depth (typically -0.01 to -0.1 °C/m)
/// * `mixed_layer_depth` — depth of the surface mixed layer in meters (typically 20–200 m)
/// * `depth` — evaluation depth in meters
#[must_use]
pub fn ocean_speed_at_depth(
    surface: &OceanProfile,
    thermocline_gradient: f32,
    mixed_layer_depth: f32,
    depth: f32,
) -> f32 {
    let temp = if depth <= mixed_layer_depth {
        surface.temperature_celsius
    } else {
        surface.temperature_celsius + thermocline_gradient * (depth - mixed_layer_depth)
    };

    ocean_sound_speed(&OceanProfile {
        temperature_celsius: temp.max(-2.0),
        salinity_ppt: surface.salinity_ppt,
        depth_m: depth,
    })
}

/// Acoustic absorption in seawater using the Francois-Garrison model (1982).
///
/// Models frequency-dependent absorption from chemical relaxation of
/// boric acid (B(OH)₃) and magnesium sulphate (MgSO₄), plus pure water
/// viscous absorption.
///
/// Valid for: 100 Hz – 1 MHz, T = -6–35°C, S = 7.7–43 ppt, D = 0–7000 m.
///
/// Returns absorption in dB/km.
///
/// Reference: R.E. Francois & G.R. Garrison, "Sound absorption based on
/// ocean measurements," JASA 72(6), 1982.
#[must_use]
pub fn ocean_absorption_db_per_km(frequency_hz: f32, profile: &OceanProfile) -> f32 {
    if frequency_hz <= 0.0 {
        return 0.0;
    }

    let t = profile.temperature_celsius;
    let s = profile.salinity_ppt;
    let d = profile.depth_m;
    let f = frequency_hz / 1000.0; // convert to kHz for formula
    let f2 = f * f;

    let t_kelvin = t + 273.15;
    let c = ocean_sound_speed(profile);

    // Pressure factor (depth dependent)
    let p_factor = 1.0 - 1.37e-4 * d + 6.2e-9 * d * d;

    // Boric acid contribution (low frequency, < 1 kHz)
    let a1 = 8.86 / c * 10.0_f32.powf(0.78 * (s / 35.0).log10() - 5.0);
    let f1 = 2.8 * (s / 35.0).sqrt() * 10.0_f32.powf(4.0 - 1245.0 / t_kelvin);
    let boric = a1 * f1 * f2 / (f1 * f1 + f2) * p_factor;

    // Magnesium sulphate contribution (mid frequency, 1–100 kHz)
    let a2 = 21.44 * s / c * (1.0 + 0.025 * t);
    let f2_relax = 8.17 * 10.0_f32.powf(8.0 - 1990.0 / t_kelvin) / (1.0 + 0.0018 * (s - 35.0));
    let mgso4 = a2 * f2_relax * f2 / (f2_relax * f2_relax + f2) * p_factor;

    // Pure water viscous absorption (high frequency, > 100 kHz)
    let a3 = if t <= 20.0 {
        4.937e-4 - 2.59e-5 * t + 9.11e-7 * t * t - 1.50e-8 * t * t * t
    } else {
        3.964e-4 - 1.146e-5 * t + 1.45e-7 * t * t - 6.5e-10 * t * t * t
    };
    let pure_water = a3 * f2 * p_factor;

    (boric + mgso4 + pure_water).max(0.0)
}

/// Per-band ocean absorption in dB/km.
#[must_use]
pub fn ocean_absorption_bands(profile: &OceanProfile) -> [f32; NUM_BANDS] {
    std::array::from_fn(|band| {
        ocean_absorption_db_per_km(crate::material::FREQUENCY_BANDS[band], profile)
    })
}

/// Seabed sediment properties for bottom reflection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeabedSediment {
    /// Compressional wave speed in the sediment in m/s.
    pub speed: f32,
    /// Sediment density in kg/m³.
    pub density: f32,
    /// Compressional attenuation in dB/wavelength.
    pub attenuation_db_per_wavelength: f32,
}

impl SeabedSediment {
    /// Sand (Hamilton 1980).
    #[must_use]
    pub fn sand() -> Self {
        Self {
            speed: 1749.0,
            density: 1941.0,
            attenuation_db_per_wavelength: 0.88,
        }
    }

    /// Silt (Hamilton 1980).
    #[must_use]
    pub fn silt() -> Self {
        Self {
            speed: 1575.0,
            density: 1740.0,
            attenuation_db_per_wavelength: 1.05,
        }
    }

    /// Clay (Hamilton 1980).
    #[must_use]
    pub fn clay() -> Self {
        Self {
            speed: 1500.0,
            density: 1500.0,
            attenuation_db_per_wavelength: 0.20,
        }
    }

    /// Rock / basalt.
    #[must_use]
    pub fn rock() -> Self {
        Self {
            speed: 5250.0,
            density: 2700.0,
            attenuation_db_per_wavelength: 0.10,
        }
    }
}

/// Bottom reflection coefficient magnitude using the Rayleigh model.
///
/// Computes the plane-wave reflection coefficient at the water-sediment
/// interface for a given grazing angle.
///
/// R = (m × sin θ − n) / (m × sin θ + n)
///
/// where m = ρ_sed / ρ_water (density ratio) and
/// n = √(m² − cos²θ) / (c_water / c_sed)².
///
/// Returns |R| in range 0.0–1.0.
#[must_use]
#[inline]
pub fn bottom_reflection_coefficient(
    grazing_angle_rad: f32,
    water_speed: f32,
    water_density: f32,
    sediment: &SeabedSediment,
) -> f32 {
    if water_speed <= 0.0
        || water_density <= 0.0
        || sediment.speed <= 0.0
        || sediment.density <= 0.0
    {
        return 1.0;
    }

    let sin_theta = grazing_angle_rad.sin();
    let cos_theta = grazing_angle_rad.cos();

    let m = sediment.density / water_density;
    let speed_ratio = water_speed / sediment.speed;
    let speed_ratio2 = speed_ratio * speed_ratio;

    // n = sqrt(1 - (c_sed/c_water)² × cos²θ) — refracted angle term
    let inner = 1.0 - cos_theta * cos_theta / (speed_ratio2);
    if inner < 0.0 {
        // Total internal reflection (critical angle exceeded)
        return 1.0;
    }
    let n = inner.sqrt();

    let num = (m * sin_theta - n).abs();
    let den = m * sin_theta + n;
    if den.abs() < f32::EPSILON {
        return 1.0;
    }

    (num / den).clamp(0.0, 1.0)
}

/// Sea surface scattering loss due to wave roughness.
///
/// Uses the Eckart model: scattering loss increases with frequency and
/// sea state (wave height). Returns loss in dB.
///
/// # Arguments
/// * `frequency_hz` — acoustic frequency
/// * `grazing_angle_rad` — angle from horizontal
/// * `rms_wave_height` — RMS wave height in meters (sea state dependent)
/// * `water_speed` — sound speed in water at the surface
#[must_use]
#[inline]
pub fn surface_scattering_loss(
    frequency_hz: f32,
    grazing_angle_rad: f32,
    rms_wave_height: f32,
    water_speed: f32,
) -> f32 {
    if frequency_hz <= 0.0 || rms_wave_height <= 0.0 || water_speed <= 0.0 {
        return 0.0;
    }

    let wavelength = water_speed / frequency_hz;
    let sin_theta = grazing_angle_rad.sin().abs();

    // Rayleigh roughness parameter: R = 4π × h_rms × sin(θ) / λ
    let roughness = 4.0 * std::f32::consts::PI * rms_wave_height * sin_theta / wavelength;

    // Eckart model: scattering loss ≈ exp(-R²/2) → dB
    let coherent = (-roughness * roughness / 2.0).exp();
    if coherent < 1e-10 {
        return 100.0; // cap at 100 dB loss
    }
    -20.0 * coherent.log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_ocean() -> OceanProfile {
        OceanProfile {
            temperature_celsius: 15.0,
            salinity_ppt: 35.0,
            depth_m: 100.0,
        }
    }

    #[test]
    fn mackenzie_standard_conditions() {
        // Standard ocean: ~15°C, 35 ppt, 100m → speed ≈ 1507 m/s
        let c = ocean_sound_speed(&standard_ocean());
        assert!(
            c > 1490.0 && c < 1530.0,
            "standard ocean speed should be ~1507 m/s, got {c}"
        );
    }

    #[test]
    fn speed_increases_with_temperature() {
        let cold = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 5.0,
            salinity_ppt: 35.0,
            depth_m: 0.0,
        });
        let warm = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 25.0,
            salinity_ppt: 35.0,
            depth_m: 0.0,
        });
        assert!(warm > cold, "warmer water should have higher speed");
    }

    #[test]
    fn speed_increases_with_depth() {
        let shallow = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 15.0,
            salinity_ppt: 35.0,
            depth_m: 0.0,
        });
        let deep = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 15.0,
            salinity_ppt: 35.0,
            depth_m: 4000.0,
        });
        assert!(
            deep > shallow,
            "deeper water should have higher speed (pressure effect)"
        );
    }

    #[test]
    fn speed_increases_with_salinity() {
        let fresh = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 15.0,
            salinity_ppt: 0.0,
            depth_m: 0.0,
        });
        let salt = ocean_sound_speed(&OceanProfile {
            temperature_celsius: 15.0,
            salinity_ppt: 35.0,
            depth_m: 0.0,
        });
        assert!(salt > fresh, "saltier water should have higher speed");
    }

    #[test]
    fn thermocline_speed_profile() {
        let surface = OceanProfile {
            temperature_celsius: 20.0,
            salinity_ppt: 35.0,
            depth_m: 0.0,
        };
        let c_surface = ocean_speed_at_depth(&surface, -0.05, 50.0, 0.0);
        let c_deep = ocean_speed_at_depth(&surface, -0.05, 50.0, 500.0);
        // Below thermocline, temperature drops → speed changes
        // But depth increases → competing effects. At 500m with -0.05°C/m below 50m:
        // temp ≈ 20 - 0.05*(500-50) = 20 - 22.5 = -2.5°C (clamped to -2)
        assert!(c_surface != c_deep, "speed should vary with depth");
    }

    #[test]
    fn francois_garrison_absorption() {
        let profile = standard_ocean();
        // At 10 kHz, typical absorption ≈ 1 dB/km
        let a_10k = ocean_absorption_db_per_km(10000.0, &profile);
        assert!(
            a_10k > 0.5 && a_10k < 5.0,
            "10 kHz absorption should be ~1 dB/km, got {a_10k}"
        );
    }

    #[test]
    fn absorption_increases_with_frequency() {
        let profile = standard_ocean();
        let a_1k = ocean_absorption_db_per_km(1000.0, &profile);
        let a_100k = ocean_absorption_db_per_km(100000.0, &profile);
        assert!(
            a_100k > a_1k,
            "higher frequency should have more absorption"
        );
    }

    #[test]
    fn absorption_bands_valid() {
        let profile = standard_ocean();
        let bands = ocean_absorption_bands(&profile);
        for &a in &bands {
            assert!(a >= 0.0, "absorption should be non-negative");
        }
    }

    #[test]
    fn bottom_reflection_sand() {
        let sand = SeabedSediment::sand();
        let r = bottom_reflection_coefficient(0.3, 1500.0, 1025.0, &sand);
        assert!(
            (0.0..=1.0).contains(&r),
            "reflection should be [0,1], got {r}"
        );
    }

    #[test]
    fn bottom_reflection_grazing_high() {
        // At very grazing angles, reflection should be high
        let sand = SeabedSediment::sand();
        let r = bottom_reflection_coefficient(0.05, 1500.0, 1025.0, &sand);
        assert!(
            r > 0.5,
            "grazing angle should have high reflection, got {r}"
        );
    }

    #[test]
    fn bottom_reflection_rock_vs_clay() {
        let rock = SeabedSediment::rock();
        let clay = SeabedSediment::clay();
        let r_rock = bottom_reflection_coefficient(0.5, 1500.0, 1025.0, &rock);
        let r_clay = bottom_reflection_coefficient(0.5, 1500.0, 1025.0, &clay);
        assert!(
            r_rock > r_clay,
            "rock ({r_rock}) should reflect more than clay ({r_clay})"
        );
    }

    #[test]
    fn surface_scattering_calm_low_loss() {
        // Calm sea (small waves) at low frequency → minimal scattering
        let loss = surface_scattering_loss(1000.0, 0.3, 0.1, 1500.0);
        assert!(
            loss < 5.0,
            "calm sea should have low scattering, got {loss}"
        );
    }

    #[test]
    fn surface_scattering_rough_more_loss() {
        let calm = surface_scattering_loss(10000.0, 0.5, 0.1, 1500.0);
        let rough = surface_scattering_loss(10000.0, 0.5, 2.0, 1500.0);
        assert!(
            rough > calm,
            "rough sea ({rough}) should scatter more than calm ({calm})"
        );
    }

    #[test]
    fn surface_scattering_zero_waves() {
        assert_eq!(surface_scattering_loss(1000.0, 0.5, 0.0, 1500.0), 0.0);
    }

    #[test]
    fn sediment_presets_valid() {
        for sed in [
            SeabedSediment::sand(),
            SeabedSediment::silt(),
            SeabedSediment::clay(),
            SeabedSediment::rock(),
        ] {
            assert!(sed.speed > 0.0);
            assert!(sed.density > 0.0);
            assert!(sed.attenuation_db_per_wavelength >= 0.0);
        }
    }

    #[test]
    fn ocean_profile_serializes() {
        let p = standard_ocean();
        let json = serde_json::to_string(&p).unwrap();
        let back: OceanProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
