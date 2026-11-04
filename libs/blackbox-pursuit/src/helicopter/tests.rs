use super::*;
use control::Flat;

#[test]
fn retained_goal_moves_on_every_physics_step_between_decisions() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    let dt = 1.0 / 60.0;
    heli.decide(dt * 8.0, &player(), OPEN, &clear, &Flat(0.0), SEARCH);
    let fuel = heli.fuel;
    for _ in 0..8 {
        let previous = heli.body.position;
        heli.integrate(dt, &Flat(0.0));
        assert!(heli.body.position.distance(previous) > 0.0, "flight cannot freeze between AI updates");
        assert!(heli.body.position.is_finite());
    }
    assert_eq!(heli.fuel, fuel, "physics steps cannot debit the decision clock twice");
}

#[test]
fn fuel_and_skid_clocks_use_the_actual_decision_interval() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    let dt = 8.0 / 60.0;
    heli.decide(dt, &player(), OPEN, &clear, &Flat(0.0), SEARCH);
    assert!((heli.fuel - (60.0 - dt)).abs() < 1e-5);
    assert!((heli.skid_timer + dt).abs() < 1e-6);
}

fn player() -> Player {
    Player { position: Vec3::ZERO, velocity: Vec3::new(30.0, 0.0, 0.0), forward: Vec3::X }
}

const OPEN: Target = Target { hidden: false, in_tunnel: false };
const SEARCH: SearchInput = SearchInput { perp_in_sight: true, last_known: Vec3::ZERO, unit_random: 0.5 };
const LOST: SearchInput = SearchInput { perp_in_sight: false, last_known: Vec3::ZERO, unit_random: 0.5 };

fn clear(_: Vec3) -> bool {
    true
}

fn run(heli: &mut Helicopter, seconds: f32, target: Target, line: &dyn Fn(Vec3) -> bool) -> bool {
    run_in(heli, seconds, target, line, SEARCH)
}

fn run_in(
    heli: &mut Helicopter,
    seconds: f32,
    target: Target,
    line: &dyn Fn(Vec3) -> bool,
    input: SearchInput,
) -> bool {
    let steps = (seconds / THINK_PERIOD) as usize;
    let mut done = false;
    for _ in 0..steps {
        done |= heli.think(THINK_PERIOD, &player(), target, line, &Flat(0.0), input);
    }
    done
}

#[test]
fn the_game_values_are_the_defaults() {
    let t = Tuning::default();
    assert_eq!(t.spawn_distance, 250.0);
    assert_eq!(t.despawn_distance, 600.0);
    assert_eq!(t.speed_cap, 100.0);
}

#[test]
fn fuel_runs_out_and_the_helicopter_exits() {
    let tuning = Tuning { fuel_time: 1.0, ..Tuning::default() };
    let mut heli = Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, tuning);
    run(&mut heli, 2.0, OPEN, &clear);
    assert!(matches!(heli.mode, Mode::Exit(_)));
}

#[test]
fn a_visible_player_is_followed_closely() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    run(&mut heli, 10.0, OPEN, &clear);
    assert_eq!(heli.mode, Mode::Follow);
    let gap = horizontal_distance(heli.body.position, player().position);
    assert!(gap < 150.0, "keeps up with the car: {gap} m");
}

#[test]
fn a_player_the_pursuit_has_lost_sends_it_to_search() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    run_in(&mut heli, 0.5, OPEN, &clear, LOST);
    assert!(matches!(heli.mode, Mode::Search { .. }));
}

#[test]
fn its_own_blocked_line_does_not_send_it_to_search_while_the_pursuit_has_the_player() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    let blocked = |_: Vec3| false;
    run(&mut heli, 1.0, OPEN, &blocked);
    assert!(!heli.sight.visible(), "the helicopter itself has lost the line");
    assert_eq!(heli.mode, Mode::Follow, "but another cop sees the player, so it keeps flying to it");
}

#[test]
fn the_search_keeps_its_point_until_it_is_within_thirty_metres() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    run_in(&mut heli, THINK_PERIOD, OPEN, &clear, LOST);
    let Mode::Search { point } = heli.mode else { panic!("{:?}", heli.mode) };
    run_in(&mut heli, 2.0, OPEN, &clear, SearchInput { unit_random: 0.9, ..LOST });
    assert_eq!(heli.mode, Mode::Search { point }, "a new random number does not move the point");
    heli.body.position = Vec3::new(point.x, point.y, point.z + 10.0);
    run_in(&mut heli, THINK_PERIOD, OPEN, &clear, SearchInput { unit_random: 0.9, ..LOST });
    let Mode::Search { point: next } = heli.mode else { panic!("{:?}", heli.mode) };
    assert_ne!(next, point, "within 30 m the next point is chosen");
}

#[test]
fn the_search_spreads_over_the_levels_radius_around_the_last_known_place() {
    let tuning = Tuning { search_radius: 10_000.0, ..Tuning::default() };
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, tuning);
    let input = SearchInput { last_known: Vec3::new(500.0, 0.0, 0.0), unit_random: 0.0, ..LOST };
    run_in(&mut heli, THINK_PERIOD, OPEN, &clear, input);
    let Mode::Search { point } = heli.mode else { panic!("{:?}", heli.mode) };
    let from_centre = horizontal_distance(point, input.last_known);
    assert!((from_centre - 2000.0).abs() < 1.0, "0.2 of the radius at the smallest roll: {from_centre}");
}

#[test]
fn a_hidden_player_is_not_seen_once_latched() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    let blocked = |_: Vec3| false;
    let hidden = Target { hidden: true, in_tunnel: false };
    run(&mut heli, 0.25, hidden, &blocked);
    assert!(!heli.sight.visible());
}

#[test]
fn with_hiding_spots_off_a_hidden_player_stays_in_sight() {
    let tuning = Tuning { hiding_spots: false, ..Tuning::default() };
    let mut heli = Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, tuning);
    run(&mut heli, 0.25, OPEN, &clear);
    let blocked = |_: Vec3| false;
    let hidden = Target { hidden: true, in_tunnel: false };
    run(&mut heli, 0.25, hidden, &blocked);
    assert!(heli.sight.visible(), "the zone is ignored, so the player near its last seen place is still seen");
}

#[test]
fn with_skid_hit_it_comes_in_beside_the_player_when_in_window() {
    let tuning = Tuning { skid_hit: true, ..Tuning::default() };
    let mut heli = Helicopter::spawn(Vec3::new(20.0, 8.0, 0.0), 0.0, tuning);
    heli.skid_timer = -10.0;
    run(&mut heli, THINK_PERIOD, OPEN, &clear);
    assert!(matches!(heli.mode, Mode::SkidApproach { .. }), "mode {:?}", heli.mode);
}

#[test]
fn a_skid_is_not_started_during_cooldown() {
    let tuning = Tuning { skid_hit: true, ..Tuning::default() };
    let mut heli = Helicopter::spawn(Vec3::new(20.0, 8.0, 0.0), 0.0, tuning);
    heli.skid_timer = 0.0;
    run(&mut heli, THINK_PERIOD, OPEN, &clear);
    assert_eq!(heli.mode, Mode::Follow, "cooldown not over");
}

#[test]
fn the_exit_reports_done_once_it_is_clear_of_the_car() {
    let mut heli = Helicopter::spawn(Vec3::new(-20.0, 10.0, 0.0), 0.0, Tuning::default());
    heli.leave();
    let done = run(&mut heli, 60.0, OPEN, &clear);
    assert!(done, "the exit finishes within a minute");
}

/// A helicopter at `at` whose spawn timer lets the cop manager take it out of play (past the 10 s of `CanRespawn`).
fn settled(at: Vec3) -> Helicopter {
    let mut heli = Helicopter::spawn(at, 0.0, Tuning::default());
    heli.age = crate::respawn::NEVER_SEEN_SECS + 1.0;
    heli
}

#[test]
fn a_helicopter_out_of_view_far_from_the_point_ahead_is_out_of_play() {
    // The player drives along +X: the point 75 m ahead is (75, 0, 0). 650 m behind the player is 725 m from it.
    let mut far_back = settled(Vec3::new(-650.0, 40.0, 0.0));
    assert!(far_back.out_of_play(&player()));
    // 560 m ahead of the player is 485 m from the point ahead: still in play, though 560 m from the player.
    let mut ahead = settled(Vec3::new(560.0, 40.0, 0.0));
    assert!(!ahead.out_of_play(&player()));
    let mut near = settled(Vec3::new(-500.0, 40.0, 0.0));
    assert!(!near.out_of_play(&player()), "575 m from the point ahead");
}

#[test]
fn a_helicopter_that_never_stood_in_play_waits_ten_seconds_before_it_is_out_of_play() {
    let mut heli = Helicopter::spawn(Vec3::new(-650.0, 40.0, 0.0), 0.0, Tuning::default());
    heli.age = 9.9;
    assert!(!heli.out_of_play(&player()), "far and out of view, but too young");
    heli.age = 10.1;
    assert!(heli.out_of_play(&player()));
}

#[test]
fn a_helicopter_that_stood_in_play_may_go_eight_seconds_after_its_spawn() {
    let mut heli = Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, Tuning::default());
    heli.age = 1.0;
    assert!(!heli.out_of_play(&player()), "100 m from the player its place may not be recycled");
    heli.body.position = Vec3::new(-650.0, 40.0, 0.0);
    heli.age = 7.9;
    assert!(!heli.out_of_play(&player()));
    heli.age = 8.1;
    assert!(heli.out_of_play(&player()), "after a stay in play 8 s suffice");
}

#[test]
fn the_spawn_timer_counts_with_each_decision() {
    let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
    heli.decide(0.5, &player(), OPEN, &clear, &Flat(0.0), LOST);
    heli.decide(0.25, &player(), OPEN, &clear, &Flat(0.0), LOST);
    assert!((heli.age - 0.75).abs() < 1e-6);
}

#[test]
fn a_far_helicopter_the_camera_looks_at_stays_in_play() {
    let mut heli = settled(Vec3::new(-650.0, 40.0, 0.0));
    heli.camera =
        Some(camera::Camera { position: Vec3::new(10.0, 3.0, 0.0), forward: Vec3::NEG_X, velocity: Vec3::ZERO });
    assert!(!heli.out_of_play(&player()), "the camera looks back at it");
    heli.camera = Some(camera::Camera { position: Vec3::new(-10.0, 3.0, 0.0), forward: Vec3::X, velocity: Vec3::ZERO });
    assert!(heli.out_of_play(&player()), "the camera looks away from it");
}

/// A level floor at a height, with its normal straight up (the sheet over a flat roof).
struct Roof(f32);

impl control::Heights for Roof {
    fn elevation(&self, _x: f32, _z: f32) -> Option<f32> {
        Some(self.0)
    }

    fn floor(&self, x: f32, z: f32) -> Option<control::Floor> {
        self.elevation(x, z).map(|height| control::Floor { height, normal: Vec3::Y })
    }
}

/// Chases a car driving along +x at 70 m/s from 250 m behind it for `seconds`, over `floor`; returns the helicopter.
fn chase_over(floor: &dyn control::Heights, seconds: f32) -> (Helicopter, Player) {
    let mut heli = Helicopter::spawn(Vec3::new(-250.0, 20.0, 0.0), 0.0, Tuning::default());
    let dt = 1.0 / 60.0;
    let mut p = Player { position: Vec3::ZERO, velocity: Vec3::new(70.0, 0.0, 0.0), forward: Vec3::X };
    let mut clock = THINK_PERIOD;
    for _ in 0..(seconds / dt) as usize {
        clock += dt;
        while clock >= THINK_PERIOD {
            clock -= THINK_PERIOD;
            heli.decide(THINK_PERIOD, &p, OPEN, &clear, floor, SEARCH);
        }
        heli.integrate(dt, floor);
        p.position += p.velocity * dt;
    }
    (heli, p)
}

#[test]
fn a_helicopter_from_behind_catches_a_fast_car() {
    let (heli, p) = chase_over(&Flat(-2.0), 12.0);
    assert!(heli.body.velocity.length() > 95.0, "top speed: {}", heli.body.velocity.length());
    assert!(p.position.x - heli.body.position.x < 150.0, "closing: {}", p.position.x - heli.body.position.x);
}

#[test]
fn a_level_roof_over_the_chase_does_not_push_the_helicopter_aside() {
    let (heli, p) = chase_over(&Roof(26.0), 12.0);
    assert!(heli.body.velocity.z.abs() < 5.0 && heli.body.position.z.abs() < 20.0, "{:?}", heli.body);
    assert!(heli.body.velocity.x > 60.0, "it keeps up: {:?}", heli.body.velocity);
    assert!(p.position.x - heli.body.position.x < 250.0, "closing: {}", p.position.x - heli.body.position.x);
}
