//! Blend trees as plain data. A [`Graph`] names its parameters; the caller fills a slice of floats each frame and
//! asks for the pose.

use serde::{Deserialize, Serialize};

use crate::blend_space::BlendSpace;
use crate::clip::Clip;
use crate::error::{Error, Result};
use crate::pose::Pose;
use crate::skeleton::Skeleton;

/// A number: a constant, or a parameter put through `clamp(param * scale + bias, min, max)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Const(f32),
    Param { index: usize, scale: f32, bias: f32, min: f32, max: f32 },
}

impl Value {
    /// The parameter as it is, unclamped.
    #[must_use]
    pub const fn param(index: usize) -> Self {
        Self::Param { index, scale: 1.0, bias: 0.0, min: f32::MIN, max: f32::MAX }
    }

    /// The parameter mapped linearly so that `from.0` gives `to.0` and `from.1` gives `to.1`, clamped to `to`.
    #[must_use]
    pub fn remap(index: usize, from: (f32, f32), to: (f32, f32)) -> Self {
        let scale = if (from.1 - from.0).abs() < f32::EPSILON { 0.0 } else { (to.1 - to.0) / (from.1 - from.0) };
        let (min, max) = if to.0 <= to.1 { (to.0, to.1) } else { (to.1, to.0) };
        Self::Param { index, scale, bias: to.0 - from.0 * scale, min, max }
    }

    #[must_use]
    pub fn eval(&self, params: &[f32]) -> f32 {
        match *self {
            Self::Const(value) => value,
            Self::Param { index, scale, bias, min, max } => {
                (params.get(index).copied().unwrap_or(0.0) * scale + bias).clamp(min, max)
            }
        }
    }
}

/// How a clip node picks its time.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeMode {
    /// Plays and loops: clip time is `context time * rate`.
    Loop { rate: f32 },
    /// Clip time is the value (0 to 1, clamped) times the clip's length.
    Fraction(Value),
    /// Always the first frame.
    Hold,
    /// Plays once and holds the last frame: clip time is `context time * rate`, clamped to the clip's length.
    Once { rate: f32 },
}

/// One node of a blend tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Node {
    /// A clip, by index into the clip list.
    Clip { clip: usize, time: TimeMode },
    /// `a` at weight 0, `b` at weight 1.
    Blend { a: Box<Node>, b: Box<Node>, weight: Value },
    /// `param` in -1..=1: below 0 blends `zero` towards `negative`, above 0 towards `positive`.
    ThreeWay { negative: Box<Node>, zero: Box<Node>, positive: Box<Node>, param: Value },
    /// `delta` (an additive pose) added to `base`.
    Additive { base: Box<Node>, delta: Box<Node>, weight: Value },
    /// `layer` over `base`, per bone by `mask`, all scaled by `weight`.
    Layer { base: Box<Node>, layer: Box<Node>, mask: Vec<f32>, weight: Value },
    /// A blend space, all its clips at the same phase.
    Space { space: BlendSpace, x: Value, y: Value, time: TimeMode },
    /// One of the options, picked by the value (rounded, clamped).
    Select { options: Vec<Node>, index: Value },
}

/// What a node needs to evaluate.
pub struct EvalContext<'a> {
    pub skeleton: &'a Skeleton,
    pub clips: &'a [Clip],
    pub params: &'a [f32],
    /// Seconds, for [`TimeMode::Loop`].
    pub time: f32,
}

impl Node {
    /// The pose of this node.
    #[must_use]
    pub fn evaluate(&self, ctx: &EvalContext) -> Pose {
        match self {
            Self::Clip { clip, time } => clip_pose(ctx, *clip, *time),
            Self::Blend { a, b, weight } => {
                a.evaluate(ctx).blend(&b.evaluate(ctx), weight.eval(ctx.params).clamp(0.0, 1.0))
            }
            Self::ThreeWay { negative, zero, positive, param } => {
                let p = param.eval(ctx.params).clamp(-1.0, 1.0);
                if p == 0.0 {
                    return zero.evaluate(ctx);
                }
                let side = if p < 0.0 { negative } else { positive };
                zero.evaluate(ctx).blend(&side.evaluate(ctx), p.abs())
            }
            Self::Additive { base, delta, weight } => {
                let mut pose = base.evaluate(ctx);
                pose.add_additive(&delta.evaluate(ctx), weight.eval(ctx.params).clamp(0.0, 1.0));
                pose
            }
            Self::Layer { base, layer, mask, weight } => {
                base.evaluate(ctx).blend_masked(&layer.evaluate(ctx), mask, weight.eval(ctx.params).clamp(0.0, 1.0))
            }
            Self::Space { space, x, y, time } => space_pose(ctx, space, x.eval(ctx.params), y.eval(ctx.params), *time),
            Self::Select { options, index } => {
                let last = options.len().saturating_sub(1);
                let i = (index.eval(ctx.params).round().max(0.0) as usize).min(last);
                match options.get(i) {
                    Some(node) => node.evaluate(ctx),
                    None => ctx.skeleton.rest_pose(),
                }
            }
        }
    }

    /// The clip with the largest share of the pose at these parameters (the first on a tie), or `None` for a tree with no clip. A
    /// state machine that moves on when the clip it plays is about to end watches this one.
    #[must_use]
    pub fn dominant_clip(&self, params: &[f32]) -> Option<usize> {
        let mut shares = Vec::new();
        self.shares(params, 1.0, &mut shares);
        let mut best: Option<(usize, f32)> = None;
        for (clip, share) in shares {
            if best.is_none_or(|(_, kept)| share > kept) {
                best = Some((clip, share));
            }
        }
        best.map(|(clip, _)| clip)
    }

    /// The clips under this node with their shares of the pose, `weight` being the share of the node itself.
    fn shares(&self, params: &[f32], weight: f32, out: &mut Vec<(usize, f32)>) {
        if weight <= 0.0 {
            return;
        }
        match self {
            Self::Clip { clip, .. } => out.push((*clip, weight)),
            Self::Blend { a, b, weight: w } => {
                let k = w.eval(params).clamp(0.0, 1.0);
                a.shares(params, weight * (1.0 - k), out);
                b.shares(params, weight * k, out);
            }
            Self::ThreeWay { negative, zero, positive, param } => {
                let p = param.eval(params).clamp(-1.0, 1.0);
                zero.shares(params, weight * (1.0 - p.abs()), out);
                let side = if p < 0.0 { negative } else { positive };
                side.shares(params, weight * p.abs(), out);
            }
            Self::Additive { base, .. } => base.shares(params, weight, out),
            Self::Layer { base, layer, weight: w, .. } => {
                let k = w.eval(params).clamp(0.0, 1.0);
                base.shares(params, weight * (1.0 - k), out);
                layer.shares(params, weight * k, out);
            }
            Self::Space { space, x, y, .. } => {
                for (i, w) in space.weights(x.eval(params), y.eval(params)) {
                    out.push((space.samples[i].clip, weight * w));
                }
            }
            Self::Select { options, index } => {
                let last = options.len().saturating_sub(1);
                let i = (index.eval(params).round().max(0.0) as usize).min(last);
                if let Some(node) = options.get(i) {
                    node.shares(params, weight, out);
                }
            }
        }
    }

    /// Checks every clip index, parameter index and mask length.
    pub fn validate(&self, clips: usize, bones: usize, params: usize) -> Result<()> {
        let value = |v: &Value| match v {
            Value::Param { index, .. } if *index >= params => Err(Error::BadValue("parameter index out of range")),
            _ => Ok(()),
        };
        let time = |t: &TimeMode| match t {
            TimeMode::Fraction(v) => value(v),
            _ => Ok(()),
        };
        match self {
            Self::Clip { clip, time: t } => {
                time(t)?;
                if *clip >= clips {
                    return Err(Error::BadValue("clip index out of range"));
                }
                Ok(())
            }
            Self::Blend { a, b, weight } => {
                value(weight)?;
                a.validate(clips, bones, params)?;
                b.validate(clips, bones, params)
            }
            Self::ThreeWay { negative, zero, positive, param } => {
                value(param)?;
                for n in [negative, zero, positive] {
                    n.validate(clips, bones, params)?;
                }
                Ok(())
            }
            Self::Additive { base, delta, weight } => {
                value(weight)?;
                base.validate(clips, bones, params)?;
                delta.validate(clips, bones, params)
            }
            Self::Layer { base, layer, mask, weight } => {
                value(weight)?;
                if mask.len() != bones {
                    return Err(Error::CountMismatch { expected: bones, got: mask.len() });
                }
                base.validate(clips, bones, params)?;
                layer.validate(clips, bones, params)
            }
            Self::Space { space, x, y, time: t } => {
                value(x)?;
                value(y)?;
                time(t)?;
                if space.samples.iter().any(|s| s.clip >= clips) {
                    return Err(Error::BadValue("blend space clip index out of range"));
                }
                Ok(())
            }
            Self::Select { options, index } => {
                value(index)?;
                options.iter().try_for_each(|n| n.validate(clips, bones, params))
            }
        }
    }
}

fn clip_pose(ctx: &EvalContext, clip: usize, mode: TimeMode) -> Pose {
    let Some(c) = ctx.clips.get(clip) else { return ctx.skeleton.rest_pose() };
    match mode {
        TimeMode::Loop { rate } => c.sample(ctx.skeleton, ctx.time * rate, true),
        TimeMode::Fraction(v) => c.sample(ctx.skeleton, v.eval(ctx.params).clamp(0.0, 1.0) * c.length, false),
        TimeMode::Hold => c.sample(ctx.skeleton, 0.0, false),
        TimeMode::Once { rate } => c.sample(ctx.skeleton, (ctx.time * rate).clamp(0.0, c.length), false),
    }
}

fn space_pose(ctx: &EvalContext, space: &BlendSpace, x: f32, y: f32, mode: TimeMode) -> Pose {
    let weights = space.weights(x, y);
    let clips: Vec<(&Clip, f32)> =
        weights.iter().filter_map(|&(i, w)| ctx.clips.get(space.samples[i].clip).map(|c| (c, w))).collect();
    if clips.is_empty() {
        return ctx.skeleton.rest_pose();
    }
    let mean_length: f32 = clips.iter().map(|(c, w)| c.length * w).sum();
    let phase = match mode {
        TimeMode::Loop { rate } if mean_length > 0.0 => (ctx.time * rate / mean_length).rem_euclid(1.0),
        TimeMode::Once { rate } if mean_length > 0.0 => (ctx.time * rate / mean_length).clamp(0.0, 1.0),
        TimeMode::Loop { .. } | TimeMode::Once { .. } | TimeMode::Hold => 0.0,
        TimeMode::Fraction(v) => v.eval(ctx.params).clamp(0.0, 1.0),
    };
    let mut accumulated = 0.0;
    let mut pose: Option<Pose> = None;
    for (clip, weight) in clips {
        let next = clip.sample(ctx.skeleton, phase * clip.length, false);
        accumulated += weight;
        pose = Some(match pose {
            Some(p) => p.blend(&next, weight / accumulated),
            None => next,
        });
    }
    pose.unwrap_or_else(|| ctx.skeleton.rest_pose())
}

/// A tree and the names of its parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub param_names: Vec<String>,
    pub root: Node,
}

impl Graph {
    /// The index of a parameter, for filling the slice passed to [`Graph::evaluate`].
    #[must_use]
    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.param_names.iter().position(|n| n == name)
    }

    /// A parameter slice with every value 0.
    #[must_use]
    pub fn new_params(&self) -> Vec<f32> {
        vec![0.0; self.param_names.len()]
    }

    /// Checks the tree against the clip list and the skeleton.
    pub fn validate(&self, clips: &[Clip], skeleton: &Skeleton) -> Result<()> {
        self.root.validate(clips.len(), skeleton.len(), self.param_names.len())
    }

    #[must_use]
    pub fn evaluate(&self, skeleton: &Skeleton, clips: &[Clip], params: &[f32], time: f32) -> Pose {
        self.root.evaluate(&EvalContext { skeleton, clips, params, time })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blend_space::{Axis, SpaceSample};
    use crate::clip::{Channel, Track};
    use crate::skeleton::tests::arm;
    use glam::Vec3;

    /// A clip that puts bone 1 at x = `x` for its whole length.
    fn at_x(x: f32, length: f32) -> Clip {
        Clip {
            name: format!("x{x}"),
            length,
            tracks: vec![Track {
                bone: 1,
                rotation: Channel::None,
                translation: Channel::Uniform(vec![Vec3::new(x, 1.0, 0.0), Vec3::new(x, 1.0, 0.0)]),
                scale: Channel::None,
            }],
            additive: false,
            root_motion: false,
            notifies: Vec::new(),
        }
    }

    fn clip_node(clip: usize) -> Box<Node> {
        Box::new(Node::Clip { clip, time: TimeMode::Hold })
    }

    fn x_of(pose: &Pose) -> f32 {
        pose.local[1].translation.x
    }

    #[test]
    fn value_remap_maps_and_clamps() {
        let v = Value::remap(0, (120.0, 150.0), (0.0, 1.0));
        assert!((v.eval(&[135.0]) - 0.5).abs() < 1e-6);
        assert_eq!(v.eval(&[0.0]), 0.0);
        assert_eq!(v.eval(&[500.0]), 1.0);
        assert_eq!(Value::param(3).eval(&[1.0]), 0.0);
    }

    #[test]
    fn three_way_blends_towards_the_signed_side() {
        let skel = arm();
        let clips = vec![at_x(-2.0, 1.0), at_x(0.0, 1.0), at_x(4.0, 1.0)];
        let graph = Graph {
            param_names: vec!["lean".into()],
            root: Node::ThreeWay {
                negative: clip_node(0),
                zero: clip_node(1),
                positive: clip_node(2),
                param: Value::param(0),
            },
        };
        graph.validate(&clips, &skel).unwrap();
        let at = |lean: f32| x_of(&graph.evaluate(&skel, &clips, &[lean], 0.0));
        assert!((at(0.0)).abs() < 1e-6);
        assert!((at(0.5) - 2.0).abs() < 1e-5);
        assert!((at(-1.0) + 2.0).abs() < 1e-5);
        assert!((at(7.0) - 4.0).abs() < 1e-5);
    }

    #[test]
    fn layer_additive_and_select() {
        let skel = arm();
        let clips = vec![at_x(1.0, 1.0), at_x(3.0, 1.0)];
        let layered = Node::Layer {
            base: clip_node(0),
            layer: clip_node(1),
            mask: vec![0.0, 0.5, 0.0, 0.0],
            weight: Value::Const(1.0),
        };
        let ctx = EvalContext { skeleton: &skel, clips: &clips, params: &[], time: 0.0 };
        assert!((x_of(&layered.evaluate(&ctx)) - 2.0).abs() < 1e-5);
        let select = Node::Select { options: vec![*clip_node(0), *clip_node(1)], index: Value::Const(9.0) };
        assert!((x_of(&select.evaluate(&ctx)) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn blend_space_mixes_at_a_shared_phase() {
        let skel = arm();
        let clips = vec![at_x(0.0, 2.0), at_x(10.0, 4.0)];
        let node = Node::Space {
            space: BlendSpace {
                x: Axis { min: 0.0, max: 1.0, divisions: 1 },
                y: Axis { min: 0.0, max: 0.0, divisions: 0 },
                samples: vec![SpaceSample { x: 0, y: 0, clip: 0 }, SpaceSample { x: 1, y: 0, clip: 1 }],
            },
            x: Value::param(0),
            y: Value::Const(0.0),
            time: TimeMode::Hold,
        };
        let ctx = EvalContext { skeleton: &skel, clips: &clips, params: &[0.25], time: 0.0 };
        assert!((x_of(&node.evaluate(&ctx)) - 2.5).abs() < 1e-5);
    }

    #[test]
    fn validate_catches_bad_indices_and_graph_serialises() {
        let skel = arm();
        let clips = vec![at_x(0.0, 1.0)];
        let bad = Graph { param_names: vec![], root: *clip_node(5) };
        assert!(bad.validate(&clips, &skel).is_err());
        let bad_param = Graph {
            param_names: vec![],
            root: Node::Blend { a: clip_node(0), b: clip_node(0), weight: Value::param(2) },
        };
        assert!(bad_param.validate(&clips, &skel).is_err());
        let good = Graph {
            param_names: vec!["a".into()],
            root: Node::Blend { a: clip_node(0), b: clip_node(0), weight: Value::param(0) },
        };
        let text = serde_json::to_string(&good).unwrap();
        let back: Graph = serde_json::from_str(&text).unwrap();
        assert_eq!(back, good);
        assert_eq!(good.param_index("a"), Some(0));
    }
}
