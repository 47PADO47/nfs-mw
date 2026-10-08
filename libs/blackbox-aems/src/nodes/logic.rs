//! Trigger and chance nodes.

use super::Rng;
use crate::mem::Mem;

/// Reads a flag word and clears it.
pub fn flag(m: &mut Mem, at: usize) -> i32 {
    let v = m.i32(at);
    m.set_i32(at, 0);
    v
}

/// Nodes that call into the host's function system (unused by the shipped banks): the function node's trigger
/// byte is read and cleared, the others do nothing.
pub fn function_flag(m: &mut Mem, id: u32, p: usize) -> i32 {
    if id != 37 {
        return 0;
    }
    let v = m.u8(p + 0x19);
    m.set_u8(p + 0x19, 0);
    i32::from(v)
}

pub fn counter(m: &mut Mem, p: usize) -> i32 {
    let (min, max, direction) = (m.i32(p), m.i32(p + 4), i32::from(m.i8(p + 0xC)));
    let over = m.i32(p + 0x14);
    if over >= min && over <= max {
        return over;
    }
    let mut value = m.i32(p + 8);
    if m.i32(p + 0x10) > 0 {
        value = value.wrapping_add(direction);
        if value > max {
            value = min;
        } else if value < min {
            value = max;
        }
        m.set_i32(p + 8, value);
    }
    value
}

fn draw(rng: &mut Rng, range: u32) -> u32 {
    if range == 0 { 0 } else { rng.next() % range }
}

pub fn random(m: &mut Mem, rng: &mut Rng, p: usize) -> i32 {
    if m.i32(p + 0xC) == 0 {
        return m.i32(p + 8);
    }
    let out = (draw(rng, m.i32(p + 4) as u32) as i32).wrapping_add(m.i32(p));
    m.set_i32(p + 8, out);
    out
}

pub fn random_shuffle(m: &mut Mem, rng: &mut Rng, p: usize) -> i32 {
    let inputs = p + usize::from(m.u16(p));
    if m.i32(inputs) == 0 {
        return m.i32(p + 0xC);
    }
    let (wide, avoid) = (m.u8(p + 2) != 1, i32::from(m.i8(p + 3)));
    let (index, range) = (usize::from(m.u16(p + 8)), i32::from(m.u16(p + 0xA)));
    let span = (range - index as i32 - avoid).max(0) as u32;
    let swap = draw(rng, span) as usize + index;
    let (set, size) = (p + 0x10, if wide { 2 } else { 1 });
    let (a, b) = (set + swap * size, set + index * size);
    let out = if wide {
        let (va, vb) = (m.u16(a), m.u16(b));
        m.set_u16(a, vb);
        m.set_u16(b, va);
        i32::from(va)
    } else {
        let (va, vb) = (m.u8(a), m.u8(b));
        m.set_u8(a, vb);
        m.set_u8(b, va);
        i32::from(va)
    };
    let next = index + 1;
    let out = out.wrapping_add(m.i32(p + 4));
    if next as i32 >= range {
        m.set_u16(p + 8, 0);
        m.set_i8(p + 3, 1);
    } else {
        m.set_u16(p + 8, next as u16);
        m.set_i8(p + 3, 0);
    }
    m.set_i32(p + 0xC, out);
    out
}

pub fn random_weighted(m: &mut Mem, rng: &mut Rng, p: usize) -> i32 {
    if m.i32(p + 0x10) != 0 {
        let roll = draw(rng, 100) as i32;
        let table = m.u32(p) as usize;
        let mut total = 0;
        for i in 0..m.i32(p + 8).max(0) as usize {
            total += i32::from(m.i8(table + 0x10 + i));
            if total > roll {
                m.set_i32(p + 0xC, (i as i32).wrapping_add(m.i32(p + 4)));
                break;
            }
        }
    }
    m.i32(p + 0xC)
}

pub fn range_trigger(m: &mut Mem, p: usize) -> i32 {
    let input = m.i32(p + 0x14);
    let (trip_lo, trip_hi, reset_lo, reset_hi) = (m.i32(p), m.i32(p + 4), m.i32(p + 8), m.i32(p + 0xC));
    if input >= trip_lo && input <= trip_hi {
        if m.i8(p + 0x10) == 0 {
            m.set_i8(p + 0x10, 1);
            m.set_i8(p + 0x11, 1);
            return 1;
        }
    } else if input >= reset_lo && input <= reset_hi {
        m.set_i8(p + 0x10, 0);
    }
    m.set_i8(p + 0x11, 0);
    0
}

pub fn delay_trigger(m: &mut Mem, period: f32, p: usize) -> i32 {
    let mut time = m.f32(p);
    if m.i32(p + 8) != 0 {
        time = 0.0;
    }
    if time >= 0.0 {
        if time >= m.i32(p + 0xC) as f32 {
            m.set_f32(p, -1.0);
            m.set_i8(p + 4, 1);
            return 1;
        }
        time += period;
    }
    m.set_f32(p, time);
    m.set_i8(p + 4, 0);
    0
}

pub fn state_generator(m: &mut Mem, p: usize) -> i32 {
    let inputs = p + usize::from(m.u16(p));
    for i in 0..usize::from(m.u8(p + 2)) {
        if m.i32(inputs + 4 * i) != 0 {
            let value = m.i32(p + 8 + 4 * i);
            m.set_i32(p + 4, value);
            break;
        }
    }
    m.i32(p + 4)
}

pub fn merge(m: &mut Mem, p: usize) -> i32 {
    i32::from((0..usize::from(m.u8(p))).any(|i| m.i32(p + 4 + 4 * i) != 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(words: &[i32]) -> Mem {
        Mem::new(words.iter().flat_map(|w| w.to_le_bytes()).collect())
    }

    #[test]
    fn a_counter_steps_wraps_and_obeys_the_override() {
        let mut m = mem(&[0, 2, 2, 1, 1, -1]);
        m.set_i8(0xC, 1);
        assert_eq!(counter(&mut m, 0), 0, "2 + 1 wraps to the minimum");
        m.set_i32(0x10, 0);
        assert_eq!(counter(&mut m, 0), 0, "not triggered: holds");
        m.set_i32(0x14, 1);
        assert_eq!(counter(&mut m, 0), 1, "override inside the range");
    }

    #[test]
    fn a_range_trigger_fires_once_until_it_is_reset() {
        let mut m = mem(&[10, 20, 0, 5, 0, 0]);
        m.set_i32(0x14, 15);
        assert_eq!(range_trigger(&mut m, 0), 1);
        assert_eq!(range_trigger(&mut m, 0), 0, "held");
        m.set_i32(0x14, 3);
        assert_eq!(range_trigger(&mut m, 0), 0);
        m.set_i32(0x14, 12);
        assert_eq!(range_trigger(&mut m, 0), 1, "tripped again after the reset range");
    }

    #[test]
    fn a_delay_trigger_waits_then_fires_once() {
        let mut m = mem(&[0, 0, 0, 40]);
        m.set_i32(8, 1);
        let mut outs = vec![delay_trigger(&mut m, 16.0, 0)];
        m.set_i32(8, 0);
        outs.extend((0..5).map(|_| delay_trigger(&mut m, 16.0, 0)));
        assert_eq!(outs, vec![0, 0, 0, 1, 0, 0]);
    }

    #[test]
    fn merge_and_state_generator() {
        let mut m = mem(&[3, 0, 0, 7]);
        m.set_u8(0, 3);
        assert_eq!(merge(&mut m, 0), 1);
        m.set_i32(0xC, 0);
        assert_eq!(merge(&mut m, 0), 0);
        // inputs at +0x10 (offset 16), two of them; values from +8.
        let mut s = mem(&[16, 0, 11, 22, 0, 1]);
        s.set_u16(0, 0x10);
        s.set_u8(2, 2);
        assert_eq!(state_generator(&mut s, 0), 22);
    }

    #[test]
    fn a_shuffle_visits_every_value_once_per_round() {
        // 4 values 0..3 in u8s from +0x10, inputs at +0x20.
        let mut m = Mem::new(vec![0; 0x30]);
        m.set_u16(0, 0x20);
        m.set_u8(2, 1);
        m.set_u16(0xA, 4);
        for i in 0..4 {
            m.set_u8(0x10 + i, i as u8);
        }
        m.set_i32(0x20, 1);
        let mut rng = Rng::new(7);
        let mut seen: Vec<i32> = (0..4).map(|_| random_shuffle(&mut m, &mut rng, 0)).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2, 3]);
        assert_eq!(m.u16(8), 0, "the round restarted");
    }

    #[test]
    fn random_stays_in_range_and_holds_without_a_trigger() {
        let mut m = mem(&[5, 3, 0, 1]);
        let mut rng = Rng::new(3);
        for _ in 0..50 {
            let v = random(&mut m, &mut rng, 0);
            assert!((5..8).contains(&v));
        }
        m.set_i32(0xC, 0);
        let held = random(&mut m, &mut rng, 0);
        assert_eq!(random(&mut m, &mut rng, 0), held);
    }
}
