//! The wing of a 2012 car that several animations move with its speed and brake. Spec:
//! `docs/formats/chameleon-active-aero.md`, "Kind 1".
//!
//! The machine itself is code of the game; the data names its five animations (states `Handling Anim`, `Brake Anim`,
//! `Brake from Handling Anim`, `Brake from Top Speed Anim`, `Top Speed Anim`) and the numbers it tests: speeds of 125, 140
//! and 200 mph and a 0.4 second wait. A bundle that has these states and those numbers is read as that machine.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::active_aero::key_poses;
use crate::skeleton::Skeleton;

/// The resource type of a state machine.
const HSM_KIND: u32 = 0x501;
const ID_MASK: u64 = 0x00FF_FFFF_FFFF;
/// The input the machine tests.
const SPEED_INPUT: &str = "speedinmph";
/// The states that play each animation, in the order of the machine's five clocks.
const SLOT_STATES: [&str; 5] =
    ["Handling Anim", "Brake Anim", "Brake from Handling Anim", "Brake from Top Speed Anim", "Top Speed Anim"];
/// The numbers of the machine: the code of the game tests them (`open`, `brake`, `top`, the hold and the cross-fade), and
/// the data holds the speeds as conditions, which are checked.
pub const OPEN_MPH: f32 = 140.0;
pub const BRAKE_MPH: f32 = 125.0;
pub const TOP_MPH: f32 = 200.0;
pub const BRAKE_HOLD_SECONDS: f32 = 0.4;
pub const FADE_SECONDS: f32 = 0.25;

/// The keys of one animation for one bone.
#[derive(Debug, Clone, PartialEq)]
pub struct WingBoneTrack {
    pub bone: usize,
    pub slot: usize,
    /// The absolute slide at each key, metres, in the engine space (x left, y up, z forward).
    pub translation: Vec<[f32; 3]>,
    /// The absolute turn at each key as a quaternion (x, y, z, w), in the same space.
    pub rotation: Vec<[f32; 4]>,
}

/// A wing machine: the length of its five animations and the keys of every bone they move.
#[derive(Debug, Clone, PartialEq)]
pub struct WingMachine {
    pub seconds: [f32; 5],
    pub tracks: Vec<WingBoneTrack>,
}

/// The wing machine of the car in `bundle`, `None` for a car without one.
pub fn wing_machine(bundle: &Bundle, skeleton: &Skeleton) -> Option<WingMachine> {
    let kind = ResourceKind::Animation.id();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let machine = bundle.resource(index).ok()?;
        let block = &machine.blocks[0];
        let states = crate::hsm::states(block);
        let animations: Option<Vec<u64>> = SLOT_STATES
            .iter()
            .map(|name| states.iter().find(|state| state.name == *name).map(|state| state.animation))
            .collect();
        let Some(animations) = animations else { continue };
        let speeds: Vec<f32> =
            crate::hsm::conditions(block).iter().filter(|c| c.input == SPEED_INPUT).map(|c| c.value).collect();
        if ![OPEN_MPH, BRAKE_MPH, TOP_MPH].iter().all(|wanted| speeds.contains(wanted)) {
            continue;
        }
        let mut seconds = [0.0f32; 5];
        let mut tracks = Vec::new();
        for (slot, id) in animations.iter().enumerate() {
            let at = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == *id)?;
            let animation = bundle.resource(at).ok()?;
            let block = &animation.blocks[0];
            for bone in 0..skeleton.bones.len() {
                let Some((translation, rotation, length)) = key_poses(block, bone) else { continue };
                seconds[slot] = length;
                tracks.push(WingBoneTrack { bone, slot, translation, rotation });
            }
            if seconds[slot] == 0.0 {
                return None;
            }
        }
        return Some(WingMachine { seconds, tracks });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_states_are_in_the_order_of_the_machines_clocks() {
        assert_eq!(SLOT_STATES[0], "Handling Anim");
        assert_eq!(SLOT_STATES[4], "Top Speed Anim");
        const { assert!(OPEN_MPH > BRAKE_MPH && TOP_MPH > OPEN_MPH) };
    }
}
