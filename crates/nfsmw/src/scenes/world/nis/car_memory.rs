//! What the world remembers of each linked car between the frames of a scene: the ground it last stood on, so a frame
//! whose probe finds none keeps the car where it was instead of dropping it to its clip height; and where it stood and
//! how far its wheels had turned, so the wheels roll with the distance the scene moves the car.

use glam::Vec3;

use super::cars::Target;

/// How far above a car's clip position its ground is looked for: enough for a kerb or a bump of the clip, not for the
/// bridge over the road. And how far below.
const GROUND_ABOVE: f32 = 3.0;
const GROUND_BELOW: f32 = 20.0;
/// A car that moves farther than this in one frame was cut to another place: its wheels do not roll over the jump.
const CUT_JUMP: f32 = 5.0;

/// Where a car stood last frame and how far its wheels had turned (radians, the model's wheel order).
#[derive(Debug, Clone, Copy)]
struct Rolling {
    target: Target,
    at: Vec3,
    spin: [f32; 4],
}

/// The playing scene's memory of its cars.
#[derive(Debug, Default)]
pub struct CarMemory {
    grounds: Vec<(Target, f32)>,
    rolling: Vec<Rolling>,
}

impl CarMemory {
    /// The ground `target` last stood on in this scene.
    pub fn ground(&self, target: Target) -> Option<f32> {
        self.grounds.iter().find(|(t, _)| *t == target).map(|&(_, h)| h)
    }

    pub fn set_ground(&mut self, target: Target, height: f32) {
        match self.grounds.iter_mut().find(|(t, _)| *t == target) {
            Some(entry) => entry.1 = height,
            None => self.grounds.push((target, height)),
        }
    }
}

impl CarMemory {
    /// How far the wheels of `target` have turned with the car at `at` (render space) facing `forward` this frame: on
    /// its first frame in the scene they keep `start` (the physics car's); then each wheel rolls by the distance moved
    /// along `forward` over its radius in `radii`. A jump of more than [`CUT_JUMP`] (a cut) rolls nothing.
    pub fn roll(&mut self, target: Target, at: Vec3, forward: Vec3, radii: [f32; 4], start: [f32; 4]) -> [f32; 4] {
        let Some(rolling) = self.rolling.iter_mut().find(|r| r.target == target) else {
            self.rolling.push(Rolling { target, at, spin: start });
            return start;
        };
        let moved = at - std::mem::replace(&mut rolling.at, at);
        if moved.length() > CUT_JUMP {
            return rolling.spin;
        }
        rolling.spin = rolled(rolling.spin, radii, moved.dot(forward));
        rolling.spin
    }
}

/// Wheels at `spin` after rolling `travel` metres forward (negative backward), each by `travel / radius`.
pub fn rolled(spin: [f32; 4], radii: [f32; 4], travel: f32) -> [f32; 4] {
    std::array::from_fn(|i| spin[i] + travel / radii[i].max(0.05))
}

/// The ground under a car at height `z`: the first surface `probe(from, to)` finds going down from [`GROUND_ABOVE`]
/// above it to [`GROUND_BELOW`] below, else the ground it stood on `last`.
pub fn first_ground_below(probe: impl Fn(f32, f32) -> Option<f32>, z: f32, last: Option<f32>) -> Option<f32> {
    probe(z + GROUND_ABOVE, z - GROUND_BELOW).or(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A road at 0 m under a bridge at 10 m: the first of them going down from `from`, above `to`.
    fn stacked(from: f32, to: f32) -> Option<f32> {
        [10.0, 0.0].into_iter().find(|&h| h <= from && h >= to)
    }

    #[test]
    fn a_car_under_the_bridge_stands_on_the_road() {
        assert_eq!(first_ground_below(stacked, 0.4, None), Some(0.0));
    }

    #[test]
    fn a_car_on_the_bridge_stands_on_the_bridge() {
        assert_eq!(first_ground_below(stacked, 10.3, None), Some(10.0));
    }

    #[test]
    fn a_car_a_little_below_its_road_still_finds_it() {
        assert_eq!(first_ground_below(stacked, -2.5, None), Some(0.0));
    }

    #[test]
    fn no_ground_keeps_the_last_one() {
        let nothing = |_: f32, _: f32| None;
        assert_eq!(first_ground_below(nothing, 4.0, Some(1.5)), Some(1.5));
        assert_eq!(first_ground_below(nothing, 4.0, None), None);
    }

    #[test]
    fn a_metre_of_travel_turns_a_wheel_one_over_its_radius() {
        let spun = rolled([0.0, 1.0, 0.0, 0.0], [0.5, 0.5, 0.25, 0.25], 1.0);
        assert_eq!(spun, [2.0, 3.0, 4.0, 4.0]);
        assert_eq!(rolled([0.0; 4], [0.5; 4], -1.0), [-2.0; 4], "backward turns them back");
    }

    #[test]
    fn the_wheels_roll_with_the_car_and_not_over_a_cut() {
        let mut memory = CarMemory::default();
        let radii = [0.5; 4];
        let start = memory.roll(Target::Player, Vec3::ZERO, Vec3::X, radii, [0.3; 4]);
        assert_eq!(start, [0.3; 4], "the first frame keeps the car's own wheels");
        let after = memory.roll(Target::Player, Vec3::new(1.0, 0.5, 0.0), Vec3::X, radii, [0.0; 4]);
        assert!((after[0] - 2.3).abs() < 1e-5, "a metre forward: {after:?}");
        let cut = memory.roll(Target::Player, Vec3::new(100.0, 0.0, 0.0), Vec3::X, radii, [0.0; 4]);
        assert_eq!(cut, after, "a cut does not spin the wheels");
        let still = memory.roll(Target::Player, Vec3::new(100.0, 0.0, 0.0), Vec3::X, radii, [0.0; 4]);
        assert_eq!(still, after, "standing still");
        let other = memory.roll(Target::Traffic(2), Vec3::ZERO, Vec3::X, radii, [0.0; 4]);
        assert_eq!(other, [0.0; 4], "each car has its own wheels");
    }

    #[test]
    fn the_memory_keeps_one_ground_per_car() {
        let mut memory = CarMemory::default();
        memory.set_ground(Target::Player, 1.0);
        memory.set_ground(Target::Traffic(3), 2.0);
        memory.set_ground(Target::Player, 4.0);
        assert_eq!(memory.ground(Target::Player), Some(4.0));
        assert_eq!(memory.ground(Target::Traffic(3)), Some(2.0));
        assert_eq!(memory.ground(Target::Spawned(3)), None);
    }
}
