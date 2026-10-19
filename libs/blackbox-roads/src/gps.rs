//! The GPS route: a path from the car to a chosen point, and the point a look-ahead distance along it that the
//! arrow aims at. Engine-generic: the caller gives the car's position, heading and speed, and the target.
//! Spec: `docs/specs/ai-pathfinder.md` (§7). The original's constants are below, all **[decomp]**.

use glam::Vec3;

use crate::{
    LaneType, NavKind, PathState, PathType, RoadNav, RoadNetwork, SegmentFilter, SegmentIndex, SplitMix, centre_line,
};

/// The look-ahead distance is `speed * LOOK_AHEAD_SECONDS`, kept between the two bounds (metres).
pub const LOOK_AHEAD_MIN: f32 = 20.0;
pub const LOOK_AHEAD_MAX: f32 = 80.0;
pub const LOOK_AHEAD_SECONDS: f32 = 3.0;
/// How fast the look-ahead moves towards its wanted value, metres per second.
pub const LOOK_AHEAD_CATCH_UP: f32 = 10.0;
/// The largest distance the car may be from the route before the GPS is lost (metres). Asking for a smaller
/// value (below 0.1 m) uses this default.
pub const MAX_DEVIATION: f32 = 200.0;
/// The car within this distance of the target finishes the route (metres).
pub const ARRIVAL_DISTANCE: f32 = 30.0;
/// Samples taken along each path segment to find the point closest to the car.
const SAMPLES_PER_SEGMENT: usize = 8;

/// Whether the car is on the route. A GPS that is searching has lost its lane but still points at the route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpsState {
    Tracking,
    Searching,
}

/// Why the route ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpsEnd {
    Finished,
    Lost,
}

/// A route being followed. Create one with [`Gps::engage`], then call [`Gps::update`] every frame.
#[derive(Debug, Clone)]
pub struct Gps {
    nav: RoadNav,
    target: Vec3,
    max_deviation: f32,
    look_ahead: f32,
    state: GpsState,
    rng: SplitMix,
}

impl Gps {
    /// Starts a route from the car to `target`. `max_deviation` of 0.1 m or less uses [`MAX_DEVIATION`].
    /// Returns `None` if the car is not on the road or there is no route; nothing changes then.
    pub fn engage(
        net: &RoadNetwork,
        index: &SegmentIndex,
        car: Vec3,
        heading: Vec3,
        target: Vec3,
        max_deviation: f32,
    ) -> Option<Self> {
        let filter = SegmentFilter::default();
        let mut nav = RoadNav::new(NavKind::Path, LaneType::Racing, filter);
        if !nav.init_at_point(net, index, car, heading, true) {
            return None;
        }
        if nav.find_path_to(net, index, target, None, PathType::Gps) != PathState::Full {
            return None;
        }
        Some(Self {
            nav,
            target,
            max_deviation: if max_deviation < 0.1 { MAX_DEVIATION } else { max_deviation },
            look_ahead: LOOK_AHEAD_MIN,
            state: GpsState::Tracking,
            rng: SplitMix(0),
        })
    }

    /// One frame: re-places the car on the route, moves the look-ahead point and reports the end of the route.
    /// `speed` is the car's speed in metres per second; `dt` the frame time in seconds.
    pub fn update(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        car: Vec3,
        heading: Vec3,
        speed: f32,
        dt: f32,
    ) -> Option<GpsEnd> {
        if car.distance(self.target) <= ARRIVAL_DISTANCE {
            return Some(GpsEnd::Finished);
        }
        let placed = self.nav.init_at_point(net, index, car, heading, true);
        if placed && self.nav.on_path(net) {
            self.state = GpsState::Tracking;
        } else {
            self.state = GpsState::Searching;
            let Some((segment, t, distance)) = closest_on_path(net, &self.nav.path, car) else {
                return Some(GpsEnd::Lost);
            };
            if distance > self.max_deviation {
                return Some(GpsEnd::Lost);
            }
            self.nav.init_at_segment(net, segment, t, car, heading, true);
        }
        let wanted = (speed * LOOK_AHEAD_SECONDS).clamp(LOOK_AHEAD_MIN, LOOK_AHEAD_MAX);
        let step = LOOK_AHEAD_CATCH_UP * dt;
        self.look_ahead += (wanted - self.look_ahead).clamp(-step, step);
        self.nav.advance(net, self.look_ahead, Vec3::ZERO, &mut self.rng);
        None
    }

    /// The point the arrow aims at: a look-ahead distance along the route from the car.
    pub fn arrow_point(&self) -> Vec3 {
        self.nav.position
    }

    /// Unit direction from the car towards the arrow point, or `None` when it is on top of the car.
    pub fn arrow_direction(&self, car: Vec3) -> Option<Vec3> {
        (self.arrow_point() - car).try_normalize()
    }

    pub fn state(&self) -> GpsState {
        self.state
    }

    pub fn target(&self) -> Vec3 {
        self.target
    }

    pub fn look_ahead(&self) -> f32 {
        self.look_ahead
    }

    /// The route's segments, start first (for drawing it on the map).
    pub fn path(&self) -> &[u16] {
        &self.nav.path
    }
}

/// The point on the route closest to `point`: its segment, its stored-direction parameter and the distance.
fn closest_on_path(net: &RoadNetwork, path: &[u16], point: Vec3) -> Option<(u16, f32, f32)> {
    let mut best: Option<(u16, f32, f32)> = None;
    for &segment in path {
        let line = centre_line(net, segment, 1);
        for i in 0..=SAMPLES_PER_SEGMENT {
            let t = i as f32 / SAMPLES_PER_SEGMENT as f32;
            let distance = line.position(t).distance(point);
            if best.is_none_or(|(_, _, d)| distance < d) {
                best = Some((segment, t, distance));
            }
        }
    }
    best
}
