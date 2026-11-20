//! A scene's car between an `ENISDetach` and its `ENISReattach`. In the original the detach hands the car from its
//! channel to its physics: it keeps the velocity the channel gave it last and drives on with the scene's input (half
//! throttle, the brakes of the brake lock) and collides with what it meets, while the channel goes on without it and is
//! often cut to the next shot (`docs/specs/nis-events-3.md` section 3). The world here does not step its cars while a
//! scene plays, so a detached car follows its channel, which the scene's authors drew along the car's path, until the
//! channel jumps away (a cut): from there it coasts straight on at the channel's last velocity, slowed while a brake is
//! locked, on the ground under it [ours]. Measured against the running original over the detaches of `IntroNisDD`, this
//! put the car a median 8.7 m from the original's, against 13.1 m for following the channel throughout and 44.6 m for
//! coasting from the detach (private comparison, 1 run). Collisions are not modelled.

use glam::{Quat, Vec3};

use super::cars::{CarSpot, Target};

/// How fast a locked brake slows a coasting car, m/s² [ours: a tyre at about 0.8 g].
const LOCKED_DECELERATION: f32 = 8.0;
/// A channel that moves its car farther than this in one frame was cut to another place, metres (`car_memory.rs`).
const CUT_JUMP: f32 = 5.0;

/// Where a car is in its detach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// On its channel.
    Attached,
    /// Detached, its channel not cut yet: the car still follows it.
    Following,
    /// Detached after its channel was cut: the car coasts.
    Free,
}

/// One car's motion: where it was last drawn, its velocity, and its phase.
#[derive(Debug, Clone, Copy)]
struct Track {
    target: Target,
    at: Vec3,
    rotation: Quat,
    velocity: Vec3,
    phase: Phase,
}

/// The playing scene's record of its cars' channel motion and of the cars that coast.
#[derive(Debug, Default)]
pub struct Coasts {
    tracks: Vec<Track>,
}

impl Coasts {
    /// Where `target` stands this frame, `dt` scene seconds after the last: at `channel` while `attached`, and while
    /// detached until its channel is cut; then coasting from where it was at the channel's last velocity, slowed while
    /// `braking`.
    pub fn place(&mut self, target: Target, channel: CarSpot, attached: bool, braking: bool, dt: f32) -> CarSpot {
        let Some(track) = self.tracks.iter_mut().find(|track| track.target == target) else {
            let phase = if attached { Phase::Attached } else { Phase::Following };
            self.tracks.push(Track {
                target,
                at: channel.position,
                rotation: channel.rotation,
                velocity: Vec3::ZERO,
                phase,
            });
            return channel;
        };
        let cut = channel.position.distance(track.at) > CUT_JUMP;
        track.phase = match (attached, track.phase) {
            (true, _) => Phase::Attached,
            (false, Phase::Free) => Phase::Free,
            (false, _) if cut => Phase::Free,
            (false, _) => Phase::Following,
        };
        if track.phase != Phase::Free {
            if !cut {
                track.velocity = channel_velocity(track.at, channel.position, dt).unwrap_or(track.velocity);
            }
            track.at = channel.position;
            track.rotation = channel.rotation;
            return channel;
        }
        if braking && dt > 0.0 {
            let speed = track.velocity.length();
            let slower = (speed - LOCKED_DECELERATION * dt).max(0.0);
            track.velocity = track.velocity.normalize_or_zero() * slower;
        }
        track.at += track.velocity * dt.max(0.0);
        CarSpot { position: track.at, rotation: track.rotation }
    }
}

/// The velocity of a channel that moved a car from `before` to `after` in `dt` seconds; `None` for no time.
fn channel_velocity(before: Vec3, after: Vec3, dt: f32) -> Option<Vec3> {
    if !dt.is_finite() || dt <= 0.0 {
        return None;
    }
    let velocity = (after - before) / dt;
    velocity.is_finite().then_some(velocity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> CarSpot {
        CarSpot { position: Vec3::new(x, 0.0, 0.0), rotation: Quat::IDENTITY }
    }

    #[test]
    fn a_detached_car_follows_its_channel_until_the_channel_is_cut_then_coasts_on() {
        let mut coasts = Coasts::default();
        let step = 0.1;
        coasts.place(Target::Player, at(0.0), true, false, 0.0);
        coasts.place(Target::Player, at(2.0), true, false, step);
        let following = coasts.place(Target::Player, at(3.0), false, false, step);
        assert_eq!(following.position.x, 3.0, "detached, the car still follows an uncut channel");
        let coasting = coasts.place(Target::Player, at(500.0), false, false, step);
        assert!((coasting.position.x - 4.0).abs() < 1e-4, "cut: on at the last 10 m/s: {coasting:?}");
        let later = coasts.place(Target::Player, at(501.0), false, false, step);
        assert!((later.position.x - 5.0).abs() < 1e-4, "free until the reattach, even when the channel moves slowly");
        let back = coasts.place(Target::Player, at(601.0), true, false, step);
        assert_eq!(back.position.x, 601.0, "a reattached car is on its channel again");
    }

    #[test]
    fn a_locked_brake_stops_a_coasting_car() {
        let mut coasts = Coasts::default();
        coasts.place(Target::Traffic(1), at(0.0), true, false, 0.0);
        coasts.place(Target::Traffic(1), at(1.0), true, false, 0.1);
        let mut last = 0.0;
        for _ in 0..30 {
            last = coasts.place(Target::Traffic(1), at(-100.0), false, true, 0.1).position.x;
        }
        // 10 m/s at 8 m/s² stops after 1.25 s and 6.25 m (plus the first steps' rounding).
        assert!(last > 5.5 && last < 7.5, "{last}");
        let still = coasts.place(Target::Traffic(1), at(-100.0), false, true, 0.1).position.x;
        assert_eq!(still, last, "stopped");
    }

    #[test]
    fn no_time_gives_no_velocity() {
        assert_eq!(channel_velocity(Vec3::ZERO, Vec3::X, 0.0), None);
        assert_eq!(channel_velocity(Vec3::ZERO, Vec3::X, 0.5), Some(Vec3::X * 2.0));
    }
}
