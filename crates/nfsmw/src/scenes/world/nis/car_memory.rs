//! What the world remembers of each linked car between the frames of a scene: the ground it last stood on, so a frame
//! whose probe finds none keeps the car where it was instead of dropping it to its clip height.

use super::cars::Target;

/// How far above a car's clip position its ground is looked for: enough for a kerb or a bump of the clip, not for the
/// bridge over the road. And how far below.
const GROUND_ABOVE: f32 = 3.0;
const GROUND_BELOW: f32 = 20.0;

/// The playing scene's memory of its cars.
#[derive(Debug, Default)]
pub struct CarMemory {
    grounds: Vec<(Target, f32)>,
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
