//! Evaluating a track at a time: step, linear and move-to interpolation, and where the value goes.

use glam::Quat;

use crate::package::{Interp, Key, ParamType, Track};

fn f(w: u32) -> f32 {
    f32::from_bits(w)
}

fn quat(k: &[u32; 4]) -> Quat {
    Quat::from_xyzw(f(k[0]), f(k[1]), f(k[2]), f(k[3]))
}

/// The key a time falls on: the first delta key whose time is not before `t`, or the last one.
fn key_index(keys: &[Key], t: i32) -> Option<usize> {
    if keys.is_empty() {
        return None;
    }
    Some(keys.iter().position(|k| k.time >= t).unwrap_or(keys.len() - 1))
}

/// Adds a delta to a base value, per the parameter type.
fn add(param: ParamType, base: &[u32; 4], delta: &[u32; 4]) -> [u32; 4] {
    let mut out = *base;
    match param {
        ParamType::Quat => {
            let q = quat(base) * quat(delta);
            out = [q.x.to_bits(), q.y.to_bits(), q.z.to_bits(), q.w.to_bits()];
        }
        p if p.is_integer() => {
            for i in 0..p.components() {
                out[i] = (base[i] as i32).wrapping_add(delta[i] as i32) as u32;
            }
        }
        p => {
            for i in 0..p.components() {
                out[i] = (f(base[i]) + f(delta[i])).to_bits();
            }
        }
    }
    out
}

/// `base + a + (b - a) * t`, rounding integers.
fn lerp(param: ParamType, base: &[u32; 4], a: &[u32; 4], b: &[u32; 4], t: f32) -> [u32; 4] {
    let mut out = *base;
    match param {
        ParamType::Quat => {
            let (qa, mut qb) = (quat(a), quat(b));
            let mut dot = qa.dot(qb);
            if dot < 0.0 {
                qb = -qb;
                dot = -dot;
            }
            let q = if dot < 0.999 {
                let angle = dot.clamp(-1.0, 1.0).acos();
                let sin = angle.sin();
                ((qa * (angle * (1.0 - t)).sin()) + (qb * (angle * t).sin())) * (1.0 / sin)
            } else {
                (qa * (1.0 - t) + qb * t).normalize()
            };
            let q = quat(base) * q;
            out = [q.x.to_bits(), q.y.to_bits(), q.z.to_bits(), q.w.to_bits()];
        }
        p if p.is_integer() => {
            for i in 0..p.components() {
                let (x, y) = (a[i] as i32, b[i] as i32);
                let v = (base[i] as i32).wrapping_add(x).wrapping_add(((y - x) as f32 * t + 0.5) as i32);
                out[i] = v as u32;
            }
        }
        p => {
            for i in 0..p.components() {
                let (x, y) = (f(a[i]), f(b[i]));
                out[i] = (f(base[i]) + x + (y - x) * t).to_bits();
            }
        }
    }
    out
}

/// The value of `track` at time `t`, or `None` when the track leaves the value alone.
pub fn evaluate(track: &Track, t: i32) -> Option<[u32; 4]> {
    let keys = &track.keys;
    // A ping-pong track plays backwards after its end.
    let t = if track.action & 0x7F == 2 && t > track.length { 2 * track.length - t } else { t };
    match track.interp {
        Interp::Unsupported => None,
        Interp::None => {
            // Steps: keys hold the values themselves. Before every key the base applies.
            let i = if t < 0 { None } else { key_index(keys, t) };
            let Some(i) = i else { return Some(track.base.value) };
            let key = &keys[i];
            if i > 0 && key.time > t { Some(keys[i - 1].value) } else { Some(key.value) }
        }
        Interp::Linear | Interp::MoveTo => {
            let Some(i) = key_index(keys, t) else { return Some(track.base.value) };
            let base = &track.base.value;
            let key = &keys[i];
            let prev = i.checked_sub(1).map(|p| &keys[p]);
            let (mut a, mut b, mut frac) = (key, key, 1.0_f32);
            match track.action & 0x7F {
                1 => {
                    // Loop: past the last key the track blends back toward the first one.
                    if key.time < t {
                        let first = &keys[0];
                        let div = (track.length - key.time + first.time) as f32;
                        frac = if div > 1e-5 { (t - key.time) as f32 / div } else { 0.0 };
                        a = key;
                        b = first;
                    } else if key.time != t {
                        let (p, div, num) = match prev {
                            Some(p) => (p, (key.time - p.time) as f32, (t - p.time) as f32),
                            None => {
                                let last = &keys[keys.len() - 1];
                                (
                                    last,
                                    (track.length - last.time + key.time) as f32,
                                    (t + track.length - last.time) as f32,
                                )
                            }
                        };
                        frac = if div > 1e-5 { num / div } else { 0.0 };
                        a = p;
                        b = key;
                    } else {
                        frac = 0.0;
                    }
                }
                _ => {
                    if let Some(p) = prev.filter(|_| t < key.time) {
                        let div = (key.time - p.time) as f32;
                        frac = if div > 1e-5 { (t - p.time) as f32 / div } else { 0.0 };
                        a = p;
                        b = key;
                    }
                }
            }
            Some(if frac <= 0.0 {
                add(track.param, base, &a.value)
            } else if frac >= 1.0 {
                add(track.param, base, &b.value)
            } else {
                lerp(track.param, base, &a.value, &b.value, frac)
            })
        }
    }
}

/// Rewrites the first key of every move-to track so the motion starts at the current value.
pub fn setup_move_to(tracks: &mut [Track], words: &[u32]) {
    for t in tracks.iter_mut().filter(|t| t.interp == Interp::MoveTo) {
        let Some(first) = t.keys.first_mut() else { continue };
        if t.param == ParamType::Quat {
            let current = std::array::from_fn(|i| words.get(t.offset + i).copied().unwrap_or(0));
            let delta = quat(&t.base.value).inverse() * quat(&current);
            first.value = [delta.x.to_bits(), delta.y.to_bits(), delta.z.to_bits(), delta.w.to_bits()];
            continue;
        }
        for i in 0..t.param.components() {
            let cur = words.get(t.offset + i).copied().unwrap_or(0);
            first.value[i] = if t.param.is_integer() {
                (cur as i32).wrapping_sub(t.base.value[i] as i32) as u32
            } else {
                (f(cur) - f(t.base.value[i])).to_bits()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(param: ParamType, interp: Interp, base: [u32; 4], keys: Vec<(i32, [u32; 4])>) -> Track {
        Track {
            param,
            interp,
            action: 0,
            length: 600,
            offset: 0,
            base: Key { time: -1, value: base },
            keys: keys.into_iter().map(|(time, value)| Key { time, value }).collect(),
        }
    }

    fn fl(v: f32) -> [u32; 4] {
        [v.to_bits(), 0, 0, 0]
    }

    #[test]
    fn linear_floats_blend_between_delta_keys() {
        let t = track(ParamType::Float, Interp::Linear, fl(10.0), vec![(0, fl(0.0)), (100, fl(20.0))]);
        assert_eq!(f(evaluate(&t, 0).unwrap()[0]), 10.0);
        assert_eq!(f(evaluate(&t, 50).unwrap()[0]), 20.0);
        assert_eq!(f(evaluate(&t, 100).unwrap()[0]), 30.0);
        assert_eq!(f(evaluate(&t, 500).unwrap()[0]), 30.0, "past the end the last key stays");
    }

    #[test]
    fn the_fade_in_goes_from_zero_to_full_alpha() {
        let t = track(
            ParamType::Colour,
            Interp::Linear,
            [255, 255, 255, 255],
            vec![(0, [0, 0, 0, (-255i32) as u32]), (600, [0, 0, 0, 0])],
        );
        assert_eq!(evaluate(&t, 0).unwrap()[3] as i32, 0);
        assert_eq!(evaluate(&t, 300).unwrap()[3] as i32, 128);
        assert_eq!(evaluate(&t, 600).unwrap()[3] as i32, 255);
        assert_eq!(evaluate(&t, 601).unwrap()[3] as i32, 255);
    }

    #[test]
    fn step_tracks_hold_the_key_value() {
        let t = track(ParamType::Int, Interp::None, [7, 0, 0, 0], vec![(10, [1, 0, 0, 0]), (20, [2, 0, 0, 0])]);
        assert_eq!(evaluate(&t, 0).unwrap()[0], 1, "before the first key the first key shows");
        assert_eq!(evaluate(&t, 10).unwrap()[0], 1);
        assert_eq!(evaluate(&t, 15).unwrap()[0], 1);
        assert_eq!(evaluate(&t, 20).unwrap()[0], 2);
        let empty = track(ParamType::Int, Interp::None, [7, 0, 0, 0], vec![]);
        assert_eq!(evaluate(&empty, 5).unwrap()[0], 7);
    }

    #[test]
    fn quaternion_keys_rotate_the_base() {
        let q = Quat::from_rotation_z(1.0);
        let key = [q.x.to_bits(), q.y.to_bits(), q.z.to_bits(), q.w.to_bits()];
        let ident = [0, 0, 0, 1.0f32.to_bits()];
        let t = track(ParamType::Quat, Interp::Linear, ident, vec![(0, ident), (100, key)]);
        let v = evaluate(&t, 50).unwrap();
        let got = quat(&v);
        assert!((got.to_axis_angle().1 - 0.5).abs() < 0.01, "half way: {got:?}");
        let end = quat(&evaluate(&t, 100).unwrap());
        assert!((end.to_axis_angle().1 - 1.0).abs() < 1e-4);
    }

    fn quat_words(q: Quat) -> [u32; 4] {
        [q.x.to_bits(), q.y.to_bits(), q.z.to_bits(), q.w.to_bits()]
    }

    fn assert_rotation(actual: Quat, expected: Quat) {
        assert!((actual.length() - 1.0).abs() < 1e-5, "not normalized: {actual:?}");
        assert!((actual.dot(expected).abs() - 1.0).abs() < 1e-6, "{actual:?} != {expected:?}");
    }

    #[test]
    fn equal_rotation_keys_remain_constant() {
        let q = Quat::from_rotation_z(0.7);
        let t = track(
            ParamType::Quat,
            Interp::Linear,
            quat_words(Quat::IDENTITY),
            vec![(0, quat_words(q)), (100, quat_words(q))],
        );
        for time in [0, 25, 50, 75, 100] {
            assert_rotation(quat(&evaluate(&t, time).unwrap()), q);
        }
    }

    #[test]
    fn near_equal_rotation_keys_blend_and_compose_with_base() {
        let base = Quat::from_rotation_x(0.6);
        let a = Quat::from_rotation_z(0.7);
        let b = Quat::from_rotation_z(0.74);
        let t =
            track(ParamType::Quat, Interp::Linear, quat_words(base), vec![(0, quat_words(a)), (100, quat_words(b))]);
        for (time, angle) in [(0, 0.7), (25, 0.71), (50, 0.72), (75, 0.73), (100, 0.74)] {
            assert_rotation(quat(&evaluate(&t, time).unwrap()), base * Quat::from_rotation_z(angle));
        }
    }

    #[test]
    fn opposite_quaternion_signs_represent_the_same_rotation() {
        let q = Quat::from_rotation_y(0.7);
        let t = track(
            ParamType::Quat,
            Interp::Linear,
            quat_words(Quat::IDENTITY),
            vec![(0, quat_words(q)), (100, quat_words(-q))],
        );
        for time in [0, 25, 50, 75, 100] {
            assert_rotation(quat(&evaluate(&t, time).unwrap()), q);
        }
    }

    #[test]
    fn unsupported_tracks_do_nothing() {
        let t = track(ParamType::Float, Interp::Unsupported, fl(1.0), vec![(0, fl(1.0))]);
        assert!(evaluate(&t, 0).is_none());
    }

    #[test]
    fn move_to_starts_at_the_current_value() {
        let mut tracks = vec![track(ParamType::Float, Interp::MoveTo, fl(10.0), vec![(0, fl(0.0)), (100, fl(5.0))])];
        tracks[0].offset = 1;
        let words = [0, 25.0f32.to_bits()];
        setup_move_to(&mut tracks, &words);
        assert_eq!(f(tracks[0].keys[0].value[0]), 15.0);
        assert_eq!(f(evaluate(&tracks[0], 0).unwrap()[0]), 25.0);
    }

    #[test]
    fn quaternion_move_to_starts_at_current_rotation_and_preserves_other_data() {
        for base in [Quat::IDENTITY, Quat::from_rotation_x(0.6)] {
            let current = Quat::from_rotation_z(0.9);
            let final_delta = Quat::from_rotation_y(0.5);
            let mut rotation = track(
                ParamType::Quat,
                Interp::MoveTo,
                quat_words(base),
                vec![(0, quat_words(Quat::IDENTITY)), (100, quat_words(final_delta))],
            );
            rotation.offset = 2;
            let untouched = track(
                ParamType::Quat,
                Interp::Linear,
                quat_words(base),
                vec![(0, quat_words(current)), (100, quat_words(final_delta))],
            );
            let mut tracks = vec![rotation, untouched];
            let current_words = quat_words(current);
            let words = [123, 456, current_words[0], current_words[1], current_words[2], current_words[3], 789];
            let original_words = words;
            let untouched_first = tracks[1].keys[0].value;
            let untouched_last = tracks[1].keys[1].value;
            setup_move_to(&mut tracks, &words);
            assert_eq!(words, original_words);
            assert_eq!(tracks[0].keys[1].value, quat_words(final_delta));
            assert_eq!(tracks[1].keys[0].value, untouched_first);
            assert_eq!(tracks[1].keys[1].value, untouched_last);
            for time in [0, 50, 100] {
                let actual = quat(&evaluate(&tracks[0], time).unwrap());
                assert!(actual.is_finite());
                assert!((actual.length() - 1.0).abs() < 1e-5, "time {time}: {actual:?}");
            }
            assert_rotation(quat(&evaluate(&tracks[0], 0).unwrap()), current);
            assert_rotation(quat(&evaluate(&tracks[0], 100).unwrap()), base * final_delta);
        }
        let mut integer = track(ParamType::Int, Interp::MoveTo, [10, 0, 0, 0], vec![(0, [0; 4]), (100, [5, 0, 0, 0])]);
        integer.offset = 1;
        let mut tracks = [integer];
        setup_move_to(&mut tracks, &[99, 25, 77]);
        assert_eq!(tracks[0].keys[0].value[0], 15);
        assert_eq!(evaluate(&tracks[0], 0).unwrap()[0], 25);
        assert_eq!(tracks[0].keys[1].value[0], 5);
    }

    #[test]
    fn ping_pong_linear_and_move_to_blend_in_both_directions() {
        for interp in [Interp::Linear, Interp::MoveTo] {
            let mut t = track(ParamType::Float, interp, fl(10.0), vec![(0, fl(0.0)), (100, fl(20.0))]);
            t.action = 2;
            t.length = 100;
            for (time, expected) in [
                (0, 10.0),
                (25, 15.0),
                (50, 20.0),
                (75, 25.0),
                (100, 30.0),
                (125, 25.0),
                (150, 20.0),
                (175, 15.0),
                (200, 10.0),
            ] {
                assert_eq!(f(evaluate(&t, time).unwrap()[0]), expected, "time {time}");
            }
        }
    }

    #[test]
    fn ping_pong_reflected_near_equal_quaternions_compose_with_base() {
        let base = Quat::from_rotation_x(0.6);
        let a = Quat::from_rotation_z(0.7);
        let b = Quat::from_rotation_z(0.74);
        let mut t =
            track(ParamType::Quat, Interp::Linear, quat_words(base), vec![(0, quat_words(a)), (100, quat_words(b))]);
        t.action = 2;
        t.length = 100;
        assert_rotation(quat(&evaluate(&t, 150).unwrap()), base * Quat::from_rotation_z(0.72));
    }

    #[test]
    fn ping_pong_steps_and_clamp_and_loop_controls() {
        let mut step = track(ParamType::Int, Interp::None, [7, 0, 0, 0], vec![(0, [1, 0, 0, 0]), (100, [2, 0, 0, 0])]);
        step.action = 2;
        step.length = 100;
        for (time, value) in [(25, 1), (100, 2), (150, 1), (200, 1)] {
            assert_eq!(evaluate(&step, time).unwrap()[0], value);
        }
        let mut t = track(ParamType::Float, Interp::Linear, fl(10.0), vec![(0, fl(0.0)), (100, fl(20.0))]);
        t.length = 100;
        assert_eq!(f(evaluate(&t, 150).unwrap()[0]), 30.0);
        t.action = 1;
        assert_eq!(f(evaluate(&t, 50).unwrap()[0]), 20.0);
    }

    #[test]
    fn loop_tracks_wrap_toward_the_first_key() {
        let mut t = track(ParamType::Float, Interp::Linear, fl(0.0), vec![(100, fl(10.0)), (300, fl(30.0))]);
        t.action = 1;
        t.length = 400;
        // Between the last key (300) and the end (400) the value heads back to the first key (10) at 500.
        let v = f(evaluate(&t, 350).unwrap()[0]);
        assert!(v < 30.0 && v > 10.0, "{v}");
    }
}
