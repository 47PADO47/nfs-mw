//! The animation of a character thrown to the ground: which clips play while he flies, tumbles and lies, and when he gets up.
//!
//! A physics ragdoll alone is limp. Games that want a convincing fall let the body *follow an animation* with strong drives (a
//! physical animation) while the pelvis is free to carry the physics; the animation is picked by what the ragdoll is doing. This
//! module is that animation, as plain data and a small state machine, free of any game:
//!
//! 1. a **launch** tree, by the kind of fall (a name the caller gives), plays once from the moment he is thrown;
//! 2. when the clip it is dominated by is about to end, the **tumble** tree (flying, sliding, rolling, by how he lies) fades in and
//!    loops;
//! 3. once he is nearly still, on the ground, and lying in one of the **rest** bands (face down, on a side, face up), the matching
//!    get-up tree fades in and plays once.
//!
//! The trees are ordinary [`Graph`]s whose parameters are named [`PARAM_TIME`], [`PARAM_FACE`], [`PARAM_SIDE`] and
//! [`PARAM_FLYING`]; a parameter a tree does not name is never read. [`FallPlayer`] is the state machine; the caller feeds it what the
//! ragdoll is doing ([`FallInput`]) and reads the pose it asks the ragdoll to follow. The data is a [`FallSet`], which a converter
//! writes and the game reads (`docs/formats/ride6-rider.md`, `falls.json`).

mod player;
#[cfg(test)]
mod tests;

pub use player::{FallInput, FallPlayer};

use serde::{Deserialize, Serialize};

use crate::clip::Clip;
use crate::error::{Error, Result};
use crate::graph::Graph;
use crate::skeleton::Skeleton;

/// Seconds since the state the tree belongs to began.
pub const PARAM_TIME: &str = "t";
/// Where the front of the body points, vertically: +1 face up, -1 face down, 0 standing or on a side.
pub const PARAM_FACE: &str = "face";
/// Where the right of the body points, vertically: +1 lying on the left side, -1 on the right, 0 on the back or the front.
pub const PARAM_SIDE: &str = "side";
/// How far he is in the air, 0 on the ground to 1 high ([`FallSet::flying_ramp`]).
pub const PARAM_FLYING: &str = "flying";

/// The opening tree for one kind of fall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Launch {
    /// The kind of fall this tree is for, as the caller names it. A kind with no launch goes straight to the tumble.
    pub kind: String,
    pub graph: Graph,
}

/// A get-up: the tree that plays when he lies still with `face_above < face < face_below`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rest {
    pub face_above: f32,
    pub face_below: f32,
    pub graph: Graph,
    /// Seconds the tumble takes to fade into this tree.
    pub blend: f32,
}

/// Everything the animation of a fall needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FallSet {
    pub launches: Vec<Launch>,
    /// Seconds the tumble takes to fade in over the end of a launch; the launch hands over when this much of its dominant clip is
    /// left (or at once when it is shorter).
    pub launch_blend: f32,
    pub tumble: Graph,
    /// The get-ups, tried in order.
    pub rests: Vec<Rest>,
    /// He is nearly still under this speed (m/s).
    pub low_speed: f32,
    /// He is in the air above this distance from the ground (m): no get-up then.
    pub flying_above: f32,
    /// The distance from the ground (m) at which the `flying` parameter starts to rise and at which it is 1.
    pub flying_ramp: (f32, f32),
    /// The rate (1/s) at which the pose he follows moves from the one he had on the vehicle to the fall's.
    pub fade_rate: f32,
}

impl FallSet {
    /// Checks every tree against the clips and the skeleton, the bands and the times.
    pub fn validate(&self, clips: &[Clip], skeleton: &Skeleton) -> Result<()> {
        let trees = self
            .launches
            .iter()
            .map(|l| &l.graph)
            .chain(std::iter::once(&self.tumble))
            .chain(self.rests.iter().map(|r| &r.graph));
        for graph in trees {
            graph.validate(clips, skeleton)?;
        }
        let times = [self.launch_blend, self.low_speed, self.flying_above, self.fade_rate]
            .into_iter()
            .chain(self.rests.iter().map(|r| r.blend))
            .chain([self.flying_ramp.0, self.flying_ramp.1]);
        if times.into_iter().any(|t| !t.is_finite() || t < 0.0) {
            return Err(Error::BadValue("a time, speed or distance of the fall set is negative"));
        }
        if self
            .rests
            .iter()
            .any(|r| !r.face_above.is_finite() || !r.face_below.is_finite() || r.face_above >= r.face_below)
        {
            return Err(Error::BadValue("a rest band of the fall set is empty"));
        }
        Ok(())
    }

    /// The index of the launch for `kind`.
    #[must_use]
    pub fn launch_for(&self, kind: &str) -> Option<usize> {
        self.launches.iter().position(|l| l.kind == kind)
    }

    /// The `flying` parameter for a distance `airborne` from the ground (m): 0 up to the ramp's start, 1 from its end.
    #[must_use]
    pub fn flying_amount(&self, airborne: f32) -> f32 {
        let (from, to) = self.flying_ramp;
        match to > from {
            true => ((airborne - from) / (to - from)).clamp(0.0, 1.0),
            false => f32::from(airborne > to),
        }
    }
}
