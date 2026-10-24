//! A hiding spot through the streamed world: the installed HIDDEN zone holds a parked player at its height, the latch
//! hides it, the evade meter fills several times faster, the HUD mark is raised and the console lists the spot.

use super::*;
use crate::input::{ActionState, Bindings};
use crate::scenes::world::{DriveOptions, Options, WorldScene, drive::STEP, road::Spawn};
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use blackbox_roads::from_zone_space;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

/// The seconds the cool-down takes to end a heat 1 chase that nobody sees (the grace, then the meter), unhidden, and
/// the longest a run lasts.
const LONGEST: f32 = 60.0;

struct Run {
    scene: WorldScene,
    renderer: NullBackend,
    pad: ActionState,
}

/// A parked player at `at` (physics space) in the streamed city, and a chase against it that no cop can see.
fn start(at: Vec3) -> Run {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let map = space::to_render(at.to_array());
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some([map.x, map.y]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    let mut pad = ActionState::default();
    pad.update_for_test(&Bindings::default(), &[KeyCode::Space], &[], false);
    for _ in 0..2000 {
        scene.update(&mut renderer, &pad, STEP);
        if scene.ready() && scene.residency.complete() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready() && scene.residency.complete(), "the streamed tiles must be resident");
    {
        let ground = WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
        let sim = scene.players.lead_mut().unwrap().sim_mut().unwrap();
        let spawn = Spawn { position: space::to_render(at.to_array()), heading: 0.0 };
        assert!(sim.place(&ground, spawn), "the player stands at the spot");
    }
    for _ in 0..120 {
        scene.update(&mut renderer, &pad, STEP);
    }
    let performance = scene.players.lead().unwrap().performance();
    let player = scene.players.lead().unwrap().ai_body().position;
    let world = scene.traffic.as_mut().unwrap();
    world.cars.clear();
    world.configure(0, 0);
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    for row in &mut rows {
        row.cops.clear();
    }
    world.player_position = Some(player);
    // Stock starting career bounds keep this heat-1 comparison at heat 1.
    // A world without a career uses the installed unscaled heat growth time.
    world.set_career_bin(Some(16));
    world.start_pursuit(1, rows, performance);
    assert_eq!(world.police.level(), Some(1));
    Run { scene, renderer, pad }
}

impl Run {
    /// Seconds until the chase ends (the cops give up), or [`LONGEST`].
    fn seconds_to_evade(&mut self) -> f32 {
        for tick in 0..(LONGEST / STEP) as usize {
            self.scene.update(&mut self.renderer, &self.pad, STEP);
            let police = &self.scene.traffic.as_ref().unwrap().police;
            assert!(police.level().is_none_or(|level| level == 1), "the fixture stays at heat 1");
            if !self.scene.traffic.as_ref().unwrap().is_pursuing() {
                return tick as f32 * STEP;
            }
        }
        LONGEST
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a hiding spot"]
fn a_parked_player_in_an_installed_hiding_spot_escapes_several_times_faster() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let zones = index.track_zones.expect("the track has path zones");
    // The spot under an overpass on the road at physics (-2194, 2730), 106.5 m up.
    let zone =
        zones.zones.iter().find(|z| z.kind == 8 && (z.elevation - 106.5).abs() < 0.01).expect("the installed spot");
    let centre = zone.polygon.iter().copied().sum::<Vec2>() / zone.polygon.len() as f32;
    let inside = from_zone_space(centre, zone.elevation + 0.7);

    let mut hidden = start(inside);
    let report = crate::scenes::world::ai::inspect::hiding(&hidden.scene).unwrap();
    assert!(report.contains("34 hiding spots on this track"), "{report}");
    assert!(report.contains("PLAYER INSIDE"), "the console says the player is in the spot: {report}");
    let with_spot = hidden.seconds_to_evade();

    // The same chase, parked 160 m down the road with no spot around.
    let outside = {
        let world = hidden.scene.traffic.as_ref().unwrap();
        let near = inside + Vec3::new(0.0, 0.0, 160.0);
        let hit = blackbox_roads::closest_segment(
            &world.network,
            &world.index,
            near,
            Vec3::ZERO,
            0.0,
            blackbox_roads::SegmentFilter::default(),
        )
        .expect("a road runs on");
        assert!(!zones.zones.iter().any(|z| z.kind == 8 && z.contains(blackbox_roads::to_zone_space(hit.point))));
        hit.point + Vec3::Y * 0.7
    };
    drop(hidden);
    let mut open = start(outside);
    let without_spot = open.seconds_to_evade();
    eprintln!("evaded after {with_spot:.1} s in the spot, {without_spot:.1} s in the open");
    assert!(with_spot < 20.0, "heat 1 with the multiplier 4: {with_spot}");
    assert!(without_spot > with_spot + 6.0, "{without_spot} s in the open against {with_spot} s in the spot");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a hiding spot"]
fn the_hiding_mark_shows_in_the_hud_frame_while_a_spot_hides_a_player_who_is_losing_the_cops() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let zones = index.track_zones.expect("the track has path zones");
    let zone = zones.zones.iter().find(|z| z.kind == 8 && (z.elevation - 106.5).abs() < 0.01).unwrap();
    let centre = zone.polygon.iter().copied().sum::<Vec2>() / zone.polygon.len() as f32;
    let mut run = start(from_zone_space(centre, zone.elevation + 0.7));
    let mut shown = false;
    for _ in 0..(8.0 / STEP) as usize {
        run.scene.update(&mut run.renderer, &run.pad, STEP);
        let frame = run.scene.hud_frame().unwrap();
        shown |= frame.pursuit.is_some_and(|p| p.hiding.is_some());
    }
    assert!(shown, "a hidden player's chase raises the hiding mark");
}
