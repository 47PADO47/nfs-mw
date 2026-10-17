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
    car: Option<CarScene>,
    shading: CarShading,
    /// The stage's room, when the install has it (`room.rs`).
    room: Option<Room>,
    /// The car's and the room's parts together, for the frame.
    parts: Vec<Instance>,
    /// The camera's rest pose (yaw, distance), which the intro settles onto.
    rest: (f32, f32),
    /// Seconds since the menus came up, until the intro is over.
    intro: f32,
}

impl MenuScene {
    pub fn new(dir: Option<GameDir>) -> Self {
        Self { dir, intro: INTRO_SECONDS, ..Default::default() }
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
        let Some(dir) = &self.dir else { return Ok(()) };
        let options = LoadOptions { lod: 'A', all_parts: false, preset: Some(MENU_PRESET.to_owned()) };
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
        let mut car = CarScene::new(model, 0.0).with_source(dir.clone(), options);
        car.set_car_shading(self.shading);
        // The room has its own floor at the same height; the car's placeholder floor would z-fight with it.
        car.placeholder_floor = room.is_none();
        car.init(renderer)?;
        let camera = car.camera_mut();
        self.rest = (camera.yaw, camera.distance);
        self.intro = 0.0;
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

    fn update(&mut self, renderer: &mut dyn RenderBackend, _input: &ActionState, dt: f32) {
        let Some(car) = &mut self.car else { return };
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

#[path = "pausable_scene.rs"]
mod forwarding;

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
