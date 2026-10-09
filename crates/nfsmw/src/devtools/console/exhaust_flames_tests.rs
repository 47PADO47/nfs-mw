use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use blackbox_render::{FrameParams, Instance, Renderer};

use super::{exec, parse, settings_cmd};
use crate::app::Host;
use crate::settings::{Partial, Settings};
use crate::viewer::Scene;

struct ObservedScene(Arc<Mutex<Vec<bool>>>);

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
    fn set_exhaust_flames(&mut self, on: bool) {
        self.0.lock().unwrap().push(on);
    }
}

#[test]
fn the_scene_gets_the_setting_at_startup_and_after_each_change_only() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let initial = Settings::from(Partial { exhaust_flames: Some(false), ..Partial::default() });
    let host = Host::new(Box::new(ObservedScene(seen.clone())), &initial, None);
    assert_eq!(*seen.lock().unwrap(), vec![false]);
    let mut app = App::new();
    app.insert_resource(initial).insert_non_send(host).add_systems(Update, exec::sync_settings);
    app.update();
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 2, "synced once at the first frame, then only on change");
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "exhaust_flames", "on").unwrap();
    app.update();
    assert_eq!(*seen.lock().unwrap(), vec![false, false, true]);
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "hud", "off").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 3, "unrelated settings do not resend it");
}

#[test]
fn the_console_gets_sets_and_flips_the_setting_at_once() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut settings = Settings::from(Partial::default());
    let mut host = Host::new(
        Box::new(ObservedScene(seen.clone())),
        &settings,
        Some(crate::app::Plan::new("unused.png".into(), 0.0, 1, 0.0)),
    );
    assert_eq!(settings_cmd::get(&settings, "exhaust_flames").unwrap(), "exhaust_flames = on");
    for (line, expected) in [("exhaust_flames off", false), ("set exhaust_flames on", true), ("exhaust_flames", false)]
    {
        let Some(parse::Command::Set { key, value }) = parse::parse(line).unwrap() else { panic!("expected setting") };
        let text = exec::set_live(&mut settings, &mut host, &key, &value).unwrap();
        assert_eq!(text, settings_cmd::get(&settings, &key).unwrap());
        assert_eq!(seen.lock().unwrap().last(), Some(&expected), "{line}: before the next frame");
        assert_eq!(settings.exhaust_flames, expected);
    }
    let count = seen.lock().unwrap().len();
    assert!(exec::set_live(&mut settings, &mut host, "exhaust_flames", "maybe").is_err());
    assert!(parse::parse("exhaust_flames on extra").is_err());
    assert!(parse::complete("exhaust_flames", &[]).contains(&"exhaust_flames".to_owned()));
    assert_eq!(settings_cmd::get_all(&settings).lines().filter(|l| l.starts_with("exhaust_flames")).count(), 1);
    assert_eq!(seen.lock().unwrap().len(), count, "a rejected value changes nothing");
}
