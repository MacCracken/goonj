/// Axial room mode frequency for a single dimension.
///
/// f_n = n × c / (2 × L)
///
/// Where n = mode number (1, 2, 3...), c = speed of sound, L = dimension length.
#[must_use]
#[inline]
pub fn room_mode(dimension_length: f32, mode_number: u32, speed_of_sound: f32) -> f32 {
    if dimension_length <= 0.0 || mode_number == 0 {
        return 0.0;
    }
    mode_number as f32 * speed_of_sound / (2.0 * dimension_length)
}

/// All axial modes for a room dimension below a maximum frequency.
#[must_use]
pub fn axial_modes(dimension_length: f32, max_frequency: f32, speed_of_sound: f32) -> Vec<f32> {
    let mut modes = Vec::new();
    let mut n = 1;
    loop {
        let f = room_mode(dimension_length, n, speed_of_sound);
        if f > max_frequency || f <= 0.0 {
            break;
        }
        modes.push(f);
        n += 1;
    }
    modes
}

/// All axial modes for a shoebox room (length, width, height) below max_frequency.
///
/// Returns sorted list of all axial mode frequencies.
#[must_use]
pub fn all_axial_modes(
    length: f32,
    width: f32,
    height: f32,
    max_frequency: f32,
    speed_of_sound: f32,
) -> Vec<f32> {
    let mut modes = Vec::new();
    modes.extend(axial_modes(length, max_frequency, speed_of_sound));
    modes.extend(axial_modes(width, max_frequency, speed_of_sound));
    modes.extend(axial_modes(height, max_frequency, speed_of_sound));
    modes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    modes
}

/// Schroeder frequency — the transition frequency between modal and diffuse behavior.
///
/// f_s = 2000 × √(RT60 / V)
///
/// Below this frequency, room modes dominate. Above, the sound field is diffuse.
#[must_use]
#[inline]
pub fn schroeder_frequency(rt60: f32, volume: f32) -> f32 {
    if volume <= 0.0 || rt60 <= 0.0 {
        return 0.0;
    }
    2000.0 * (rt60 / volume).sqrt()
}

/// Modal density — average number of modes per Hz at a given frequency.
///
/// n(f) = 4π × V × f² / c³
///
/// Higher modal density means more diffuse behavior.
#[must_use]
#[inline]
pub fn modal_density(frequency: f32, volume: f32, speed_of_sound: f32) -> f32 {
    if speed_of_sound <= 0.0 {
        return 0.0;
    }
    let c3 = speed_of_sound * speed_of_sound * speed_of_sound;
    4.0 * std::f32::consts::PI * volume * frequency * frequency / c3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_mode_5m_room() {
        // c ≈ 343 m/s, L = 5m → f1 = 343 / 10 = 34.3 Hz
        let f = room_mode(5.0, 1, 343.0);
        assert!(
            (f - 34.3).abs() < 0.1,
            "first mode of 5m room should be ~34.3 Hz, got {f}"
        );
    }

    #[test]
    fn second_mode_double_first() {
        let f1 = room_mode(5.0, 1, 343.0);
        let f2 = room_mode(5.0, 2, 343.0);
        assert!(
            (f2 / f1 - 2.0).abs() < 0.01,
            "second mode should be 2× first"
        );
    }

    #[test]
    fn mode_zero_returns_zero() {
        assert_eq!(room_mode(5.0, 0, 343.0), 0.0);
    }

    #[test]
    fn mode_zero_length_returns_zero() {
        assert_eq!(room_mode(0.0, 1, 343.0), 0.0);
    }

    #[test]
    fn axial_modes_count() {
        let modes = axial_modes(5.0, 200.0, 343.0);
        // f1=34.3, f2=68.6, f3=102.9, f4=137.2, f5=171.5 → 5 modes below 200 Hz
        assert_eq!(
            modes.len(),
            5,
            "should have 5 modes below 200 Hz, got {}",
            modes.len()
        );
    }

    #[test]
    fn all_axial_modes_sorted() {
        let modes = all_axial_modes(10.0, 8.0, 3.0, 200.0, 343.0);
        for window in modes.windows(2) {
            assert!(window[0] <= window[1], "modes should be sorted");
        }
    }

    #[test]
    fn schroeder_frequency_basic() {
        // RT60 = 1.0s, V = 100 m³ → fs = 2000 × √(1/100) = 200 Hz
        let fs = schroeder_frequency(1.0, 100.0);
        assert!(
            (fs - 200.0).abs() < 1.0,
            "Schroeder frequency should be ~200 Hz, got {fs}"
        );
    }

    #[test]
    fn schroeder_larger_room_lower_frequency() {
        let fs_small = schroeder_frequency(1.0, 50.0);
        let fs_large = schroeder_frequency(1.0, 500.0);
        assert!(
            fs_large < fs_small,
            "larger room should have lower Schroeder frequency"
        );
    }

    #[test]
    fn schroeder_longer_rt60_higher_frequency() {
        let fs_short = schroeder_frequency(0.5, 100.0);
        let fs_long = schroeder_frequency(2.0, 100.0);
        assert!(
            fs_long > fs_short,
            "longer RT60 should raise Schroeder frequency"
        );
    }

    #[test]
    fn modal_density_increases_with_frequency() {
        let d100 = modal_density(100.0, 200.0, 343.0);
        let d1000 = modal_density(1000.0, 200.0, 343.0);
        assert!(d1000 > d100, "modal density should increase with frequency");
    }

    #[test]
    fn modal_density_increases_with_volume() {
        let d_small = modal_density(500.0, 50.0, 343.0);
        let d_large = modal_density(500.0, 500.0, 343.0);
        assert!(
            d_large > d_small,
            "modal density should increase with volume"
        );
    }
}
