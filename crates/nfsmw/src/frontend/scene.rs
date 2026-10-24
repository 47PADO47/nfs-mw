//! The scenes the front end puts in the window: an empty backdrop for the menus, and a wrapper that freezes the
//! driving scene while the pause menu is up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, Projection, RenderBackend};
use glam::{Mat4, Vec3};

use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions};

use super::camera_rig::CameraRig;
use super::look::Look;
use super::room::Room;
use crate::input::ActionState;
use crate::scenes::car::CarScene;
use crate::scenes::car::FLOOR_PART;
use crate::scenes::car::shadows::{Part, shadowed};
use crate::settings::CarShading;
use crate::settings::fe_shadows;
use crate::viewer::Scene;

use self::build::{Build, Staged};

/// Seconds the camera takes to settle on the menu pose after the menus come up. The original's camera mover runs
/// its moves for 0.5 s (a constant read from the executable); its shots are not read yet, so this one shot is ours.
const INTRO_SECONDS: f32 = 0.5;
/// Where the intro starts: the camera swung this far round the car (degrees) and pulled back by this factor. [ours]
const INTRO_YAW_DEGREES: f32 = -30.0;
const INTRO_DISTANCE_SCALE: f32 = 1.5;

/// What the menus sit on: the stock car on the car viewer's floor when the install has it, and a dark flat colour
/// otherwise. The camera settles onto the car once when the menus come up and then holds still.
#[derive(Default)]
pub struct MenuScene {
    dir: Option<GameDir>,
    /// What the backdrop shows (`look.rs`).
    look: Look,
    car: Option<CarScene>,
    shading: CarShading,
    /// The stage's room, when the install has it (`room.rs`).
    room: Option<Room>,
    /// The car's and the room's parts together, for the frame.
    parts: Vec<Instance>,
    /// The camera's rest pose (yaw, distance), which the intro settles onto.
    rest: (f32, f32),
    /// The front end's camera, shared with the backdrops that come before and after this one (`camera_rig.rs`). It holds
    /// no pose when the install has none, or the look is one the player turns (the Showcase): the car viewer's orbit
    /// camera shows then.
    rig: CameraRig,
    /// Seconds since the menus came up, until the intro is over.
    intro: f32,
    /// The car and the room being read and uploaded (`backdrop_build.rs`); the backdrop is ready when it is done.
    build: Build,
    /// Resolution and quality of the stationary room capture currently submitted to the renderer.
    reflection: Option<(blackbox_gfx::ReflectionQuality, u32)>,
}

impl MenuScene {
    pub fn new(dir: Option<GameDir>) -> Self {
        Self::with_look(dir, Look::Main)
    }

    /// The backdrop of a menu (`look.rs`).
    pub fn with_look(dir: Option<GameDir>, look: Look) -> Self {
        Self { dir, look, intro: INTRO_SECONDS, ..Default::default() }
    }

    /// The same backdrop with the front end's camera: a camera that already plays keeps playing across a change of look.
    pub fn with_rig(mut self, rig: CameraRig) -> Self {
        self.rig = rig;
        self
    }

    /// Whether the front end's camera, not the orbit camera, shows this backdrop.
    fn uses_rig(&self) -> bool {
        self.look.camera_intro().is_some() && self.rig.pose().is_some()
    }
}

impl MenuScene {
    /// Takes what the worker read: the room is uploaded now, the car on the next update. A worker that panicked, or found
    /// no car, leaves the flat backdrop.
    fn staged(&mut self, renderer: &mut dyn RenderBackend, staged: Staged) {
        let Some((model, options)) = staged.car else { return };
        self.room = staged.room.map(|data| Room::upload(renderer, &data));
        self.build = Build::Car(Box::new(model), options);
    }

    /// Uploads the car and starts its camera.
    fn upload_car(&mut self, renderer: &mut dyn RenderBackend, model: CarModel, options: LoadOptions) {
        let Some(dir) = self.dir.clone() else { return };
        let mut car = CarScene::new(model, 0.0).with_source(dir, options);
        car.set_car_shading(self.shading);
        // The room has its own floor at the same height; the car's placeholder floor would z-fight with it.
        car.placeholder_floor = self.room.is_none();
        let stand = self.look.stage().car_stand();
        car.stand_on(stand.yaw, stand.sink);
        if let Err(e) = car.init(renderer) {
            log::warn!("the menu backdrop has no car: {e:#}");
            car.close(renderer);
            return;
        }
        let camera = car.camera_mut();
        self.rest = (camera.yaw, camera.distance);
        // With the front end's camera there is no intro of the orbit camera: the camera's own intro is the one.
        self.intro = if self.uses_rig() { INTRO_SECONDS } else { 0.0 };
        self.car = Some(car);
    }

    /// One step of the build, never blocking: takes the worker's result once it is done, then uploads the room, then the
    /// car, a step a frame.
    fn advance(&mut self, renderer: &mut dyn RenderBackend) {
        match std::mem::take(&mut self.build) {
            Build::Idle => {}
            Build::Reading(worker) => {
                if !worker.is_finished() {
                    self.build = Build::Reading(worker);
                    return;
                }
                match worker.join() {
                    Ok(staged) => self.staged(renderer, staged),
                    Err(_) => log::warn!("the menu backdrop worker panicked"),
                }
            }
            Build::Car(model, options) => self.upload_car(renderer, *model, options),
        }
    }

    /// Builds the whole backdrop now, waiting for the worker (tests that need the car in place).
    #[cfg(test)]
    pub fn settle(&mut self, renderer: &mut dyn RenderBackend) {
        while self.build.pending() {
            self.advance(renderer);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
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

impl MenuScene {
    fn update_car(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        let Some(car) = &mut self.car else { return };
        if self.look.camera_intro().is_some() && self.rig.advance(dt) {
            return car.update(renderer, &ActionState::default(), dt);
        }
        if matches!(self.look, Look::Showcase(_) | Look::LotShowcase(_)) {
            showcase_camera(car, input, self.rest, dt);
            return car.update(renderer, &ActionState::default(), dt);
        }
        // The backdrop takes no input and the car stands still: only the camera moves, once, into its rest pose.
        self.intro = (self.intro + dt).min(INTRO_SECONDS);
        let (rest_yaw, rest_distance) = self.rest;
        let t = self.intro / INTRO_SECONDS;
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let camera = car.camera_mut();
        camera.yaw = rest_yaw + INTRO_YAW_DEGREES.to_radians() * (1.0 - ease);
        camera.distance = rest_distance * (1.0 + (INTRO_DISTANCE_SCALE - 1.0) * (1.0 - ease));
        car.update(renderer, &ActionState::default(), dt);
    }
}

impl Scene for MenuScene {
    fn set_car_shading(&mut self, shading: CarShading) {
        self.shading = shading;
        if let Some(car) = &mut self.car {
            car.set_car_shading(shading);
        }
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(car) = &mut self.car {
            car.refresh_effects(renderer);
        }
    }

    fn title(&self) -> String {
        "nfsmw".to_owned()
    }

    /// Starts reading the car and the room on a worker; `update` uploads them as they arrive. Until then the backdrop is flat.
    fn init(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        let Some(dir) = self.dir.clone() else { return Ok(()) };
        // Screen selection runs before the worker uploads the car. Start the stage now so those selections
        // reach this stage's mover, rather than the previous stage (or an empty camera on first startup).
        if let Some(intro) = self.look.camera_intro() {
            self.rig.begin(&dir, self.look.stage(), intro);
        }
        self.build = Build::start(dir, self.look.clone());
        Ok(())
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_shadow_casters(None);
        renderer.set_planar_reflection_scene(None);
        renderer.set_reflection_scene(None);
        self.reflection = None;
        // A worker still reading is left to finish; its result is dropped with the handle.
        self.build = Build::Idle;
        if let Some(mut car) = self.car.take() {
            car.close(renderer);
        }
        if let Some(room) = self.room.take() {
            room.destroy(renderer);
        }
        self.parts.clear();
    }

    fn reattached(&mut self, renderer: &mut dyn RenderBackend) {
        self.reflection = None;
        let Some(car) = self.car.as_mut() else { return };
        car.reshow(renderer);
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.advance(renderer);
        self.update_reflections(renderer);
        self.update_car(renderer, input, dt);
        self.update_menu_casters(renderer);
        self.update_floor_reflection(renderer);
    }

    /// Not ready until the car and the room are uploaded, so a screenshot waits for the backdrop.
    fn ready(&self) -> bool {
        !self.build.pending()
    }

    fn radio_on_menus(&self) -> bool {
        true
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let options = fe_shadows::active();
        let Self { car, room, parts, rig, look, .. } = self;
        let has_car = car.is_some();
        let (mut params, car_parts) = match car {
            Some(car) => car.frame(aspect),
            None => (flat_frame(), &[] as &[Instance]),
        };
        if let Some(pose) = rig.pose().filter(|_| look.camera_intro().is_some()) {
            let lens = pose.lens(aspect);
            (params.view, params.projection, params.camera_position) = (lens.view, lens.projection, lens.position);
        }
        params.shadow_tuning = fe_shadows::tuning(options);
        parts.clear();
        // The renderer wants instances of one mesh next to each other, and draws blended parts in the order it is given
        // them. The room has one instance per mesh and keeps its own order (`Room::instances`); the car's parts are
        // already grouped by mesh. The room goes first, so the car's glass blends over its floor. The room is drawn from
        // the car's camera, so it waits for the car.
        if let Some(room) = room.as_ref().filter(|_| has_car) {
            parts.extend(room.instances().iter().map(|i| shadowed(*i, Part::Room, options.options)));
        }
        parts.extend(car_parts.iter().map(|i| shadowed(*i, car_part(i), options.options)));
        (params, parts.as_slice())
    }
}

/// Radians a second the arrow keys turn the Showcase camera, and the share of the distance a second the zoom keys cover.
const SHOWCASE_TURN: f32 = 1.6;
const SHOWCASE_ZOOM: f32 = 0.9;
/// The nearest and farthest the Showcase camera goes, as shares of the menu camera's distance, and how far it tilts (radians).
const SHOWCASE_DISTANCE: (f32, f32) = (0.45, 1.6);
const SHOWCASE_PITCH: (f32, f32) = (-0.05, 1.1);

/// The Showcase's camera: left and right turn it round the car, up and down tilt it, and the zoom keys move it nearer or
/// farther (`Showcase.fng`'s footer: `9 0 Zoom`, arrows `View Car`).
fn showcase_camera(car: &mut CarScene, input: &ActionState, rest: (f32, f32), dt: f32) {
    use crate::input::Action;
    let axis = |plus: Action, minus: Action| {
        f32::from(u8::from(input.pressed(plus))) - f32::from(u8::from(input.pressed(minus)))
    };
    let camera = car.camera_mut();
    camera.yaw -= axis(Action::MenuRight, Action::MenuLeft) * SHOWCASE_TURN * dt;
    camera.pitch = (camera.pitch + axis(Action::MenuUp, Action::MenuDown) * SHOWCASE_TURN * dt)
        .clamp(SHOWCASE_PITCH.0, SHOWCASE_PITCH.1);
    let zoom = 1.0 - axis(Action::MenuZoomIn, Action::MenuZoomOut) * SHOWCASE_ZOOM * dt;
    camera.distance = (camera.distance * zoom).clamp(rest.1 * SHOWCASE_DISTANCE.0, rest.1 * SHOWCASE_DISTANCE.1);
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

#[path = "backdrop_build.rs"]
mod build;

#[path = "garage_backdrop.rs"]
pub(crate) mod garage;

#[path = "backdrop_floor.rs"]
mod backdrop_floor;
#[path = "backdrop_shadows.rs"]
mod backdrop_shadows;
#[path = "backdrop_reflections.rs"]
mod reflections;

#[path = "pausable_audio.rs"]
mod audio_forwarding;

#[path = "pausable_update.rs"]
mod scene_update;

#[path = "pausable_scene.rs"]
mod forwarding;

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "backdrop_tests.rs"]
mod backdrop_tests;
