//! Cutting holes in the trail for other cars so that the corridor passes on one side.
//! Specs: `docs/specs/ai-road-nav-trail.md` (§5), `docs/specs/ai-pursuit-obstacle-selection.md` (corridor cuts).

use glam::{Vec2, Vec3};

use super::{CUT, CUT_BEHIND, Cookie, xz};

#[cfg(test)]
#[path = "predictive_punch_tests.rs"]
mod tests;

/// Cars higher or lower than this relative to the trail are ignored.
pub(super) const MAX_HEIGHT_DIFF: f32 = 5.0;
/// Half a metre of room on every side of a car.
const CAR_MARGIN: f32 = 0.5;
/// Time to closest approach returned when two cars move alike, seconds; meetings this late cut nothing.
const NO_APPROACH_TIME: f32 = 3.0;
/// Corridor width kept after a cut, metres (traffic, others).
const MIN_WIDTH_TRAFFIC: f32 = 0.1;
const MIN_WIDTH: f32 = 1.0;
/// How far the closeness of a cut ramps, metres either side of the car along the road.
const CLOSENESS_RANGE: f32 = 6.0;

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

/// How the car moves across the road section it is on: its lateral offset and lateral speed there, and the heading of
/// its lane cursor. The side to pass on is chosen against the offset these predict.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct Drift {
    pub offset: f32,
    pub lateral_speed: f32,
    pub heading: Vec2,
}

/// Seconds until the two cars' noses are closest (negative once they separate), or 3 s when they move alike.
fn approach_time(rel: Vec2, vel: Vec2) -> f32 {
    let speed2 = vel.length_squared();
    match speed2 < 1e-4 {
        true => NO_APPROACH_TIME,
        false => -rel.dot(vel) / speed2,
    }
}

/// A cross-section ahead of the point, found at the nearest forward-projection sign change.
fn closest_cookie_ahead(cookies: &[Cookie], point: Vec2) -> Option<usize> {
    let mut best = None;
    let mut closest = f32::INFINITY;
    let mut previous = 0.0;
    for (i, cookie) in cookies.iter().enumerate() {
        let offset = point - xz(cookie.centre);
        let projection = offset.dot(cookie.forward);
        let crossing = match i {
            0 => projection < 0.0,
            _ => projection * previous < 0.0,
        };
        let distance = offset.length_squared();
        if crossing && distance < closest {
            best = Some(i);
            closest = distance;
        }
        previous = projection;
    }
    best
}

/// How another car stands against mine along the road, once it is known to cut.
struct Judged {
    dist_ahead: f32,
    combined: f32,
    inside: bool,
    time: f32,
}

/// Narrows the bounds of `cookies` (the visibility sweep's first cookie first) around each of `avoidables`. Returns
/// the closest cut that is further ahead than the two cars' extents.
pub(super) fn punch_avoidables(
    cookies: &mut [Cookie],
    me: &Body,
    avoidables: impl IntoIterator<Item = Body>,
    traffic: bool,
    vehicle_half_width: f32,
    drift: Drift,
) -> Option<Blocker> {
    let reference = *cookies.get(closest_cookie_ahead(cookies, xz(me.position))?)?;
    let min_width = match traffic {
        true => MIN_WIDTH_TRAFFIC,
        false => MIN_WIDTH,
    };
    let mut blocker: Option<(usize, Blocker)> = None;
    for a in avoidables.into_iter().take(32) {
        if (a.position.y - reference.centre.y).abs() > MAX_HEIGHT_DIFF {
            continue;
        }
        let Some(judged) = judge(&reference, me, &a, traffic) else { continue };
        // Cut where the vehicles will meet, rather than beside the other car's current body.
        let trailing = a.speed_along(reference.forward);
        let impact = xz(a.position) + reference.forward * (trailing * judged.time - judged.combined);
        let Some(start) = closest_cookie_ahead(cookies, impact) else { continue };
        if judged.dist_ahead > judged.combined && blocker.is_none_or(|(index, _)| start < index) {
            blocker = Some((start, Blocker { trail_speed: trailing }));
        }
        let cut = Cut::plan(&cookies[start], me, &a, &judged, vehicle_half_width, drift, impact);
        for (step, c) in cookies[start..].iter_mut().enumerate() {
            if !cut.apply(c, judged.inside, min_width) && step == 0 {
                break;
            }
            // How far past the meeting this cookie lies. The original takes the other car's height in place of the
            // second horizontal coordinate here; that measure is kept.
            let before = (impact.x - c.centre.x) * c.forward.x + (a.position.y - c.centre.y) * c.forward.y;
            if 2.0 * judged.combined + before < 0.0 {
                break;
            }
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

/// The extents, inside flag and approach time of `a` seen from `me` along `reference`, or `None` when `a` cuts
/// nothing: entirely behind me, behind my position while beside me, not closing and too far, or met too late.
fn judge(reference: &Cookie, me: &Body, a: &Body, traffic: bool) -> Option<Judged> {
    let f = reference.forward;
    let (ar, mr) = (a.right(), me.right());
    let my_extent = me.half_width + CAR_MARGIN + me.half_length;
    let his_extent =
        a.half_width + CAR_MARGIN + f.dot(a.forward).abs() * a.half_length + f.dot(ar).abs() * a.half_width;
    let combined = my_extent + his_extent;
    let relative = xz(a.position - me.position);
    let dist_ahead = relative.dot(f);
    let my_speed = xz(me.velocity).length();
    let inside = match traffic || my_speed < 20.0 {
        true => dist_ahead + his_extent < my_extent,
        false => dist_ahead - his_extent <= my_extent,
    };
    if dist_ahead < -combined {
        return None;
    }
    // A car whose front is behind me while it runs beside me is passing or being passed: it cuts nothing.
    let side_extent = ar.dot(mr).abs() * a.half_width + a.forward.dot(mr).abs() * a.half_length + me.half_width;
    let lateral = relative.dot(mr).abs();
    if dist_ahead + his_extent < 0.0 && lateral < side_extent {
        return None;
    }
    // Where the noses meet if both keep going.
    let gap = (dist_ahead - combined).max(0.0);
    let my_nose = xz(me.position) + f * my_extent.min(gap);
    let his_nose = xz(a.position) - f * his_extent.min(gap);
    let rel_velocity = xz(a.velocity - me.velocity);
    let time = approach_time(his_nose - my_nose, rel_velocity);
    let closing = (his_nose - my_nose).dot(rel_velocity) < 0.0;
    let judged = Judged { dist_ahead, combined, inside, time };
    let timely = time < NO_APPROACH_TIME;
    // Traffic cuts for any closing car; everyone else only for a meeting within the approach limit.
    if inside || closing {
        return (timely || (traffic && closing && !inside)).then_some(judged);
    }
    let reach = combined
        + match traffic {
            true => 0.5 * my_speed + 2.0 * my_extent,
            false => 0.0,
        };
    if dist_ahead > reach {
        return None;
    }
    match traffic {
        true => (lateral <= side_extent + 1.0).then_some(judged),
        false => timely.then_some(judged),
    }
}

/// One car's cut: a point, a half width and the side left open, decided once at its first cookie.
struct Cut {
    point: Vec2,
    half: f32,
    pass_left: bool,
}

impl Cut {
    fn plan(c: &Cookie, me: &Body, a: &Body, judged: &Judged, half_width: f32, drift: Drift, impact: Vec2) -> Self {
        let ra = c.right_axis();
        let clamped = judged.time.clamp(0.0, 1.0);
        let shift = xz(a.velocity).dot(ra) * clamped;
        let point = impact + ra * (0.8 * shift);
        let along = |p: Vec2| (p - xz(c.centre)).dot(c.forward);
        let closeness = ramp(along(point) - along(xz(me.position)), -CLOSENESS_RANGE, CLOSENESS_RANGE);
        let ar = a.right();
        let diag = |s: f32| (a.forward * a.half_length + ar * (a.half_width * s)).dot(ra).abs();
        // The drift widens the cut by its signed amount, as in the original.
        let body = diag(1.0).max(diag(-1.0)) + 0.2 * shift * closeness;
        let margin = closeness + half_width;
        let lateral = c.lateral(point);
        let fits_left = margin < lateral - body - c.left_offset;
        let fits_right = margin < c.right_offset - lateral - body;
        let predicted = drift.offset + 0.2 * closeness * clamped * drift.lateral_speed + 2.0 * drift.heading.dot(ra);
        let pass_left = match fits_left != fits_right {
            true => fits_left,
            false => lateral > predicted,
        };
        Self { point, half: body + margin, pass_left }
    }

    /// Narrows `c` when the cut overlaps its corridor; returns whether it did.
    fn apply(&self, c: &mut Cookie, inside: bool, min_width: f32) -> bool {
        let lateral = c.lateral(self.point);
        if lateral < c.left_offset - self.half || lateral > c.right_offset + self.half {
            return false;
        }
        let (mut left, mut right) = (c.left_offset, c.right_offset);
        match self.pass_left {
            true => right = (lateral - self.half).max(left + min_width),
            false => left = (lateral + self.half).min(right - min_width),
        }
        c.set_offsets(left, right);
        c.flags |= CUT;
        if inside {
            c.flags |= CUT_BEHIND;
        }
        true
    }
}

fn ramp(v: f32, a: f32, b: f32) -> f32 {
    ((v - a) / (b - a)).clamp(0.0, 1.0)
}
