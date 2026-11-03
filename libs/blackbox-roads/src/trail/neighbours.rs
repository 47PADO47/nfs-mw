//! Adapt a world-body iterator to the local avoidable list before applying its capacity.
//! Spec: ai-pursuit-obstacle-selection.md (avoidable neighbourhood). Exact native spatial ordering remains separate.

use glam::Vec3;

use super::Body;
use super::punch::MAX_HEIGHT_DIFF;

const CAPACITY: usize = 32;
/// The sweep radius of a vehicle that has not set its own (the other cars' radii are not carried here).
const DEFAULT_RADIUS: f32 = 20.0;

/// A vehicle's sweep: the larger of its avoidance radius and twice its speed.
fn sweep(radius: f32, body: &Body) -> f32 {
    radius.max(2.0 * body.velocity.length())
}

/// The bodies whose sweep squares overlap mine on both horizontal axes, nearest first beyond the capacity. My radius is
/// the cursor distance; the others use the default radius.
pub(super) fn nearby(me: &Body, cursor: Vec3, bodies: impl IntoIterator<Item = Body>) -> Vec<Body> {
    let mine = sweep(cursor.distance(me.position), me);
    let mut found: Vec<_> = bodies
        .into_iter()
        .enumerate()
        .filter_map(|(index, body)| {
            let delta = body.position - me.position;
            let reach = mine + sweep(DEFAULT_RADIUS, &body);
            let overlaps = delta.x.abs() <= reach && delta.z.abs() <= reach;
            (delta.y.abs() <= MAX_HEIGHT_DIFF && overlaps).then_some((index, delta.length_squared(), body))
        })
        .collect();
    if found.len() > CAPACITY {
        found.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        found.truncate(CAPACITY);
        found.sort_by_key(|entry| entry.0);
    }
    found.into_iter().map(|(_, _, body)| body).collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;

    fn body(position: Vec3, velocity: Vec3) -> Body {
        Body { position, velocity, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
    }

    #[test]
    fn a_fast_oncoming_car_is_a_neighbour_through_its_own_sweep() {
        let me = body(Vec3::ZERO, Vec3::Z * 10.0);
        // My sweep is 30 m (the cursor), its own 2 * 30 m: their squares overlap 80 m ahead.
        let oncoming = body(Vec3::Z * 80.0, -Vec3::Z * 30.0);
        assert_eq!(nearby(&me, Vec3::Z * 30.0, [oncoming]).len(), 1);
        let parked = body(Vec3::Z * 80.0, Vec3::ZERO);
        assert!(nearby(&me, Vec3::Z * 30.0, [parked]).is_empty(), "a parked car sweeps only its 20 m default");
        assert_eq!(nearby(&me, Vec3::Z * 30.0, [body(Vec3::Z * 50.0, Vec3::ZERO)]).len(), 1);
    }

    #[test]
    fn the_neighbourhood_is_a_square_not_a_circle() {
        let me = body(Vec3::ZERO, Vec3::ZERO);
        let diagonal = body(Vec3::new(45.0, 0.0, 45.0), Vec3::ZERO);
        assert_eq!(nearby(&me, Vec3::Z * 30.0, [diagonal]).len(), 1);
        let stacked = body(Vec3::new(0.0, 6.0, 10.0), Vec3::ZERO);
        assert!(nearby(&me, Vec3::Z * 30.0, [stacked]).is_empty());
    }
}
