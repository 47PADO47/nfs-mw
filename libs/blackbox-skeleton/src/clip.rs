//! Animation clips: per-bone channels of rotation, translation and scale over time.

use glam::{Quat, Vec3};

use crate::pose::Pose;
use crate::skeleton::Skeleton;
use crate::transform::Transform;

/// A value that can sit between two keys.
pub trait Interpolate: Copy {
    fn interpolate(a: Self, b: Self, weight: f32) -> Self;
}

impl Interpolate for Quat {
    fn interpolate(a: Self, b: Self, weight: f32) -> Self {
        a.slerp(b, weight).normalize()
    }
}

impl Interpolate for Vec3 {
    fn interpolate(a: Self, b: Self, weight: f32) -> Self {
        a.lerp(b, weight)
    }
}

/// One property of one bone over time.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel<T> {
    /// The clip does not animate the property: the pose keeps what it had.
    None,
    /// The same value at every time.
    Constant(T),
    /// Values at evenly spaced times: the first at 0, the last at the clip's length.
    Uniform(Vec<T>),
    /// Values at the given times, in increasing order. Before the first key and after the last the value is held.
    Keyed { times: Vec<f32>, values: Vec<T> },
}

impl<T: Interpolate> Channel<T> {
    /// The value at `time` (seconds, within `0..=length`), or `None` when the channel is empty.
    #[must_use]
    pub fn sample(&self, time: f32, length: f32) -> Option<T> {
        match self {
            Self::None => None,
            Self::Constant(value) => Some(*value),
            Self::Uniform(values) => sample_uniform(values, time, length),
            Self::Keyed { times, values } => sample_keyed(times, values, time),
        }
    }

    /// The number of keys stored.
    #[must_use]
    pub fn key_count(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Constant(_) => 1,
            Self::Uniform(values) | Self::Keyed { values, .. } => values.len(),
        }
    }
}

fn sample_uniform<T: Interpolate>(values: &[T], time: f32, length: f32) -> Option<T> {
    let last = values.len().checked_sub(1)?;
    if last == 0 || length <= 0.0 {
        return Some(values[0]);
    }
    let position = (time / length).clamp(0.0, 1.0) * last as f32;
    let index = (position.floor() as usize).min(last - 1);
    Some(T::interpolate(values[index], values[index + 1], position - index as f32))
}

fn sample_keyed<T: Interpolate>(times: &[f32], values: &[T], time: f32) -> Option<T> {
    let count = times.len().min(values.len());
    if count == 0 {
        return None;
    }
    let next = times[..count].partition_point(|&t| t <= time);
    if next == 0 {
        return Some(values[0]);
    }
    if next == count {
        return Some(values[count - 1]);
    }
    let (t0, t1) = (times[next - 1], times[next]);
    let weight = if t1 > t0 { (time - t0) / (t1 - t0) } else { 0.0 };
    Some(T::interpolate(values[next - 1], values[next], weight))
}

/// The three channels of one bone.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// Index into the skeleton.
    pub bone: usize,
    pub rotation: Channel<Quat>,
    pub translation: Channel<Vec3>,
    pub scale: Channel<Vec3>,
}

/// A named event at a time of the clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Notify {
    pub name: String,
    pub time: f32,
}

/// A clip: tracks for some bones, a length, and flags.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub name: String,
    /// Seconds.
    pub length: f32,
    pub tracks: Vec<Track>,
    /// The tracks are differences to add to another pose, not poses (see [`Pose::add_additive`]).
    pub additive: bool,
    /// The source asked for the root bone's motion to move the character. Informational: this crate does not.
    pub root_motion: bool,
    pub notifies: Vec<Notify>,
}

impl Clip {
    /// Maps a time to a position inside the clip: wraps when `looping`, otherwise clamps.
    #[must_use]
    pub fn wrap_time(&self, time: f32, looping: bool) -> f32 {
        if self.length <= 0.0 {
            return 0.0;
        }
        if looping {
            return time.rem_euclid(self.length);
        }
        time.clamp(0.0, self.length)
    }

    /// Overwrites the properties the clip animates in `pose`; the rest is left alone.
    pub fn apply(&self, time: f32, looping: bool, pose: &mut Pose) {
        let time = self.wrap_time(time, looping);
        for track in &self.tracks {
            let Some(local) = pose.local.get_mut(track.bone) else { continue };
            if let Some(rotation) = track.rotation.sample(time, self.length) {
                local.rotation = rotation;
            }
            if let Some(translation) = track.translation.sample(time, self.length) {
                local.translation = translation;
            }
            if let Some(scale) = track.scale.sample(time, self.length) {
                local.scale = scale;
            }
        }
    }

    /// The pose at `time`: bones without a track stay at rest (or at identity for an additive clip).
    #[must_use]
    pub fn sample(&self, skeleton: &Skeleton, time: f32, looping: bool) -> Pose {
        let mut pose = if self.additive { Pose::identity(skeleton.len()) } else { skeleton.rest_pose() };
        self.apply(time, looping, &mut pose);
        pose
    }

    /// The notifies passed when the clip plays from `from` to `to` (seconds of clip time, `to` included, `from`
    /// not). When `to < from` the clip looped and both ends of it count.
    #[must_use]
    pub fn notifies_between(&self, from: f32, to: f32) -> Vec<&Notify> {
        let hit = |n: &&Notify, lo: f32, hi: f32| n.time > lo && n.time <= hi;
        if to >= from {
            return self.notifies.iter().filter(|n| hit(n, from, to)).collect();
        }
        let tail = self.notifies.iter().filter(|n| hit(n, from, self.length));
        let head = self.notifies.iter().filter(|n| n.time >= 0.0 && n.time <= to);
        tail.chain(head).collect()
    }

    /// Converts a clip of poses into an additive clip: every channel becomes its difference to `base`, sampled at
    /// `frames` even steps. Bones that never differ get no track.
    #[must_use]
    pub fn to_additive(&self, skeleton: &Skeleton, base: &Pose, frames: usize) -> Self {
        let frames = frames.max(2);
        let step = self.length / (frames - 1) as f32;
        let poses: Vec<Pose> = (0..frames).map(|f| self.sample(skeleton, step * f as f32, false)).collect();
        let mut tracks = Vec::new();
        for (bone, b) in base.local.iter().enumerate() {
            let deltas: Vec<Transform> = poses.iter().map(|p| delta(p.local[bone], *b)).collect();
            if deltas.iter().all(is_identity) {
                continue;
            }
            tracks.push(Track {
                bone,
                rotation: Channel::Uniform(deltas.iter().map(|d| d.rotation).collect()),
                translation: Channel::Uniform(deltas.iter().map(|d| d.translation).collect()),
                scale: Channel::Uniform(deltas.iter().map(|d| d.scale).collect()),
            });
        }
        Self { tracks, additive: true, ..self.clone() }
    }
}

fn delta(pose: Transform, base: Transform) -> Transform {
    let safe = |v: f32| if v.abs() < f32::EPSILON { 1.0 } else { v };
    Transform {
        rotation: (pose.rotation * base.rotation.inverse()).normalize(),
        translation: pose.translation - base.translation,
        scale: Vec3::new(
            pose.scale.x / safe(base.scale.x),
            pose.scale.y / safe(base.scale.y),
            pose.scale.z / safe(base.scale.z),
        ),
    }
}

fn is_identity(t: &Transform) -> bool {
    t.rotation.angle_between(Quat::IDENTITY) < 1e-5
        && t.translation.length() < 1e-6
        && (t.scale - Vec3::ONE).length() < 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::tests::arm;

    fn clip_with(rotation: Channel<Quat>, translation: Channel<Vec3>) -> Clip {
        Clip {
            name: "c".into(),
            length: 2.0,
            tracks: vec![Track { bone: 1, rotation, translation, scale: Channel::None }],
            additive: false,
            root_motion: false,
            notifies: vec![Notify { name: "a".into(), time: 0.5 }, Notify { name: "b".into(), time: 1.5 }],
        }
    }

    #[test]
    fn uniform_channel_interpolates_between_evenly_spaced_keys() {
        let ch = Channel::Uniform(vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 0.0)]);
        assert!(ch.sample(1.0, 2.0).unwrap().distance(Vec3::new(2.0, 0.0, 0.0)) < 1e-6);
        assert!(ch.sample(0.5, 2.0).unwrap().distance(Vec3::new(1.0, 0.0, 0.0)) < 1e-6);
        assert!(ch.sample(9.0, 2.0).unwrap().distance(Vec3::new(2.0, 4.0, 0.0)) < 1e-6);
        assert_eq!(Channel::<Vec3>::None.sample(0.0, 1.0), None);
    }

    #[test]
    fn keyed_channel_holds_ends_and_interpolates() {
        let ch = Channel::Keyed { times: vec![0.5, 1.5], values: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)] };
        assert_eq!(ch.sample(0.0, 2.0), Some(Vec3::ZERO));
        assert_eq!(ch.sample(2.0, 2.0), Some(Vec3::new(1.0, 0.0, 0.0)));
        assert!((ch.sample(1.0, 2.0).unwrap().x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sample_leaves_untracked_bones_at_rest() {
        let skel = arm();
        let clip = clip_with(Channel::Constant(Quat::from_rotation_z(0.3)), Channel::None);
        let pose = clip.sample(&skel, 0.7, true);
        assert!(pose.local[1].rotation.angle_between(Quat::from_rotation_z(0.3)) < 1e-5);
        assert_eq!(pose.local[2], skel.bones()[2].rest);
        assert_eq!(pose.local[1].translation, skel.bones()[1].rest.translation);
    }

    #[test]
    fn time_wraps_or_clamps() {
        let clip = clip_with(Channel::None, Channel::None);
        assert!((clip.wrap_time(2.5, true) - 0.5).abs() < 1e-6);
        assert!((clip.wrap_time(-0.5, true) - 1.5).abs() < 1e-6);
        assert!((clip.wrap_time(2.5, false) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn notifies_fire_once_across_a_loop() {
        let clip = clip_with(Channel::None, Channel::None);
        let names = |v: Vec<&Notify>| v.iter().map(|n| n.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(clip.notifies_between(0.0, 1.0)), vec!["a"]);
        assert_eq!(names(clip.notifies_between(1.0, 2.0)), vec!["b"]);
        assert_eq!(names(clip.notifies_between(1.9, 0.6)), vec!["a"]);
        assert_eq!(names(clip.notifies_between(1.2, 0.6)), vec!["b", "a"]);
    }

    #[test]
    fn additive_conversion_round_trips() {
        let skel = arm();
        let clip = clip_with(
            Channel::Uniform(vec![Quat::IDENTITY, Quat::from_rotation_z(0.8)]),
            Channel::Uniform(vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 0.0)]),
        );
        let base = clip.sample(&skel, 0.0, false);
        let add = clip.to_additive(&skel, &base, 5);
        assert!(add.additive);
        assert_eq!(add.tracks.len(), 1);
        let mut rebuilt = base.clone();
        rebuilt.add_additive(&add.sample(&skel, 2.0, false), 1.0);
        let want = clip.sample(&skel, 2.0, false);
        assert!(rebuilt.local[1].rotation.angle_between(want.local[1].rotation) < 1e-4);
        assert!(rebuilt.local[1].translation.distance(want.local[1].translation) < 1e-5);
    }
}
