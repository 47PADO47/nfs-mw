use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use super::*;
use crate::app::test_backend::NullBackend;
use crate::input::ActionState;
use crate::settings::{Partial, Settings, test_caps};

type Log = Arc<Mutex<Vec<String>>>;

/// A scene that writes what the host does to it into a shared log.
struct Probe {
    name: &'static str,
    log: Log,
    init_fails: bool,
}

impl Probe {
    fn boxed(name: &'static str, log: &Log) -> Box<dyn Scene> {
        Box::new(Self { name, log: log.clone(), init_fails: false })
    }

    fn note(&self, what: &str) {
        self.log.lock().unwrap().push(format!("{}: {what}", self.name));
    }
}

impl Scene for Probe {
    fn title(&self) -> String {
        self.name.to_owned()
    }
    fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
        self.note("init");
        if self.init_fails {
            return Err(anyhow!("no room for {}", self.name));
        }
        Ok(())
    }
    fn release(&mut self, _: &mut dyn RenderBackend) {
        self.note("release");
    }
    fn reattached(&mut self, _: &mut dyn RenderBackend) {
        self.note("shown");
    }
    fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
}

/// A host with a renderer that draws nothing, showing `scene` (which has not been initialised, as at start).
fn host_showing(scene: Box<dyn Scene>) -> Host {
    let settings = Settings::from(Partial::default());
    let mut host = Host::new(scene, &settings, None);
    host.renderer = Some(Box::new(NullBackend::new(test_caps::native())));
    host
}

#[test]
fn a_replaced_scene_is_released() {
    let log = Log::default();
    let mut host = host_showing(Probe::boxed("a", &log));
    host.replace_scene(Probe::boxed("b", &log)).unwrap();
    assert_eq!(*log.lock().unwrap(), ["b: init", "a: release"]);
    assert_eq!(host.scene.title(), "b");
    assert_eq!(host.scene_changes, 1);
}

#[test]
fn a_parked_scene_is_kept_and_comes_back_without_init() {
    let log = Log::default();
    let mut host = host_showing(Probe::boxed("menu", &log));
    host.replace_scene_parking_old(Probe::boxed("world", &log)).unwrap();
    assert_eq!(*log.lock().unwrap(), ["world: init"], "the menu is parked, not released");
    assert!(host.parked_menu.is_some());

    assert!(host.restore_parked_menu());
    assert_eq!(*log.lock().unwrap(), ["world: init", "world: release", "menu: shown"]);
    assert_eq!(host.scene.title(), "menu");
    assert!(host.parked_menu.is_none());
    assert_eq!(host.scene_changes, 2, "the audio hears the menu come back");
}

#[test]
fn nothing_is_restored_when_no_menu_is_parked() {
    let log = Log::default();
    let mut host = host_showing(Probe::boxed("menu", &log));
    assert!(!host.restore_parked_menu());
    assert!(log.lock().unwrap().is_empty());
    assert_eq!(host.scene.title(), "menu");
    assert_eq!(host.scene_changes, 0);
}

#[test]
fn a_second_parked_scene_releases_the_first() {
    let log = Log::default();
    let mut host = host_showing(Probe::boxed("menu", &log));
    host.replace_scene_parking_old(Probe::boxed("world", &log)).unwrap();
    host.replace_scene_parking_old(Probe::boxed("other", &log)).unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        ["world: init", "other: init", "menu: release"],
        "the world is parked now, so the menu parked before is the one released"
    );
}

#[test]
fn a_scene_that_fails_to_start_is_released_and_the_window_keeps_its_scene() {
    let log = Log::default();
    let mut host = host_showing(Probe::boxed("menu", &log));
    let failing = Box::new(Probe { name: "world", log: log.clone(), init_fails: true });
    assert!(host.replace_scene(failing).is_err());
    assert_eq!(*log.lock().unwrap(), ["world: init", "world: release"]);
    assert_eq!(host.scene.title(), "menu");
    assert_eq!(host.scene_changes, 0);
}
