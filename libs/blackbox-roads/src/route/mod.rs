//! An ordered route through goals: the road from a start through each goal in turn, as one list of directed segment
//! pieces. A leg is the part from one goal to the next; a leg whose search found no road is *missing* and has no pieces.
//! Distances are measured along the centre lines, and a missing leg counts its straight line, so the distance along the
//! route grows from the start to the end.
//!
//! - [`build_leg`] searches one leg and [`RouteBuilder`] puts the legs together.
//! - [`Route::locate`] finds the point of the route nearest a position, in three dimensions.
//! - [`Route::point_at`] is the point at a distance, and [`Route::window`] the segments a navigator follows.

mod build;
mod locate;

use std::ops::Range;

use glam::Vec3;

use crate::{NodeInd, RoadNetwork, centre_line};

pub use build::{LegPath, RouteBuilder, build_leg};
pub use locate::RoutePoint;

/// A point the route passes through and the way it is crossed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    pub position: Vec3,
    /// Unit direction the goal is crossed in.
    pub forward: Vec3,
}

/// One piece of the route: the part of a segment it drives, in its direction of travel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteStep {
    pub segment: u16,
    /// The end of the segment the route heads to: `1` is the stored direction.
    pub node_ind: NodeInd,
    /// Where the piece starts and ends, as progress along the segment in the travel direction (0 to 1).
    pub from_t: f32,
    pub to_t: f32,
    /// Distance along the route where the piece starts, metres.
    pub start: f32,
    /// The leg the piece belongs to.
    pub leg: usize,
}

impl RouteStep {
    /// Metres of road the piece covers.
    pub fn length(&self, net: &RoadNetwork) -> f32 {
        (self.to_t - self.from_t) * net.segment(self.segment).length
    }
}

/// One leg: from one goal to the next.
#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    pub from: Goal,
    pub to: Goal,
    /// The leg's pieces in [`Route::steps`]; empty for a missing leg.
    pub steps: Range<usize>,
    /// Distance along the route where the leg starts, and its length, metres.
    pub start: f32,
    pub length: f32,
    /// The segment the leg ends on and the stored-direction parameter of its goal; `None` for a missing leg.
    pub goal: Option<(u16, f32)>,
}

impl Leg {
    /// The leg's search found no road: the route has a straight line here.
    pub fn missing(&self) -> bool {
        self.steps.is_empty()
    }
}

/// A route through goals.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Route {
    pub steps: Vec<RouteStep>,
    pub legs: Vec<Leg>,
    /// Metres from the start to the last goal.
    pub length: f32,
}

impl Route {
    /// The leg the distance `distance` along the route falls in (the last one past the end).
    pub fn leg_at(&self, distance: f32) -> usize {
        self.legs.partition_point(|leg| leg.start <= distance).saturating_sub(1)
    }

    /// The point `distance` metres along the route and the unit direction of travel there; `None` for an empty route.
    /// The distance is clamped to the route.
    pub fn point_at(&self, net: &RoadNetwork, distance: f32) -> Option<(Vec3, Vec3)> {
        let distance = distance.clamp(0.0, self.length);
        let leg = self.legs.get(self.leg_at(distance))?;
        if leg.missing() {
            let along = (distance - leg.start) / leg.length.max(1e-3);
            let forward = (leg.to.position - leg.from.position).try_normalize().unwrap_or(leg.to.forward);
            return Some((leg.from.position.lerp(leg.to.position, along.clamp(0.0, 1.0)), forward));
        }
        let pieces = &self.steps[leg.steps.clone()];
        let index = pieces.partition_point(|step| step.start <= distance).saturating_sub(1);
        let step = pieces[index];
        let length = net.segment(step.segment).length.max(1e-3);
        let t = (step.from_t + (distance - step.start) / length).clamp(step.from_t, step.to_t);
        let curve = centre_line(net, step.segment, step.node_ind);
        let forward = curve.tangent(t).try_normalize().unwrap_or(leg.to.forward);
        Some((curve.position(t), forward))
    }

    /// The segments a navigator follows on leg `leg`: that leg's and the next one's, in order, each once where it
    /// repeats (a leg ends on the segment the next begins on). Missing legs add nothing.
    pub fn window(&self, leg: usize) -> Vec<u16> {
        let mut segments: Vec<u16> = Vec::new();
        for leg in self.legs.iter().skip(leg).take(2) {
            for step in &self.steps[leg.steps.clone()] {
                if segments.last() != Some(&step.segment) {
                    segments.push(step.segment);
                }
            }
        }
        segments
    }

    /// The goal at the end of [`Route::window`]: the end of the next leg, else of leg `leg`; `None` when neither has
    /// road.
    pub fn window_goal(&self, leg: usize) -> Option<(u16, f32)> {
        let next = self.legs.get(leg + 1).and_then(|l| l.goal);
        next.or_else(|| self.legs.get(leg).and_then(|l| l.goal))
    }
}
