use super::*;

struct Open;

impl Scene for Open {
    fn clear(&self, _: Vec3, _: Vec3) -> bool {
        true
    }
    fn ground(&self, _: Vec3) -> Option<f32> {
        Some(0.0)
    }
    fn road_ahead(&self, at: Vec3, forward: Vec3, ahead: f32) -> Option<(Vec3, [Vec3; 2])> {
        let centre = at + forward * ahead;
        Some((centre, [centre + Vec3::Y * 5.0, centre - Vec3::Y * 5.0]))
    }
}

const STEP: f32 = 1.0 / 60.0;

fn facts(cops: &[Vec3]) -> Facts<'_> {
    Facts {
        car: Car { position: Vec3::ZERO, velocity: Vec3::X * 30.0, forward: Vec3::X, left: Vec3::Y, up: Vec3::Z },
        pursued: true,
        camera: Camera { eye: Vec3::new(-6.0, 0.0, 2.0), target: Vec3::ZERO, fov: 12000.0 },
        cops,
        scene: &Open,
    }
}

/// Runs `secs` of real time; returns the seconds the cop shot played.
fn run(cams: &mut MomentCams, secs: f32, facts: &Facts<'_>) -> f32 {
    let mut cop = 0.0;
    for _ in 0..(secs / STEP).round() as usize {
        cams.update(STEP, facts);
        if cams.hides_hud() {
            cop += STEP;
        }
    }
    cop
}

#[test]
fn the_pursuit_shot_slows_the_world_then_looks_at_the_cop_for_two_seconds() {
    let cops = [Vec3::new(40.0, 5.0, 0.0)];
    let facts = facts(&cops);
    let mut cams = MomentCams::default();
    cams.start(Start::Pursuit, &facts);
    assert_eq!(cams.take_sounds(), ["spot_cop_camera"]);
    run(&mut cams, 0.3, &facts);
    assert!(!cams.playing(), "the lead-in stays on the gameplay camera");
    assert!((cams.sim_speed() - 0.25).abs() < 1e-3, "and slows the world to a quarter: {}", cams.sim_speed());
    let played = run(&mut cams, 3.0, &facts);
    assert!((played - 2.0).abs() < 0.05, "the cop shot plays two seconds: {played}");
    assert!(!cams.playing());
    assert!(cams.sim_speed() < 0.7, "it returns in slow motion: {}", cams.sim_speed());
    run(&mut cams, 2.0, &facts);
    assert_eq!(cams.sim_speed(), 1.0, "and recovers within two seconds");
}

#[test]
fn a_pursuit_with_no_cop_in_sight_returns_at_once() {
    let facts = facts(&[]);
    let mut cams = MomentCams::default();
    cams.start(Start::Pursuit, &facts);
    let played = run(&mut cams, 1.0, &facts);
    assert!(played <= 2.0 * STEP, "{played}");
    assert!(!cams.playing());
}

#[test]
fn a_jump_shot_plays_its_length_in_slow_motion_and_takes_no_second_jump() {
    let facts = facts(&[]);
    let mut cams = MomentCams::default();
    cams.start(Start::Jump(2.0), &facts);
    assert!(cams.playing() && cams.car_shot());
    cams.update(STEP, &facts);
    assert_eq!(cams.sim_speed(), track_car::SIM_SPEED);
    assert!(cams.view(&facts.car).is_some());
    cams.start(Start::Jump(4.0), &facts);
    run(&mut cams, 2.1, &facts);
    assert!(!cams.playing(), "the first jump's length, not the second's");
    assert_eq!(cams.take_sounds(), ["jump_cam"]);
}

#[test]
fn a_crash_cuts_only_from_the_gameplay_camera() {
    let cops = [Vec3::new(40.0, 5.0, 0.0)];
    let facts = facts(&cops);
    let mut cams = MomentCams::default();
    cams.start(Start::Pursuit, &facts);
    run(&mut cams, 0.6, &facts);
    assert!(cams.hides_hud());
    cams.start(Start::Crash { roadblock: true }, &facts);
    assert!(cams.hides_hud(), "the cop shot goes on");
    run(&mut cams, 4.0, &facts);
    cams.start(Start::Crash { roadblock: true }, &facts);
    assert!(cams.car_shot());
    assert!(cams.status().contains("car shot"), "{}", cams.status());
}

#[test]
fn stopping_drops_the_shot_and_its_slow_motion() {
    let facts = facts(&[]);
    let mut cams = MomentCams::default();
    cams.start(Start::Jump(3.0), &facts);
    cams.update(STEP, &facts);
    cams.stop();
    assert!(!cams.playing());
    assert_eq!(cams.sim_speed(), 1.0);
    assert!(cams.take_cut());
}
