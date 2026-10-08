//! Arithmetic and selection nodes.

use super::ftoi;
use crate::mem::Mem;

fn inputs(m: &Mem, count: usize, from: usize) -> impl Iterator<Item = i32> + '_ {
    (0..count).map(move |i| m.i32(from + 4 * i))
}

pub fn mux(m: &mut Mem, p: usize) -> i32 {
    let control = m.i32(p + 4);
    if control <= 0 || control > i32::from(m.u8(p)) {
        return 0;
    }
    m.i32(p + 4 + 4 * control as usize)
}

pub fn demux(m: &mut Mem, p: usize) -> i32 {
    let count = i32::from(m.u8(p));
    let previous = i32::from(m.i16(p + 2));
    let slot = |n: i32| p + 0xC + 4 * (n - 1).max(0) as usize;
    m.set_i32(slot(previous), 0);
    let control = m.i32(p + 4);
    if control > 0 && control <= count {
        let value = m.i32(p + 8);
        m.set_i32(slot(control), value);
        m.set_i16(p + 2, control as i16);
    }
    m.i32(p + 0xC)
}

pub fn min(m: &mut Mem, p: usize) -> i32 {
    inputs(m, usize::from(m.u8(p)).max(1), p + 4).min().unwrap_or(0)
}

pub fn max(m: &mut Mem, p: usize) -> i32 {
    inputs(m, usize::from(m.u8(p)).max(1), p + 4).max().unwrap_or(0)
}

pub fn scale(m: &mut Mem, p: usize) -> i32 {
    let product = inputs(m, usize::from(m.u8(p)).max(1), p + 8).fold(1.0f64, |acc, v| acc * f64::from(v));
    ftoi((product * f64::from(m.f32(p + 4))) as f32)
}

pub fn add(m: &mut Mem, p: usize) -> i32 {
    inputs(m, usize::from(m.u8(p)).max(1), p + 4).fold(0i32, i32::wrapping_add)
}

pub fn subtract(m: &mut Mem, p: usize) -> i32 {
    m.i32(p).wrapping_sub(m.i32(p + 4))
}

pub fn multiply(m: &mut Mem, p: usize) -> i32 {
    m.i32(p).wrapping_mul(m.i32(p + 4))
}

pub fn divide(m: &mut Mem, p: usize) -> i32 {
    let b = m.i32(p + 4);
    if b == 0 { 0 } else { m.i32(p).wrapping_div(b) }
}

pub fn modulo(m: &mut Mem, p: usize) -> i32 {
    let b = m.i32(p + 4);
    if b == 0 { 0 } else { m.i32(p).wrapping_rem(b) }
}

pub fn add_capped(m: &mut Mem, p: usize) -> i32 {
    let sum = inputs(m, usize::from(m.u8(p)).max(1), p + 8).fold(0i32, i32::wrapping_add);
    sum.min(m.i32(p + 4))
}

pub fn subtract_floored(m: &mut Mem, p: usize) -> i32 {
    m.i32(p + 4).wrapping_sub(m.i32(p + 8)).max(m.i32(p))
}

pub fn multiply_capped(m: &mut Mem, p: usize) -> i32 {
    m.i32(p + 4).wrapping_mul(m.i32(p + 8)).min(m.i32(p))
}

pub fn min2(m: &mut Mem, p: usize) -> i32 {
    m.i32(p).min(m.i32(p + 4))
}

pub fn max2(m: &mut Mem, p: usize) -> i32 {
    m.i32(p).max(m.i32(p + 4))
}

pub fn scale2(m: &mut Mem, p: usize) -> i32 {
    let product = f64::from(m.i32(p + 8)) * f64::from(m.i32(p + 4)) * f64::from(m.f32(p));
    ftoi(product as f32)
}

pub fn add2(m: &mut Mem, p: usize) -> i32 {
    m.i32(p).wrapping_add(m.i32(p + 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(words: &[i32]) -> Mem {
        Mem::new(words.iter().flat_map(|w| w.to_le_bytes()).collect())
    }

    #[test]
    fn two_input_arithmetic() {
        let mut m = mem(&[7, 2]);
        assert_eq!(
            (subtract(&mut m, 0), multiply(&mut m, 0), divide(&mut m, 0), modulo(&mut m, 0), add2(&mut m, 0)),
            (5, 14, 3, 1, 9)
        );
        assert_eq!((min2(&mut m, 0), max2(&mut m, 0)), (2, 7));
        m.set_i32(4, 0);
        assert_eq!((divide(&mut m, 0), modulo(&mut m, 0)), (0, 0), "no division by zero");
    }

    #[test]
    fn capped_and_floored_arithmetic() {
        let mut m = mem(&[10, 6, 9]);
        assert_eq!(subtract_floored(&mut m, 0), 10, "6 - 9 is below the floor 10");
        assert_eq!(multiply_capped(&mut m, 0), 10, "capped at 10");
        let mut a = mem(&[0, 100, 70, 60]);
        a.set_u8(0, 2);
        assert_eq!(add_capped(&mut a, 0), 100);
    }

    #[test]
    fn scale2_rounds_to_nearest() {
        let mut m = mem(&[0, 3, 5]);
        m.set_f32(0, 0.5);
        assert_eq!(scale2(&mut m, 0), 8, "7.5 rounds to the even 8");
        m.set_f32(0, 1.0 / 3.0);
        assert_eq!(scale2(&mut m, 0), 5);
    }

    #[test]
    fn variable_arity_nodes() {
        let mut m = mem(&[3, 4, 9, 2]);
        assert_eq!((min(&mut m, 0), max(&mut m, 0), add(&mut m, 0)), (2, 9, 15));
        let mut s = mem(&[2, 0, 6, 7]);
        s.set_f32(4, 0.25);
        assert_eq!(scale(&mut s, 0), 10, "42 / 4 = 10.5 rounds to the even 10");
    }

    #[test]
    fn mux_and_demux_route_by_control() {
        let mut m = mem(&[3, 2, 10, 20, 30]);
        assert_eq!(mux(&mut m, 0), 20);
        m.set_i32(4, 0);
        assert_eq!(mux(&mut m, 0), 0, "control 0 selects nothing");
        m.set_i32(4, 9);
        assert_eq!(mux(&mut m, 0), 0);
        let mut d = mem(&[0, 1, 77, 0, 0, 0]);
        d.set_u8(0, 3);
        d.set_i32(4, 2);
        d.set_i16(2, 0);
        demux(&mut d, 0);
        assert_eq!((d.i32(0xC), d.i32(0x10), d.i16(2)), (0, 77, 2));
        d.set_i32(4, 1);
        assert_eq!(demux(&mut d, 0), 77);
        assert_eq!(d.i32(0x10), 0, "the old output was cleared");
    }
}
