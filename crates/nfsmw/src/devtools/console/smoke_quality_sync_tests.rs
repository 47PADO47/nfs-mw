use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use blackbox_render::{FrameParams, Instance, Renderer};

use super::{exec, parse, settings_cmd};
use crate::app::Host;
use crate::settings::{Partial, Settings, SmokeQuality};
use crate::viewer::Scene;

struct ObservedScene(Arc<Mutex<Vec<SmokeQuality>>>);

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
    fn set_smoke_quality(&mut self, quality: SmokeQuality) {
        self.0.lock().unwrap().push(quality);
    }
}

#[test]
fn startup_and_changed_quality_reach_the_scene_without_reapplying_unchanged_values() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let initial = Settings::from(Partial { smoke_quality: Some(SmokeQuality::High), ..Partial::default() });
    let host = Host::new(Box::new(ObservedScene(seen.clone())), &initial, None);
    assert_eq!(*seen.lock().unwrap(), vec![SmokeQuality::High], "Host applies the startup quality");
    let mut settings = initial;
    settings_cmd::set(&mut settings, "smoke_quality", "standard").unwrap();
    let mut app = App::new();
    app.insert_resource(settings).insert_non_send(host).add_systems(Update, exec::sync_settings);
    app.update();
    assert_eq!(*seen.lock().unwrap(), vec![SmokeQuality::High, SmokeQuality::Standard]);
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 2);
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "smoke_quality", "high").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().last(), Some(&SmokeQuality::High));
    assert_eq!(seen.lock().unwrap().len(), 3);
    assert!(settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "smoke_quality", "ultra").is_err());
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 3, "invalid input cannot change or resend quality");
}

#[test]
fn console_shorthand_and_live_setter_forward_quality_before_the_next_frame() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut settings = Settings::from(Partial::default());
    let mut host = Host::new(Box::new(ObservedScene(seen.clone())), &settings, Some("unused.png".into()));
    let Some(parse::Command::Set { key, value }) = parse::parse("smoke_quality high").unwrap() else {
        panic!("expected setting")
    };
    assert_eq!(exec::set_live(&mut settings, &mut host, &key, &value).unwrap(), "smoke_quality = high");
    assert_eq!(*seen.lock().unwrap(), vec![SmokeQuality::Standard, SmokeQuality::High]);
    assert_eq!(settings_cmd::get(&settings, "smoke_quality").unwrap(), "smoke_quality = high");
    assert!(settings_cmd::get_all(&settings).contains("smoke_quality = high"));
    assert!(parse::complete("smoke_", &[]).contains(&"smoke_quality".to_owned()));
    let before = settings;
    assert!(exec::set_live(&mut settings, &mut host, "smoke_quality", "ultra").is_err());
    assert_eq!(settings, before);
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(
        exec::set_live(&mut settings, &mut host, "smoke_quality", "standard").unwrap(),
        "smoke_quality = standard"
    );
    assert_eq!(seen.lock().unwrap().last(), Some(&SmokeQuality::Standard));
}
