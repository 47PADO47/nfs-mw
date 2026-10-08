//! One frame of the mixer. Spec: `docs/specs/dynamic-mixer.md` §4.

use super::{Mixer, event, output, spatial};
use crate::shape::{UNITY, curve, db_from_q15};

pub(super) fn process(m: &mut Mixer, dt: f32) {
    let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
    controls(m);
    spatial::update(m);
    event::update(m, dt * 1000.0);
    for i in 0..m.subs.len() {
        let node = &m.subs[i];
        let sum = node.inputs.iter().fold(0i32, |acc, &src| acc.saturating_add(m.graph.read(src)));
        m.graph.sub[i] = sum.clamp(node.lower.min(node.upper), node.upper);
    }
    for i in 0..m.masters.len() {
        let node = &m.masters[i];
        let attached = m.blocks[node.block].attached;
        let sum = node.inputs.iter().fold(0i32, |acc, &src| acc.saturating_add(m.graph.read(src)));
        m.graph.master[i] = if attached { node.base.saturating_add(sum) } else { -10_000 };
    }
    output::write(m);
}

/// The curves of all controls first (a control may read another's curve), then their levels.
fn controls(m: &mut Mixer) {
    for i in 0..m.controls.len() {
        let node = &m.controls[i];
        m.graph.ctl_curve[i] = curve(node.shape, m.graph.read(node.input));
    }
    for i in 0..m.controls.len() {
        let node = &m.controls[i];
        let cut = ((UNITY - m.graph.ctl_curve[i]) * node.ratio) >> 15;
        let level = db_from_q15(UNITY - cut) + node.offset;
        let scale =
            node.scales.iter().fold(UNITY, |acc, &src| ((i64::from(m.graph.read(src)) * i64::from(acc)) >> 15) as i32);
        m.graph.ctl_level[i] = ((i64::from(scale) * i64::from(level)) >> 15) as i32;
    }
}
