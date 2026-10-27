//! Quantized octant angles generated mathematically, without external lookup data.

pub(super) fn reference_angle(lateral: f32, forward: f32) -> f32 {
    quantized_angle(lateral, forward, true)
}

pub(super) fn rolling_angle(lateral: f32, forward: f32) -> f32 {
    quantized_angle(lateral, forward, false)
}

fn quantized_angle(lateral: f32, forward: f32, single_precision: bool) -> f32 {
    static OCTANT: std::sync::OnceLock<[u16; 257]> = std::sync::OnceLock::new();
    let nodes = OCTANT.get_or_init(|| {
        std::array::from_fn(|i| ((i as f64 / 256.0).atan() * 65536.0 / std::f64::consts::TAU).round() as u16)
    });
    let y = match lateral.abs() <= 1e-6 {
        true => 0.0,
        false => lateral.abs(),
    };
    let x = match forward.abs() <= 1e-6 {
        true => 0.0,
        false => forward.abs(),
    };
    let largest = x.max(y);
    if largest == 0.0 {
        return 0.0;
    }
    let ratio = match single_precision {
        true => (x.min(y) / largest * 65536.0) as u32,
        false => (f64::from(x.min(y)) / f64::from(largest) * 65536.0) as u32,
    };
    let index = (ratio >> 8) as usize;
    let base = u32::from(nodes[index]);
    let next = u32::from(nodes[(index + 1).min(256)]);
    let interpolated = base + (((next - base) * (ratio & 255)) >> 8);
    let angle = match y > x {
        true => 16384 - interpolated,
        false => interpolated,
    };
    let sign = match lateral < 0.0 {
        true => -1.0,
        false => 1.0,
    };
    sign * angle as f32 / 65536.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_angle_matches_original_axes_octants_and_near_zero_vectors() {
        // Independently executed original helper; outputs are exact binary angle fractions.
        let cases = [
            (0.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 0.0, 0.25),
            (-1.0, 0.0, -0.25),
            (1.0, 1.0, 0.125),
            (-1.0, 1.0, -0.125),
            (0.3, 15.0, 208.0 / 65536.0),
            (2.0, 20.0, 1039.0 / 65536.0),
            (1e-6, 1.0, 0.0),
            (1.0, 1e-6, 0.25),
            (1e-7, 1e-7, 0.0),
            (2e-6, 1.0, 0.0),
            (1.0, 2e-6, 0.25),
        ];
        for (lateral, forward, expected) in cases {
            assert_eq!(reference_angle(lateral, forward), expected, "{lateral}, {forward}");
            assert_eq!(reference_angle(lateral, -forward), expected, "reverse {lateral}, {forward}");
        }
    }

    #[test]
    fn reference_ratio_rounds_at_single_precision_before_integer_interpolation() {
        let cases = [
            (2.7855225, f32::from_bits(0x42a5_947e), 351.0),
            (-14.129293, 10.194452, -9865.0),
            (1.0246124, -0.25790414, 13812.0),
            (-4.02332, 9.628348, -4128.0),
        ];
        for (lateral, forward, units) in cases {
            assert_eq!(reference_angle(lateral, forward), units / 65536.0);
            assert_eq!((reference_angle(lateral, forward) - rolling_angle(lateral, forward)).abs(), 1.0 / 65536.0);
        }
    }
}
