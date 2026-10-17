use std::sync::{Arc, Mutex};

use bevy_app::{App, Update};
use blackbox_gfx::RenderBackend;
use blackbox_render::{FrameParams, Instance};

use super::{exec, settings_cmd};
use crate::app::Host;
use crate::settings::{MotionBlur, Partial, Settings, WorldTuning};
use crate::viewer::Scene;

struct ObservedScene(Arc<Mutex<Vec<MotionBlur>>>);

impl Scene for ObservedScene {
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
    fn set_world_tuning(&mut self, tuning: WorldTuning) {
        self.0.lock().unwrap().push(tuning.motion_blur);
    }
}

#[test]
fn a_running_scene_follows_the_motion_blur_switch_and_strength() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let initial = Settings::from(Partial::default());
    let host = Host::new(Box::new(ObservedScene(seen.clone())), &initial, None);
    assert_eq!(seen.lock().unwrap().len(), 1);
    let mut app = App::new();
    app.insert_resource(initial).insert_non_send(host).add_systems(Update, exec::sync_settings);
    app.update();
    app.update();
    assert_eq!(seen.lock().unwrap().len(), 1, "unchanged settings are not sent again");

    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "motion_blur", "off").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().last(), Some(&MotionBlur { enabled: false, strength: 1.0 }));

    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "motion_blur_strength", "0.5").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().last(), Some(&MotionBlur { enabled: false, strength: 0.5 }));
    let count = seen.lock().unwrap().len();

    settings_cmd::set(&mut app.world_mut().resource_mut::<Settings>(), "hud", "off").unwrap();
    app.update();
    assert_eq!(seen.lock().unwrap().len(), count, "unrelated settings do not resend the tuning");
}
