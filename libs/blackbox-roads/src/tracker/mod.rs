//! Persistent free-roam target roads. Spec: `docs/specs/ai-road-target-tracker.md`.

mod spot;
#[cfg(test)]
mod tests;
mod walker;

use glam::Vec3;

use crate::{Body, LaneType, NavKind, PathType, RoadNav, RoadNetwork, SegmentIndex};
use spot::{advance, snap};

/// A shared target's current and one-second future road positions, in physics space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadObservation {
    pub current: Vec3,
    pub future: Vec3,
}

/// Target-vehicle road history, independent of chasing vehicles and renderer state.
/// Race-route tracking remains with the caller until its complete route filters are available.
#[derive(Debug, Clone)]
pub struct RoadTracker {
    current: Option<RoadNav>,
    future: Option<RoadNav>,
    far: Vec3,
    far_direction: Vec3,
    previous: Option<(u16, usize)>,
    clock: f64,
    last_rethink: f64,
    last_increment: f64,
    last_seek: f64,
    seek: Option<Vec3>,
    active: bool,
}

impl Default for RoadTracker {
    fn default() -> Self {
        Self {
            current: None,
            future: None,
            far: Vec3::ZERO,
            far_direction: Vec3::ZERO,
            previous: None,
            clock: 0.0,
            last_rethink: -100.0,
            last_increment: -100.0,
            last_seek: -100.0,
            seek: None,
            active: false,
        }
    }
}

impl RoadTracker {
    /// Ages simulation clocks even without an observation; absent targets expose no snapshot.
    pub fn update(&mut self, net: &RoadNetwork, index: &SegmentIndex, body: Option<Body>, dt: f32) {
        self.active = false;
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.clock += f64::from(dt);
        let Some(body) = body else { return };
        if !body.position.is_finite() || !body.velocity.is_finite() || !body.forward.is_finite() {
            return;
        }
        self.active = true;
        let valid = self.current.as_ref().is_some_and(|nav| body.position.distance(nav.position) < 20.0)
            && self.future.is_some();
        if valid && self.clock - self.last_increment < 0.02 {
            return;
        }
        if valid && self.clock - self.last_rethink < 0.33 {
            self.last_increment = self.clock;
            self.increment(net, body);
            return;
        }
        self.last_rethink = self.clock;
        let Some(result) = walker::walk(net, index, body, self.previous) else { return };
        self.previous = Some((result.future.segment, result.future.node_ind));
        self.current = Some(result.current);
        self.future = Some(result.future);
        self.far = result.far.position;
        self.far_direction = result.far.forward;
    }

    fn increment(&mut self, net: &RoadNetwork, body: Body) {
        let (Some(current), Some(future)) = (&mut self.current, &mut self.future) else { return };
        let direction = current.forward.normalize_or_zero();
        let distance = (body.position - current.position).dot(direction);
        if distance > 0.05 {
            snap(net, current, body.position);
            advance(current, net, distance, direction);
        }
        let speed = body.velocity.length();
        let deficit = speed - future.position.distance(current.position);
        if deficit <= 0.05 || speed <= 0.0 {
            return;
        }
        let toward = (self.far - future.position).normalize_or_zero();
        let forward = future.forward.normalize_or_zero();
        let side = Vec3::new(forward.z, 0.0, -forward.x);
        let desired = future.position + side * (body.velocity.dot(side) * deficit / speed);
        snap(net, future, desired);
        advance(future, net, deficit * toward.dot(forward), toward);
    }

    pub fn current(&self) -> Option<&RoadNav> {
        if !self.active {
            return None;
        }
        self.current.as_ref()
    }

    pub fn future(&self) -> Option<&RoadNav> {
        if !self.active {
            return None;
        }
        self.future.as_ref()
    }

    pub fn snapshot(&self) -> Option<RoadObservation> {
        Some(RoadObservation { current: self.current()?.position, future: self.future()?.position })
    }

    /// Refreshes the shared prediction after its separate one-third-second cache expires.
    /// `prediction_seconds` is supplied by game policy; road traversal uses a zero direction hint.
    pub fn seek_ahead(
        &mut self,
        net: &RoadNetwork,
        speed: f32,
        prediction_seconds: f32,
        cop_filter: bool,
    ) -> Option<Vec3> {
        if !self.active {
            return None;
        }
        if self.clock - self.last_seek <= 0.33 {
            return self.seek;
        }
        let mut nav = self.current.as_ref()?.clone();
        if !speed.is_finite() || !prediction_seconds.is_finite() {
            return self.seek;
        }
        nav.kind = NavKind::Direction;
        nav.lane_type = LaneType::Cop;
        nav.path_type = PathType::Cop;
        nav.filter.cop = cop_filter;
        self.last_seek = self.clock;
        advance(&mut nav, net, (speed.max(0.0) * prediction_seconds.max(0.0) + 8.0).min(500.0), Vec3::ZERO);
        self.seek = Some(nav.position);
        self.seek
    }

    /// Stable state words, including clocks and cached predictions that affect later callers.
    pub fn visit_state(&self, visit: &mut impl FnMut(u32)) {
        visit(u32::from(self.active));
        for value in [self.clock, self.last_rethink, self.last_increment, self.last_seek] {
            let bits = value.to_bits();
            visit(bits as u32);
            visit((bits >> 32) as u32);
        }
        for nav in [&self.current, &self.future] {
            visit(u32::from(nav.is_some()));
            if let Some(nav) = nav {
                nav.visit_state(visit);
            }
        }
        for point in [Some(self.far), Some(self.far_direction), self.seek] {
            visit(u32::from(point.is_some()));
            if let Some(point) = point {
                for value in point.to_array() {
                    visit(value.to_bits());
                }
            }
        }
        visit(u32::from(self.previous.is_some()));
        if let Some((segment, node)) = self.previous {
            visit(u32::from(segment));
            visit(node as u32);
        }
    }
}
