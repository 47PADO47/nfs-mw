//! A wing that several animations move: a machine over the car's speed and brake that plays five keyed animations, one at
//! a time or cross-faded (`docs/formats/chameleon-active-aero.md`, "Kind 1").
//!
//! The machine is the one of the 2012 game's brake wing, with its numbers in the manifest (`[wing]`): the wing opens above
//! `open_mph` (animation 0), the brake opens it at rest above `brake_mph` (animation 1), above `top_mph` it opens further
//! (animation 4), and braking while it is out cross-fades to the brake poses (animations 2 and 3). The brake counts once it
//! has been held `brake_hold` seconds. Every animation plays at a constant rate between its first and last key, forwards or
//! backwards; a part's pose is the blend of the keys of the animations that are playing ([`Blend`]).

use std::collections::BTreeSet;

use glam::{Mat4, Quat, Vec3};

/// Animations of a wing.
pub const SLOTS: usize = 5;
/// The first number of the wing marks, above the zone-level and deployment marks.
const WING_BASE: usize = 16384;
/// Bits of a clock fraction and of a weight in a mark.
const BITS: u32 = 8;
const LEVELS: f32 = ((1 << BITS) - 1) as f32;

/// The numbers of a wing machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Wing {
    pub open_mph: f32,
    pub brake_mph: f32,
    pub top_mph: Option<f32>,
    /// How long the brake has to be held before it counts, seconds.
    pub brake_hold: f32,
    /// How long a cross-fade takes, seconds.
    pub fade: f32,
    /// The length of each animation, seconds.
    pub seconds: [f32; SLOTS],
}

/// The keys of one animation for one part: absolute poses (a slide and a turn) at keys spaced evenly over the animation.
#[derive(Debug, Clone, PartialEq)]
pub struct WingTrack {
    pub slot: usize,
    pub translation: Vec<[f32; 3]>,
    pub rotation: Vec<[f32; 4]>,
}

/// A pose of a part: its slide and its turn about its pivot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Sample {
    pub const REST: Self = Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY };

    /// This pose of a part with `pivot`: turned about the pivot, then slid.
    pub fn matrix(&self, pivot: Vec3) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_translation(pivot)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_translation(-pivot)
    }

    /// From this pose toward `other` by `weight` (0 to 1).
    pub fn toward(&self, other: &Self, weight: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, weight),
            rotation: self.rotation.slerp(other.rotation, weight),
        }
    }
}

impl WingTrack {
    /// The pose at `fraction` (0 to 1) of the animation: the keys are spread evenly and the pose is between the two around.
    pub fn sample(&self, fraction: f32) -> Sample {
        let keys = self.translation.len().min(self.rotation.len());
        if keys == 0 {
            return Sample::REST;
        }
        let at = fraction.clamp(0.0, 1.0) * (keys - 1) as f32;
        let low = (at.floor() as usize).min(keys - 1);
        let high = (low + 1).min(keys - 1);
        let along = at - low as f32;
        let key = |n: usize| Sample {
            translation: Vec3::from_array(self.translation[n]),
            rotation: Quat::from_array(self.rotation[n]).normalize(),
        };
        key(low).toward(&key(high), along)
    }
}

/// What the machine plays: an animation at a clock fraction, and perhaps a second one blended over it by a weight.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blend {
    pub first: Option<(usize, f32)>,
    pub second: Option<(usize, f32, f32)>,
}

impl Blend {
    /// The pose of a part whose animations are `tracks`: the first animation's key pose (rest where the part has no track in
    /// it), moved toward the second's by its weight where the part has a track in that.
    pub fn pose(&self, tracks: &[WingTrack]) -> Sample {
        let track = |slot: usize| tracks.iter().find(|track| track.slot == slot);
        let mut pose = self
            .first
            .and_then(|(slot, fraction)| track(slot).map(|track| track.sample(fraction)))
            .unwrap_or(Sample::REST);
        if let Some((slot, fraction, weight)) = self.second {
            if let Some(track) = track(slot) {
                pose = pose.toward(&track.sample(fraction), weight);
            }
        }
        pose
    }

    /// The mark that carries the blend in the set of revealed slots. Fractions and weight are told to 1/255.
    pub fn mark(&self) -> Option<usize> {
        let (slot, fraction) = self.first?;
        let level = |value: f32| (value.clamp(0.0, 1.0) * LEVELS).round() as usize;
        let (other, other_fraction, weight) = self.second.unwrap_or((0, 0.0, 0.0));
        let packed = (slot << (3 * BITS + 4))
            | (level(fraction) << (2 * BITS + 4))
            | (usize::from(self.second.is_some()) << (2 * BITS + 3))
            | (other << (2 * BITS))
            | (level(other_fraction) << BITS)
            | level(weight);
        Some(WING_BASE + packed)
    }

    /// The blend the marks carry, `None` when there is none.
    pub fn from_marks(marks: &BTreeSet<usize>) -> Option<Self> {
        let packed = marks.range(WING_BASE..).next()? - WING_BASE;
        let level = |shift: u32| ((packed >> shift) & ((1 << BITS) - 1)) as f32 / LEVELS;
        let first = (packed >> (3 * BITS + 4), level(2 * BITS + 4));
        let second = (packed >> (2 * BITS + 3)) & 1 == 1;
        let other = ((packed >> (2 * BITS)) & 7, level(BITS), level(0));
        Some(Self { first: Some(first), second: second.then_some(other) })
    }
}

/// Where the machine is: its state and the clock of each animation, seconds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WingState {
    state: u8,
    clock: [f32; SLOTS],
    held: f32,
}

impl WingState {
    /// The state number, 0 at rest.
    pub fn state(&self) -> u8 {
        self.state
    }

    /// Advance by `dt` seconds with the car at `mph` and the brake at `brake` (0 to 1), and say what to play.
    pub fn step(&mut self, wing: &Wing, mph: f32, brake: f32, dt: f32) -> Blend {
        let dt = dt.max(0.0);
        self.held = match brake > 0.0 {
            true => self.held + dt,
            false => 0.0,
        };
        let braking = brake > 0.0 && self.held >= wing.brake_hold;
        let len = wing.seconds;
        let (open, top, brake_mph) = (wing.open_mph, wing.top_mph.unwrap_or(f32::INFINITY), wing.brake_mph);
        let forward = |clock: &mut f32, slot: usize| {
            *clock = (*clock + dt).min(len[slot]);
        };
        let backward = |clock: &mut f32| {
            *clock = (*clock - dt).max(0.0);
        };
        let fraction = |clock: f32, slot: usize| clock / len[slot].max(1e-6);
        let fade = |clock: f32| (clock / wing.fade.max(1e-6)).min(1.0);
        let one = |slot: usize, clock: f32| Blend { first: Some((slot, fraction(clock, slot))), second: None };
        let two = |a: usize, ca: f32, b: usize, cb: f32, weight: f32| Blend {
            first: Some((a, fraction(ca, a))),
            second: Some((b, fraction(cb, b), weight)),
        };
        let c = &mut self.clock;
        match self.state {
            0 => {
                c[0] = 0.0;
                if mph > open {
                    self.state = 1;
                } else if mph > brake_mph && braking {
                    self.state = 4;
                }
                one(0, 0.0)
            }
            1 | 2 => {
                forward(&mut c[0], 0);
                if braking {
                    self.state = 9;
                } else if mph >= open {
                    if self.state == 1 && c[0] >= len[0] {
                        self.state = 2;
                    } else if self.state == 2 && mph > top {
                        self.state = 6;
                    }
                } else {
                    self.state = 3;
                }
                one(0, c[0])
            }
            3 => {
                backward(&mut c[0]);
                if c[0] <= 0.0 {
                    self.state = 0;
                }
                one(0, c[0])
            }
            4 => {
                forward(&mut c[1], 1);
                if !braking {
                    self.state = 5;
                }
                one(1, c[1])
            }
            5 => {
                backward(&mut c[1]);
                if c[1] <= 0.0 {
                    self.state = 0;
                }
                one(1, c[1])
            }
            6 => {
                forward(&mut c[4], 4);
                if braking {
                    self.state = 11;
                } else if mph >= top {
                    if c[4] >= len[4] {
                        self.state = 7;
                    }
                } else {
                    self.state = 8;
                }
                one(4, c[4])
            }
            7 => {
                if braking {
                    self.state = 11;
                } else if mph < top {
                    self.state = 8;
                }
                one(4, c[4])
            }
            8 => {
                backward(&mut c[4]);
                if c[4] <= 0.0 {
                    self.state = 2;
                } else if mph > top {
                    self.state = 6;
                }
                one(4, c[4])
            }
            9 => {
                forward(&mut c[2], 2);
                forward(&mut c[0], 0);
                let blend = two(2, c[2], 0, c[0], 1.0 - fade(c[2]));
                if !braking {
                    if mph > open {
                        self.state = 10;
                    } else if c[2] >= len[2] {
                        self.state = 5;
                        c[1] = len[1];
                        c[2] = 0.0;
                        c[0] = 0.0;
                    }
                }
                blend
            }
            10 => {
                backward(&mut c[2]);
                forward(&mut c[0], 0);
                let blend = two(0, c[0], 2, c[2], fade(c[2]));
                if c[2] <= 0.0 {
                    self.state = 2;
                }
                blend
            }
            11 => {
                forward(&mut c[3], 3);
                forward(&mut c[4], 4);
                let blend = two(3, c[3], 4, c[4], 1.0 - fade(c[3]));
                if !braking {
                    if mph > top {
                        self.state = 12;
                    } else if c[3] >= len[3] {
                        match mph <= open {
                            true => {
                                self.state = 5;
                                c[1] = len[1];
                            }
                            false => {
                                self.state = 10;
                                c[2] = len[2];
                            }
                        }
                        c[3] = 0.0;
                        c[4] = 0.0;
                    }
                }
                blend
            }
            _ => {
                backward(&mut c[3]);
                forward(&mut c[4], 4);
                let blend = two(4, c[4], 3, c[3], fade(c[3]));
                if c[3] <= 0.0 {
                    self.state = 7;
                }
                blend
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn wing() -> Wing {
        Wing {
            open_mph: 140.0,
            brake_mph: 125.0,
            top_mph: Some(200.0),
            brake_hold: 0.4,
            fade: 0.25,
            seconds: [1.5, 3.0, 1.6, 3.0, 1.5],
        }
    }

    fn run(state: &mut WingState, mph: f32, brake: f32, seconds: f32) -> Blend {
        let mut last = Blend::default();
        for _ in 0..(seconds / DT).round() as usize {
            last = state.step(&wing(), mph, brake, DT);
        }
        last
    }

    #[test]
    fn it_stays_at_rest_below_the_open_speed_and_opens_above_it_in_the_time_of_the_animation() {
        let mut state = WingState::default();
        assert_eq!(run(&mut state, 139.0, 0.0, 2.0).first, Some((0, 0.0)));
        assert_eq!(state.state(), 0);
        let half = run(&mut state, 150.0, 0.0, 0.75);
        let (slot, fraction) = half.first.unwrap();
        assert_eq!(slot, 0);
        assert!((fraction - 0.5).abs() < 0.05, "half way after half the time: {fraction}");
        let open = run(&mut state, 150.0, 0.0, 1.0);
        assert_eq!((open.first, state.state()), (Some((0, 1.0)), 2));
    }

    #[test]
    fn it_closes_when_the_speed_falls_below_the_open_speed() {
        let mut state = WingState::default();
        run(&mut state, 150.0, 0.0, 2.0);
        let closing = run(&mut state, 130.0, 0.0, 0.75);
        assert!((closing.first.unwrap().1 - 0.5).abs() < 0.05, "{closing:?}");
        run(&mut state, 130.0, 0.0, 1.0);
        assert_eq!(state.state(), 0);
    }

    #[test]
    fn the_brake_counts_once_it_has_been_held_and_plays_the_brake_animation() {
        let mut state = WingState::default();
        run(&mut state, 130.0, 1.0, 0.3);
        assert_eq!(state.state(), 0, "not held long enough yet");
        run(&mut state, 130.0, 1.0, 0.3);
        assert_eq!(state.state(), 4);
        let out = run(&mut state, 130.0, 1.0, 3.5);
        assert_eq!(out.first, Some((1, 1.0)), "the brake animation to its end");
        run(&mut state, 130.0, 0.0, 4.0);
        assert_eq!(state.state(), 0, "released: it plays back and rests");
        let mut slow = WingState::default();
        run(&mut slow, 100.0, 1.0, 1.0);
        assert_eq!(slow.state(), 0, "below the brake speed the brake does nothing");
    }

    #[test]
    fn braking_with_the_wing_out_cross_fades_to_the_brake_pose() {
        let mut state = WingState::default();
        run(&mut state, 160.0, 0.0, 2.0);
        let start = run(&mut state, 160.0, 1.0, 1.0);
        assert_eq!(state.state(), 9);
        let (first, second) = (start.first.unwrap().0, start.second.unwrap());
        assert_eq!((first, second.0), (2, 0), "the brake pose over the open wing");
        assert!(second.2 < 0.05, "the open wing has faded out after the first quarter second: {second:?}");
        let early =
            WingState { state: 9, clock: [1.5, 0.0, 0.0, 0.0, 0.0], held: 1.0 }.step(&wing(), 160.0, 1.0, 0.125);
        assert!((early.second.unwrap().2 - 0.5).abs() < 0.02, "half faded after 0.125 s: {early:?}");
    }

    #[test]
    fn a_wing_without_a_top_speed_animation_never_goes_past_the_open_pose() {
        let mclaren = Wing { open_mph: 120.0, brake_mph: 80.0, top_mph: None, ..wing() };
        let mut state = WingState::default();
        let mut last = Blend::default();
        for _ in 0..(3.0 / DT) as usize {
            last = state.step(&mclaren, 250.0, 0.0, DT);
        }
        assert_eq!((last.first, state.state()), (Some((0, 1.0)), 2), "held open at any speed");
        let mut state = WingState::default();
        for _ in 0..(1.0 / DT) as usize {
            state.step(&mclaren, 100.0, 1.0, DT);
        }
        assert_eq!(state.state(), 4, "braking above 80 mph at rest plays the brake animation");
    }

    #[test]
    fn above_the_top_speed_the_wing_opens_further_and_closes_back_to_the_open_pose() {
        let mut state = WingState::default();
        run(&mut state, 210.0, 0.0, 2.0);
        assert_eq!(state.state(), 6);
        let top = run(&mut state, 210.0, 0.0, 2.0);
        assert_eq!((top.first, state.state()), (Some((4, 1.0)), 7));
        run(&mut state, 190.0, 0.0, 2.0);
        assert_eq!(state.state(), 2, "back to the open wing");
    }

    #[test]
    fn a_track_is_sampled_between_its_keys_and_a_blend_moves_a_pose_toward_the_second() {
        let track = WingTrack {
            slot: 0,
            translation: vec![[0.0; 3], [0.0, 0.2, 0.0], [0.0, 0.2, 0.4]],
            rotation: vec![[0.0, 0.0, 0.0, 1.0]; 3],
        };
        let half = track.sample(0.25);
        assert!((half.translation - Vec3::new(0.0, 0.1, 0.0)).length() < 1e-6, "{half:?}");
        assert!((track.sample(1.0).translation - Vec3::new(0.0, 0.2, 0.4)).length() < 1e-6);
        let other =
            WingTrack { slot: 2, translation: vec![[1.0, 0.0, 0.0]; 2], rotation: vec![[0.0, 0.0, 0.0, 1.0]; 2] };
        let blend = Blend { first: Some((0, 1.0)), second: Some((2, 0.0, 0.5)) };
        let pose = blend.pose(&[track, other]);
        assert!((pose.translation - Vec3::new(0.5, 0.1, 0.2)).length() < 1e-6, "{pose:?}");
        assert_eq!(Blend { first: Some((3, 0.5)), second: None }.pose(&[]), Sample::REST, "no track: at rest");
    }

    #[test]
    fn a_turn_is_about_the_pivot_and_the_blend_travels_in_a_mark() {
        let sample = Sample { translation: Vec3::ZERO, rotation: Quat::from_rotation_y(90f32.to_radians()) };
        let pivot = Vec3::new(1.0, 0.0, 0.0);
        assert!((sample.matrix(pivot).transform_point3(pivot) - pivot).length() < 1e-5, "the pivot stays");
        let blend = Blend { first: Some((2, 0.5)), second: Some((4, 0.25, 0.75)) };
        let marks: BTreeSet<usize> = blend.mark().into_iter().collect();
        let back = Blend::from_marks(&marks).unwrap();
        let (a, b) = (blend.first.unwrap(), back.first.unwrap());
        assert_eq!(a.0, b.0);
        assert!((a.1 - b.1).abs() < 0.005);
        let (c, d) = (blend.second.unwrap(), back.second.unwrap());
        assert_eq!(c.0, d.0);
        assert!((c.2 - d.2).abs() < 0.005 && (c.1 - d.1).abs() < 0.005);
        assert_eq!(Blend::from_marks(&BTreeSet::from([1, 2, 3])), None, "other marks are not a wing");
    }
}
