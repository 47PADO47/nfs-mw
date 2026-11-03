//! The state machine of a fall: launch, tumble, rest (see the module above).

use crate::clip::Clip;
use crate::fall::{FallSet, PARAM_FACE, PARAM_FLYING, PARAM_SIDE, PARAM_TIME};
use crate::graph::Graph;
use crate::pose::Pose;
use crate::skeleton::Skeleton;

/// What the ragdoll is doing, which is what picks the clips.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FallInput {
    /// Where the front of the body points, vertically (+1 up, -1 down).
    pub face: f32,
    /// Where the right of the body points, vertically.
    pub side: f32,
    /// The speed of the pelvis (m/s).
    pub speed: f32,
    /// The distance from the pelvis to the ground (m).
    pub airborne: f32,
}

/// A state whose tree is still being evaluated while another fades over it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Launch { index: usize, time: f32 },
    Tumble { time: f32 },
}

impl Source {
    fn advance(&mut self, dt: f32) {
        match self {
            Self::Launch { time, .. } | Self::Tumble { time } => *time += dt,
        }
    }
}

/// A crossfade from `source` to the state the fade belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fade {
    source: Source,
    elapsed: f32,
    duration: f32,
}

impl Fade {
    fn weight(&self) -> f32 {
        let k = match self.duration > 0.0 {
            true => (self.elapsed / self.duration).clamp(0.0, 1.0),
            false => 1.0,
        };
        k * k * (3.0 - 2.0 * k)
    }

    fn done(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Launch { index: usize, time: f32 },
    Tumble { time: f32, from: Option<Fade> },
    Rest { index: usize, time: f32, from: Fade },
}

/// A fall in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct FallPlayer {
    phase: Phase,
    /// Seconds since he was thrown.
    age: f32,
    input: FallInput,
}

/// The parameter slice of `graph` for a state `time` seconds old.
fn params(graph: &Graph, set: &FallSet, input: &FallInput, time: f32) -> Vec<f32> {
    graph
        .param_names
        .iter()
        .map(|name| match name.as_str() {
            PARAM_TIME => time,
            PARAM_FACE => input.face,
            PARAM_SIDE => input.side,
            PARAM_FLYING => set.flying_amount(input.airborne),
            _ => 0.0,
        })
        .collect()
}

/// The length of the clip that dominates `graph` now (s); 0 for a tree with no clip.
fn dominant_length(graph: &Graph, set: &FallSet, input: &FallInput, clips: &[Clip]) -> f32 {
    let values = params(graph, set, input, 0.0);
    graph.root.dominant_clip(&values).and_then(|clip| clips.get(clip)).map_or(0.0, |clip| clip.length)
}

impl FallPlayer {
    /// A fall of `kind` (the name a [`crate::fall::Launch`] carries) that begins now. A kind with no launch starts in the tumble.
    #[must_use]
    pub fn new(set: &FallSet, kind: &str) -> Self {
        let phase = match set.launch_for(kind) {
            Some(index) => Phase::Launch { index, time: 0.0 },
            None => Phase::Tumble { time: 0.0, from: None },
        };
        Self { phase, age: 0.0, input: FallInput::default() }
    }

    /// Seconds since he was thrown.
    #[must_use]
    pub fn age(&self) -> f32 {
        self.age
    }

    /// Whether he is in a get-up.
    #[must_use]
    pub fn resting(&self) -> bool {
        matches!(self.phase, Phase::Rest { .. })
    }

    /// The index of the get-up he is in (`FallSet::rests`).
    #[must_use]
    pub fn rest_index(&self) -> Option<usize> {
        match self.phase {
            Phase::Rest { index, .. } => Some(index),
            _ => None,
        }
    }

    /// Advance by `dt` seconds with the ragdoll doing `input`. From the moment he is in a get-up the input is no longer read: the
    /// body is no longer simulated.
    pub fn step(&mut self, set: &FallSet, clips: &[Clip], dt: f32, input: FallInput) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.age += dt;
        if !self.resting() {
            self.input = input;
        }
        let input = self.input;
        self.phase = match self.phase {
            Phase::Launch { index, time } => {
                let time = time + dt;
                let graph = &set.launches[index].graph;
                let left = dominant_length(graph, set, &input, clips) - time;
                match left <= set.launch_blend {
                    true => Phase::Tumble {
                        time: 0.0,
                        from: Some(Fade {
                            source: Source::Launch { index, time },
                            elapsed: 0.0,
                            duration: set.launch_blend,
                        }),
                    },
                    false => Phase::Launch { index, time },
                }
            }
            Phase::Tumble { time, from } => {
                let time = time + dt;
                let from = from.and_then(|fade| Self::advance_fade(fade, dt));
                match Self::rest_for(set, &input) {
                    Some(index) => Phase::Rest {
                        index,
                        time: 0.0,
                        from: Fade { source: Source::Tumble { time }, elapsed: 0.0, duration: set.rests[index].blend },
                    },
                    None => Phase::Tumble { time, from },
                }
            }
            Phase::Rest { index, time, from } => {
                let from = Self::advance_fade(from, dt).unwrap_or(Fade { elapsed: from.duration, ..from });
                Phase::Rest { index, time: time + dt, from }
            }
        };
    }

    /// The fade moved on by `dt`; `None` once it is over.
    fn advance_fade(mut fade: Fade, dt: f32) -> Option<Fade> {
        fade.elapsed += dt;
        fade.source.advance(dt);
        (!fade.done()).then_some(fade)
    }

    /// The first get-up whose band he lies in, if he is still enough and on the ground.
    fn rest_for(set: &FallSet, input: &FallInput) -> Option<usize> {
        if input.speed >= set.low_speed || input.airborne > set.flying_above {
            return None;
        }
        set.rests.iter().position(|rest| input.face > rest.face_above && input.face < rest.face_below)
    }

    /// The weight of the fall's pose over the pose he had before it (on the vehicle): 0 as he is thrown, towards 1 at the
    /// set's rate.
    #[must_use]
    pub fn fade(&self, set: &FallSet) -> f32 {
        1.0 - (-set.fade_rate * self.age).exp()
    }

    /// Whether the get-up has played to its end (false while he is not in one).
    #[must_use]
    pub fn finished(&self, set: &FallSet, clips: &[Clip]) -> bool {
        let Phase::Rest { index, time, .. } = self.phase else { return false };
        time >= dominant_length(&set.rests[index].graph, set, &self.input, clips)
    }

    /// The pose of the fall now (local transforms).
    #[must_use]
    pub fn pose(&self, set: &FallSet, skeleton: &Skeleton, clips: &[Clip]) -> Pose {
        let tree =
            |graph: &Graph, time: f32| graph.evaluate(skeleton, clips, &params(graph, set, &self.input, time), time);
        let source = |source: Source| match source {
            Source::Launch { index, time } => tree(&set.launches[index].graph, time),
            Source::Tumble { time } => tree(&set.tumble, time),
        };
        match self.phase {
            Phase::Launch { index, time } => tree(&set.launches[index].graph, time),
            Phase::Tumble { time, from } => {
                let now = tree(&set.tumble, time);
                match from {
                    Some(fade) => source(fade.source).blend(&now, fade.weight()),
                    None => now,
                }
            }
            Phase::Rest { index, time, from } => {
                let now = tree(&set.rests[index].graph, time);
                match from.done() {
                    true => now,
                    false => source(from.source).blend(&now, from.weight()),
                }
            }
        }
    }
}
