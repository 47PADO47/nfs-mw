//! The look-ahead trail: cross-sections of the drivable corridor ahead of a car ("cookies"), a pass that
//! cuts holes in it for other cars and a visibility sweep that turns it into a steering target.
//! Spec: `docs/specs/ai-road-nav-trail.md`.
//!
//! All the maths is planar: a 2D vector `(x, y)` is the `(x, z)` of physics space, as in the original.

mod curvature;
mod occlusion;
mod punch;

use glam::{Vec2, Vec3};

pub use curvature::trail_curvature;
pub use occlusion::{Occlusion, update_occluded_position};
pub use punch::{Avoidable, Body};

/// Cookies in the ring (about 96 m of road at the default gap).
pub const CAPACITY: usize = 32;
/// Default distance between cookies, metres.
pub const DEFAULT_GAP: f32 = 3.0;

/// The bounds were cut by an avoidable.
pub const CUT: u8 = 1;
/// The avoidable that cut them is at or behind the car.
pub const CUT_BEHIND: u8 = 2;

/// `cross(a, b) = a.x·b.y − a.y·b.x`.
pub fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// The planar part of a physics-space vector.
pub fn xz(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// One cross-section of the corridor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cookie {
    /// The cursor's position when the cookie was recorded.
    pub centre: Vec3,
    pub left: Vec2,
    pub right: Vec2,
    /// Unit vector perpendicular to the left-to-right line, pointing along the road.
    pub forward: Vec2,
    /// Signed lateral distances of the bounds from the centre (left is normally negative).
    pub left_offset: f32,
    pub right_offset: f32,
    /// Distance from the previous cookie.
    pub length: f32,
    pub curvature: f32,
    pub flags: u8,
    pub segment: u16,
}

impl Cookie {
    /// A cookie at `centre` with corridor bounds `left` and `right`.
    pub fn new(centre: Vec3, left: Vec2, right: Vec2, curvature: f32, segment: u16) -> Self {
        let across = right - left;
        let forward = Vec2::new(-across.y, across.x).try_normalize().unwrap_or(Vec2::Y);
        let c = xz(centre);
        Self {
            centre,
            left,
            right,
            forward,
            left_offset: cross(left - c, forward),
            right_offset: cross(right - c, forward),
            length: 0.0,
            curvature,
            flags: 0,
            segment,
        }
    }

    /// The unit vector to the right of the road at this cookie.
    pub fn right_axis(&self) -> Vec2 {
        Vec2::new(self.forward.y, -self.forward.x)
    }

    /// Signed lateral position of `p` across the corridor (positive right of the centre).
    pub fn lateral(&self, p: Vec2) -> f32 {
        cross(p - xz(self.centre), self.forward)
    }

    /// Sets the bounds from lateral offsets and updates the points.
    pub fn set_offsets(&mut self, left: f32, right: f32) {
        let (c, axis) = (xz(self.centre), self.right_axis());
        self.left_offset = left;
        self.right_offset = right;
        self.left = c + axis * left;
        self.right = c + axis * right;
    }

    pub fn width(&self) -> f32 {
        self.right_offset - self.left_offset
    }
}

/// The ring of cookies, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Trail {
    cookies: Vec<Cookie>,
}

impl Trail {
    pub fn cookies(&self) -> &[Cookie] {
        &self.cookies
    }

    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    /// Records a cookie when the cursor is at least `gap` metres from the newest one. A cursor that
    /// turned round (it moved against the newest cookie's forward) clears the trail first.
    pub fn record(&mut self, mut cookie: Cookie, gap: f32) {
        if let Some(newest) = self.cookies.last() {
            let step = xz(cookie.centre - newest.centre);
            let distance = cookie.centre.distance(newest.centre);
            if distance < gap {
                return;
            }
            if step.try_normalize().is_some_and(|d| d.dot(newest.forward) < -0.99) {
                self.cookies.clear();
            } else {
                cookie.length = distance;
            }
        }
        if self.cookies.len() == CAPACITY {
            self.cookies.remove(0);
        }
        self.cookies.push(cookie);
    }

    /// Replaces the trail by a single cookie at the cursor.
    pub fn reset(&mut self, cookie: Cookie) {
        self.cookies.clear();
        self.cookies.push(cookie);
    }
}
