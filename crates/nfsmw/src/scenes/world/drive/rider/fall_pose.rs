//! Keeping the fallen rider in one piece (`docs/specs/ride6-rider-fall.md`, "One piece").
//!
//! Two things drew him torn: the helmet and the head left the neck, and the helmet shifted against the head while he got back on.
//! Both are the same fault, bones placed one by one: a bone with a body took the body's place whatever its parent did (the joints are
//! soft, a hard landing stretches them by centimetres, and the bones between two bodies, the neck, follow the body above them), and
//! the way back blended every bone's world place on its own (two bones turning through different angles come out nearer or
//! farther than their length). Here a bone's place is always its parent's frame composed with the bone's own offset, so a bone's
//! distance from its parent is its length in every frame; only the *turn* of a simulated bone is the body's.

use blackbox_skeleton::{Skeleton, Transform};
use glam::{Quat, Vec3};

/// A simulated bone drawn joined to its parent: the body's turn, at the place the parent's frame gives it (`parent` is the parent's
/// component transform as drawn, `local` the bone's own offset in it). A bone with no parent is where its body is.
pub(super) fn hung(body: Transform, parent: Option<Transform>, local: Transform) -> Transform {
    let Some(parent) = parent else { return body };
    let joined = parent.compose(local);
    if !joined.translation.is_finite() {
        return body;
    }
    Transform { translation: joined.translation, ..body }
}

/// The pose on the way back onto the bike: the pose he lay in (`from`, the render space, absolute) blended with the riding pose
/// (`riding`, rider space) put on the bike (`turn` and `seat`: the rider space in the render space), `k` of the way (0 to 1), as local
/// transforms: every bone keeps its length and the roots (the bones with no parent) are blended in the render space. Returns the
/// component transforms relative to `seat`.
pub(super) fn blend_back(
    skeleton: &Skeleton,
    from: &[Transform],
    riding: &[Transform],
    (turn, seat): (Quat, Vec3),
    k: f32,
) -> Vec<Transform> {
    let onto = |b: &Transform| Transform { translation: turn * b.translation + seat, ..Transform::from_rt(turn * b.rotation, Vec3::ZERO) };
    let mut world: Vec<Transform> = Vec::with_capacity(skeleton.len());
    for (i, bone) in skeleton.bones().iter().enumerate() {
        let (Some(a), Some(b)) = (from.get(i), riding.get(i)) else {
            world.push(Transform::IDENTITY);
            continue;
        };
        let b = onto(b);
        let Some(parent) = bone.parent else {
            world.push(a.lerp(b, k));
            continue;
        };
        let (pa, pb) = (from.get(parent), riding.get(parent).map(onto));
        let (Some(pa), Some(pb)) = (pa, pb) else {
            world.push(a.lerp(b, k));
            continue;
        };
        let local = pa.inverse().compose(*a).lerp(pb.inverse().compose(b), k);
        world.push(world[parent].compose(local));
    }
    world.into_iter().map(|t| Transform { translation: t.translation - seat, ..t }).collect()
}
