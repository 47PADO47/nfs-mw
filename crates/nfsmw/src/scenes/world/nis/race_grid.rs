//! Where a race's intro NIS leaves its cars: the end pose of each car channel, which the race takes as that grid slot
//! (`docs/specs/ai-racers-route.md` section 3). The scene is read here once, when the race is planned; the player reads
//! it again to play it.

use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::nis::NisBundle;

use super::car_world::heading_of;
use super::cars::{CAR_SLOTS, CarTrack, read_tracks};
use super::player::SCENE_FPS;
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::scene_space::{Anchor, placed};

/// The end pose of each racer channel (`car1` to `car8`, slots 0 to 7) of the scene `scene`, stood at the race start
/// `start` as the player stands a race's intro (`race/intro.rs`): a scene of type 0, 2 or 3 at the start marker facing
/// its heading, one of type 1 or 4 where it was authored. Channels the scene does not drive are absent. Each end keeps the
/// start's height: a clip's height is not the road's (the player stands its cars on the ground under them, and
/// `IntroNis10` ends `car2` 3 m below the others), and the racer's placement finds the road under the start's height as
/// it does for any grid slot.
pub(in crate::scenes::world) fn intro_grid(
    dir: &GameDir,
    scene: &str,
    start: Spawn,
) -> Result<Vec<(usize, Spawn)>, String> {
    let bundle = NisBundle::load(dir, scene).map_err(|e| format!("{e:#}"))?;
    let forward = Vec3::new(start.heading.cos(), start.heading.sin(), 0.0);
    let anchor = placed(bundle.scene.scene_type, Anchor::facing(start.position, heading_angle(forward)));
    let ends = channel_ends(&read_tracks(&bundle.anim.banks), &anchor);
    Ok(ends
        .into_iter()
        .map(|(slot, end)| {
            (slot, Spawn { position: Vec3::new(end.position.x, end.position.y, start.position.z), ..end })
        })
        .collect())
}

/// Samples a rolling intro's race cars at the scene cut, with speed from the preceding authored frame.
pub(in crate::scenes::world) fn rolling_intro_grid(
    dir: &GameDir,
    scene: &str,
    start: Spawn,
) -> Result<Vec<(usize, Spawn, f32)>, String> {
    let bundle = NisBundle::load(dir, scene).map_err(|error| format!("{error:#}"))?;
    let forward = Vec3::new(start.heading.cos(), start.heading.sin(), 0.0);
    let anchor = placed(bundle.scene.scene_type, Anchor::facing(start.position, heading_angle(forward)));
    if bundle.scene.vanish_frame <= 0 {
        return Err(format!("{scene} has no rolling handoff frame"));
    }
    let cut = bundle.scene.vanish_frame as f32 / SCENE_FPS;
    let prior = (bundle.scene.vanish_frame - 1) as f32 / SCENE_FPS;
    let grid = read_tracks(&bundle.anim.banks)
        .into_iter()
        .filter(|track| track.slot < CAR_SLOTS / 2)
        .filter_map(|track| {
            let at = track.spot(&anchor, cut);
            let before = track.spot(&anchor, prior);
            let speed = (at.position - before.position).length() * SCENE_FPS;
            (speed.is_finite() && speed > 0.0).then_some((
                track.slot,
                Spawn { position: at.position, heading: heading_of(at.rotation) },
                speed,
            ))
        })
        .collect();
    Ok(grid)
}

/// The end pose of each racer channel of `tracks` for a scene placed at `anchor`.
pub(super) fn channel_ends(tracks: &[CarTrack], anchor: &Anchor) -> Vec<(usize, Spawn)> {
    tracks
        .iter()
        .filter(|track| track.slot < CAR_SLOTS / 2)
        .map(|track| {
            let spot = track.spot(anchor, track.end_time());
            (track.slot, Spawn { position: spot.position, heading: heading_of(spot.rotation) })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires installed IntroNisDD data; set NFSMW_GAME_DIR"]
    fn installed_opening_roll_uses_scene_cut_and_each_actors_velocity() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("installed NIS data")).unwrap();
        let marker = Spawn { position: Vec3::new(2100.0, -529.0, 114.0), heading: 0.2 };
        let grid = rolling_intro_grid(&dir, "IntroNisDD", marker).unwrap();
        assert_eq!(grid.iter().map(|(slot, ..)| *slot).collect::<Vec<_>>(), [0, 1]);
        let (_, player, player_speed) = grid[0];
        let (_, opponent, opponent_speed) = grid[1];
        assert!((2094.0..2098.0).contains(&player.position.x), "{player:?}");
        assert!((2.7..3.4).contains(&player.position.distance(opponent.position)), "{grid:?}");
        assert!(player.heading.abs() < 0.05 && opponent.heading.abs() < 0.05, "{grid:?}");
        for speed in [player_speed, opponent_speed] {
            assert!((32.0..35.0).contains(&speed), "{grid:?}");
        }
    }

    /// `16_1_1_circuit`'s start marker (`16_1_1_startgrid`) in render space, and its heading.
    const START: Vec3 = Vec3::new(1631.384, 382.1315, 94.96969);
    const START_DEG: f32 = -173.8037;

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn intro_nis_10_leaves_four_cars_side_by_side_behind_the_start() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(std::path::PathBuf::from(dir)).unwrap();
        // The heading as the race's spawn heading (`race/grid.rs`): physics (x, y, z) is render (z, -x, y).
        let nose = nfsmw_data::gameplay::world_heading(START_DEG);
        let heading = (-nose.x).atan2(nose.z);
        let ends = intro_grid(&dir, "IntroNis10", Spawn { position: START, heading }).unwrap();
        assert_eq!(ends.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(), [0, 1, 2, 3]);
        let forward = Vec3::new(heading.cos(), heading.sin(), 0.0);
        let side = Vec3::Z.cross(forward);
        let mut across: Vec<f32> = Vec::new();
        for (slot, end) in &ends {
            let along = (end.position - START).dot(forward);
            assert!((-3.5..-2.5).contains(&along), "car{} stands {along:.2} m along the start", slot + 1);
            let turn =
                (end.heading - heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            assert!(turn.abs() < 0.05, "car{} is turned {turn:.3} rad from the start", slot + 1);
            assert_eq!(end.position.z, START.z, "car{} keeps the start's height", slot + 1);
            across.push((end.position - START).dot(side));
        }
        across.sort_by(f32::total_cmp);
        for pair in across.windows(2) {
            assert!((2.5..3.1).contains(&(pair[1] - pair[0])), "the row's gaps {across:?}");
        }
    }
}
