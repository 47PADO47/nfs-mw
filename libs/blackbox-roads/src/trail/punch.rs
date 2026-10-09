//! Cutting holes in the trail for other cars so that the corridor passes on one side.
//! Spec: `docs/specs/ai-road-nav-trail.md` (§5). The original routine is marked unfinished: this follows
//! the structure and the constants of the spec where they are given.

use glam::{Vec2, Vec3};

use super::{CUT, CUT_BEHIND, Cookie, xz};

/// Cars higher or lower than this relative to the trail are ignored.
const MAX_HEIGHT_DIFF: f32 = 5.0;
/// Half a metre of room on every side of a car.
const CAR_MARGIN: f32 = 0.5;
/// Time to closest approach returned when two cars move alike, seconds.
const NO_APPROACH_TIME: f32 = 3.0;
/// Corridor width kept after a cut, metres (traffic, others).
const MIN_WIDTH_TRAFFIC: f32 = 0.1;
const MIN_WIDTH: f32 = 1.0;

/// A car as the trail sees it: where it is, how it moves and how big it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Unit planar forward vector.
    pub forward: Vec2,
    pub half_width: f32,
    pub half_length: f32,
}

pub type Avoidable = Body;

impl Body {
    fn right(&self) -> Vec2 {
        Vec2::new(self.forward.y, -self.forward.x)
    }

    fn speed_along(&self, direction: Vec2) -> f32 {
        xz(self.velocity).dot(direction)
    }
}

/// What the closest cut tells the occlusion step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blocker {
    /// The blocking car's speed along the road.
    pub trail_speed: f32,
}

/// Seconds until the two cars' noses are closest, or 3 s when they move alike.
fn approach_time(rel: Vec2, vel: Vec2) -> f32 {
    let speed2 = vel.length_squared();
    match speed2 < 1e-4 {
        true => NO_APPROACH_TIME,
        false => (-rel.dot(vel) / speed2).max(0.0),
    }
}

/// Narrows the bounds of `cookies` (the current one first) around each of `avoidables`. Returns the closest
/// cut that is further ahead than the two cars' extents.
pub fn punch_avoidables(
    cookies: &mut [Cookie],
    me: &Body,
    avoidables: &[Body],
    traffic: bool,
    vehicle_half_width: f32,
) -> Option<Blocker> {
    let first = *cookies.first()?;
    let f = first.forward;
    let my_extent = me.half_width + CAR_MARGIN + me.half_length;
    let min_width = match traffic {
        true => MIN_WIDTH_TRAFFIC,
        false => MIN_WIDTH,
    };
    let my_speed = xz(me.velocity).length();
    let mut blocker: Option<(f32, Blocker)> = None;

    for a in avoidables.iter().take(32) {
        if (a.position.y - first.centre.y).abs() > MAX_HEIGHT_DIFF {
            continue;
        }
        let ar = a.right();
        let his_extent =
            a.half_width + CAR_MARGIN + f.dot(a.forward).abs() * a.half_length + f.dot(ar).abs() * a.half_width;
        let relative = xz(a.position - me.position);
        let dist_ahead = relative.dot(f);
        let inside = match traffic || my_speed < 20.0 {
            true => dist_ahead + his_extent < my_extent,
            false => dist_ahead - his_extent <= my_extent,
        };
        if dist_ahead < -(my_extent + his_extent) {
            continue;
        }

        // Where the noses meet if both keep going.
        let gap = (dist_ahead - my_extent - his_extent).max(0.0);
        let nose = my_extent.min(gap);
        let (my_nose, his_nose) = (xz(me.position) + me.forward * nose, xz(a.position) - a.forward * nose);
        let rel_velocity = xz(a.velocity - me.velocity);
        let time = approach_time(his_nose - my_nose, rel_velocity);
        let closing = (his_nose - my_nose).dot(rel_velocity) < 0.0;
        let far = relative.length()
            > my_extent
                + his_extent
                + match traffic {
                    true => 0.5 * my_speed + 2.0 * my_extent,
                    false => 0.0,
                };
        if !closing && far {
            continue;
        }
        if closing && time >= NO_APPROACH_TIME {
            continue;
        }

        // Cut the cookies along the car.
        let trailing = a.speed_along(f);
        let lateral_speed = xz(a.velocity).dot(first.right_axis());
        let shift = lateral_speed * time.clamp(0.0, 1.0);
        let close = ramp((dist_ahead - my_extent) / my_extent.max(1.0) * 6.0, -6.0, 6.0);
        let margin = close;
        let mut cut_any = false;
        for c in cookies.iter_mut() {
            let along = (xz(c.centre) - xz(a.position)).dot(f);
            if along < -(my_extent + his_extent) || along > his_extent {
                continue;
            }
            let lateral = c.lateral(xz(a.position)) + 0.8 * shift;
            let ra = c.right_axis();
            let diag = |s: f32| ((a.forward * a.half_length + ar * (a.half_width * s)).dot(ra)).abs();
            let half = diag(1.0).max(diag(-1.0)) + 0.2 * shift.abs() + vehicle_half_width + margin;
            let gap_right = c.right_offset - (lateral + half);
            let gap_left = (lateral - half) - c.left_offset;
            let (fits_right, fits_left) = (gap_right > 0.0, gap_left > 0.0);
            let mine = c.lateral(xz(me.position));
            let pass_right = match (fits_right, fits_left) {
                (true, false) => true,
                (false, true) => false,
                _ => lateral <= mine,
            };
            let (mut left, mut right) = (c.left_offset, c.right_offset);
            match pass_right {
                true => left = left.max(lateral + half).min(right - min_width),
                false => right = right.min(lateral - half).max(left + min_width),
            }
            c.set_offsets(left, right);
            c.flags |= CUT;
            if inside {
                c.flags |= CUT_BEHIND;
            }
            cut_any = true;
        }
        if cut_any && dist_ahead > my_extent + his_extent && blocker.is_none_or(|(d, _)| dist_ahead < d) {
            blocker = Some((dist_ahead, Blocker { trail_speed: trailing }));
        }
    }

    // Cut cookies get their centres moved to the middle of what is left of the corridor.
    for c in cookies.iter_mut().filter(|c| c.flags & CUT != 0) {
        let mid = (c.left_offset + c.right_offset) * 0.5;
        let axis = c.right_axis();
        c.centre += Vec3::new(axis.x, 0.0, axis.y) * mid;
        let half = c.width() * 0.5;
        c.set_offsets(-half, half);
    }
    blocker.map(|(_, b)| b)
}

fn ramp(v: f32, a: f32, b: f32) -> f32 {
    ((v - a) / (b - a)).clamp(0.0, 1.0)
}
