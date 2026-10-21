use nfsmw_career::story::{FinalRaceEvent, GpsTarget};
use nfsmw_data::gameplay::{BoxTrigger, EventMarker, Placement, to_world};

use super::*;

fn place() -> Place {
    Place { spawn: Spawn { position: Vec3::ZERO, heading: 0.0 }, speed: 40.0 }
}

fn trigger(position: Vec3, heading_deg: f32) -> BoxTrigger {
    BoxTrigger {
        key: 1,
        name: "race_bin_01/epic_pursuit/test_trigger".to_owned(),
        position,
        heading_deg,
        dimensions: [0.0; 3],
        one_shot: false,
        fire_on_exit: false,
        particle_effect: None,
        flare_spacing: None,
        time_bonus: 0.0,
        resets_player: false,
    }
}

fn markers(win: Option<BoxTrigger>, lose: Option<BoxTrigger>, barrier: Option<EventMarker>) -> FinalRaceMarkers {
    FinalRaceMarkers {
        start_grid: None,
        win_trigger: win,
        lose_trigger: lose,
        barrier_area: barrier,
        win_nis_marker: None,
        tagged_car_markers: Vec::new(),
    }
}

/// A run whose win trigger stands at the origin and faces +Z (physics space, heading 0).
fn run_with_win_at_origin() -> FinalRun {
    let win = Some(trigger(Vec3::ZERO, 0.0));
    FinalRun::new(markers(win, None, None), world_heading(0.0), None, place())
}

/// Records the effects in the order they are played.
#[derive(Default)]
struct Recorder {
    seen: Vec<String>,
}

impl FinalEffects for Recorder {
    fn place_call(&mut self, id: u16) {
        self.seen.push(format!("call {id}"));
    }

    fn open_door(&mut self) {
        self.seen.push("door".into());
    }

    fn gps_to_barrier(&mut self) {
        self.seen.push("gps".into());
    }

    fn hide_gps(&mut self) {
        self.seen.push("hide gps".into());
    }

    fn play_outro(&mut self, scene: &str) {
        self.seen.push(format!("outro {scene}"));
    }

    fn bail_pursuit(&mut self) {
        self.seen.push("bail".into());
    }

    fn total_racer(&mut self) {
        self.seen.push("total".into());
    }
}

#[test]
fn map_points_are_the_inverse_of_the_physics_conversion() {
    let physics = Vec3::new(-3004.6, 188.4, -365.6);
    assert_eq!(to_world(map_point(physics)), physics);
    assert_eq!(map_point(Vec3::new(1.0, 2.0, 3.0)), [3.0, -1.0, 2.0]);
}

#[test]
fn the_win_plane_is_crossed_from_behind_within_reach() {
    let mut run = run_with_win_at_origin();
    assert!(!run.crossed_win(Vec3::new(0.0, 0.0, -5.0)), "the first frame has no side to cross from");
    assert!(run.crossed_win(Vec3::new(0.0, 0.0, 5.0)), "from behind the plane to in front of it");
    assert!(!run.crossed_win(Vec3::new(0.0, 0.0, 6.0)), "already in front: no second crossing");
}

#[test]
fn a_crossing_far_from_the_trigger_or_the_wrong_way_is_not_the_win() {
    let mut far = run_with_win_at_origin();
    far.crossed_win(Vec3::new(400.0, 0.0, -5.0));
    assert!(!far.crossed_win(Vec3::new(400.0, 0.0, 5.0)), "400 m from the trigger");

    let mut back = run_with_win_at_origin();
    back.crossed_win(Vec3::new(0.0, 0.0, 5.0));
    assert!(!back.crossed_win(Vec3::new(0.0, 0.0, -5.0)), "from in front to behind");
}

#[test]
fn the_input_is_the_car_in_map_units_with_its_speed_in_kmh() {
    let mut run = run_with_win_at_origin();
    let input = run.input(Vec3::new(1.0, 2.0, 3.0), Vec3::Z, 50.0);
    assert_eq!(input.player_pos, [3.0, -1.0, 2.0]);
    assert!((input.player_speed_kmh - 180.0).abs() < 1e-3);
    assert_eq!(input.player_forward, [0.0, 0.0, 1.0]);
    assert!(!input.in_barrier_area && !input.in_lose_box && !input.crossed_win_plane);
}

#[test]
fn the_lose_box_and_the_barrier_area_are_read_from_their_markers() {
    let lose = BoxTrigger { dimensions: [30.0, 50.0, 60.0], ..trigger(Vec3::new(10.0, 0.0, 0.0), 0.0) };
    let barrier = EventMarker {
        key: 2,
        name: "race_bin_01/epic_pursuit/barrier_area".to_owned(),
        placement: Placement { position: Vec3::new(500.0, 0.0, 0.0), heading_deg: 0.0 },
        dimensions: None,
    };
    let mut run = FinalRun::new(markers(None, Some(lose), Some(barrier)), world_heading(0.0), None, place());
    let inside_lose = run.input(Vec3::new(10.0, 0.0, 0.0), Vec3::Z, 0.0);
    assert!(inside_lose.in_lose_box, "inside the 30 x 50 x 60 box");
    assert!(!inside_lose.in_barrier_area);
    let at_barrier = run.input(Vec3::new(510.0, 0.0, 0.0), Vec3::Z, 0.0);
    assert!(at_barrier.in_barrier_area, "within the fallback reach of the barrier area");
    assert!(!at_barrier.in_lose_box);
}

#[test]
fn a_fast_crossing_of_the_win_plane_wins_through_the_rules() {
    let mut run = run_with_win_at_origin();
    let first = run.input(Vec3::new(0.0, 0.0, -5.0), Vec3::Z, 60.0);
    assert!(run.rules.update(first).is_empty());
    let second = run.input(Vec3::new(0.0, 0.0, 5.0), Vec3::Z, 60.0);
    let events = run.rules.update(second);
    assert!(events.contains(&FinalRaceEvent::Won), "{events:?}");
    assert_eq!(apply_events(&mut Recorder::default(), &events), Settled::Won);
}

#[test]
fn a_slow_crossing_is_not_the_win() {
    let mut run = run_with_win_at_origin();
    let first = run.input(Vec3::new(0.0, 0.0, -5.0), Vec3::Z, 20.0);
    run.rules.update(first);
    let second = run.input(Vec3::new(0.0, 0.0, 5.0), Vec3::Z, 20.0);
    assert!(run.rules.update(second).is_empty(), "72 km/h is under 180");
}

#[test]
fn the_special_car_is_given_once() {
    let mut run = FinalRun::new(markers(None, None, None), Vec3::Z, Some("e3_demo_bmw"), place());
    assert_eq!(run.take_car().map(|(car, _)| car), Some("e3_demo_bmw"));
    assert_eq!(run.take_car(), None);
}

#[test]
fn a_batch_plays_in_order_and_a_win_settles_as_won() {
    let mut fx = Recorder::default();
    let events = [
        FinalRaceEvent::PlaceCall(95),
        FinalRaceEvent::Won,
        FinalRaceEvent::PlayOutroNis("EndingNis04"),
        FinalRaceEvent::BailPursuit,
    ];
    assert_eq!(apply_events(&mut fx, &events), Settled::Won);
    assert_eq!(fx.seen, ["call 95", "outro EndingNis04", "bail"]);
}

#[test]
fn the_barrier_call_opens_the_door_and_routes_the_gps_which_then_hides() {
    let mut fx = Recorder::default();
    let events = [
        FinalRaceEvent::PlaceCall(96),
        FinalRaceEvent::OpenDoor,
        FinalRaceEvent::GpsTo(GpsTarget::BarrierArea),
        FinalRaceEvent::HideGps,
    ];
    assert_eq!(apply_events(&mut fx, &events), Settled::Running);
    assert_eq!(fx.seen, ["call 96", "door", "gps", "hide gps"]);
}

#[test]
fn a_totalled_racer_loses_and_nothing_after_it_plays() {
    let mut fx = Recorder::default();
    let events = [FinalRaceEvent::PlaceCall(96), FinalRaceEvent::Wrecked, FinalRaceEvent::PlaceCall(95)];
    assert_eq!(apply_events(&mut fx, &events), Settled::Lost);
    assert_eq!(fx.seen, ["call 96", "total"]);
}

#[test]
fn no_events_leave_the_race_running() {
    assert_eq!(apply_events(&mut Recorder::default(), &[]), Settled::Running);
}
