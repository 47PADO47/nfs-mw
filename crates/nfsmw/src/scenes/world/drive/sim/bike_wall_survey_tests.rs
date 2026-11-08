//! The installed world's roads ridden into their edges at bike speed (`docs/specs/ride6-feel-round14.md`, section 3): a survey that needs the
//! game, run by hand.

use super::bike_curb_tests::bike_sim;
use super::*;
use crate::scenes::world::drive::ground::WorldGround;

/// The installed world's roads ridden into their edges at bike speed, and what became of the bike (`docs/specs/ride6-feel-round14.md`,
/// section 3): from 40 places on its roads, the stick held to one side for 1.5 s at 40, 50 and 60 m/s, 8 s in all (the fall, the lying bike,
/// the stand-up), counting the rides on which the bike ended more than a metre under the ground below it. `RIDE6_BIKE_TOML` rides an imported
/// bike; `NFSMW_GAME_DIR` names the install.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn survey_wall_hits_at_bike_speed_in_the_installed_world() {
    use crate::scenes::world::{road, space};
    use game_install::GameDir;
    use nfsmw_data::car::physics::PhysicsData;
    use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let centre = index.centre();
    let physics = PhysicsData::load(&dir).unwrap();
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in index.sections.iter().filter(|s| s.is_spatial() && s.distance_to(centre[0], centre[1]) <= 1200.0) {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        blackbox_collision::read_collision_packs(&bytes).unwrap().into_iter().for_each(|pack| collision.insert(pack));
    }
    let ground = WorldGround { collision: &collision, surfaces: &physics.surfaces };
    let probe = |x: f32, y: f32| {
        let [px, _, pz] = space::to_physics(Vec3::new(x, y, 0.0));
        let top = collision.ground(px, pz, 1000.0, -1000.0)?.point[1] + 1.0;
        road::probe(&collision, x, y, top)
    };
    let spots: Vec<_> = (0..40)
        .filter_map(|i| {
            let (a, r) = (i as f32 * 0.157, 150.0 + 20.0 * i as f32);
            road::find([centre[0] + r * a.cos(), centre[1] + r * a.sin()], probe)
        })
        .collect();
    let (mut rides, mut falls, mut lost) = (0, 0, 0);
    for spot in &spots {
        for steer in [0.5_f32, -0.5, 1.0, -1.0] {
            for speed in [40.0_f32, 50.0, 60.0] {
                let (mut sim, mut props) = (bike_sim(), PropWorld::default());
                if !sim.place_moving(&ground, *spot, speed) {
                    continue;
                }
                rides += 1;
                let mut under = f32::MAX;
                for tick in 0..480 {
                    let throttle = ((speed - sim.telemetry().speed_mps) * 0.2 + 0.1).clamp(0.0, 0.5);
                    let steer = if (60..150).contains(&tick) { steer } else { 0.0 };
                    let input = DriveInput { throttle, steer, ..DriveInput::default() };
                    sim.step(&input, &ground, Some((&collision, &mut props)));
                    // The ground below the bike's origin (physics space), from a metre above it.
                    let at = sim.ride.as_ref().expect("a bike").bike.position();
                    if let Some(hit) = collision.ground(at.x, at.z, at.y + 1.0, at.y - 200.0) {
                        under = under.min(at.y - hit.point[1]);
                    }
                }
                falls += usize::from(sim.ride.as_ref().is_some_and(|ride| ride.bike.rider_off()));
                if under < -1.0 {
                    lost += 1;
                    println!(
                        "under the ground by {:.1} m: spot {:?}, stick {steer}, {speed} m/s",
                        -under, spot.position
                    );
                }
            }
        }
    }
    println!("{rides} rides at the roadside at bike speed, {falls} threw the rider, {lost} ended under the ground");
    assert_eq!(lost, 0);
}
