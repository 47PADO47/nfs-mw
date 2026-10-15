use super::*;

#[test]
fn a_paused_game_is_still_a_game_for_the_radio() {
    let flag = PauseFlag::default();
    let mut scene = Pausable::new(Box::new(MenuScene::default()), flag.clone());
    assert!(!scene.paused());
    flag.set(true);
    assert!(scene.paused());
    assert!(scene.car_sound().is_none(), "the car is silent while paused");
    assert!(!MenuScene::default().paused(), "the menus are not a game");
}

#[test]
fn replacement_driving_scene_inherits_live_effect_settings_through_pause_wrapper() {
    use crate::app::Host;
    use crate::settings::{CarShading, Partial, Settings, SmokeQuality, SparkStyle};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Seen {
        tires: [bool; 2],
        quality: Option<SmokeQuality>,
        vehicle: [bool; 2],
        style: Option<SparkStyle>,
        shading: Option<CarShading>,
    }
    struct Observed(Arc<Mutex<Seen>>);
    impl Scene for Observed {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
            self.0.lock().unwrap().tires = [smoke, marks];
        }
        fn set_smoke_quality(&mut self, quality: SmokeQuality) {
            self.0.lock().unwrap().quality = Some(quality);
        }
        fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
            self.0.lock().unwrap().vehicle = [sparks, trails];
        }
        fn set_spark_style(&mut self, style: SparkStyle) {
            self.0.lock().unwrap().style = Some(style);
        }
        fn set_car_shading(&mut self, shading: CarShading) {
            self.0.lock().unwrap().shading = Some(shading);
        }
    }
    let settings = Settings::from(Partial::default());
    let mut host = Host::new(Box::new(MenuScene::default()), &settings, None);
    host.set_tire_effects(false, true);
    host.set_smoke_quality(SmokeQuality::High);
    host.set_vehicle_effects(true, false);
    host.set_spark_style(SparkStyle::RestoredExperimental);
    host.set_car_shading(CarShading::Simple);
    let seen = Arc::new(Mutex::new(Seen::default()));
    let scene = Pausable::new(Box::new(Observed(seen.clone())), PauseFlag::default());
    // Preferences must reach the incoming scene before renderer init and screenshot settling.
    assert!(host.replace_scene(Box::new(scene)).is_err(), "this test has no renderer");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.tires, [false, true]);
    assert_eq!(seen.quality, Some(SmokeQuality::High));
    assert_eq!(seen.vehicle, [true, false]);
    assert_eq!(seen.style, Some(SparkStyle::RestoredExperimental));
    assert_eq!(seen.shading, Some(CarShading::Simple));
}

#[test]
fn paused_vehicle_effect_changes_forward_and_mark_buffers_for_refresh() {
    use std::sync::Mutex;

    struct Observed(Arc<Mutex<Vec<[bool; 2]>>>);
    impl Scene for Observed {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {
            panic!("a paused scene must not advance");
        }
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
            self.0.lock().unwrap().push([sparks, trails]);
        }
    }
    let flag = PauseFlag::default();
    flag.set(true);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut scene = Pausable::new(Box::new(Observed(seen.clone())), flag);
    assert!(!scene.effects_dirty);
    scene.set_vehicle_effects(true, false);
    assert!(scene.effects_dirty, "the next paused frame must refresh the effect buffers");
    assert!(scene.paused());
    assert!(!scene.hud_state().unwrap().visible);
    scene.set_vehicle_effects(false, false);
    assert_eq!(*seen.lock().unwrap(), vec![[true, false], [false, false]]);
    assert!(scene.effects_dirty, "disabling also needs a refresh of frozen buffers");
    scene.effects_dirty = false;
    scene.set_spark_style(crate::settings::SparkStyle::RestoredExperimental);
    assert!(scene.effects_dirty, "a style change must refresh frozen sprite and streak buffers");
}

#[test]
fn the_rear_view_mirror_reaches_the_driving_scene_through_the_wrapper() {
    struct Mirror;
    impl Scene for Mirror {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn rear_view(&mut self, _: f32) -> Option<(FrameParams, &[Instance])> {
            Some((flat_frame(), &[]))
        }
    }
    let mut scene = Pausable::new(Box::new(Mirror), PauseFlag::default());
    assert!(scene.rear_view(2.0).is_some(), "a driving scene's mirror must not be dropped by the wrapper");
}

#[test]
fn global_sound_events_pass_through_play_wrapper_once_and_never_replay_after_loading_or_pause() {
    use crate::audio::CommonEvent::{SpeedbreakerBegin, SpeedbreakerEnd};
    use std::sync::Mutex;
    struct Events(Arc<Mutex<Vec<crate::audio::CommonEvent>>>);
    impl Scene for Events {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }
    let queue = Arc::new(Mutex::new(vec![SpeedbreakerBegin, SpeedbreakerEnd]));
    let pause = PauseFlag::default();
    let conceal = ConcealFlag::default();
    let mut scene = Pausable::new(Box::new(Events(queue.clone())), pause.clone()).with_conceal(conceal.clone());
    assert_eq!(scene.take_common_sounds(), vec![SpeedbreakerBegin, SpeedbreakerEnd]);
    assert!(scene.take_common_sounds().is_empty());
    for hidden in [false, true] {
        pause.set(!hidden);
        conceal.set(hidden);
        queue.lock().unwrap().push(SpeedbreakerBegin);
        assert!(scene.take_common_sounds().is_empty());
        pause.set(false);
        conceal.set(false);
        assert!(scene.take_common_sounds().is_empty());
        queue.lock().unwrap().push(SpeedbreakerEnd);
        assert_eq!(scene.take_common_sounds(), vec![SpeedbreakerEnd]);
    }
}
