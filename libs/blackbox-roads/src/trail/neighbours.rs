//! Adapt a world-body iterator to the local avoidable list before applying its capacity.
//! Spec: ai-pursuit-obstacle-selection.md. Exact native spatial ordering remains separate.

use glam::Vec3;

use super::Body;
use super::punch::MAX_HEIGHT_DIFF;

const CAPACITY: usize = 32;

pub(super) fn nearby(me: &Body, cursor: Vec3, bodies: impl IntoIterator<Item = Body>) -> Vec<Body> {
    let radius = cursor.distance(me.position).max(2.0 * me.velocity.length());
    let own_extent = me.half_width.hypot(me.half_length);
    let mut found: Vec<_> = bodies
        .into_iter()
        .enumerate()
        .filter_map(|(index, body)| {
            let delta = body.position - me.position;
            let reach = radius + own_extent + body.half_width.hypot(body.half_length);
            let distance = delta.length_squared();
            (delta.y.abs() <= MAX_HEIGHT_DIFF && distance <= reach * reach).then_some((index, distance, body))
        })
        .collect();
    if found.len() > CAPACITY {
        found.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        found.truncate(CAPACITY);
        found.sort_by_key(|entry| entry.0);
    }
    found.into_iter().map(|(_, _, body)| body).collect()
}
