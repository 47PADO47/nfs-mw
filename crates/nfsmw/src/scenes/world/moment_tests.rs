use super::*;
use blackbox_chunk::ice::Key;
use nfsmw_data::world::moment::{MARKER_NAMES, Marker};

const SCENE: &str = "WMGasStation";

fn key(eye: [[f32; 3]; 2], look: [[f32; 3]; 2], sim_speed: u8) -> Key {
    Key {
        kind: 13,
        smooth: false,
        cubic_eye: true,
        cubic_look: true,
        space_eye: Space::Scene,
        space_look: Space::Scene,
        time: 0.0,
        tangent_length: [1.0, 1.0],
        eye,
        look,
        dutch: [0.0; 2],
        lens: [24.0; 2],
        near_clip: [0.5; 2],
        aperture: [37; 2],
        letterbox: [0; 2],
        sim_speed: [sim_speed; 2],
    }
}

fn moment(sim_speed: u8) -> Moment {
    let mut data = MomentData::default();
    let hash = scene_hash(SCENE);
    data.kinds.insert(hash, 1);
    let name = MARKER_NAMES[1];
    data.markers.insert(scene_hash(name), vec![Marker { position: Vec3::new(1000.0, 2000.0, 50.0), angle: 16384 }]);
    let track = Track {
        name: "Track 0".into(),
        start: 0.0,
        length: 3.0,
        keys: vec![key([[0.0, -8.0, 5.0], [10.0, -8.0, 2.0]], [[0.0, 3.0, 2.0], [0.0, 3.0, 2.0]], sim_speed)],
    };
    data.cameras.insert(hash, vec![track]);
    Moment { data: Some(Arc::new(data)), ..Default::default() }
}

fn facts(pursuers: &[Vec3]) -> Facts<'_> {
    Facts { touch_age: Some(0.5), player: Vec3::new(1010.0, 2000.0, 50.0), forward: Vec3::X, pursuers, arrested: false }
}

fn flat(_: Vec3) -> Option<f32> {
    None
}

const BEHIND: Vec3 = Vec3::new(1000.0, 2000.0, 50.0);

#[test]
fn a_cop_right_behind_is_in_reach_up_to_eighty_metres_and_one_right_ahead_only_to_twenty() {
    let player = Vec3::ZERO;
    let forward = Vec3::X;
    assert!(pursuer_in_reach(player, forward, &[Vec3::new(-79.0, 0.0, 0.0)]));
    assert!(!pursuer_in_reach(player, forward, &[Vec3::new(-81.0, 0.0, 0.0)]));
    assert!(pursuer_in_reach(player, forward, &[Vec3::new(19.0, 0.0, 0.0)]));
    assert!(!pursuer_in_reach(player, forward, &[Vec3::new(21.0, 0.0, 0.0)]));
    // Abreast: the half way, 50 m.
    assert!(pursuer_in_reach(player, forward, &[Vec3::new(0.0, 49.0, 0.0)]));
    assert!(!pursuer_in_reach(player, forward, &[Vec3::new(0.0, 51.0, 0.0)]));
    assert!(!pursuer_in_reach(player, forward, &[]), "no cop, no cut");
}

#[test]
fn height_does_not_count_in_the_reach() {
    assert!(pursuer_in_reach(Vec3::ZERO, Vec3::X, &[Vec3::new(-30.0, 0.0, 400.0)]));
}

#[test]
fn headings_count_turns_from_forward_towards_left() {
    assert_eq!(heading_angle(Vec3::X), 0);
    assert_eq!(heading_angle(Vec3::Y), 16384);
    assert_eq!(heading_angle(-Vec3::X), 32768);
    assert_eq!(heading_angle(-Vec3::Y), 49152);
}

#[test]
fn the_scene_is_turned_a_quarter_turn_less_than_its_marker_and_stood_at_the_marker() {
    let marker = Marker { position: Vec3::new(10.0, 20.0, 5.0), angle: 16384 };
    let upright = anchor_of(marker, flat);
    assert_eq!(upright.turn, 0);
    assert!((upright.to_render([1.0, 2.0, 3.0]) - Vec3::new(11.0, 22.0, 8.0)).length() < 1e-4);
    // A marker facing -x: the scene's forward axis (+y) points to -x.
    let west = anchor_of(Marker { angle: 32768, ..marker }, flat);
    assert!((west.to_render([0.0, 1.0, 0.0]) - Vec3::new(9.0, 20.0, 5.0)).length() < 1e-4);
    let grounded = anchor_of(marker, |_| Some(7.5));
    assert_eq!(grounded.origin.z, 7.5, "the origin is stood on the ground");
}

#[test]
fn a_breaker_plays_its_scene_when_the_car_touched_it_and_a_cop_is_close() {
    let mut m = moment(100);
    assert_eq!(m.request(SCENE, &facts(&[BEHIND]), flat), Ok(()));
    assert!(m.is_playing() && m.hides_hud() && m.holds_busted());
    assert!(m.take_cut(), "the camera cuts in");
    assert!(!m.take_cut());
    assert!(m.status().contains("playing WMGasStation"), "{}", m.status());
}

#[test]
fn the_gates_refuse_in_order() {
    let mut m = moment(100);
    let mut f = facts(&[BEHIND]);
    f.arrested = true;
    assert_eq!(m.request(SCENE, &f, flat), Err(Refusal::Arrested));
    f.arrested = false;
    f.touch_age = None;
    assert_eq!(m.request(SCENE, &f, flat), Err(Refusal::NotTouched), "never touched");
    f.touch_age = Some(2.5);
    assert_eq!(m.request(SCENE, &f, flat), Err(Refusal::NotTouched), "touched too long ago");
    f.touch_age = Some(2.0);
    assert_eq!(m.request(SCENE, &facts_without_cops(&f), flat), Err(Refusal::NoPursuer));
    assert_eq!(m.request("WMNothing", &facts(&[BEHIND]), flat), Err(Refusal::NoScene));
    assert!(!m.is_playing());
    assert_eq!(m.request(SCENE, &f_with(&f, &[BEHIND]), flat), Ok(()));
    assert_eq!(m.request(SCENE, &facts(&[BEHIND]), flat), Err(Refusal::Playing), "one cut at a time");
}

fn facts_without_cops<'a>(f: &Facts<'a>) -> Facts<'a> {
    f_with(f, &[])
}

fn f_with<'a>(f: &Facts<'_>, pursuers: &'a [Vec3]) -> Facts<'a> {
    Facts { touch_age: f.touch_age, player: f.player, forward: f.forward, pursuers, arrested: f.arrested }
}

#[test]
fn a_structure_far_from_every_marker_plays_nothing() {
    let mut m = moment(100);
    let chaser = [Vec3::new(1070.0, 2000.0, 50.0)];
    let mut f = facts(&chaser);
    f.player = Vec3::new(1100.0, 2000.0, 50.0);
    assert_eq!(m.request(SCENE, &f, flat), Err(Refusal::NoMarker));
}

#[test]
fn the_cut_runs_for_the_length_of_the_track_and_hands_the_camera_back() {
    let mut m = moment(100);
    m.request(SCENE, &facts(&[BEHIND]), flat).unwrap();
    m.take_cut();
    m.advance(2.9);
    assert!(m.is_playing());
    m.advance(0.2);
    assert!(!m.is_playing() && !m.hides_hud() && !m.holds_busted());
    assert!(m.take_cut(), "the camera cuts back");
    assert_eq!(m.sim_speed(), 1.0);
    assert!(m.lens(16.0 / 9.0).is_none());
}

#[test]
fn the_world_slows_to_the_tracks_speed_while_it_plays() {
    let mut m = moment(60);
    assert_eq!(m.sim_speed(), 1.0, "before the cut");
    m.request(SCENE, &facts(&[BEHIND]), flat).unwrap();
    assert!((m.sim_speed() - 0.6).abs() < 1e-6);
}

#[test]
fn the_camera_starts_at_the_tracks_first_eye_in_world_space_and_moves() {
    let mut m = moment(100);
    m.request(SCENE, &facts(&[BEHIND]), flat).unwrap();
    // The marker's angle is a quarter turn, so the scene is not turned: its origin is the marker.
    let start = m.lens(4.0 / 3.0).unwrap();
    assert!((start.position - Vec3::new(1000.0, 1992.0, 55.0)).length() < 0.01, "{:?}", start.position);
    m.advance(3.0 - 0.001);
    let end = m.lens(4.0 / 3.0).unwrap();
    assert!((end.position - Vec3::new(1010.0, 1992.0, 52.0)).length() < 0.05, "{:?}", end.position);
    assert!(start.forward.dot(end.forward) < 1.0);
}

#[test]
fn a_cut_whose_data_is_missing_never_plays() {
    let mut m = Moment::default();
    assert_eq!(m.request(SCENE, &facts(&[BEHIND]), flat), Err(Refusal::NoScene));
    assert!(m.status().contains("not played"));
}

/// A gas station of the installed city: physics position of its roof (`docs/specs/pursuit-breakers.md`).
const GAS_STATION: [f32; 3] = [-2147.85, 157.14, 1156.97];

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_gas_station_cut_looks_at_the_station_it_was_authored_for() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let data = nfsmw_data::world::moment::load(&dir).unwrap();
    let hash = scene_hash(SCENE);
    let roof = super::super::space::to_render(GAS_STATION);
    // The player stands at the station facing the marker's own way, so the marker nearest the roof wins.
    let (marker, score) = (0..8u16)
        .map(|i| data.closest_marker(hash, roof, i * 8192).unwrap())
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    assert!(score < 15.0, "a marker stands by the station: {score}");
    let track = data.track(hash, 0).unwrap();
    let anchor = anchor_of(marker, flat);
    for t in [0.0, 0.5, 1.0] {
        let sample = track.sample(t).unwrap();
        let eye = place(&anchor, sample.eye_space, sample.eye).unwrap();
        let look = place(&anchor, sample.look_space, sample.look).unwrap();
        let from_roof = |p: Vec3| Vec3::new(p.x - roof.x, p.y - roof.y, 0.0).length();
        eprintln!(
            "t {t}: eye {:.1} m and look {:.1} m from the roof, eye {:.1} m above it",
            from_roof(eye),
            from_roof(look),
            eye.z - roof.z
        );
        assert!(from_roof(look) < 12.0, "the camera looks at the station at t {t}");
        assert!((4.0..40.0).contains(&from_roof(eye)), "the eye stands back from it at t {t}");
        assert!(eye.z > roof.z - 15.0 && eye.z < roof.z + 25.0, "at t {t}");
    }
}
