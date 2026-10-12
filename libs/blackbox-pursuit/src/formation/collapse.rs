//! The collapse: when the target slows, the cops close into a ring around it. The ring is placed as the spec gives it
//! (section 5.1): the farthest candidates form an outer ring and the nearest an inner one, each ring is ordered from the
//! front of the target, and the cops take evenly spaced slots starting at the front-most one. Spec:
//! `docs/specs/ai-pursuit-formations.md` section 5.1 (values verified on the install, the table at the end of it).

use glam::Vec2;

/// Heat levels 1 to 10 (the spec's table): the collapse speed (km/h), inner and outer radius (m), and the most cops that
/// collapse at once (the spec's verified table; heats 8 and 10 take the race levels' radii, heat 9 its own).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub speed_kmh: f32,
    pub inner: f32,
    pub outer: f32,
    pub max_cops: usize,
}

pub fn level(heat: u32) -> Level {
    const TABLE: [Level; 10] = [
        Level { speed_kmh: 58.0, inner: 6.0, outer: 15.0, max_cops: 4 },
        Level { speed_kmh: 68.0, inner: 5.0, outer: 15.0, max_cops: 5 },
        Level { speed_kmh: 78.0, inner: 4.0, outer: 8.0, max_cops: 6 },
        Level { speed_kmh: 88.0, inner: 3.0, outer: 5.0, max_cops: 7 },
        Level { speed_kmh: 96.0, inner: 3.0, outer: 5.0, max_cops: 8 },
        Level { speed_kmh: 120.0, inner: 3.0, outer: 7.0, max_cops: 8 },
        Level { speed_kmh: 78.0, inner: 4.0, outer: 10.0, max_cops: 9 },
        Level { speed_kmh: 15.0, inner: 3.0, outer: 15.0, max_cops: 6 },
        Level { speed_kmh: 1.0, inner: 2.0, outer: 5.0, max_cops: 6 },
        Level { speed_kmh: 15.0, inner: 3.0, outer: 15.0, max_cops: 6 },
    ];
    TABLE[(heat as usize).clamp(1, 10) - 1]
}

/// A cop's offset from the target in the target's frame: `x` to its side (right is positive), `z` to its front.
pub type Offset = Vec2;

/// The collapse slots: for each cop given (by its index in `cops`), the in-position offset it drives to. Cops beyond the
/// candidates (more than `max_cops`) take the outer ring; the nearest `max_cops` take the inner one.
pub fn ring_slots(cops: &[Offset], level: Level) -> Vec<(usize, Offset)> {
    let inner = level.inner.max(3.0);
    let outer = level.outer.max(inner + 1.0);
    let mut by_distance: Vec<usize> = (0..cops.len()).collect();
    by_distance.sort_by(|&a, &b| cops[a].length().total_cmp(&cops[b].length()));
    let (near, far) = if cops.len() > level.max_cops {
        (by_distance[..level.max_cops].to_vec(), by_distance[level.max_cops..].to_vec())
    } else {
        (by_distance, Vec::new())
    };
    let mut out = ring(cops, &far, outer);
    out.extend(ring(cops, &near, inner));
    out
}

/// One ring: the cops ordered from the front, then slots at `2 pi i / n` starting at the front-most cop.
fn ring(cops: &[Offset], members: &[usize], radius: f32) -> Vec<(usize, Offset)> {
    if members.is_empty() {
        return Vec::new();
    }
    let mut ordered = members.to_vec();
    ordered.sort_by(|&a, &b| pseudo_angle(cops[a]).total_cmp(&pseudo_angle(cops[b])));
    let n = ordered.len() as f32;
    ordered
        .into_iter()
        .enumerate()
        .map(|(i, cop)| {
            let a = std::f32::consts::TAU * i as f32 / n;
            (cop, Vec2::new(a.sin(), a.cos()) * radius)
        })
        .collect()
}

/// A cheap angle from the front: 0 straight ahead (`z` positive), increasing clockwise, in `0..4` (the spec's ratio
/// function). Cops are ordered by it so the front-most comes first.
pub fn pseudo_angle(offset: Offset) -> f32 {
    let (x, z) = (offset.x, offset.y);
    let sum = x.abs() + z.abs();
    if sum == 0.0 {
        return 0.0;
    }
    let ratio = z / sum;
    if x >= 0.0 { 1.0 - ratio } else { 3.0 + ratio }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_matches_the_spec_for_heat_one() {
        let one = level(1);
        assert_eq!((one.speed_kmh, one.inner, one.outer, one.max_cops), (58.0, 6.0, 15.0, 4));
        assert_eq!(level(9).speed_kmh, 1.0);
    }

    #[test]
    fn the_front_cop_takes_the_front_slot() {
        // Three cops: one straight ahead (front), two behind. The front one takes the slot at angle 0: (0, radius).
        let cops = [Vec2::new(0.0, -8.0), Vec2::new(0.0, 6.0), Vec2::new(0.0, -3.0)];
        let slots = ring_slots(&cops, Level { speed_kmh: 0.0, inner: 4.0, outer: 10.0, max_cops: 8 });
        let front = slots.iter().find(|(cop, _)| *cop == 1).expect("the front cop has a slot").1;
        assert!((front.y - 4.0).abs() < 1e-4 && front.x.abs() < 1e-4, "front slot {front:?}");
    }

    #[test]
    fn the_farthest_cops_form_the_outer_ring() {
        let cops: Vec<Vec2> = (0..5).map(|i| Vec2::new(0.0, -(i as f32) * 2.0 - 1.0)).collect();
        let slots = ring_slots(&cops, Level { speed_kmh: 0.0, inner: 3.0, outer: 8.0, max_cops: 2 });
        assert_eq!(slots.len(), 5);
        // The two nearest (indices 0 and 1, at 1 m and 3 m) are on the inner ring of 3 m; the rest on the outer of 8 m.
        for (cop, offset) in slots {
            let radius = offset.length();
            if cop <= 1 {
                assert!((radius - 3.0).abs() < 1e-4, "cop {cop} inner: {radius}");
            } else {
                assert!((radius - 8.0).abs() < 1e-4, "cop {cop} outer: {radius}");
            }
        }
    }

    #[test]
    fn the_rings_never_sit_closer_than_three_metres_or_inside_one_another() {
        let l = Level { speed_kmh: 0.0, inner: 0.5, outer: 1.0, max_cops: 4 };
        let cops = [Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];
        for (_, offset) in ring_slots(&cops, l) {
            assert!(offset.length() >= 3.0 - 1e-4, "{offset:?}");
        }
    }

    #[test]
    fn pseudo_angle_runs_from_the_front_clockwise() {
        assert_eq!(pseudo_angle(Vec2::new(0.0, 1.0)), 0.0, "front");
        assert!((pseudo_angle(Vec2::new(1.0, 0.0)) - 1.0).abs() < 1e-5, "right");
        assert!((pseudo_angle(Vec2::new(0.0, -1.0)) - 2.0).abs() < 1e-5, "behind");
        assert!((pseudo_angle(Vec2::new(-1.0, 0.0)) - 3.0).abs() < 1e-5, "left");
    }
}
