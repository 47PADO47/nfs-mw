//! The rev of the race-start scene **[ours]**: while the player's car stands on its grid slot in the scene, its body
//! rocks a little, as if the engine were being revved. The original's body and wheel channels are not decoded
//! (`docs/specs/nis-player.md` section 7.4), and the scene's audio gives no rev cue to follow, so the rock is a fixed
//! oscillation: a roll about the nose and a smaller pitch, faded in once the car stands still and out when it moves.

use std::f32::consts::TAU;

use glam::Quat;

/// The largest roll and pitch, radians.
const ROLL: f32 = 1.5 * std::f32::consts::PI / 180.0;
const PITCH: f32 = 0.5 * std::f32::consts::PI / 180.0;
/// How fast the body rocks, Hz.
const FREQUENCY: f32 = 6.0;
/// Seconds the rock takes to come in once the car stands still, and to go once it moves.
const ATTACK: f32 = 0.4;
const DECAY: f32 = 0.2;
/// Below this speed (m/s) the scene's car stands still.
const STILL: f32 = 1.0;

/// The rock's envelope and its clock.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RevRock {
    level: f32,
    time: f32,
}

impl RevRock {
    /// Advances `dt` scene seconds with the car moving at `speed` m/s, and gives the body's turn now (in the car's
    /// frame: x forward, y left, z up).
    pub fn step(&mut self, dt: f32, speed: f32) -> Quat {
        self.time += dt;
        self.level = match speed < STILL {
            true => (self.level + dt / ATTACK).min(1.0),
            false => (self.level - dt / DECAY).max(0.0),
        };
        rock(self.time, self.level)
    }
}

/// The body's turn at `time` seconds with the envelope at `level` (0 to 1): a roll about the nose at [`FREQUENCY`] and a
/// smaller pitch a quarter turn behind it.
pub fn rock(time: f32, level: f32) -> Quat {
    let phase = TAU * FREQUENCY * time;
    Quat::from_rotation_x(ROLL * level * phase.sin()) * Quat::from_rotation_y(PITCH * level * phase.cos())
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn no_level_no_rock() {
        assert!(rock(0.3, 0.0).angle_between(Quat::IDENTITY) < 1e-6);
    }

    #[test]
    fn the_roll_peaks_at_a_quarter_period_and_stays_small() {
        let peak = rock(0.25 / FREQUENCY, 1.0);
        let up = peak * Vec3::Z;
        let roll = up.y.atan2(up.z).abs();
        assert!((roll - ROLL).abs() < 1e-3, "{}", roll.to_degrees());
        for i in 0..100 {
            assert!(rock(i as f32 * 0.013, 1.0).angle_between(Quat::IDENTITY) <= ROLL + PITCH + 1e-4);
        }
    }

    #[test]
    fn the_rock_fades_in_standing_and_out_moving() {
        let mut rev = RevRock::default();
        rev.step(0.1, 0.0);
        assert!((rev.level - 0.25).abs() < 1e-5);
        for _ in 0..10 {
            rev.step(0.1, 0.2);
        }
        assert_eq!(rev.level, 1.0);
        rev.step(0.1, 5.0);
        assert!((rev.level - 0.5).abs() < 1e-5, "driving off fades it out");
        rev.step(0.2, 5.0);
        assert_eq!(rev.level, 0.0);
        assert!(rev.step(0.0, 5.0).angle_between(Quat::IDENTITY) < 1e-6);
    }
}
