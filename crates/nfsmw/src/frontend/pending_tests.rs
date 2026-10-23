//! The front end's move from loading to running: the host is held while it loads, and the front end is put in place
//! (or the UI stays off) once its loader thread is over. The full build needs the install (`NFSMW_GAME_DIR`; that test
//! returns at once without it).

use std::time::{Duration, Instant};

use bevy_ecs::system::RunSystemOnce;
use bevy_ecs::world::World;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use super::*;
use crate::settings::{Partial, Settings};
use crate::viewer::Scene;

struct Empty;

impl Scene for Empty {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
}

/// A world with the host and the loading list, as the app has them before the first frame.
fn world_with(pending: PendingFrontend) -> World {
    let mut world = World::new();
    let settings = Settings::from(Partial::default());
    world.insert_non_send(Host::new(Box::new(Empty), &settings, None));
    world.init_resource::<UiLoading>();
    world.resource_mut::<UiLoading>().begin(PART);
    world.insert_resource(pending);
    world
}

/// Runs the poll system until the pending front end is gone (a test fails instead of hanging).
fn poll_until_over(world: &mut World) {
    let started = Instant::now();
    while world.contains_resource::<PendingFrontend>() {
        assert!(started.elapsed() < Duration::from_secs(120), "the front end never finished loading");
        world.run_system_once(poll).expect("the poll system runs");
        std::thread::yield_now();
    }
}

#[test]
fn the_host_is_held_while_the_front_end_loads() {
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let job = Job::spawn("frontend-test", move || {
        let _ = wait.recv();
        None
    });
    let mut world = world_with(PendingFrontend(job));
    world.run_system_once(poll).expect("the poll system runs");
    assert!(world.contains_resource::<PendingFrontend>(), "the loader thread still waits");
    assert!(world.resource::<UiLoading>().busy(), "a screenshot or a benchmark waits");
    let host = world.non_send::<Host>();
    assert!(host.cancel_handled, "Escape belongs to the front end that is coming: it does not quit");
    assert!(host.flow_driven);
    go.send(()).unwrap();
    poll_until_over(&mut world);
    assert!(!world.resource::<UiLoading>().busy());
}

#[test]
fn a_front_end_without_assets_leaves_the_scene_alone() {
    let mut world = world_with(PendingFrontend::ready(None));
    poll_until_over(&mut world);
    assert!(!world.contains_resource::<Frontend>(), "the UI is off");
    assert!(!world.resource::<UiLoading>().busy());
    let host = world.non_send::<Host>();
    assert!(!host.cancel_handled && !host.flow_driven, "the scene runs alone, as without a front end");
}

#[test]
fn the_front_end_is_put_in_place_once_its_screens_are_read() {
    let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let setup = Setup {
        dir: dir.clone(),
        start: Start::Menu,
        script: None,
        browse: None,
        career: CareerHandle::default(),
        look: None,
        camera: CameraRig::default(),
    };
    let mut world = world_with(PendingFrontend::start(setup, AssetsLoad::start(dir)));
    poll_until_over(&mut world);
    assert!(world.contains_resource::<Frontend>(), "the front end runs once its screens are read");
    assert!(!world.resource::<UiLoading>().busy());
}
