//! Envelope events (AR, ASR, ATR). Spec: `docs/specs/dynamic-mixer.md` §5.

use super::{EventNode, Mixer};
use crate::map::EnvelopeKind;
use crate::shape::{UNITY, curve};

/// What an envelope writes: its level in hundredths of a dB and its Q15 position (`UNITY` = at rest).
#[derive(Debug, Clone, Copy)]
struct Out {
    level: i32,
    q: i32,
}

pub(super) fn update(m: &mut Mixer, ms: f32) {
    for i in 0..m.events.len() {
        let trigger = m.graph.read(m.events[i].trigger);
        let previous = Out { level: m.graph.event_db[i], q: m.graph.event_q[i] };
        let node = &mut m.events[i];
        if node.elapsed == 0.0 && trigger == 0 {
            node.reset = false;
            node.reset_level = -10_000;
            m.graph.event_db[i] = 0;
            m.graph.event_q[i] = UNITY;
            continue;
        }
        node.elapsed += ms;
        let mut out = run(node, trigger, previous);
        for &src in &node.scales {
            out.level = ((i64::from(out.level) * i64::from(m.graph.read(src))) >> 15) as i32;
        }
        m.graph.event_db[i] = out.level;
        m.graph.event_q[i] = out.q;
    }
}

fn run(node: &mut EventNode, trigger: i32, previous: Out) -> Out {
    match node.kind {
        EnvelopeKind::Ar => attack_release(node),
        EnvelopeKind::Asr => attack_sustain_release(node),
        EnvelopeKind::Atr => attack_trigger_release(node, trigger, previous),
        EnvelopeKind::Lfo => previous,
    }
}

/// The curve position for a ratio `t` (Q15) on `shape`; a fade (negative swing) uses the shape as it is, a rise
/// is mirrored.
fn position(node: &EventNode, shape: u8, t: i32) -> i32 {
    let c = curve(shape, t);
    if node.swing < 0 { c } else { UNITY - c }
}

fn attack_ratio(node: &EventNode, t0: f32) -> i32 {
    (node.elapsed * 32767.0 / t0) as i32
}

fn release_ratio(elapsed: f32, t2: f32) -> i32 {
    (32767.0 - elapsed * 32767.0 / t2) as i32
}

/// The level for a position, with the envelope state `node` as it is after the step.
fn finish(node: &EventNode, q: i32) -> Out {
    let fraction = (32767.0 - q as f32) / 32767.0;
    Out { level: (fraction * node.swing as f32) as i32, q }
}

fn attack_release(node: &mut EventNode) -> Out {
    let [t0, _, t2] = node.times;
    if node.elapsed < t0 {
        return finish(node, position(node, node.shapes[0], attack_ratio(node, t0)));
    }
    if node.elapsed - t0 < t2 {
        return finish(node, position(node, node.shapes[1], release_ratio(node.elapsed - t0, t2)));
    }
    node.elapsed = 0.0;
    finish(node, UNITY)
}

fn attack_sustain_release(node: &mut EventNode) -> Out {
    let [t0, t1, t2] = node.times;
    if node.elapsed < t0 {
        return finish(node, position(node, node.shapes[0], attack_ratio(node, t0)));
    }
    if node.elapsed - t0 <= t1 {
        return finish(node, 0);
    }
    let into = node.elapsed - (t0 + t1);
    if into < t2 {
        return finish(node, position(node, node.shapes[1], release_ratio(into, t2)));
    }
    node.elapsed = 0.0;
    finish(node, UNITY)
}

/// The position of an attack-trigger-release envelope this step, or the output to keep when the step only
/// changes the state.
enum Step {
    At(i32),
    Keep(Out),
}

fn attack_trigger_release(node: &mut EventNode, trigger: i32, previous: Out) -> Out {
    let q = match trigger_step(node, trigger, previous) {
        Step::At(q) => q,
        Step::Keep(out) => return out,
    };
    let fraction = (32767.0 - q as f32) / 32767.0;
    if !node.reset {
        return Out { level: (fraction * node.swing as f32) as i32, q };
    }
    if trigger == 0 {
        return Out { level: (fraction * node.reset_level as f32) as i32, q };
    }
    let rise = (fraction * (node.swing - node.reset_level) as f32) as i32;
    Out { level: rise + node.reset_level, q }
}

fn trigger_step(node: &mut EventNode, trigger: i32, previous: Out) -> Step {
    let [t0, _, t2] = node.times;
    if node.elapsed < t0 {
        if trigger == 0 {
            node.reset = true;
            node.reset_level = previous.level;
            node.elapsed = t0;
            return Step::Keep(previous);
        }
        return Step::At(position(node, node.shapes[0], attack_ratio(node, t0)));
    }
    if trigger == 0 {
        if node.reset_time == 0.0 {
            node.reset_time = node.elapsed;
        }
        if node.elapsed - node.reset_time > t2 {
            node.reset = false;
            node.reset_level = 0;
            node.elapsed = 0.0;
            node.reset_time = 0.0;
            return Step::Keep(Out { level: previous.level, q: UNITY });
        }
        return Step::At(position(node, node.shapes[1], release_ratio(node.elapsed - node.reset_time, t2)));
    }
    if trigger == 1 && node.reset_time != 0.0 {
        node.reset = true;
        node.elapsed = 0.0;
        node.reset_level = previous.level;
        node.reset_time = 0.0;
        return Step::Keep(Out { level: previous.level, q: UNITY });
    }
    node.reset = false;
    node.reset_level = 0;
    node.reset_time = 0.0;
    node.elapsed = t0;
    Step::At(0)
}
