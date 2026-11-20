//! The standstill in the rider's blend tree: the rider puts his feet down as the bike stops, waits with them on the ground and
//! takes them back up as it pulls away (the `Standing`, `Stop` and `Start` states of the blueprint's `Main_SM`,
//! `docs/specs/ride6-rider-standstill.md`).
//!
//! The blueprint's `Standing/StartStop_*_W_I_P` functions drive the start and the stop clips by an explicit time that goes from a
//! start second to an end second of the clip as a smoothed 0 to 1 value moves; `IK_Alpha_StartStop/Ground_*_W_I_P` say in which
//! second of the clips each foot leaves the peg and touches the ground, and the other way round. `SET Animations` calls them
//! with numbers that depend on the bike type and on the foot that goes down (`StandingLeg`: both feet, the left or the right).
//! The numbers below are those calls' arguments; the clip names are `graph_sets`'. The graph reads five parameters, all made by
//! the game's rider runtime from the bike's speed: `stand` (how much of the pose is the standstill), `stand_state` (0 the stop
//! clip, 1 the standing clip, 2 the start clip), `stand_leg` (0 both feet, 1 the left, 2 the right), `stop_time` and
//! `start_time` (the seconds into the stop and the start clip).

use anyhow::Result;
use blackbox_skeleton::{Node, Pack, TimeMode, Value};
use serde_json::{Value as Json, json};

use super::graph_sets::ClipSet;

/// `StartStop Speed Treshold`: the bike is moving above this speed (km/h, the blueprint's `VehicleSpeed`).
pub const SPEED_KMH: f32 = 6.0;
/// `Racing blend start (0 to 1)`: the start clip hands over to the riding pose when its 0 to 1 value reaches this.
pub const RACING_AT: f32 = 0.85;

/// The numbers of one foot choice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// The seconds of the start clip that play (`Start Clip Start`, `Start Clip End`), and of the stop clip.
    pub start_seconds: [f32; 2],
    pub stop_seconds: [f32; 2],
    /// How fast the 0 to 1 value of the start clip and of the stop clip moves (`Start Anim Speed`, `Stop Anim Speed`: per second).
    pub start_rate: f32,
    pub stop_rate: f32,
    /// For the left and the right foot: the second of the start clip at which the foot leaves the ground, the second at which it
    /// is on the peg again, the second of the stop clip at which it leaves the peg and the second at which it is on the ground
    /// (`Left - Start: off Ground at:`, `on Footboard at:`, `Stop: off Footboard at:`, `on Ground at:`). A foot that does not
    /// go down (the other one, when one foot is put down) is `None`: it stays on its peg.
    pub left_ik: Option<[f32; 4]>,
    pub right_ik: Option<[f32; 4]>,
    /// `Left Foot IK Smooth Speed`, `Right Foot IK Smooth Speed`: the rates at which the feet's pegs hold fade (per second).
    pub ik_rate: [f32; 2],
}

const fn window(
    start_seconds: [f32; 2],
    stop_seconds: [f32; 2],
    (start_rate, stop_rate): (f32, f32),
    (left_ik, right_ik): (Option<[f32; 4]>, Option<[f32; 4]>),
    ik_rate: f32,
) -> Window {
    Window { start_seconds, stop_seconds, start_rate, stop_rate, left_ik, right_ik, ik_rate: [ik_rate; 2] }
}

/// The numbers of the race and naked bikes (`BikeType` 0, 1 and 5), for both feet, the left and the right foot down.
pub const ROAD: [Window; 3] = [
    window([0.0, 3.7], [1.0, 3.7], (0.7, 0.6), (Some([0.3, 3.2, 1.3, 1.7]), Some([0.3, 1.9, 3.0, 3.3])), 3.0),
    window([0.9, 3.2], [0.0, 3.3], (1.15, 1.0), (Some([0.91, 3.0, 0.3, 1.5]), None), 3.0),
    window([0.2, 3.6], [1.0, 4.0], (0.85, 1.0), (None, Some([0.22, 3.2, 2.5, 3.2])), 3.0),
];

/// The numbers of the supermoto, the scooter and the enduro bike (`BikeType` 2, 3 and 4).
pub const OTHER: [Window; 3] = [
    window([0.0, 1.15], [1.1, 3.7], (1.2, 1.0), (Some([0.1, 1.14, 1.09, 1.85]), Some([0.2, 1.14, 2.6, 3.2])), 4.0),
    window([0.0, 1.65], [1.1, 2.5], (0.7, 1.4), (Some([0.01, 1.64, 1.3, 2.1]), None), 3.0),
    window([0.0, 1.5], [1.1, 2.4], (0.8, 1.4), (None, Some([0.01, 1.49, 1.25, 2.1])), 3.0),
];

/// The numbers a bike type's standstill runs on.
pub fn windows(bike_type: u8) -> &'static [Window; 3] {
    match bike_type {
        2..=4 => &OTHER,
        _ => &ROAD,
    }
}

/// Parameter indices of the standstill, as `graph.rs` orders them.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub weight: usize,
    pub state: usize,
    pub leg: usize,
    pub stop_time: usize,
    pub start_time: usize,
}

/// `base` with the standstill blended over it by the `stand` parameter, when the pack has every clip of the set; else `base`.
pub fn with_stand(pack: &Pack, set: &ClipSet, base: Node, params: Params) -> Result<Node> {
    let seconds = |clip: usize, param: usize| {
        let length = pack.clips[clip].length.max(f32::EPSILON);
        Node::Clip { clip, time: TimeMode::Fraction(Value::remap(param, (0.0, length), (0.0, 1.0))) }
    };
    let mut legs = Vec::new();
    for leg in set.legs {
        let found = (pack.clip_index(leg.stand), pack.clip_index(leg.start), pack.clip_index(leg.stop));
        let (Some(stand), Some(start), Some(stop)) = found else { return Ok(base) };
        let standing = Node::Clip { clip: stand, time: TimeMode::Loop { rate: 1.0 } };
        let states = vec![seconds(stop, params.stop_time), standing, seconds(start, params.start_time)];
        legs.push(Node::Select { options: states, index: Value::param(params.state) });
    }
    let standstill = Node::Select { options: legs, index: Value::param(params.leg) };
    Ok(Node::Blend { a: Box::new(base), b: Box::new(standstill), weight: Value::param(params.weight) })
}

/// The `stand` entry of a bike type in `styles.json`.
pub fn entry(set: &ClipSet) -> Json {
    let leg = |window: &Window| {
        json!({
            "start_seconds": window.start_seconds,
            "stop_seconds": window.stop_seconds,
            "start_rate": window.start_rate,
            "stop_rate": window.stop_rate,
            "left_ik": window.left_ik,
            "right_ik": window.right_ik,
            "ik_rate": window.ik_rate,
        })
    };
    let legs: Vec<Json> = windows(set.bike_type).iter().map(leg).collect();
    json!({ "speed_kmh": SPEED_KMH, "racing_at": RACING_AT, "legs": legs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rider::graph_sets::{ENDURO, MOTARD, NAKED, RACE, SCOOTER};

    #[test]
    fn the_numbers_follow_the_bike_type() {
        assert_eq!(windows(RACE.bike_type), &ROAD);
        assert_eq!(windows(NAKED.bike_type), &ROAD);
        assert_eq!(windows(5), &ROAD, "the bagger is played as a naked bike");
        for set in [&MOTARD, &SCOOTER, &ENDURO] {
            assert_eq!(windows(set.bike_type), &OTHER, "{}", set.name);
        }
    }

    #[test]
    fn a_foot_put_down_alone_leaves_the_other_on_its_peg() {
        for table in [&ROAD, &OTHER] {
            assert!(table[0].left_ik.is_some() && table[0].right_ik.is_some(), "both feet go down together");
            assert!(table[1].left_ik.is_some() && table[1].right_ik.is_none());
            assert!(table[2].left_ik.is_none() && table[2].right_ik.is_some());
        }
    }

    #[test]
    fn the_windows_run_forwards_and_the_feet_leave_the_pegs_inside_the_stop_clip() {
        for window in ROAD.iter().chain(&OTHER) {
            assert!(
                window.start_seconds[0] < window.start_seconds[1] && window.stop_seconds[0] < window.stop_seconds[1]
            );
            assert!(window.start_rate > 0.0 && window.stop_rate > 0.0 && window.ik_rate.iter().all(|r| *r > 0.0));
            for ik in [window.left_ik, window.right_ik].into_iter().flatten() {
                assert!(ik[0] < ik[1], "the foot is on the peg again after it has left the ground: {ik:?}");
            }
        }
    }

    #[test]
    fn the_styles_file_carries_every_number_the_game_needs() {
        let json = entry(&MOTARD);
        assert_eq!(json["speed_kmh"], 6.0);
        assert_eq!(json["legs"].as_array().map(Vec::len), Some(3));
        assert_eq!(json["legs"][1]["right_ik"], Json::Null);
        assert_eq!(json["legs"][0]["stop_seconds"], json!([1.1f32, 3.7f32]));
    }
}
