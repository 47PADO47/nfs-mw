//! The installed arrest scenes against the rules the world plays them by: which car channel is the perp's and which the
//! police car's, and how a scene placed at the player's heading stands its cars. Needs the game (set `NFSMW_GAME_DIR`).
//! Facts: `docs/specs/nis-player.md` sections 5.2 and 7.1.

use blackbox_anim::Clip;
use blackbox_hash::bstring_hash;
use game_install::GameDir;
use glam::{Vec2, Vec3};
use nfsmw_data::nis::NisBundle;

use super::cars::{self, CarTrack};
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::scene_space::Anchor;

/// The arrests the chooser reaches (`ai/police/busted/arrest.rs`).
const REACHABLE: [&str; 16] = [
    "ArrestM01",
    "ArrestM16",
    "ArrestF02",
    "ArrestF18",
    "ArrestM01b",
    "ArrestM16b",
    "ArrestF02b",
    "ArrestF18b",
    "ArrestM06",
    "ArrestM19",
    "ArrestF06",
    "ArrestF07",
    "ArrestM06b",
    "ArrestM19b",
    "ArrestF06b",
    "ArrestF07b",
];
/// The calm arrests, whose description says the cop puts the perp into the car (the low heat set).
const INTO_THE_CAR: [&str; 8] =
    ["ArrestM01", "ArrestM16", "ArrestF02", "ArrestF18", "ArrestM01b", "ArrestM16b", "ArrestF02b", "ArrestF18b"];
const PERP_CAR: usize = 0;
const POLICE_CAR: usize = 8;

fn install() -> GameDir {
    GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR"))).unwrap()
}

fn track(tracks: &[CarTrack], slot: usize) -> &CarTrack {
    tracks.iter().find(|track| track.slot == slot).unwrap_or_else(|| panic!("no car slot {slot}"))
}

/// Where the perp's pelvis is in scene space when the perp's clip ends.
fn perp_end(bundle: &NisBundle) -> Vec3 {
    let banks = &bundle.anim.banks;
    for record in &bundle.scene.characters {
        let found = banks.iter().find_map(|(object, bank)| {
            let animation = bank.iter().find(|a| bstring_hash(&a.name) == record.skeletal_anim_hash)?;
            Some((animation.name.clone(), Clip::read(object, animation).ok()?))
        });
        let Some((name, clip)) = found else { continue };
        if !name.contains("_Perp_") {
            continue;
        }
        let skeleton = bundle.anim.skeletons.iter().find(|s| bstring_hash(&s.name) == record.skeleton_name_hash);
        let skeleton = skeleton.expect("the perp's skeleton");
        let pose = clip.sample_pose(clip.duration_secs(), &skeleton.bind_pose());
        return record.local_matrix.transform_point3(skeleton.world(&pose)[1].w_axis.truncate());
    }
    panic!("{}: no perp", bundle.name);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_reachable_arrest_has_the_perps_car_and_one_police_car() {
    let dir = install();
    for name in REACHABLE {
        let bundle = NisBundle::load(&dir, name).unwrap();
        assert!(bundle.scene.scene_type.places_at_marker(), "{name}: an arrest stands at its marker");
        let slots: Vec<usize> = cars::read_tracks(&bundle.anim.banks).iter().map(|track| track.slot).collect();
        assert_eq!(slots, [PERP_CAR, POLICE_CAR], "{name}: car1 and cop1 only");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_perp_is_put_into_the_police_car_not_back_into_car1() {
    let dir = install();
    for name in INTO_THE_CAR {
        let bundle = NisBundle::load(&dir, name).unwrap();
        let tracks = cars::read_tracks(&bundle.anim.banks);
        let end = perp_end(&bundle).truncate();
        let at = |slot| track(&tracks, slot).spot(&Anchor::IDENTITY, 0.0).position.truncate();
        let (to_police, to_own) = (end.distance(at(POLICE_CAR)), end.distance(at(PERP_CAR)));
        assert!(to_police < 1.5, "{name}: the perp ends {to_police:.2} m from cop1");
        assert!(to_own > 5.0, "{name}: the perp ends {to_own:.2} m from car1");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn an_arrest_placed_at_the_players_heading_stands_its_cars_along_the_road() {
    let dir = install();
    let origin = Vec3::new(100.0, -50.0, 3.0);
    for name in REACHABLE {
        let bundle = NisBundle::load(&dir, name).unwrap();
        let tracks = cars::read_tracks(&bundle.anim.banks);
        for heading in [0.0_f32, 1.0, 2.5, -2.0] {
            let nose = Vec2::from_angle(heading);
            let anchor = Anchor::facing(origin, heading_angle(nose.extend(0.0)));
            let spot = |slot| track(&tracks, slot).spot(&anchor, 0.0);
            let angle_to = |slot| nose.angle_to((spot(slot).rotation * Vec3::X).truncate()).to_degrees().abs();
            assert!(angle_to(POLICE_CAR) < 10.0, "{name}: cop1 is {:.0} degrees off the road", angle_to(POLICE_CAR));
            assert!(angle_to(PERP_CAR) < 30.0, "{name}: car1 is {:.0} degrees off the road", angle_to(PERP_CAR));
            let behind = (spot(PERP_CAR).position - origin).truncate().dot(nose);
            assert!(behind < -1.0, "{name}: car1 stands {behind:.1} m along the nose, behind the scene's origin");
        }
    }
}
