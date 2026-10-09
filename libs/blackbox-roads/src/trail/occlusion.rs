//! The steering target: the point a car aims at so that it follows the corridor of its trail.
//! Spec: `docs/specs/ai-road-nav-trail.md` (§3, §4).

use glam::{Vec2, Vec3};

use super::punch::{Body, punch_avoidables};
use super::{CUT, CUT_BEHIND, Cookie, Trail, cross, xz};

/// The car is "past" a cookie when it is on the far side of its cross-section; the cookies at least this
/// far ahead (widened when the car is out of bounds) start the visibility sweep.
const LOOK_MIN: f32 = 2.0;
const LOOK_MAX_TRAFFIC: f32 = 4.0;
const LOOK_MAX: f32 = 8.0;
const OUT_SCALE: f32 = 2.0;
const OUT_BOUNDS_TRAFFIC: f32 = 1.5;
const OUT_BOUNDS: f32 = 1.0;

/// What the sweep found.
#[derive(Debug, Clone, PartialEq)]
pub struct Occlusion {
    /// The trail interpolated at the car.
    pub current: Cookie,
    /// Index of the cookie the car has most recently passed.
    pub current_index: usize,
    /// Half the car's width plus how far its body sticks out of the corridor (large when off the road).
    pub out_of_bounds: f32,
    /// The view is blocked by the road's bend: -1 on the left, 1 on the right, 0 not at all.
    pub road: i8,
    /// The view is blocked by another car's cut: -1 left, 1 right, 0 not at all.
    pub avoidable: i8,
    /// The blocking cut is at or behind the car.
    pub from_behind: bool,
    /// The point that blocks the view (the cursor when nothing does).
    pub apex: Vec3,
    /// The width of the corridor at the apex.
    pub apex_width: f32,
    /// Where to steer: the cursor when nothing blocks the view, otherwise a point round the apex.
    pub position: Vec3,
    /// Speed of the car that blocks the view, along the road.
    pub trail_speed: f32,
    /// The car's speed towards the blocking point minus the blocker's.
    pub closing_speed: f32,
}

impl Occlusion {
    pub fn occluded(&self) -> bool {
        self.road != 0 || self.avoidable != 0
    }
}

/// Finds the steering target of `car` on `trail`, with the cursor at `cursor` and other cars in
/// `avoidables`. `traffic` selects the narrower look-ahead of traffic cars.
pub fn update_occluded_position(
    trail: &Trail,
    car: &Body,
    cursor: Vec3,
    avoidables: &[Body],
    traffic: bool,
    vehicle_half_width: f32,
) -> Option<Occlusion> {
    let cookies = trail.cookies();
    if cookies.is_empty() {
        return None;
    }
    let car2 = xz(car.position);
    let (look_max, out_bounds) = match traffic {
        true => (LOOK_MAX_TRAFFIC, OUT_BOUNDS_TRAFFIC),
        false => (LOOK_MAX, OUT_BOUNDS),
    };

    // Locate the car: the last cookie it has passed, and the first one far enough ahead to start the sweep.
    let (mut current, mut look, mut n) = (0, LOOK_MIN, cookies.len());
    for (i, c) in cookies.iter().enumerate() {
        let dot = c.forward.dot(car2 - xz(c.centre));
        if dot >= 0.0 {
            current = i;
            let lateral = c.lateral(car2);
            let out = out_bounds + (lateral - c.right_offset).max(c.left_offset - lateral);
            look = (LOOK_MIN + OUT_SCALE * out.max(0.0)).min(look_max);
        } else if dot < -look {
            n = i;
            break;
        }
    }

    let blended = blend(cookies, current, car2);
    let lateral = blended.lateral(car2);
    let out_of_bounds = vehicle_half_width + (lateral - blended.right_offset).max(blended.left_offset - lateral);

    // Cut holes for the other cars in a local copy of the cookies from the current one on.
    let mut local: Vec<Cookie> = cookies[current..].to_vec();
    let blocker = punch_avoidables(&mut local, car, avoidables, traffic, vehicle_half_width);
    let skip = n.saturating_sub(current).min(local.len().saturating_sub(1));
    let sweep = sweep(&local[skip..], car2, cursor);

    let mut out = Occlusion {
        current: blended,
        current_index: current,
        out_of_bounds,
        road: 0,
        avoidable: 0,
        from_behind: false,
        apex: cursor,
        apex_width: 0.0,
        position: cursor,
        trail_speed: 0.0,
        closing_speed: 0.0,
    };
    let Some((side, apex, cookie)) = sweep else { return Some(out) };
    let flags = local[skip + cookie].flags;
    match flags & CUT != 0 {
        true => out.avoidable = side,
        false => out.road = side,
    }
    out.from_behind = flags & CUT_BEHIND != 0;
    out.apex = Vec3::new(apex.x, cursor.y, apex.y);
    out.apex_width = local[skip + cookie].width();
    if let Some(b) = blocker {
        out.trail_speed = b.trail_speed;
        let toward = (apex - car2).try_normalize().unwrap_or(Vec2::Y);
        out.closing_speed = xz(car.velocity).dot(toward) - b.trail_speed;
    }
    out.position = aim_point(car2, apex, xz(cursor), &out, car);
    Some(out)
}

/// The trail interpolated between cookie `current` and the next at the car.
fn blend(cookies: &[Cookie], current: usize, car: Vec2) -> Cookie {
    let a = cookies[current];
    let Some(b) = cookies.get(current + 1) else { return a };
    let dot_a = a.forward.dot(car - xz(a.centre)).max(0.0);
    let dot_b = b.forward.dot(car - xz(b.centre)).abs();
    // How far the car is from `a` towards `b`.
    let t = match dot_a + dot_b > 1e-6 {
        true => dot_a / (dot_a + dot_b),
        false => 0.0,
    }
    .clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    let mut c = a;
    c.centre = a.centre.lerp(b.centre, t);
    c.left = a.left.lerp(b.left, t);
    c.right = a.right.lerp(b.right, t);
    c.forward = a.forward.lerp(b.forward, t).try_normalize().unwrap_or(a.forward);
    c.left_offset = mix(a.left_offset, b.left_offset);
    c.right_offset = mix(a.right_offset, b.right_offset);
    c
}

/// The visibility sweep ("string pulling" through the corridor). Returns the side the view is blocked
/// on (-1 left, 1 right), the blocking bound point and the index of its cookie, or `None` when the cursor is
/// visible.
fn sweep(cookies: &[Cookie], car: Vec2, cursor: Vec3) -> Option<(i8, Vec2, usize)> {
    let first = cookies.first()?;
    let (mut left, mut right) = (first.left - car, first.right - car);
    let (mut left_apex, mut right_apex) = ((first.left, 0), (first.right, 0));
    let mut blocked: Option<(i8, Vec2, usize)> = None;
    let mut test = |centre: Vec2, left: Vec2, right: Vec2, left_apex: (Vec2, usize), right_apex: (Vec2, usize)| {
        let to = centre - car;
        let result = match () {
            _ if cross(left, to) > 0.0 => Some((-1, left_apex.0, left_apex.1)),
            _ if cross(right, to) < 0.0 => Some((1, right_apex.0, right_apex.1)),
            _ => None,
        };
        if result.is_some() {
            blocked = result;
        }
    };
    for (k, c) in cookies.iter().enumerate().skip(1) {
        let (to_left, to_right) = (c.left - car, c.right - car);
        if cross(left, to_left) < 0.0 {
            (left, left_apex) = (to_left, (c.left, k));
        }
        if cross(right, to_right) > 0.0 {
            (right, right_apex) = (to_right, (c.right, k));
        }
        test(xz(c.centre), left, right, left_apex, right_apex);
        if cross(left, right) > 0.0 {
            break;
        }
    }
    // The cursor itself is the last point that has to be visible.
    test(xz(cursor), left, right, left_apex, right_apex);
    blocked
}

/// The point to steer at when the view is blocked: the cursor reflected through the apex, projected on the
/// bisector of the directions apex-to-car and apex-to-cursor, no further than the corridor is wide.
fn aim_point(car: Vec2, apex: Vec2, cursor: Vec2, o: &Occlusion, body: &Body) -> Vec3 {
    let to_car = (car - apex).try_normalize().unwrap_or(Vec2::NEG_Y);
    let to_cursor = (cursor - apex).try_normalize().unwrap_or(Vec2::Y);
    let bisector = (to_car + to_cursor).try_normalize().unwrap_or(to_cursor);
    let desired = apex + to_cursor * (car - apex).length();
    let mut length = (desired - apex).dot(bisector);
    let width = o.apex_width.max(0.1);
    if o.avoidable != 0 {
        let own = xz(body.velocity).dot((apex - car).try_normalize().unwrap_or(Vec2::Y)).abs().max(1e-3);
        let ratio = (o.closing_speed / (2.0 * own)).clamp(0.0, 1.0);
        length = (length * 2.0 * ratio).min(width);
    }
    let p = apex + bisector * length.clamp(0.0, width);
    Vec3::new(p.x, o.apex.y, p.y)
}
