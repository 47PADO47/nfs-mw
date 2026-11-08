//! `vehicle` and `devtools` through the console's own entry points, against a scene that offers a vehicle.

use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use nfsmw_data::car::customization::{Catalog, Category, Choice, Selection};
use nfsmw_data::car::physics::PerformancePackage;

use super::dev_cmd::run;
use super::dev_parse::{self, DevCommand};
use crate::app::Host;
use crate::app::test_backend::NullBackend;
use crate::devtools::pad_nav::Button;
use crate::devtools::panels::DevPanels;
use crate::garage::options::{Change, Edit, Inputs, VehicleOptions};
use crate::settings::{Partial, Settings, test_caps};
use crate::viewer::Scene;

/// A vehicle with two wings, whose options are built as the real scenes build them.
struct Showroom {
    catalog: Catalog,
    parts: Selection,
}

impl Showroom {
    fn new() -> Self {
        let mut catalog = Catalog::default();
        let wing = |hash, label: &str| Choice { hash, label: label.to_owned(), language_hash: None, carbon: false };
        catalog.choices[Category::Spoiler as usize] = vec![wing(1, "Wing One"), wing(2, "Wing Two")];
        Self { catalog, parts: Selection::default() }
    }

    fn options(&self) -> VehicleOptions {
        VehicleOptions::build(&Inputs {
            folder: "SHOWROOMCAR",
            name: "Showroom car",
            catalog: Some(&self.catalog),
            parts: &self.parts,
            color: None,
            paintable: false,
            original_color: None,
            performance: None,
        })
    }
}

impl Scene for Showroom {
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
    fn vehicle(&mut self) -> Option<&mut dyn crate::viewer::VehicleScene> {
        Some(self)
    }
}

impl crate::viewer::VehicleScene for Showroom {
    fn options(&mut self) -> Option<VehicleOptions> {
        Some(Showroom::options(self))
    }
    fn change(&mut self, _: &mut dyn RenderBackend, change: &Change) -> Result<String, String> {
        let (options, catalog) = (Showroom::options(self), self.catalog.clone());
        let (mut color, mut performance) = (None, PerformancePackage::default());
        let mut edit = Edit { parts: &mut self.parts, color: &mut color, performance: &mut performance };
        options.apply(Some(&catalog), &mut edit, change)
    }
}

fn host(scene: impl Scene + 'static) -> Host {
    let settings = Settings::from(Partial::default());
    let mut host = Host::new(Box::new(scene), &settings, None);
    host.renderer = Some(Box::new(NullBackend::new(test_caps::native())));
    host
}

fn command(line: &str) -> DevCommand {
    let words: Vec<&str> = line.split_whitespace().collect();
    dev_parse::parse(words[0], &words[1..]).unwrap().unwrap()
}

#[test]
fn the_console_lists_the_slots_of_the_vehicle_and_sets_a_choice_at_once() {
    let mut host = host(Showroom::new());
    let mut dev = DevPanels::default();
    let slots = run(command("vehicle slots"), &mut host, &mut dev).unwrap();
    assert!(slots.contains("spoiler (Spoiler): 0: Stock*, 1: Wing One, 2: Wing Two"), "{slots}");
    assert_eq!(run(command("vehicle set spoiler two"), &mut host, &mut dev).unwrap(), "Spoiler: Wing Two");
    let slots = run(command("vehicle"), &mut host, &mut dev).unwrap();
    assert!(slots.contains("2: Wing Two*"), "the scene changed: {slots}");
    assert!(run(command("vehicle set spoiler 9"), &mut host, &mut dev).is_err());
    assert!(run(command("vehicle set hood 1"), &mut host, &mut dev).unwrap_err().contains("no slot"));
    assert_eq!(run(command("vehicle reset"), &mut host, &mut dev).unwrap(), "Showroom car is stock again");
    assert!(run(command("vehicle groups"), &mut host, &mut dev).unwrap().contains("parts: spoiler"));
}

#[test]
fn a_scene_without_a_vehicle_says_so() {
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
    let mut host = host(Empty);
    let mut dev = DevPanels::default();
    assert!(run(command("vehicle slots"), &mut host, &mut dev).unwrap_err().contains("no vehicle on show"));
    assert!(run(command("vehicle set a b"), &mut host, &mut dev).unwrap_err().contains("no vehicle on show"));
    assert_eq!(run(command("devtools garage"), &mut host, &mut dev).unwrap_err(), "this scene has no garage");
}

#[test]
fn devtools_opens_panels_through_requests_and_presses_buttons() {
    let mut host = host(Showroom::new());
    let mut dev = DevPanels::default();
    assert_eq!(run(command("devtools vehicle"), &mut host, &mut dev).unwrap(), "Vehicle (F11)");
    assert_eq!(run(command("devtools press down"), &mut host, &mut dev).unwrap(), "");
    assert_eq!(dev.pressed, [Button::Down]);
    assert!(run(command("vehicle panel"), &mut host, &mut dev).is_ok());
}
