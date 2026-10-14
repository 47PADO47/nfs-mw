//! The processor's authored approximations and symmetric FIR coefficients.
use crate::{Result, invalid};

const PI: f32 = std::f32::consts::PI;
const TAU: f32 = std::f32::consts::TAU;

fn reduced(value: f32) -> f64 {
    let mut value = f64::from(value);
    while value > f64::from(TAU) {
        value -= f64::from(TAU);
    }
    value
}

pub(super) fn cosine(value: f32) -> f64 {
    let x = reduced(value);
    let square = x * x;
    let fourth = square * square;
    let sixth = fourth * square;
    let eighth = fourth * fourth;
    let tenth = eighth * square;
    let twelfth = sixth * sixth;
    1.0 - square * 0.5 + fourth * f64::from(1.0f32 / 24.0) - sixth * f64::from(1.0f32 / 720.0)
        + eighth * f64::from(1.0f32 / 40320.0)
        - tenth * f64::from(1.0f32 / 3628800.0)
        + twelfth * f64::from(1.0f32 / 479001600.0)
}

fn sine(value: f32) -> f64 {
    let x = reduced(value);
    let square = x * x;
    let third = x * square;
    let fifth = third * square;
    let seventh = fifth * square;
    let ninth = seventh * square;
    let eleventh = (ninth * square) as f32;
    x - third * f64::from(1.0f32 / 6.0) + fifth * f64::from(0.00833333f32) - seventh * f64::from(1.0f32 / 5040.0)
        + ninth * f64::from(1.0f32 / 362880.0)
        - f64::from(eleventh) * f64::from(1.0f32 / 39916800.0)
        + f64::from(eleventh) * square * f64::from(1.6059e-10f32)
}

pub(super) fn square_root_one_plus(value: f32) -> f64 {
    let x = f64::from(value);
    let mut term = x * 0.5;
    let mut terms = [0.0; 7];
    terms[0] = term;
    for (i, multiplier) in [-0.25f32, -0.5, -0.625, -0.7, -0.75, -0.7857].into_iter().enumerate() {
        term *= x * f64::from(multiplier);
        terms[i + 1] = term;
    }
    1.0 + terms.into_iter().rev().sum::<f64>()
}

pub(super) fn high_pass(rate: u32, cutoff: i16) -> [f32; 5] {
    let normalized = (f32::from(cutoff) * 2.0 / rate as f32).min(0.8);
    let mut coefficients = [0.0; 5];
    for (index, coefficient) in coefficients.iter_mut().enumerate() {
        let distance = 4 - index;
        let raw = match distance {
            0 => f64::from(normalized),
            _ => {
                let angle = distance as f32 * PI;
                f64::from(
                    (sine((distance as f64 * f64::from(PI) * f64::from(normalized)) as f32) / f64::from(angle)) as f32,
                )
            }
        };
        let window = f64::from(0.54f32) - cosine(index as f32 * (PI * 0.25)) * f64::from(0.46f32);
        *coefficient = (-raw * window) as f32;
    }
    let mut alternating = 0.0f32;
    for (i, &coefficient) in coefficients.iter().enumerate() {
        alternating += match i % 2 {
            0 => coefficient,
            _ => -coefficient,
        };
    }
    coefficients[4] += 1.0;
    alternating += 1.0;
    let normalization = (f64::from(alternating) * 2.0 - f64::from(coefficients[4])).abs().recip();
    for coefficient in &mut coefficients {
        *coefficient = (f64::from(*coefficient) * normalization) as f32;
    }
    coefficients
}

pub(super) fn delay_length(rate: u32, milliseconds: i16) -> Result<usize> {
    if milliseconds < 0 {
        return Err(invalid("negative delay"));
    }
    let mut candidate = (u64::from(rate) * (milliseconds as u64 / 16) / 1000).max(3);
    while (2..).take_while(|divisor| divisor * divisor <= candidate).any(|divisor| candidate.is_multiple_of(divisor)) {
        candidate += 1;
    }
    let length = usize::try_from(candidate * 16).map_err(|_| invalid("delay size overflow"))?;
    if length > 2_000_000 {
        return Err(invalid("delay resource limit"));
    }
    Ok(length)
}
