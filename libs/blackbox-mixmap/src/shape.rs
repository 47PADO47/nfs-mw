//! Levels, decibels and curve shapes of the dynamic mixer. Spec: `docs/specs/dynamic-mixer.md` §2.
//!
//! A level is a Q15 number (`0x7FFF` = unity) or hundredths of a dB (`-10000` = silence). The original uses two
//! lookup tables for the conversions; both are mathematical functions and are computed here.

use std::sync::OnceLock;

/// Unity in Q15.
pub const UNITY: i32 = 0x7FFF;
/// Silence in hundredths of a dB.
pub const SILENCE_DB: i32 = -10_000;
/// Hundredths of a dB per halving of the amplitude.
const STEP: i32 = 602;

struct Tables {
    /// `round(16384 * 10^(j / 2000))` for `j` in 0..=601.
    gain: [i32; 602],
    /// `floor(602 * log2(1 + m / 512))` for the 9-bit mantissa `m`.
    mantissa_db: [i32; 512],
    /// `floor(32767 * cos(i * pi / 1024))` for `i` in 0..=513.
    cosine: [i32; 514],
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut t = Tables { gain: [0; 602], mantissa_db: [0; 512], cosine: [0; 514] };
        for (j, g) in t.gain.iter_mut().enumerate() {
            *g = (16384.0 * 10f64.powf(j as f64 / 2000.0)).round() as i32;
        }
        for (m, d) in t.mantissa_db.iter_mut().enumerate() {
            *d = (f64::from(STEP) * (1.0 + m as f64 / 512.0).log2() + 1e-9).floor() as i32;
        }
        for (i, c) in t.cosine.iter_mut().enumerate() {
            *c = (32767.0 * (i as f64 * std::f64::consts::PI / 1024.0).cos() + 1e-9).floor().max(0.0) as i32;
        }
        t
    })
}

/// The Q15 level of `d` hundredths of a dB (`d <= 0`; a positive value is read as unity).
pub fn q15_from_db(d: i32) -> i32 {
    if d >= 0 {
        return UNITY;
    }
    let shift = d / -STEP;
    let rem = -(d % STEP);
    if shift >= 31 {
        return 0;
    }
    let value = tables().gain[(601 - rem) as usize] >> shift;
    if value < 0x32 { 0 } else { value }
}

/// Hundredths of a dB of a Q15 level; `-10000` for 0 and below.
pub fn db_from_q15(q: i32) -> i32 {
    if q <= 0 {
        return SILENCE_DB;
    }
    let q = q.min(UNITY);
    let n = if q & 0x4000 != 0 {
        0
    } else {
        match (1..=16).find(|n| (0x4000 >> n) & q != 0) {
            Some(n) => n,
            None => return SILENCE_DB,
        }
    };
    let rem = q - (0x4000 >> n);
    let mantissa = if n > 4 { (rem << (n - 5)) | ((1 << (n - 5)) - 1) } else { rem >> (5 - n) };
    let d = -STEP * (n + 1) + tables().mantissa_db[(mantissa as usize).min(511)];
    d.max(SILENCE_DB)
}

fn equal_power(x: i32) -> i32 {
    let t = &tables().cosine;
    let i = (x >> 6) as usize;
    let rem = ((x & 0x1F) << 9) | 0x3FF;
    let main = t[i];
    let diff = ((t[i + 1] - main) * rem) >> 15;
    if main == 0 { 0 } else { main + diff }
}

fn one_minus_equal_power(x: i32) -> i32 {
    let t = &tables().cosine;
    let i = (0x1FF - (x >> 6)) as usize;
    let rem = ((x & 0x1F) << 9) | 0x3FF;
    let out = UNITY - t[i];
    out + (((t[i] - t[i + 1]) * rem) >> 15)
}

/// The Q15 output of curve shape `shape` (0 to 9) for the Q15 input `x`; 0 for an unknown shape.
pub fn curve(shape: u8, x: i32) -> i32 {
    let x = x.clamp(0, UNITY);
    match shape {
        0 => equal_power(x),
        1 => equal_power(UNITY - x),
        2 => (equal_power(x) * equal_power(x)) >> 15,
        3 => curve(2, UNITY - x),
        4 => one_minus_equal_power(x),
        5 => one_minus_equal_power(UNITY - x),
        6 => (one_minus_equal_power(x) * one_minus_equal_power(x)) >> 15,
        7 => curve(6, UNITY - x),
        8 => UNITY - x,
        9 => x,
        _ => 0,
    }
}

fn cut(d: i32) -> i32 {
    if d < -0x2580 { SILENCE_DB } else { d }
}

/// The output of curve shape `shape` in hundredths of a dB (the engine's distance filter asks for it so).
pub fn curve_db(shape: u8, x: i32) -> i32 {
    let x = x.clamp(0, UNITY);
    match shape {
        0 => cut(db_from_q15(equal_power(x))),
        2 => cut(curve_db(0, x) * 2),
        4 => cut(db_from_q15(one_minus_equal_power(x))),
        6 => cut(curve_db(4, x) * 2),
        1 | 3 | 5 | 7 => -curve_db(shape - 1, x),
        8 => db_from_q15(UNITY - x),
        9 => -curve_db(8, x),
        _ => 0,
    }
}

/// The playback ratio of `cents` (1200 per octave): 1.0 at 0.
pub fn pitch_ratio(cents: i32) -> f32 {
    (2f64.powf(f64::from(cents) / 1200.0)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_conversions_are_inverse_within_a_unit() {
        for d in (-1500..=0).step_by(7) {
            let back = db_from_q15(q15_from_db(d));
            assert!((back - d).abs() <= 3, "{d} -> {back}");
        }
    }

    #[test]
    fn known_points() {
        assert_eq!(q15_from_db(0), 0x7FFF);
        assert_eq!(q15_from_db(-601), 16384);
        // -6.02 dB is a halving; -12.04 a quartering (within the table's accuracy).
        assert!((q15_from_db(-602) - 16384).abs() < 40);
        assert!((q15_from_db(-1204) - 8192).abs() < 40);
        assert_eq!(db_from_q15(0), -10000);
        assert_eq!(db_from_q15(0x7FFF), -1);
        assert_eq!(db_from_q15(0x4000), -602);
        assert_eq!(q15_from_db(-10000), 0);
        assert_eq!(db_from_q15(100), -5023);
    }

    #[test]
    fn q15_rises_with_the_level() {
        let mut last = 0;
        for d in (-5000..=0).step_by(11) {
            let q = q15_from_db(d);
            assert!(q >= last, "{d}: {q} < {last}");
            last = q;
        }
        assert!(q15_from_db(-2000) < q15_from_db(-1000));
        assert!(q15_from_db(-1000) < q15_from_db(-10));
    }

    #[test]
    fn the_cosine_table_ends_at_zero() {
        let t = &tables().cosine;
        assert_eq!(t[0], 32767);
        assert_eq!(t[1], 32766);
        assert_eq!(t[511], 100);
        assert_eq!(t[512], 0);
    }

    #[test]
    fn down_shapes_fall_and_up_shapes_rise() {
        for shape in [0u8, 2, 4, 6, 8] {
            let (a, b, c) = (curve(shape, 0), curve(shape, 16384), curve(shape, UNITY));
            assert!(a >= b && b >= c, "shape {shape}: {a} {b} {c}");
        }
        assert!(curve(0, 0) > 32000 && curve(0, UNITY) < 100);
        for shape in [1u8, 3, 5, 7, 9] {
            let (a, b, c) = (curve(shape, 0), curve(shape, 16384), curve(shape, UNITY));
            assert!(a <= b && b <= c, "shape {shape}: {a} {b} {c}");
        }
        assert_eq!(curve(8, 1000), UNITY - 1000);
        assert_eq!(curve(9, 1000), 1000);
        assert_eq!(curve(10, 1000), 0);
    }

    #[test]
    fn equal_power_curves_are_close_to_a_quarter_cosine() {
        for x in (0..=UNITY).step_by(997) {
            let ideal = 32767.0 * (f64::from(x) / 32767.0 * std::f64::consts::FRAC_PI_2).cos();
            assert!((f64::from(curve(0, x)) - ideal).abs() < 600.0, "{x}");
        }
    }

    #[test]
    fn out_of_range_inputs_are_clamped() {
        assert_eq!(curve(8, -5), UNITY);
        assert_eq!(curve(8, 99999), 0);
        assert_eq!(curve(0, i32::MAX), curve(0, UNITY));
    }

    #[test]
    fn decibel_shapes_follow_the_q15_shapes() {
        for shape in [0u8, 2, 4, 6, 8] {
            let q = curve(shape, 12000);
            let d = curve_db(shape, 12000);
            if q > 400 {
                assert!((d - db_from_q15(q)).abs() <= 602 + 3, "shape {shape}: {d} vs {}", db_from_q15(q));
            }
        }
        assert_eq!(curve_db(9, 5000), -curve_db(8, 5000));
    }

    #[test]
    fn pitch_ratio_is_octaves() {
        assert!((pitch_ratio(0) - 1.0).abs() < 1e-6);
        assert!((pitch_ratio(1200) - 2.0).abs() < 1e-5);
        assert!((pitch_ratio(-1200) - 0.5).abs() < 1e-5);
        assert!((pitch_ratio(-4800) - 0.0625).abs() < 1e-5);
    }
}
