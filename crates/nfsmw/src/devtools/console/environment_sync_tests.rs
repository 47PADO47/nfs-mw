use crate::{
    frontend::{Pausable, PauseFlag},
    input::ActionState,
    viewer::Scene,
};
use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use std::sync::Arc;

#[test]
fn environment_and_live_rain_reach_paused_play_and_replacement() {
    use crate::{
        app::Host,
        settings::{Partial, Percent, Settings},
        viewer::Environment,
    };
    use std::sync::Mutex;
    #[derive(Default)]
    struct Observed {
        time: Option<Environment>,
        rain: (bool, f32),
    }
    struct World(Arc<Mutex<Observed>>);
    impl Scene for World {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {
            panic!("paused");
        }
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn environment(&self) -> Option<Environment> {
            self.0.lock().unwrap().time
        }
        fn set_environment(&mut self, v: Environment) {
            self.0.lock().unwrap().time = Some(v);
        }
        fn set_rain(&mut self, on: bool, amount: f32) {
            self.0.lock().unwrap().rain = (on, amount);
        }
    }
    let seen = Arc::new(Mutex::new(Observed::default()));
    let flag = PauseFlag::default();
    flag.set(true);
    let mut scene = Pausable::new(Box::new(World(seen.clone())), flag.clone());
    let value = Environment { hour: 18.0, seconds_per_hour: 60.0 };
    scene.set_environment(value);
    assert_eq!(scene.environment(), Some(value));
    let settings = Settings::from(Partial::default());
    let host = Host::new(Box::new(scene), &settings, None);
    let mut app = bevy_app::App::new();
    app.insert_resource(settings).insert_non_send(host).add_systems(bevy_app::Update, super::exec::sync_settings);
    app.update();
    {
        let mut settings = app.world_mut().resource_mut::<Settings>();
        settings.always_rain = true;
        settings.rain_amount = Percent(37);
    }
    app.update();
    assert_eq!(seen.lock().unwrap().rain, (true, 0.37));
    let next = Arc::new(Mutex::new(Observed::default()));
    let replacement = Pausable::new(Box::new(World(next.clone())), flag);
    assert!(app.world_mut().non_send_mut::<Host>().replace_scene(Box::new(replacement)).is_err());
    assert_eq!(next.lock().unwrap().rain, (true, 0.37));
}
