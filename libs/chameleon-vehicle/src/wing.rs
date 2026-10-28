//! The wing of a 2012 car that several animations move with its speed and brake. Spec:
//! `docs/formats/chameleon-active-aero.md`, "Wing machine".
//!
//! Two graphs of the state machine data are read as the same machine:
//!
//! - the Veyron's, with five animations (states `Handling Anim`, `Brake Anim`, `Brake from Handling Anim`,
//!   `Brake from Top Speed Anim`, `Top Speed Anim`) and the numbers the executable's code tests: speeds of 125, 140 and 200
//!   mph, the 0.4 second hold of the brake and the 0.25 second cross-fade are literals of the executable (the speeds are
//!   vector constants copied at start-up, the hold and the fade immediates of its update routines) and equal the data's
//!   conditions **[confirmed in the executable]**;
//! - the McLaren's and the Venom's, the same graph without the top-speed states (`Spoiler Up Anim`, `Brake Anim`,
//!   `Brake From Speed Anim`), whose numbers are read from the data: the speed that opens the wing is the condition that is
//!   a single speed test, the speed that lets the brake open it is the speed test of the condition that is the brake and a
//!   speed, and the brake hold is the timed transition (0.4 s in the data, as in the Veyron's code). The Veyron's class
//!   cannot be what plays them: it keeps five clocks and tests 140, 125 and 200 mph, and neither 120 nor 80 mph is a
//!   constant of the executable. That a graph of this shape is played by the same state table is therefore an assumption
//!   **[unconfirmed]**: the executable's generic condition evaluator, which would read the data, was not found.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::active_aero::key_poses;
use crate::hsm::{ConditionRecord, State, Threshold};
use crate::skeleton::Skeleton;

/// The resource type of a state machine.
const HSM_KIND: u32 = 0x501;
const ID_MASK: u64 = 0x00FF_FFFF_FFFF;
/// The input the machine tests.
const SPEED_INPUT: &str = "speedinmph";
const BRAKE_INPUT: &str = "brake";
/// The states that play each animation of the Veyron's graph, in the order of the machine's five clocks.
const VEYRON_STATES: [&str; 5] =
    ["Handling Anim", "Brake Anim", "Brake from Handling Anim", "Brake from Top Speed Anim", "Top Speed Anim"];
/// The states of the McLaren's graph: the open wing, the brake, and the brake with the wing out.
const THREE_STATES: [&str; 3] = ["Spoiler Up Anim", "Brake Anim", "Brake From Speed Anim"];
/// The numbers of the Veyron's machine: the code of the game tests them, and the data holds the speeds as conditions.
const OPEN_MPH: f32 = 140.0;
const BRAKE_MPH: f32 = 125.0;
const TOP_MPH: f32 = 200.0;
const BRAKE_HOLD_SECONDS: f32 = 0.4;
/// The cross-fade of the Veyron's code (an immediate of its update routine); assumed for the graphs whose code was not found.
const FADE_SECONDS: f32 = 0.25;
/// The length given to the clocks of a machine with fewer than five animations.
const UNUSED_SECONDS: f32 = 1.0;

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

/// The numbers of a wing machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingParams {
    pub open_mph: f32,
    pub brake_mph: f32,
    /// `None`: no top-speed animation.
    pub top_mph: Option<f32>,
    pub brake_hold_seconds: f32,
    pub fade_seconds: f32,
}

/// A wing machine: the length of its five animations (the unused ones one second) and the keys of every bone they move.
#[derive(Debug, Clone, PartialEq)]
pub struct WingMachine {
    pub seconds: [f32; 5],
    pub tracks: Vec<WingBoneTrack>,
    pub params: WingParams,
}

/// The animations of `names` among `states`, in the order of `names`.
fn animations(states: &[State], names: &[&str]) -> Option<Vec<u64>> {
    names.iter().map(|name| states.iter().find(|state| state.name == *name).map(|state| state.animation)).collect()
}

fn is_test(test: &Threshold, input: &str, operator: &str) -> bool {
    test.input == input && test.operator == operator
}

/// The numbers of the McLaren's graph from its conditions and timed transitions.
fn three_slot_params(conditions: &[ConditionRecord], timers: &[f32]) -> Option<WingParams> {
    let speed_over = |test: &Threshold| is_test(test, SPEED_INPUT, "2").then_some(test.value);
    let alone: Vec<f32> =
        conditions.iter().filter(|c| c.tests.len() == 1).filter_map(|c| speed_over(&c.tests[0])).collect();
    let with_brake: Vec<f32> = conditions
        .iter()
        .filter(|c| !c.any && c.tests.len() == 2 && c.tests.iter().any(|t| is_test(t, BRAKE_INPUT, "1")))
        .filter_map(|c| c.tests.iter().find_map(speed_over))
        .collect();
    let (open_mph, brake_mph) = match (&alone[..], &with_brake[..]) {
        ([open], [brake]) if brake <= open => (*open, *brake),
        _ => return None,
    };
    let hold = timers.first().copied().filter(|hold| timers.iter().all(|t| t == hold))?;
    Some(WingParams { open_mph, brake_mph, top_mph: None, brake_hold_seconds: hold, fade_seconds: FADE_SECONDS })
}

/// The wing machine of the car in `bundle`, `None` for a car without one.
pub fn wing_machine(bundle: &Bundle, skeleton: &Skeleton) -> Option<WingMachine> {
    let kind = ResourceKind::Animation.id();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let machine = bundle.resource(index).ok()?;
        let block = &machine.blocks[0];
        let states = crate::hsm::states(block);
        let conditions = crate::hsm::condition_records(block);
        let speeds: Vec<f32> = conditions
            .iter()
            .flat_map(|c| c.tests.iter())
            .filter(|t| t.input == SPEED_INPUT)
            .map(|t| t.value)
            .collect();
        let veyron = animations(&states, &VEYRON_STATES)
            .filter(|_| [OPEN_MPH, BRAKE_MPH, TOP_MPH].iter().all(|wanted| speeds.contains(wanted)))
            .map(|ids| {
                let params = WingParams {
                    open_mph: OPEN_MPH,
                    brake_mph: BRAKE_MPH,
                    top_mph: Some(TOP_MPH),
                    brake_hold_seconds: BRAKE_HOLD_SECONDS,
                    fade_seconds: FADE_SECONDS,
                };
                (ids, params)
            });
        let found = veyron.or_else(|| {
            let ids = animations(&states, &THREE_STATES)?;
            Some((ids, three_slot_params(&conditions, &crate::hsm::timers(block))?))
        });
        let Some((animations, params)) = found else { continue };
        let mut seconds = [UNUSED_SECONDS; 5];
        let mut tracks = Vec::new();
        for (slot, id) in animations.iter().enumerate() {
            let at = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == *id)?;
            let animation = bundle.resource(at).ok()?;
            let block = &animation.blocks[0];
            let mut length = 0.0;
            for bone in 0..skeleton.bones.len() {
                let Some((translation, rotation, seconds)) = key_poses(block, bone) else { continue };
                length = seconds;
                tracks.push(WingBoneTrack { bone, slot, translation, rotation });
            }
            if length == 0.0 {
                return None;
            }
            seconds[slot] = length;
        }
        return Some(WingMachine { seconds, tracks, params });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(input: &str, operator: &str, value: f32) -> Threshold {
        Threshold { input: input.into(), operator: operator.into(), value }
    }

    fn record(tests: Vec<Threshold>, any: bool) -> ConditionRecord {
        ConditionRecord { name: String::new(), tests, any }
    }

    #[test]
    fn the_five_states_are_in_the_order_of_the_machines_clocks() {
        assert_eq!(VEYRON_STATES[0], "Handling Anim");
        assert_eq!(VEYRON_STATES[4], "Top Speed Anim");
        const { assert!(OPEN_MPH > BRAKE_MPH && TOP_MPH > OPEN_MPH) };
    }

    #[test]
    fn the_numbers_of_the_three_animation_graph_come_from_its_conditions() {
        let conditions = vec![
            record(vec![test(SPEED_INPUT, "2", 120.0)], false),
            record(vec![test(BRAKE_INPUT, "1", 0.0), test(SPEED_INPUT, "2", 80.0)], false),
            record(vec![test(SPEED_INPUT, "3", 10.0)], false),
            record(vec![test(BRAKE_INPUT, "0", 0.0), test(SPEED_INPUT, "5", 120.0)], false),
            record(vec![test(BRAKE_INPUT, "0", 0.0), test(SPEED_INPUT, "3", 80.0)], true),
        ];
        let params = three_slot_params(&conditions, &[0.4, 0.4]).unwrap();
        assert_eq!(
            (params.open_mph, params.brake_mph, params.top_mph, params.brake_hold_seconds),
            (120.0, 80.0, None, 0.4)
        );
        assert!(three_slot_params(&conditions, &[0.4, 0.5]).is_none(), "two different holds are not one machine");
        assert!(three_slot_params(&conditions[1..], &[0.4]).is_none(), "without the open speed it is not read");
    }
}
