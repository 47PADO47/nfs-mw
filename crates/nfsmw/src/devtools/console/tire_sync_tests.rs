use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use blackbox_render::{FrameParams, Instance, Renderer};

use super::{exec, parse, settings_cmd};
use crate::app::Host;
use crate::settings::{Partial, Settings};
use crate::viewer::Scene;

struct ObservedScene(Arc<Mutex<Vec<[bool; 2]>>>);

impl Scene for ObservedScene {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut Renderer) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut Renderer, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
    fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.0.lock().unwrap().push([smoke, marks]);
    }
}

#[test]
fn startup_console_overrides_and_later_changes_reach_the_scene() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let initial = Settings::from(Partial::default());
    let host = Host::new(Box::new(ObservedScene(seen.clone())), &initial, None);
    let mut console_settings = initial;
    settings_cmd::set(&mut console_settings, "tire_smoke", "off").unwrap();
    let mut app = App::new();
    app.insert_resource(console_settings).insert_non_send(host).add_systems(Update, exec::sync_settings);
    app.update();
    assert_eq!(*seen.lock().unwrap(), vec![[true, true], [false, true]]);
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "skid_marks", "off").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().last(), Some(&[false, false]));
    assert!(settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "tire_smoke", "maybe").is_err());
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 3, "invalid input cannot change or reapply settings");
}

#[test]
fn effect_command_aliases_update_the_same_settings_as_get_and_set() {
    let mut settings = Settings::from(Partial::default());
    for (line, expected) in [("tire-effects smoke off", "tire_smoke = off"), ("skid_marks off", "skid_marks = off")] {
        let Some(parse::Command::Set { key, value }) = parse::parse(line).unwrap() else { panic!("expected setting") };
        assert_eq!(settings_cmd::set(&mut settings, &key, &value).unwrap(), expected);
    }
    assert!(!settings.tire_smoke && !settings.skid_marks);
    assert!(matches!(parse::parse("tire-effects status").unwrap(), Some(parse::Command::Scene { .. })));
}
