use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use blackbox_render::{FrameParams, Instance, Renderer};

use super::{exec, parse, settings_cmd};
use crate::app::Host;
use crate::settings::{Partial, Settings, SmokeQuality};
use crate::viewer::Scene;

#[derive(Default)]
struct Seen {
    vehicle: Vec<[bool; 2]>,
    tires: Vec<[bool; 2]>,
    quality: Vec<SmokeQuality>,
}

struct ObservedScene(Arc<Mutex<Seen>>);

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
    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.0.lock().unwrap().vehicle.push([sparks, trails]);
    }
    fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.0.lock().unwrap().tires.push([smoke, marks]);
    }
    fn set_smoke_quality(&mut self, quality: SmokeQuality) {
        self.0.lock().unwrap().quality.push(quality);
    }
}

#[test]
fn vehicle_effects_startup_overrides_and_later_settings_sync_without_reapplying() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let initial = Settings::from(Partial { collision_sparks: Some(true), ..Partial::default() });
    let host = Host::new(Box::new(ObservedScene(seen.clone())), &initial, None);
    assert_eq!(seen.lock().unwrap().vehicle, vec![[true, false]]);
    let mut settings = initial;
    settings_cmd::set(&mut settings, "collision_sparks", "off").unwrap();
    settings_cmd::set(&mut settings, "speed_trails", "on").unwrap();
    let mut app = App::new();
    app.insert_resource(settings).insert_non_send(host).add_systems(Update, exec::sync_settings);
    app.update();
    assert_eq!(seen.lock().unwrap().vehicle, vec![[true, false], [false, true]]);
    app.update();
    assert_eq!(seen.lock().unwrap().vehicle.len(), 2);
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "collision_sparks", "on").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().vehicle, vec![[true, false], [false, true], [true, true]]);
    let before = *app.world().resource::<Settings>();
    assert!(settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "speed_trails", "maybe").is_err());
    app.update();
    assert_eq!(*app.world().resource::<Settings>(), before);
    assert_eq!(seen.lock().unwrap().vehicle.len(), 3);
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "hud", "off").unwrap();
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "tire_smoke", "off").unwrap();
    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "smoke_quality", "high").unwrap();
    app.update();
    let seen = seen.lock().unwrap();
    assert_eq!(seen.vehicle.len(), 3, "unrelated visual preferences do not resend vehicle effects");
    assert_eq!(seen.tires.last(), Some(&[false, true]));
    assert_eq!(seen.quality.last(), Some(&SmokeQuality::High));
}

#[test]
fn vehicle_effects_parsed_commands_apply_immediately_and_invalid_input_is_atomic() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let initial = Settings::from(Partial { hud: Some(false), tire_smoke: Some(false), ..Partial::default() });
    let mut settings = initial;
    let mut host = Host::new(Box::new(ObservedScene(seen.clone())), &settings, Some("unused.png".into()));
    for (line, expected) in [
        ("collision_sparks on", [true, false]),
        ("set speed_trails on", [true, true]),
        ("collision_sparks", [false, true]),
        ("speed_trails off", [false, false]),
        ("set collision_sparks on", [true, false]),
    ] {
        let Some(parse::Command::Set { key, value }) = parse::parse(line).unwrap() else { panic!("expected setting") };
        let text = exec::set_live(&mut settings, &mut host, &key, &value).unwrap();
        assert_eq!(text, settings_cmd::get(&settings, &key).unwrap());
        assert_eq!(seen.lock().unwrap().vehicle.last(), Some(&expected), "{line}: before the next frame");
        assert_eq!(settings, Settings { collision_sparks: expected[0], speed_trails: expected[1], ..initial });
    }
    let before = settings;
    for key in ["collision_sparks", "speed_trails"] {
        assert!(exec::set_live(&mut settings, &mut host, key, "maybe").is_err());
        assert!(parse::parse(&format!("{key} on extra")).is_err());
        assert!(parse::complete(key, &[]).contains(&key.to_owned()));
        assert_eq!(settings_cmd::get_all(&settings).lines().filter(|line| line.starts_with(key)).count(), 1);
    }
    assert_eq!(settings, before);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.vehicle.len(), 6);
    assert_eq!(seen.tires, vec![[false, true]], "the commands leave tire effects alone");
    assert_eq!(seen.quality, vec![SmokeQuality::Standard]);
}
