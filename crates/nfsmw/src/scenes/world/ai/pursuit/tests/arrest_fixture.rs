//! The installed world with a stopped player, one stationary cop on the player's deck and a chase at heat 1. The arrest
//! regressions and the front end's ticket/payment/resume regression start from it (`frontend/bust_resume_tests.rs`).

use super::*;
use crate::input::{ActionState, Bindings};
use crate::scenes::world::{DriveOptions, Options, WorldScene, drive::STEP};
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use game_install::GameDir;

/// The world, its renderer and the cop that arrests the player.
pub(crate) struct ArrestFixture {
    pub scene: WorldScene,
    pub renderer: NullBackend,
    /// The cop's audio identity, to find the car again.
    pub identity: u32,
    /// Where the player stands, physics space.
    pub at: Vec3,
}

/// A held space bar: the handbrake keeps the placed player still while the chase waits for the arrest.
pub(crate) fn handbrake() -> ActionState {
    let mut stopped = ActionState::default();
    stopped.update_for_test(&Bindings::default(), &[KeyCode::Space], &[], false);
    stopped
}

/// Opens the installed world, waits for the player, places a stationary police car behind it and starts a chase at heat
/// 1 whose wave is empty. `None` without `NFSMW_GAME_DIR`.
pub(crate) fn open_arrest_fixture() -> Option<ArrestFixture> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
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
    let stopped = handbrake();
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
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, &["copmidsize".into()]).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let drive = scene.players.lead().unwrap();
    let heading = drive.heading();
    let performance = drive.performance();
    let at = drive.ai_body().position;
    let physical = scene.traffic.as_ref().unwrap().player_position.expect("the real player body was simulated");
    assert!(at.distance(physical) < 1e-4, "pursuit target must use the current body origin");
    let sensed = Vec3::from_array(space::to_physics(drive.player_view().position));
    assert!(sensed.distance(physical) < 1e-4, "police sensing must use the same body origin");
    let forward = Vec3::from_array(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
    let mut world = scene.traffic.take().unwrap();
    world.cars.clear();
    world.configure(0, 0);
    let model = world.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case("copmidsize")).unwrap();
    let spot = at - forward * 8.0;
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, spot, forward, false));
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position: space::to_render(spot.to_array()), heading },
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces },
    )
    .expect("the installed cop must stand near the player");
    let identity = car.audio_emitter().id;
    world.cars.push(car);
    world.cars[0].set_roadblock();
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    for row in &mut rows {
        row.cops.clear();
    }
    world.player_position = Some(at);
    world.start_pursuit(1, rows, performance);
    scene.traffic = Some(world);
    Some(ArrestFixture { scene, renderer, identity, at })
}

impl ArrestFixture {
    /// Runs the world, with the handbrake held, until the arrest starts or `ticks` pass. Returns whether it started.
    pub(crate) fn run_to_arrest(&mut self, ticks: u32) -> bool {
        let stopped = handbrake();
        for _ in 0..ticks {
            self.scene.update(&mut self.renderer, &stopped, STEP);
            if self.scene.traffic.as_ref().is_some_and(|traffic| traffic.arrest_active()) {
                return true;
            }
        }
        false
    }
}
