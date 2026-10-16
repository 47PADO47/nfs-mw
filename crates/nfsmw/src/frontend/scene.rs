//! The scenes the front end puts in the window: an empty backdrop for the menus, and a wrapper that freezes the
//! driving scene while the pause menu is up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, Projection, RenderBackend};
use glam::{Mat4, Vec3};

use game_install::GameDir;
use nfsmw_data::car::LoadOptions;

use super::room::Room;
use crate::input::ActionState;
use crate::scenes::car::CarScene;
use crate::scenes::car::FLOOR_PART;
use crate::scenes::car::shadows::{Part, shadowed};
use crate::settings::fe_shadows;
use crate::viewer::{Fullscreen, Scene};

/// The stock car the menus turn on their backdrop. [ours] The original shows the car on a platform stage
/// (`FRONTEND/PLATFORMS/PLATFORMCRIB.BIN`, see docs/specs/frontend-menus.md); until that stage is read, the car
/// stands on the car viewer's floor.
const MENU_CAR: &str = "BMWM3GTR";
/// Degrees per second the car turns on the backdrop. [ours]
const TURN_RATE: f32 = 12.0;

/// What the menus sit on: a car turning slowly on the car viewer's floor when the install has it, and a dark flat
/// colour otherwise.
#[derive(Default)]
pub struct MenuScene {
    dir: Option<GameDir>,
    car: Option<CarScene>,
    /// The stage's room, when the install has it (`room.rs`).
    room: Option<Room>,
    /// The car's and the room's parts together, for the frame.
    parts: Vec<Instance>,
}

impl MenuScene {
    pub fn new(dir: Option<GameDir>) -> Self {
        Self { dir, car: None, room: None, parts: Vec::new() }
    }
}

pub(super) fn flat_frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.02, 0.02, 0.03],
        fog: None,
        scattering: None,
        surface_light: None,
        // Nothing 3D is drawn, so there is no camera to follow from frame to frame.
        camera_cut: true,
        shadow_tuning: None,
    }
}

impl Scene for MenuScene {
    fn title(&self) -> String {
        "nfsmw".to_owned()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let Some(dir) = &self.dir else { return Ok(()) };
        let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
        let model = match nfsmw_data::car::load(dir, MENU_CAR, &options) {
            Ok(model) => model,
            Err(e) => {
                log::warn!("the menu backdrop has no car ({MENU_CAR}): {e:#}");
                return Ok(());
            }
        };
        let room = match Room::load(renderer, dir) {
            Ok(room) => Some(room),
            Err(e) => {
                log::warn!("the menu room is not drawn: {e:#}");
                None
            }
        };
        let mut car = CarScene::new(model, 0.0);
        // The room has its own floor at the same height; the car's placeholder floor would z-fight with it.
        car.placeholder_floor = room.is_none();
        car.init(renderer)?;
        self.car = Some(car);
        self.room = room;
        Ok(())
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, _input: &ActionState, dt: f32) {
        let Some(car) = &mut self.car else { return };
        car.turn((TURN_RATE * dt).to_radians());
        // The backdrop takes no input: the car is turned by the turntable alone.
        car.update(renderer, &ActionState::default(), dt);
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let options = fe_shadows::active();
        let Self { car, room, parts, .. } = self;
        let (mut params, car_parts) = match car {
            Some(car) => car.frame(aspect),
            None => (flat_frame(), &[] as &[Instance]),
        };
        params.shadow_tuning = fe_shadows::tuning(options);
        parts.clear();
        parts.extend(car_parts.iter().map(|i| shadowed(*i, car_part(i), options.options)));
        if let Some(room) = room {
            parts.extend(room.instances().iter().map(|i| shadowed(*i, Part::Room, options.options)));
        }
        // The renderer wants instances of one mesh next to each other.
        parts.sort_by_key(|i| i.mesh);
        (params, parts.as_slice())
    }
}

/// The part of the car an instance belongs to: the placeholder floor is keyed with its own part number.
fn car_part(instance: &Instance) -> Part {
    match instance.key.parts().1 == FLOOR_PART {
        true => Part::CarFloor,
        false => Part::Car,
    }
}

/// Whether a [`Pausable`] scene is frozen. The front end keeps one end, the scene the other.
#[derive(Clone, Default)]
pub struct PauseFlag(Arc<AtomicBool>);

impl PauseFlag {
    pub fn set(&self, paused: bool) {
        self.0.store(paused, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Whether a [`Pausable`] scene is concealed: it keeps running (streaming while the loading screen is up) but shows
/// nothing and makes no sound. The front end keeps one end, the scene the other.
#[derive(Clone, Default)]
pub struct ConcealFlag(Arc<AtomicBool>);

impl ConcealFlag {
    pub fn set(&self, concealed: bool) {
        self.0.store(concealed, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// A scene that stops while paused: it is not updated, makes no engine sound, takes no mouse and shows no HUD,
/// but is still drawn, so the pause menu has the frozen road behind it. While concealed (the loading screen, see
/// `loading.rs`) it runs, but is not drawn and makes no sound.
pub struct Pausable {
    inner: Box<dyn Scene>,
    paused: PauseFlag,
    conceal: ConcealFlag,
    effects_dirty: bool,
}

impl Pausable {
    pub fn new(inner: Box<dyn Scene>, paused: PauseFlag) -> Self {
        Self { inner, paused, conceal: ConcealFlag::default(), effects_dirty: false }
    }

    pub fn with_conceal(mut self, conceal: ConcealFlag) -> Self {
        self.conceal = conceal;
        self
    }

    fn concealed(&self) -> bool {
        self.conceal.get()
    }
}

impl Scene for Pausable {
    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        self.inner.garage()
    }

    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.inner.apply_garage(renderer, selection)
    }

    fn set_garage_open(&mut self, open: bool) {
        self.inner.set_garage_open(open);
    }
    fn title(&self) -> String {
        self.inner.title()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.inner.init(renderer)
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if self.paused.get() {
            if self.effects_dirty {
                self.inner.refresh_effects(renderer);
                self.effects_dirty = false;
            }
            return;
        }
        self.inner.update(renderer, input, dt);
        self.effects_dirty = false;
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        if !self.concealed() {
            return self.inner.frame(aspect);
        }
        // The scene still takes its frame (its camera and physics read the aspect ratio), but nothing is drawn.
        let _ = self.inner.frame(aspect);
        (flat_frame(), &[] as &[Instance])
    }

    // The rear-view mirror is the driving scene's, so the wrapper must pass it on: without this the HUD mirror
    // shows its bezel over a blank picture whenever the scene runs inside the front end (the play path).
    fn rear_view(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        if self.concealed() {
            return None;
        }
        self.inner.rear_view(aspect)
    }

    // Like the rear view, speed streaks belong to the driving scene; forward them so the play path draws them.
    fn speed_streaks(&self) -> f32 {
        self.inner.speed_streaks()
    }

    fn ready(&self) -> bool {
        self.inner.ready()
    }

    fn benchmark_ready(&self) -> bool {
        self.inner.benchmark_ready()
    }

    fn captures_mouse(&self) -> bool {
        !self.paused.get() && !self.concealed() && self.inner.captures_mouse()
    }

    fn status(&self) -> Option<String> {
        self.inner.status()
    }

    fn readout(&self, level: crate::devtools::ShowReadout) -> Option<String> {
        self.inner.readout(level)
    }

    fn set_transmission(&mut self, transmission: crate::settings::Transmission) {
        self.inner.set_transmission(transmission);
    }

    fn environment(&self) -> Option<crate::viewer::Environment> {
        self.inner.environment()
    }

    fn set_environment(&mut self, value: crate::viewer::Environment) {
        self.inner.set_environment(value);
        self.effects_dirty = true;
    }

    fn time_held(&self) -> bool {
        self.inner.time_held()
    }

    fn set_time_held(&mut self, held: bool) {
        self.inner.set_time_held(held);
    }

    fn set_mod_blips(&mut self, blips: &[crate::hud::MapBlip]) {
        self.inner.set_mod_blips(blips);
    }

    fn set_rain(&mut self, always: bool, amount: f32) {
        self.inner.set_rain(always, amount);
        self.effects_dirty = true;
    }

    fn set_world_tuning(&mut self, tuning: crate::settings::WorldTuning) {
        self.inner.set_world_tuning(tuning);
    }

    fn set_camera_mode(&mut self, mode: crate::settings::CameraMode) {
        self.inner.set_camera_mode(mode);
    }

    fn next_camera_mode(&self, current: crate::settings::CameraMode) -> crate::settings::CameraMode {
        self.inner.next_camera_mode(current)
    }

    fn takes_camera_setting(&self) -> bool {
        self.inner.takes_camera_setting()
    }

    fn set_wheel_options(&mut self, wheel: crate::settings::WheelOptions) {
        self.inner.set_wheel_options(wheel);
    }

    fn set_traffic(&mut self, traffic: crate::settings::TrafficSettings) {
        self.inner.set_traffic(traffic);
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        if self.paused.get() || self.concealed() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        self.inner.hud_state()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.paused.get() || self.concealed() {
            return None;
        }
        self.inner.car_sound()
    }

    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        let sounds = self.inner.take_common_sounds();
        if self.paused.get() || self.concealed() {
            return Vec::new();
        }
        sounds
    }

    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        if self.concealed() {
            return None;
        }
        self.inner.pursuit_feed()
    }

    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        if self.concealed() {
            return None;
        }
        self.inner.take_bust_ticket()
    }

    fn pay_bust(&mut self, marker: bool) -> bool {
        self.inner.pay_bust(marker)
    }

    fn paused(&self) -> bool {
        self.paused.get() || self.inner.paused()
    }

    fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.inner.set_tire_effects(smoke, marks);
        self.effects_dirty = true;
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.inner.set_smoke_quality(quality);
        self.effects_dirty = true;
    }

    fn set_car_shading(&mut self, shading: crate::settings::CarShading) {
        self.inner.set_car_shading(shading);
    }

    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.inner.set_vehicle_effects(sparks, trails);
        self.effects_dirty = true;
    }

    fn set_speed_trail_style(&mut self, style: crate::settings::SpeedTrailStyle) {
        self.inner.set_speed_trail_style(style);
    }

    fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.inner.set_spark_style(style);
        self.effects_dirty = true;
    }

    fn set_exhaust_flames(&mut self, on: bool) {
        self.inner.set_exhaust_flames(on);
        self.effects_dirty = true;
    }

    fn note_sputters(&mut self, pops: u32) {
        self.inner.note_sputters(pops);
    }

    fn fullscreen(&mut self) -> Option<Fullscreen> {
        self.inner.fullscreen()
    }

    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.inner.take_clip()
    }

    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        let sounds = self.inner.take_traffic_sounds();
        if self.concealed() {
            return Vec::new();
        }
        sounds
    }

    fn rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        let frame = self.inner.rotor_frame(dt);
        if self.concealed() {
            return None;
        }
        frame
    }

    fn take_explosions(&mut self) -> u32 {
        self.inner.take_explosions()
    }

    fn finished(&self) -> bool {
        self.inner.finished()
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.commands()
    }

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
        self.inner.command(renderer, name, args)
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
