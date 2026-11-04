//! The respawn after a paid bust in the installed world: the car goes to the marker of the last visited safe house.
//! Skipped without `NFSMW_GAME_DIR` (`docs/specs/busted-aftermath.md` section 7).

use glam::Vec3;
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::{CarRecord, CareerRecord};

use crate::career::{Career, CareerData, CareerHandle};
use crate::scenes::world::drive::STEP;
use crate::scenes::world::entry_zones::ZoneKind;
use crate::scenes::world::{handbrake, open_arrest_fixture};
use crate::viewer::Scene;

fn career(dir: &game_install::GameDir) -> Career {
    let data = CareerData::load(dir).expect("the career tables load");
    let key = data.cars[0].frontend_key;
    let mut career = Career::new("respawn", std::sync::Arc::new(data), &NewCareer::default()).unwrap();
    career.stable.cars[0] = CarRecord {
        handle: 1,
        frontend_key: key,
        flags: 0xF_0000 | CarRecord::FLAG_OWNED,
        career_index: 0,
        customization_index: 0,
        ..CarRecord::free()
    };
    career.stable.careers[0] = CareerRecord::new(0);
    career.settings.car_handle = 1;
    career.settings.cash = 10_000;
    career
}

#[test]
#[ignore = "needs the installed world and the career tables"]
fn installed_paid_bust_puts_the_car_back_at_the_last_safe_house_marker() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut fixture) = open_arrest_fixture() else { return };
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let shared = CareerHandle::shared();
    shared.set(career(&dir));
    fixture.scene.set_career(shared.clone(), None);
    let stopped = handbrake();
    assert!(fixture.run_to_arrest(600), "the cop busts the stopped player");
    let start = Vec3::from_array(fixture.scene.player_car().expect("the player car").position);

    // The player visited a safe house 60 m down the road; the marker is where the car returns to.
    let marker = start + Vec3::new(60.0, 0.0, 0.0);
    fixture.scene.career.respawn.visit(ZoneKind::SafeHouse, Some(marker));
    let mut ticket = None;
    // The hold, the overlay and the arrest scene (up to 14 s for the rough scenes).
    for _ in 0..1800 {
        fixture.scene.update(&mut fixture.renderer, &stopped, STEP);
        ticket = fixture.scene.take_bust_ticket();
        if ticket.is_some() {
            break;
        }
    }
    assert!(ticket.is_some(), "the bust reaches its ticket");
    assert!(fixture.scene.pay_bust(false), "cash pays the ticket");
    assert!(fixture.scene.career.respawn.pending, "the respawn waits for the game to run");

    // The road search can take a few frames while the area streams in: wait for the car to arrive.
    let mut end = start;
    for _ in 0..900 {
        fixture.scene.update(&mut fixture.renderer, &stopped, STEP);
        end = Vec3::from_array(fixture.scene.player_car().expect("the player car").position);
        if !fixture.scene.career.respawn.pending && end.distance(start) > 30.0 {
            break;
        }
    }
    assert!(!fixture.scene.career.respawn.pending, "the respawn was carried out");
    assert!(end.distance(start) > 30.0, "the car left the bust spot: {start} -> {end}");
    assert!(end.distance(marker) < 40.0, "the car stands near the marker: {end} vs {marker}");
    shared.clear();
}

/// A rap sheet jump from the safe house starts a new world, which asks for the jump before its car exists: the car must
/// still land on the marker facing its `Rotation`, and the bounty place's chase must start there (`start_at_point`). The
/// point is Riverfront Stadium's (bounty place 11), as the F5 reference measures it.
#[test]
#[ignore = "needs the installed world"]
fn installed_jump_asked_before_the_world_has_a_car_lands_on_the_marker_and_starts_its_chase() {
    use crate::scenes::world::{DriveOptions, Options, WorldScene};
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(root).unwrap();
    let options = Options {
        at: Some([940.0, -596.0]),
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 1500.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
    };
    let mut scene = WorldScene::open(&dir, options).unwrap();
    let heading = 257.3_f32.to_radians();
    let point = crate::career::JumpPoint { at: Vec3::new(939.8, -595.6, 110.9), heading: Some(heading), chase: true };
    assert!(scene.start_at_point(point), "the jump is taken before the car exists");
    assert!(scene.players.lead().is_none(), "the world has no car yet");

    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    let stopped = handbrake();
    let mut chased = false;
    for _ in 0..3000 {
        scene.update(&mut renderer, &stopped, STEP);
        chased |= scene.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing());
        if scene.ready() && chased {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let drive = scene.players.lead().expect("the car is made");
    let at = drive.position();
    assert!(Vec3::new(at.x, at.y, 0.0).distance(Vec3::new(939.8, -595.6, 0.0)) < 1.5, "on the marker: {at}");
    let turn =
        (drive.heading() - heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    assert!(turn.abs() < 0.1, "facing the marker's Rotation: off by {:.1} deg", turn.to_degrees());
    assert!(chased, "the bounty place's chase starts once the car stands there");
}
