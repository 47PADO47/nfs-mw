//! The scenes the front end puts in the window: an empty backdrop for the menus, and a wrapper that freezes the
//! driving scene while the pause menu is up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, Projection, RenderBackend};
use glam::{Mat4, Vec3};

use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions};

use super::camera_move::CameraMove;
use super::look::{CarLook, Look};
use super::room::Room;
use crate::input::ActionState;
use crate::scenes::car::CarScene;
use crate::scenes::car::FLOOR_PART;
use crate::scenes::car::shadows::{Part, shadowed};
use crate::settings::CarShading;
use crate::settings::fe_shadows;
use crate::viewer::Scene;

/// The car the menus show: the career-start M3 GTR, a preset of the `BMWM3GTRE46` car type with its livery and
/// spoiler, as the original's menu shows it. [ours] The original shows it on a platform stage
/// (`FRONTEND/PLATFORMS/PLATFORMCRIB.BIN`, see docs/specs/frontend-menus.md); here it stands on the car viewer's floor.
const MENU_CAR: &str = "BMWM3GTRE46";
const MENU_PRESET: &str = "M3GTRCAREERSTART";
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
    /// The stage's own camera, when it has one: the safe house moves it onto its pose, then holds it, instead of the car
    /// viewer's orbit camera. The move begins when the backdrop is built, which is when the menu opens.
    camera: Option<CameraMove>,
    /// Seconds since the menus came up, until the intro is over.
    intro: f32,
}

impl MenuScene {
    pub fn new(dir: Option<GameDir>) -> Self {
        Self::with_look(dir, Look::Main)
    }

    /// The backdrop of a menu (`look.rs`).
    pub fn with_look(dir: Option<GameDir>, look: Look) -> Self {
        Self { dir, look, intro: INTRO_SECONDS, ..Default::default() }
    }
}

impl MenuScene {
    /// The car the look shows, assembled from the install: the main look's career-start GTR, or the look's own car. A car
    /// that cannot be loaded gives way to the default car; with that missing too, `None` leaves the flat backdrop.
    fn load_car(&self, dir: &GameDir) -> Option<(CarModel, LoadOptions)> {
        let (car, preset) = match &self.look {
            Look::Main => (MENU_CAR.to_owned(), Some(MENU_PRESET.to_owned())),
            Look::Car(look) | Look::SafeHouse(look) | Look::Showcase(look) => look.resolve(dir),
        };
        let default = CarLook::default_car();
        let attempts = [(car, preset), (default.car, default.preset)];
        for (car, preset) in attempts {
            let options = LoadOptions { lod: 'A', all_parts: false, preset };
            match nfsmw_data::car::load(dir, &car, &options) {
                Ok(model) => return Some((model, options)),
                Err(e) => log::warn!("the menu backdrop has no car ({car}): {e:#}"),
            }
        }
        None
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

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let Some(dir) = self.dir.clone() else { return Ok(()) };
        let Some((model, options)) = self.load_car(&dir) else { return Ok(()) };
        let stage = self.look.stage();
        let room = match Room::load(renderer, &dir, stage) {
            Ok(room) => Some(room),
            Err(e) => {
                log::warn!("the menu room is not drawn: {e:#}");
                None
            }
        };
        let camera_move = stage.camera(Some(&dir));
        let mut car = CarScene::new(model, 0.0).with_source(dir, options);
        car.set_car_shading(self.shading);
        // The room has its own floor at the same height; the car's placeholder floor would z-fight with it.
        car.placeholder_floor = room.is_none();
        if let Some(stand) = stage.car_stand() {
            car.stand_on(stand.yaw, stand.sink);
        }
        car.init(renderer)?;
        let camera = car.camera_mut();
        self.rest = (camera.yaw, camera.distance);
        self.camera = camera_move;
        // A stage with its own camera has no intro of the orbit camera: its move is the intro.
        self.intro = if self.camera.is_some() { INTRO_SECONDS } else { 0.0 };
        self.car = Some(car);
        self.room = room;
        Ok(())
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(mut car) = self.car.take() {
            car.close(renderer);
        }
        if let Some(room) = self.room.take() {
            room.destroy(renderer);
        }
        self.parts.clear();
    }

    fn reattached(&mut self, renderer: &mut dyn RenderBackend) {
        let Some(car) = self.car.as_mut() else { return };
        car.reshow(renderer);
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        let Some(car) = &mut self.car else { return };
        if let Some(camera) = &mut self.camera {
            camera.advance(dt);
            return car.update(renderer, &ActionState::default(), dt);
        }
        if matches!(self.look, Look::Showcase(_)) {
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

    fn radio_on_menus(&self) -> bool {
        true
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let options = fe_shadows::active();
        let Self { car, room, parts, camera, .. } = self;
        let (mut params, car_parts) = match car {
            Some(car) => car.frame(aspect),
            None => (flat_frame(), &[] as &[Instance]),
        };
        if let Some(camera) = camera {
            let lens = camera.pose().lens(aspect);
            (params.view, params.projection, params.camera_position) = (lens.view, lens.projection, lens.position);
        }
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

#[path = "pausable_scene.rs"]
mod forwarding;

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "backdrop_tests.rs"]
mod backdrop_tests;
