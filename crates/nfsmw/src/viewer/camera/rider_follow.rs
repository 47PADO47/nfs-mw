//! Who a bike's cameras look at (`docs/specs/ride6-bike-cameras.md`, section 8). While the bike is down the interest of the picture
//! is the rider, not the bike: the cameras follow him as the ragdoll flies and slides, as he gets up and as he jogs back to the
//! bike, and return to the bike once he has remounted.
//!
//! This is the pure part: which of the two is the subject ([`Subject`], from the rider's [`Phase`]), how the camera's attention moves
//! between them ([`RiderCam::weight`]: a ramp with an ease at each end, quicker towards the rider than back), where the rider's
//! heading is taken from, and the grace after a reset (`bust_grace`). The host feeds it the rider's place every physics step
//! (`scenes/world/drive/rider_cam.rs`) and the camera paths read the blend.

use glam::Vec3;

use super::damped::Damped;

/// How tightly the followed point holds to the rider's pelvis (1/s, a critically damped spring): it lags a steady 20 m/s by 1.1 m and
/// takes out what moves faster than about 5 Hz, which is the shake of a ragdoll's pelvis (`docs/specs/ride6-feel-round14.md`, section 1).
/// A modelling choice, picked in offscreen runs: 20 let him run away from the picture, 60 kept half of the noise.
pub const FOLLOW_OMEGA: f32 = 35.0;
/// The most the followed point accelerates (m/s^2): a body falling freely does 9.8, a hard landing a few hundred for a step.
pub const FOLLOW_MAX_ACCEL: f32 = 300.0;
/// The fastest the heading the camera sits behind turns (rad/s).
pub const HEADING_MAX_RATE: f32 = 5.0;
/// How far above the ground the followed point is kept (m) when the ground is just above it (a pelvis that sank into the road), and the
/// most the point is lifted for that: a point further under the ground than this is under the map, which the host's guard deals with.
pub const GROUND_CLEARANCE_M: f32 = 0.25;
pub const MAX_LIFT_M: f32 = 1.5;

/// How long the camera takes to move its attention from the bike to the rider (s). A modelling choice: the rider leaves the
/// bike at its place, so the shot only has to follow him as he flies off; a third of a second keeps the fall's start in view.
pub const TO_RIDER_S: f32 = 0.35;
/// How long it takes to move back to the bike once he has remounted (s). A modelling choice: slower than the way out, so the
/// camera does not snap to the bike when he gets on it.
pub const TO_BIKE_S: f32 = 0.9;
/// How far above the rider's pelvis the chase camera looks (m): the middle of his back, so that he is in the middle of the
/// picture lying and standing.
pub const RIDER_LOOK_UP_M: f32 = 0.25;
/// How much higher the chase camera sits over the rider than over the bike (m): a person on the ground is smaller than a bike and
/// the camera at the bike's height shows him from the side of the road; a little more height looks down on him.
pub const RIDER_CAMERA_LIFT_M: f32 = 0.6;
/// The rider's horizontal speed (m/s) under which his heading is held: a slide that has nearly stopped has no direction.
pub const HEADING_MIN_SPEED: f32 = 1.5;
/// The distance to the bike (m) under which the jogging rider's heading is held instead of turned at it.
pub const FACE_BIKE_MIN_M: f32 = 0.5;
/// How long after a reset or a remount the rider cannot be busted (s). A modelling choice (`docs/specs/fairbusts-reset.md`): the
/// original's reset gives the car a 2 s flash that makes busting *faster*; this port gives a bike's rider a longer, safe one.
pub const BIKE_RESET_GRACE_S: f32 = 4.0;

/// Where the rider is, as the cameras care.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// On the bike: he never fell, or he has remounted.
    #[default]
    Riding,
    /// Thrown: in the air, sliding, lying, getting up.
    Fallen,
    /// Back on his feet, on his way to the bike.
    Jogging,
}

impl Phase {
    /// Whether he is off the bike.
    pub fn off_bike(self) -> bool {
        self != Self::Riding
    }
}

/// What the camera looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Bike,
    Rider,
}

/// The subject for a rider in `phase`: the rider while he is off the bike and there is a body of his to look at (`located`: a
/// bike with no rider files has none), the bike otherwise (riding, remounted).
pub fn subject(phase: Phase, located: bool) -> Subject {
    match phase.off_bike() && located {
        true => Subject::Rider,
        false => Subject::Bike,
    }
}

/// The bike as the follower needs it each step (render space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bike {
    pub position: Vec3,
    pub heading: f32,
    pub velocity: Vec3,
}

/// The state of the camera's attention to the rider of one bike.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiderCam {
    /// 0 on the bike, 1 on the rider.
    amount: f32,
    phase: Phase,
    /// The rider's heading as the camera takes it (radians from +X, counter-clockwise).
    heading: f32,
    /// The rider's focus point (his pelvis) and how far from the bike's position it was last seen off the bike: after he has
    /// remounted the focus keeps that offset to the bike, so the camera eases back without a jump.
    focus: Option<Vec3>,
    /// The focus of the step before, for the frame's blend between two steps.
    before: Option<Vec3>,
    /// The spring that follows his pelvis, and the height of the ground under it (render space) the host last said.
    smooth: Option<Damped>,
    floor: Option<f32>,
    offset: Vec3,
    velocity: Vec3,
    /// The camera was cut to the bike (a placement): it keeps to the bike until the rider is on it again.
    ignore: bool,
    /// Seconds of grace left after a reset or a remount.
    grace: f32,
}

/// The angle `to`, reached from `from` the short way, `share` of the way there (radians).
pub fn blend_angle(from: f32, to: f32, share: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let gap = (to - from + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    from + gap * share
}

/// The angle `from` turned towards `to` the short way by at most `step` radians.
fn turn_towards(from: f32, to: f32, step: f32) -> f32 {
    let gap = (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + gap.clamp(-step, step)
}

/// `point` held above the ground: when the ground (render height `floor`) is above it by up to [`MAX_LIFT_M`], the point is raised to
/// [`GROUND_CLEARANCE_M`] over it. A point that is above the ground, has no ground known or is far under it is returned as it is.
pub fn above_ground(point: Vec3, floor: Option<f32>) -> Vec3 {
    let Some(floor) = floor.filter(|floor| floor.is_finite()) else { return point };
    let lift = floor + GROUND_CLEARANCE_M - point.z;
    match lift > 0.0 && lift <= MAX_LIFT_M + GROUND_CLEARANCE_M {
        true => Vec3::new(point.x, point.y, point.z + lift),
        false => point,
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl RiderCam {
    /// One physics step of `dt` seconds. `phase` and `rider` (his pelvis, if the host has his body) are the rider's; `bike` is
    /// the bike's pose.
    pub fn step(&mut self, phase: Phase, rider: Option<Vec3>, bike: Bike, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let remounted = self.phase.off_bike() && !phase.off_bike();
        self.phase = phase;
        self.grace = match remounted {
            true => BIKE_RESET_GRACE_S,
            false => (self.grace - dt).max(0.0),
        };
        if !phase.off_bike() {
            self.ignore = false;
        }
        if remounted && let Some(focus) = self.focus {
            // The bike stands up for him: its origin moves (it lay on its side), and the point must not move with it.
            self.offset = focus - bike.position;
        }
        let located = rider.filter(|_| !self.ignore);
        let subject = subject(phase, located.is_some());
        self.follow_rider(phase, located, bike, dt);
        let (target, seconds) = match subject {
            Subject::Rider => (1.0, TO_RIDER_S),
            Subject::Bike => (0.0, TO_BIKE_S),
        };
        let travel = dt / seconds;
        self.amount += (target - self.amount).clamp(-travel, travel);
        if self.amount <= 0.0 && subject == Subject::Bike {
            self.focus = None;
            self.smooth = None;
        }
    }

    /// Update the rider's point, velocity and heading for this step.
    fn follow_rider(&mut self, phase: Phase, rider: Option<Vec3>, bike: Bike, dt: f32) {
        self.before = self.focus;
        let Some(point) = rider.filter(|_| phase.off_bike()) else {
            // On the bike: his heading is its, and what is left of the blend keeps its offset to the bike.
            self.heading = bike.heading;
            self.velocity = bike.velocity;
            self.focus = self.focus.map(|_| bike.position + self.offset);
            return;
        };
        let smooth = match &mut self.smooth {
            Some(smooth) => {
                smooth.step(point, FOLLOW_OMEGA, FOLLOW_MAX_ACCEL, dt);
                *smooth
            }
            None => *self.smooth.insert(Damped::moving(point, bike.velocity)),
        };
        let point = smooth.position();
        self.velocity = smooth.velocity();
        self.focus = Some(point);
        self.offset = point - bike.position;
        let flat = Vec3::new(self.velocity.x, self.velocity.y, 0.0);
        if phase == Phase::Fallen && flat.length() >= HEADING_MIN_SPEED {
            self.heading = turn_towards(self.heading, flat.y.atan2(flat.x), HEADING_MAX_RATE * dt);
        }
        let to_bike = bike.position - point;
        let flat = Vec3::new(to_bike.x, to_bike.y, 0.0);
        if phase == Phase::Jogging && flat.length() >= FACE_BIKE_MIN_M {
            self.heading = turn_towards(self.heading, flat.y.atan2(flat.x), HEADING_MAX_RATE * dt);
        }
    }

    /// How much the camera is on the rider, 0 (the bike) to 1 (the rider), eased at both ends.
    pub fn weight(&self) -> f32 {
        smooth(self.amount)
    }

    /// Whether the camera is on the rider at all, even partly (while he is off the bike and while it eases back).
    pub fn active(&self) -> bool {
        self.amount > 0.0
    }

    /// The rider's pelvis, while the camera is on him.
    pub fn focus(&self) -> Option<Vec3> {
        self.focus.filter(|_| self.active()).map(|point| above_ground(point, self.floor))
    }

    /// The ground under the rider (render height), as the host found it this step; `None` where there is none.
    pub fn set_floor(&mut self, floor: Option<f32>) {
        self.floor = floor;
    }

    /// The rider's pelvis `alpha` (0 to 1) of the way from the step before to the last one, for a frame drawn between two steps.
    pub fn focus_at(&self, alpha: f32) -> Option<Vec3> {
        let focus = self.focus()?;
        let before = self.before.map(|before| above_ground(before, self.floor));
        Some(before.map_or(focus, |before| before.lerp(focus, alpha.clamp(0.0, 1.0))))
    }

    /// His velocity as the camera takes it (m/s, render space).
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// The point a chase camera that looks `look_height` above the point it is given follows, so that it looks at the rider
    /// ([`RIDER_LOOK_UP_M`] above his pelvis) as much as [`RiderCam::weight`] says, and at the bike (`bike`) otherwise.
    pub fn chase_point(&self, bike: Vec3, look_height: f32) -> Vec3 {
        let Some(focus) = self.focus() else { return bike };
        bike.lerp(focus - Vec3::Z * (look_height - RIDER_LOOK_UP_M), self.weight())
    }

    /// The chase camera's height factor (`Fit::height`, on the player's preferred height `preferred` in metres) raised so that
    /// the camera is [`RIDER_CAMERA_LIFT_M`] higher as the camera is on the rider.
    pub fn lifted_height(&self, factor: f32, preferred: f32) -> f32 {
        factor + self.weight() * RIDER_CAMERA_LIFT_M / preferred.max(0.1)
    }

    /// The heading the camera takes, the bike's (`bike`) blended to the rider's.
    pub fn heading(&self, bike: f32) -> f32 {
        match self.focus() {
            Some(_) => blend_angle(bike, self.heading, self.weight()),
            None => bike,
        }
    }

    /// Whether a bike's rider cannot be busted: he is off the bike, or has been put back (a reset, a remount) within
    /// [`BIKE_RESET_GRACE_S`].
    pub fn bust_grace(&self) -> bool {
        self.phase.off_bike() || self.grace > 0.0
    }

    /// A reset of the bike by hand (or the flip reset) while he rides: the grace starts.
    pub fn note_reset(&mut self) {
        self.grace = BIKE_RESET_GRACE_S;
    }

    /// The bike was placed again (the camera is cut to it): the attention goes back to the bike at once and stays there until
    /// the rider is on it.
    pub fn cut_to_bike(&mut self) {
        self.amount = 0.0;
        self.focus = None;
        self.before = None;
        self.smooth = None;
        self.ignore = self.phase.off_bike();
    }
}

#[cfg(test)]
#[path = "rider_follow_tests.rs"]
mod tests;
