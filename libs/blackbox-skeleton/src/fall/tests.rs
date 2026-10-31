use glam::Vec3;

use super::*;
use crate::clip::{Channel, Track};
use crate::graph::{Node, TimeMode, Value};
use crate::pose::Pose;
use crate::skeleton::tests::arm;

/// A clip that puts bone 1 at x = `x` for its whole length.
fn at_x(x: f32, length: f32) -> Clip {
    Clip {
        name: format!("x{x}"),
        length,
        tracks: vec![Track {
            bone: 1,
            rotation: Channel::None,
            translation: Channel::Uniform(vec![Vec3::new(x, 1.0, 0.0), Vec3::new(x, 1.0, 0.0)]),
            scale: Channel::None,
        }],
        additive: false,
        root_motion: false,
        notifies: Vec::new(),
    }
}

fn clip_node(clip: usize, time: TimeMode) -> Box<Node> {
    Box::new(Node::Clip { clip, time })
}

fn graph(names: &[&str], root: Node) -> Graph {
    Graph { param_names: names.iter().map(|n| (*n).to_owned()).collect(), root }
}

/// Clips: 0 high side (1 s, x = 1), 1 low side (0.4 s, x = 2), 2 tumble (x = 3), 3 sliding face down (x = 4), 4 get-up on the
/// back (2 s, x = 5), 5 get-up on the side (1 s, x = 6).
fn clips() -> Vec<Clip> {
    vec![at_x(1.0, 1.0), at_x(2.0, 0.4), at_x(3.0, 1.0), at_x(4.0, 1.0), at_x(5.0, 2.0), at_x(6.0, 1.0)]
}

fn set() -> FallSet {
    let once = TimeMode::Once { rate: 1.0 };
    let tumble = Node::ThreeWay {
        negative: clip_node(3, TimeMode::Loop { rate: 1.0 }),
        zero: clip_node(2, TimeMode::Loop { rate: 1.0 }),
        positive: clip_node(2, TimeMode::Loop { rate: 1.0 }),
        param: Value::param(0),
    };
    FallSet {
        launches: vec![
            Launch { kind: "high_side".into(), graph: graph(&[], *clip_node(0, once)) },
            Launch { kind: "low_side".into(), graph: graph(&[], *clip_node(1, once)) },
        ],
        launch_blend: 0.6,
        tumble: graph(&[PARAM_FACE], tumble),
        rests: vec![
            Rest { face_above: 0.6, face_below: 2.0, graph: graph(&[], *clip_node(4, once)), blend: 1.0 },
            Rest { face_above: -0.2, face_below: 0.2, graph: graph(&[], *clip_node(5, once)), blend: 0.2 },
        ],
        low_speed: 0.5,
        flying_above: 1.0,
        flying_ramp: (1.0, 2.5),
        fade_rate: 3.0,
    }
}

fn x_of(pose: &Pose) -> f32 {
    pose.local[1].translation.x
}

/// A ragdoll that has not yet settled: moving, on the ground, face down.
fn sliding() -> FallInput {
    FallInput { face: -1.0, side: 0.0, speed: 6.0, airborne: 0.1 }
}

fn run(player: &mut FallPlayer, set: &FallSet, clips: &[Clip], seconds: f32, input: FallInput) {
    let dt = 1.0 / 60.0;
    for _ in 0..(seconds / dt).round() as usize {
        player.step(set, clips, dt, input);
    }
}

#[test]
fn the_set_checks_out_and_survives_a_round_trip() {
    let skeleton = arm();
    let set = set();
    set.validate(&clips(), &skeleton).unwrap();
    let text = serde_json::to_string(&set).unwrap();
    let back: FallSet = serde_json::from_str(&text).unwrap();
    assert_eq!(back, set);
    let mut bad = set.clone();
    bad.rests[0].face_below = bad.rests[0].face_above;
    assert!(bad.validate(&clips(), &skeleton).is_err(), "an empty band");
    let mut bad = set.clone();
    bad.low_speed = -1.0;
    assert!(bad.validate(&clips(), &skeleton).is_err());
    let mut bad = set.clone();
    bad.launches[0].graph = graph(&[], *clip_node(99, TimeMode::Hold));
    assert!(bad.validate(&clips(), &skeleton).is_err(), "a clip that is not there");
}

#[test]
fn the_kind_of_fall_picks_the_launch_and_an_unknown_kind_goes_to_the_tumble() {
    let (skeleton, clips, set) = (arm(), clips(), set());
    let mut high = FallPlayer::new(&set, "high_side");
    run(&mut high, &set, &clips, 0.1, sliding());
    assert_eq!(x_of(&high.pose(&set, &skeleton, &clips)), 1.0);
    let mut low = FallPlayer::new(&set, "low_side");
    run(&mut low, &set, &clips, 0.1, sliding());
    // The low side clip is shorter than the hand-over: the tumble is already fading in over it.
    let x = x_of(&low.pose(&set, &skeleton, &clips));
    assert!(x > 2.0 && x < 4.0, "{x}");
    let mut other = FallPlayer::new(&set, "generic");
    run(&mut other, &set, &clips, 0.1, sliding());
    assert_eq!(x_of(&other.pose(&set, &skeleton, &clips)), 4.0, "straight into the tumble, face down");
}

#[test]
fn the_launch_hands_over_when_the_blend_is_all_that_is_left_of_it_and_the_tumble_takes_over_by_a_smooth_step() {
    let (skeleton, clips, set) = (arm(), clips(), set());
    let mut player = FallPlayer::new(&set, "high_side");
    // The clip is 1 s long and the blend 0.6 s: the launch plays alone for 0.4 s.
    run(&mut player, &set, &clips, 0.35, sliding());
    assert_eq!(x_of(&player.pose(&set, &skeleton, &clips)), 1.0);
    run(&mut player, &set, &clips, 0.1, sliding());
    // 0.05 s into the blend: still almost the launch (smooth step 3 k^2 - 2 k^3 at k = 0.083 is 0.02).
    let early = x_of(&player.pose(&set, &skeleton, &clips));
    assert!(early > 1.0 && early < 1.2, "{early}");
    run(&mut player, &set, &clips, 0.25, sliding());
    // About half way through the blend (k = 0.5): half the launch, half the tumble (x = 4, sliding face down).
    let half = x_of(&player.pose(&set, &skeleton, &clips));
    assert!((half - 2.5).abs() < 0.2, "{half}");
    run(&mut player, &set, &clips, 0.4, sliding());
    assert_eq!(x_of(&player.pose(&set, &skeleton, &clips)), 4.0);
    assert!(!player.resting());
}

#[test]
fn the_tumble_follows_how_he_lies() {
    let (skeleton, clips, set) = (arm(), clips(), set());
    let mut player = FallPlayer::new(&set, "generic");
    let up = FallInput { face: 1.0, speed: 6.0, ..sliding() };
    run(&mut player, &set, &clips, 0.05, up);
    assert_eq!(x_of(&player.pose(&set, &skeleton, &clips)), 3.0, "face up: the zero/positive clip");
    let half = FallInput { face: -0.5, ..up };
    run(&mut player, &set, &clips, 0.05, half);
    assert!((x_of(&player.pose(&set, &skeleton, &clips)) - 3.5).abs() < 1e-5, "half way to the face down clip");
}

#[test]
fn he_gets_up_only_when_still_on_the_ground_and_lying_in_a_band() {
    let (skeleton, clips, set) = (arm(), clips(), set());
    let still = |face: f32| FallInput { face, side: 0.0, speed: 0.2, airborne: 0.1 };
    // Moving: no.
    let mut player = FallPlayer::new(&set, "generic");
    run(&mut player, &set, &clips, 1.0, FallInput { speed: 0.6, ..still(1.0) });
    assert!(!player.resting());
    // In the air: no.
    run(&mut player, &set, &clips, 1.0, FallInput { airborne: 1.5, ..still(1.0) });
    assert!(!player.resting());
    // Between the bands (face down at -0.6 has no get-up in this set): no.
    run(&mut player, &set, &clips, 1.0, still(-0.6));
    assert!(!player.resting());
    // Still, on the ground, face up: the back get-up (the first band).
    run(&mut player, &set, &clips, 0.05, still(0.8));
    assert_eq!(player.rest_index(), Some(0));
    // The tumble fades into the get-up over 1 s and then it plays alone.
    let at_start = x_of(&player.pose(&set, &skeleton, &clips));
    assert!(at_start < 3.5, "{at_start}");
    run(&mut player, &set, &clips, 1.2, still(0.8));
    assert_eq!(x_of(&player.pose(&set, &skeleton, &clips)), 5.0);
    assert!(!player.finished(&set, &clips));
    run(&mut player, &set, &clips, 1.0, still(0.8));
    assert!(player.finished(&set, &clips), "the clip is 2 s long");
    // On the side: the other band, with its short blend.
    let mut side = FallPlayer::new(&set, "generic");
    run(&mut side, &set, &clips, 0.05, still(0.1));
    assert_eq!(side.rest_index(), Some(1));
    run(&mut side, &set, &clips, 0.3, still(0.1));
    assert_eq!(x_of(&side.pose(&set, &skeleton, &clips)), 6.0, "0.2 s blend is over");
}

#[test]
fn once_he_is_getting_up_what_the_body_does_no_longer_matters() {
    let (skeleton, clips, set) = (arm(), clips(), set());
    let mut player = FallPlayer::new(&set, "generic");
    run(&mut player, &set, &clips, 0.05, FallInput { face: 0.8, speed: 0.1, ..FallInput::default() });
    assert!(player.resting());
    run(&mut player, &set, &clips, 1.5, FallInput { face: -1.0, speed: 9.0, airborne: 5.0, side: 0.0 });
    assert_eq!(player.rest_index(), Some(0));
    assert_eq!(x_of(&player.pose(&set, &skeleton, &clips)), 5.0);
}

#[test]
fn the_pose_follows_the_fall_at_the_sets_rate_and_the_flying_amount_is_a_ramp() {
    let set = set();
    let mut player = FallPlayer::new(&set, "generic");
    assert_eq!(player.fade(&set), 0.0);
    run(&mut player, &set, &clips(), 1.0, sliding());
    // 1 - exp(-3): the weight after a second at a rate of 3 per second.
    assert!((player.fade(&set) - 0.9502).abs() < 1e-3, "{}", player.fade(&set));
    assert_eq!(set.flying_amount(0.5), 0.0);
    assert!((set.flying_amount(1.75) - 0.5).abs() < 1e-6);
    assert_eq!(set.flying_amount(4.0), 1.0);
}

#[test]
fn the_flying_parameter_blends_the_tumble() {
    let (skeleton, clips) = (arm(), clips());
    let mut set = set();
    set.tumble = graph(
        &[PARAM_FLYING],
        Node::Blend { a: clip_node(3, TimeMode::Hold), b: clip_node(2, TimeMode::Hold), weight: Value::param(0) },
    );
    let mut player = FallPlayer::new(&set, "generic");
    run(&mut player, &set, &clips, 0.05, FallInput { airborne: 1.75, speed: 8.0, ..FallInput::default() });
    assert!((x_of(&player.pose(&set, &skeleton, &clips)) - 3.5).abs() < 1e-5);
}
