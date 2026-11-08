//! The collapse round a stopped player in the installed city: heat-5 cops that come up behind a player who holds the
//! handbrake take the pull-over goal and drive to their ring slots (spec `ai-pursuit-formations.md` section 5.1).

use super::*;
use crate::scenes::world::{DriveOptions, Options, WorldScene, drive::STEP};
use crate::viewer::Scene;
use game_install::GameDir;
use std::fmt::Write as _;

/// Per step and cop: the goal, whether it has arrived, the gear, its speed and its distance to the player.
#[derive(Default)]
struct Tally {
    samples: u32,
    pull_over: u32,
    arrived: u32,
    reverse: u32,
    near: u32,
    collapse_steps: u32,
    first_collapse: Option<u32>,
    /// The most cops within 8 m of the player at one step.
    ringed: usize,
}

#[test]
#[ignore = "needs installed player/cop physics, road network and world collision"]
fn installed_heat_five_cops_collapse_round_a_stopped_player() {
    let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(root).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some([2151.0, 1399.0]),
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
    let stopped = super::arrest_fixture::handbrake();
    for _ in 0..2000 {
        scene.update(&mut renderer, &stopped, STEP);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready(), "the installed player must be placed");
    for _ in 0..60 {
        scene.update(&mut renderer, &stopped, STEP);
    }
    // Heat 5's own ground cop, so the chase does not send the cars away as surplus of a type it does not want.
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    let name = nfsmw_data::pursuit::ground_cops(&rows[4])
        .max_by_key(|record| record.count.saturating_mul(record.chance))
        .expect("heat 5 has an ordinary ground cop")
        .name
        .clone();
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, std::slice::from_ref(&name)).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let drive = scene.players.lead().unwrap();
    let heading = drive.heading();
    let performance = drive.performance();
    let at = drive.ai_body().position;
    let forward = Vec3::from_array(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
    let mut world = scene.traffic.take().unwrap();
    world.cars.clear();
    world.configure(0, 0);
    let model = world.cop_models.iter().position(|m| m.model.name.eq_ignore_ascii_case(&name)).unwrap();
    for back in [22.0, 28.0, 34.0, 40.0] {
        let spot = at - forward * back;
        let mut nav = RoadNav::traffic();
        assert!(nav.init_at_point(&world.network, &world.index, spot, forward, false));
        let spot = nav.position;
        let car = AiCar::place(
            &world.cop_models[model].model,
            Role::Traffic,
            nav,
            Spawn { position: space::to_render(spot.to_array()), heading },
            Start { speed: 8.0, stagger: 0, cruise: Cruise::default(), patrol: true },
            &WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces },
        )
        .expect("an installed cop stands behind the player");
        world.cars.push(car);
    }
    world.player_position = Some(at);
    start_with_patrols(&mut world, 5, rows, performance);
    let joined = world.cars.iter().filter(|car| car.is_cop()).count();
    scene.traffic = Some(world);
    let mut tally = Tally::default();
    let mut log = String::new();
    for tick in 1..=600u32 {
        scene.update(&mut renderer, &stopped, STEP);
        let world = scene.traffic.as_ref().unwrap();
        if world.arrest_active() || !world.is_pursuing() {
            let _ = writeln!(log, "tick {tick}: arrest {} pursuing {}", world.arrest_active(), world.is_pursuing());
            break;
        }
        if world.collapse_active() {
            tally.collapse_steps += 1;
            tally.first_collapse.get_or_insert(tick);
        }
        let player = scene.players.lead().unwrap().ai_body().position;
        let near_chase =
            |car: &&AiCar| car.is_cop() && !car.is_fleeing() && car.physics_position().distance(player) < 40.0;
        let ringed =
            world.cars.iter().filter(near_chase).filter(|car| car.physics_position().distance(player) < 8.0).count();
        tally.ringed = tally.ringed.max(ringed);
        for car in world.cars.iter().filter(near_chase) {
            let goal = car.goal_drive();
            tally.samples += 1;
            tally.pull_over += u32::from(goal.kind == Some(blackbox_pursuit::goals::GoalKind::PullOver));
            tally.arrived += u32::from(goal.arrived());
            tally.reverse += u32::from(car.in_reverse_gear());
            tally.near += u32::from(car.physics_position().distance(player) < 8.0);
        }
        if tick % 30 == 0 {
            let cars: Vec<String> = world
                .cars
                .iter()
                .filter(|car| car.is_police())
                .map(|car| {
                    let goal = car.goal_drive();
                    format!(
                        "cop{} flee{} {:.1}m {:.1}m/s {:?}{}{} {}",
                        car.is_cop(),
                        car.is_fleeing(),
                        car.physics_position().distance(player),
                        car.speed(),
                        goal.kind,
                        if goal.arrived() { " arrived" } else { "" },
                        if car.in_reverse_gear() { " R" } else { "" },
                        car.cop_drive_trace().map_or("-", |d| d.0),
                    )
                })
                .collect();
            let _ = writeln!(
                log,
                "tick {tick}: formation {:?} collapse {} sight {} bust {:.2} {cars:?}",
                world.police.last_formation(),
                world.collapse_active(),
                world.police.is_perp_in_sight(),
                world.busted_meter()
            );
        }
    }
    let share = |n: u32| n as f32 / tally.samples.max(1) as f32;
    let summary = format!(
        "first collapse step {:?}, {} collapse steps; nearby chase cops: pull-over {:.2}, arrived {:.2}, reverse gear {:.2},          within 8 m {:.2}, at most {} within 8 m at once
{log}",
        tally.first_collapse,
        tally.collapse_steps,
        share(tally.pull_over),
        share(tally.arrived),
        share(tally.reverse),
        share(tally.near),
        tally.ringed,
    );
    assert_eq!(joined, 4, "all four cops join the chase: {summary}");
    assert!(tally.first_collapse.is_some_and(|tick| tick <= 60), "the collapse starts within a second: {summary}");
    // The original's recordings: below 20 km/h nearly every chase cop within 40 m holds the pull-over goal.
    assert!(share(tally.pull_over) >= 0.5, "most nearby cops take a ring slot: {summary}");
    assert!(tally.ringed >= 2, "the ring closes round the player: {summary}");
}
