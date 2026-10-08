//! Nodes that shape a value over time or through a table.

use super::ftoi;
use crate::mem::Mem;

/// Programs the current envelope segment: its duration, and the step per tick toward its target.
fn program_segment(m: &mut Mem, period: f32, p: usize) {
    let at = p + 0x18 + 8 * usize::from(m.u8(p + 3));
    let (duration, target) = (m.f32(at), m.f32(at + 4));
    m.set_f32(p + 4, duration);
    let delta = (target - m.f32(p + 0xC)) / duration;
    m.set_f32(p + 8, delta * period);
}

pub fn envelope(m: &mut Mem, period: f32, p: usize) -> i32 {
    let control = m.i32(p + usize::from(m.u16(p)));
    let previous = i32::from(m.i8(p + 2));
    let (segment, count) = (m.u8(p + 3), m.u8(p + 0x10));
    let release = i32::from(m.i16(p + 0x12));
    if control == 1 && previous == 0 {
        m.set_f32(p + 0xC, m.f32(p + 0x14));
        m.set_u8(p + 3, u8::from(previous != 0));
        program_segment(m, period, p);
    } else if control == 3 && previous != 3 && i32::from(segment) < release {
        m.set_u8(p + 3, release as u8);
        program_segment(m, period, p);
    } else if (control == 1 || control == 3) && segment < count {
        let remaining = m.f32(p + 4) - period;
        m.set_f32(p + 4, remaining);
        if remaining <= 0.0 {
            m.set_f32(p + 0xC, m.f32(p + 0x18 + 8 * usize::from(segment) + 4));
            m.set_u8(p + 3, segment + 1);
            if segment + 1 < count {
                program_segment(m, period, p);
            } else {
                m.set_f32(p + 0xC, 0.0);
            }
        } else {
            m.set_f32(p + 0xC, m.f32(p + 0xC) + m.f32(p + 8));
        }
    } else if control != 2 {
        m.set_f32(p + 0xC, 0.0);
    }
    m.set_i8(p + 2, control as i8);
    ftoi(m.f32(p + 0xC))
}

/// One entry of a table: sizes 1 and 2 are signed.
fn entry(m: &Mem, table: usize, size: u8, i: usize) -> i32 {
    match size {
        2 => i32::from(m.i16(table + 0x10 + 2 * i)),
        1 => i32::from(m.i8(table + 0x10 + i)),
        _ => m.i32(table + 0x10 + 4 * i),
    }
}

pub fn table(m: &mut Mem, p: usize) -> i32 {
    let input = m.i32(p + 0xC);
    if input != m.i32(p + 4) {
        let t = m.u32(p) as usize;
        m.set_i32(p + 4, input);
        let (size, count, min, max, resolution) =
            (m.u8(t), usize::from(m.u16(t + 2)), m.i32(t + 4), m.i32(t + 8), m.f32(t + 0xC));
        let index = input.clamp(min.min(max), max.max(min)).wrapping_sub(min);
        let out = if resolution == 1.0 {
            entry(m, t, size, index.max(0) as usize)
        } else {
            let x = index as f32 * resolution;
            let a = x.max(0.0) as usize;
            let frac = x - a as f32;
            let b = (a + 1).min(count.saturating_sub(1));
            let (ea, eb) = (entry(m, t, size, a) as f32, entry(m, t, size, b) as f32);
            ftoi(frac * (eb - ea) + ea)
        };
        m.set_i32(p + 8, out);
    }
    m.i32(p + 8)
}

pub fn delay_line(m: &mut Mem, period: f32, p: usize) -> i32 {
    let inputs = p + usize::from(m.u16(p));
    let max = i32::from(m.u16(p + 2));
    let (mut write, mut read) = (i32::from(m.u16(p + 4)), i32::from(m.u16(p + 6)));
    let delay = m.i32(inputs + 4);
    if delay != m.i32(p + 8) {
        m.set_i32(p + 8, delay);
        let offset = ftoi(delay.max(0) as f32 / period).min(max - 1);
        write = read + offset;
    }
    if write >= max {
        write -= max;
    }
    if read >= max {
        read = 0;
    }
    let slot = |i: i32| p + 0xC + 4 * i.max(0) as usize;
    m.set_i32(slot(write), m.i32(inputs));
    let out = m.i32(slot(read));
    m.set_u16(p + 4, (write + 1) as u16);
    m.set_u16(p + 6, (read + 1) as u16);
    out
}

pub fn oscillator(m: &mut Mem, period: f32, p: usize) -> i32 {
    let length = m.i32(p + 8);
    if length <= 0 {
        return 0;
    }
    let (amplitude, mut phase) = (m.i32(p + 0xC) as f32, m.f32(p + 4));
    let step = period / length as f32;
    while phase >= 1.0 {
        phase -= 1.0;
    }
    let value = match m.u8(p) {
        0 => (std::f64::consts::TAU * f64::from(ftoi(phase * 1024.0)) / 1024.0).sin() as f32 * amplitude,
        1 if phase >= 0.5 => amplitude,
        1 => 0.0,
        2 => phase * amplitude,
        _ if phase < 0.5 => phase * 2.0 * amplitude,
        _ => (1.0 - phase) * 2.0 * amplitude,
    };
    m.set_f32(p + 4, phase + step);
    ftoi(value)
}

pub fn ramp(m: &mut Mem, period: f32, p: usize) -> i32 {
    let (target, duration) = (m.i32(p + 0x18), m.i32(p + 0x10));
    let mut current = m.f32(p);
    if target as f32 == current {
        return target;
    }
    if target != m.i32(p + 8) || duration != m.i32(p + 0xC) {
        m.set_i32(p + 8, target);
        m.set_i32(p + 0xC, duration);
        if duration <= 0 {
            m.set_f32(p, target as f32);
            return target;
        }
        let delta = (target as f32 - current) * period / duration as f32 * 0.000_244_140_63;
        m.set_f32(p + 4, delta);
    }
    let delta = m.f32(p + 4);
    current += delta * m.i32(p + 0x14) as f32;
    if (delta >= 0.0 && current > target as f32) || (delta < 0.0 && current < target as f32) {
        current = target as f32;
    }
    m.set_f32(p, current);
    ftoi(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of 8 i16 entries 0, 100, ..., 700 over inputs 10..=17, at offset 0x40, resolution `res`.
    fn with_table(res: f32) -> Mem {
        let mut m = Mem::new(vec![0; 0x100]);
        m.set_i32(0, 0x40);
        m.set_i32(4, i32::MIN);
        m.set_u8(0x40, 2);
        m.set_u16(0x42, 8);
        m.set_i32(0x44, 10);
        m.set_i32(0x48, 17);
        m.set_f32(0x4C, res);
        for i in 0..8 {
            m.set_i16(0x50 + 2 * i, (100 * i) as i16);
        }
        m
    }

    #[test]
    fn a_table_looks_up_and_interpolates() {
        let mut m = with_table(1.0);
        m.set_i32(0xC, 13);
        assert_eq!(table(&mut m, 0), 300);
        m.set_i32(0xC, 99);
        assert_eq!(table(&mut m, 0), 700, "clamped to the last entry");
        m.set_i32(0xC, -5);
        assert_eq!(table(&mut m, 0), 0, "clamped to the first");
        let mut half = with_table(0.5);
        half.set_i32(0xC, 13); // index 3 * 0.5 = 1.5: between entries 1 and 2
        assert_eq!(table(&mut half, 0), 150);
    }

    #[test]
    fn a_table_holds_its_output_while_the_input_stays() {
        let mut m = with_table(1.0);
        m.set_i32(0xC, 12);
        assert_eq!(table(&mut m, 0), 200);
        m.set_i32(8, 5);
        assert_eq!(table(&mut m, 0), 5, "no change of input, no new lookup");
    }

    #[test]
    fn a_ramp_walks_to_its_target_and_stops() {
        let mut m = Mem::new(vec![0; 0x20]);
        m.set_i32(0x18, 1000);
        m.set_i32(0x10, 100);
        m.set_i32(0x14, 4096);
        let outs: Vec<i32> = (0..12).map(|_| ramp(&mut m, 10.0, 0)).collect();
        assert!(outs.windows(2).all(|w| w[1] >= w[0]), "{outs:?}");
        assert_eq!(*outs.last().unwrap(), 1000, "reached: {outs:?}");
        assert_eq!(ramp(&mut m, 10.0, 0), 1000);
    }

    #[test]
    fn an_oscillator_follows_its_wave() {
        let mut m = Mem::new(vec![0; 0x10]);
        m.set_i32(8, 100);
        m.set_i32(0xC, 1000);
        let sine: Vec<i32> = (0..10).map(|_| oscillator(&mut m, 10.0, 0)).collect();
        assert_eq!(sine[0], 0);
        assert!(sine[2] > 500 && sine[7] < -500, "{sine:?}");
        assert_eq!(oscillator(&mut Mem::new(vec![0; 0x10]), 10.0, 0), 0, "no period, no output");
        let mut saw = Mem::new(vec![0; 0x10]);
        saw.set_u8(0, 2);
        saw.set_i32(8, 100);
        saw.set_i32(0xC, 1000);
        let outs: Vec<i32> = (0..4).map(|_| oscillator(&mut saw, 10.0, 0)).collect();
        assert_eq!(outs, vec![0, 100, 200, 300]);
    }

    #[test]
    fn a_delay_line_delays_by_the_asked_time() {
        // inputs at +0x30: value, delay (ms). 8 slots from +0xC.
        let mut m = Mem::new(vec![0; 0x40]);
        m.set_u16(0, 0x30);
        m.set_u16(2, 8);
        m.set_i32(0x34, 30);
        let outs: Vec<i32> = (1..=8)
            .map(|i| {
                m.set_i32(0x30, i * 10);
                delay_line(&mut m, 10.0, 0)
            })
            .collect();
        assert_eq!(&outs[..3], &[0, 0, 0]);
        assert_eq!(outs[3], 10, "three ticks later: {outs:?}");
    }

    #[test]
    fn an_envelope_runs_its_segments_and_releases() {
        // inputs at +0x30; two segments: 20 ms to 100, 20 ms to 0; release segment 1.
        let mut m = Mem::new(vec![0; 0x40]);
        m.set_u16(0, 0x30);
        m.set_u8(0x10, 2);
        m.set_i16(0x12, 1);
        for (i, (d, t)) in [(20.0, 100.0), (20.0, 0.0)].into_iter().enumerate() {
            m.set_f32(0x18 + 8 * i, d);
            m.set_f32(0x1C + 8 * i, t);
        }
        m.set_i32(0x30, 1);
        let up: Vec<i32> = (0..3).map(|_| envelope(&mut m, 10.0, 0)).collect();
        assert_eq!(up[0], 0);
        assert!(up[1] >= 40 && up[2] == 100, "{up:?}");
        m.set_i32(0x30, 0);
        assert_eq!(envelope(&mut m, 10.0, 0), 0, "stopped");
    }
}
