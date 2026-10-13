//! The asset viewers: a searchable list on the left, the chosen model on the right. `asset-viewer` lists the
//! cars in the install (their wheels turn and the front pair steers); `view-assets` lists the shared scenery
//! models of a track (poles, cones, bins...). Clicking a name in the list shows it.

use std::f32::consts::TAU;

use anyhow::{Result, anyhow};
use blackbox_gfx::{FrameParams, Instance, RenderBackend, Shading};
use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions};
use nfsmw_data::world::{DEFAULT_TRACK, SharedAssets};

use super::car::CarScene;
use super::sidebar;
use crate::input::ActionState;
use crate::settings::CarShading;
use crate::viewer::Scene;

/// Wheel roll in radians per second (about two turns a second).
const SPIN_RATE: f32 = 12.0;
/// The front wheels' steer: this many radians either side, swept at `STEER_RATE` radians per second.
const STEER_LIMIT: f32 = 0.35;
const STEER_RATE: f32 = 0.8;
/// The yaw the model starts at, as the car viewer's default.
const START_YAW: f32 = 35.0;

/// Where the listed models come from.
enum Source {
    /// Car folders under CARS/, loaded from the install with these options.
    Cars { dir: GameDir, options: LoadOptions },
    /// The shared scenery models of a track.
    Scenery(SharedAssets),
}

pub struct AssetScene {
    source: Source,
    /// What the list is named in its count: "cars" or "assets".
    noun: &'static str,
    /// Every name in the list, sorted.
    names: Vec<String>,
    /// Index into `names` of the model on the floor.
    selected: usize,
    /// A name clicked in the last UI pass; loaded on the next update, which has the renderer.
    requested: Option<usize>,
    panel: sidebar::Panel,
    /// Whether the pointer is over the list: the camera ignores it there, so a drag over the list does not turn
    /// the model.
    over_panel: bool,
    /// Whether Ctrl+B has hidden the list. The model then recentres on the whole window.
    collapsed: bool,
    /// Why the last click did not load its model.
    error: Option<String>,
    car: CarScene,
    /// Seconds since the viewer opened: the wheels' motion.
    clock: f32,
}

impl AssetScene {
    /// Open the car viewer on the car `start` names: a folder under CARS/, or a unique start of one.
    pub fn open_cars(dir: &GameDir, start: &str) -> Result<Self> {
        let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
        let mut names = nfsmw_data::car::list(dir);
        names.sort_by_key(|car| car.to_ascii_lowercase());
        let folder =
            nfsmw_data::car::pick_folder(&names, start).ok_or_else(|| anyhow!("no car {start:?} (try list-cars)"))?;
        let model = nfsmw_data::car::load(dir, folder, &options)?;
        let selected = names.iter().position(|car| car == folder).unwrap_or(0);
        let car = CarScene::new(model, START_YAW).with_source(dir.clone(), options.clone());
        let source = Source::Cars { dir: dir.clone(), options };
        Ok(Self::start(source, "cars", names, selected, car))
    }

    /// Open the scenery viewer on the model `start` names (a unique start is enough), or the first one.
    pub fn open_scenery(dir: &GameDir, start: Option<&str>) -> Result<Self> {
        let assets = SharedAssets::load(dir, DEFAULT_TRACK)?;
        let names = assets.names();
        let selected = match start {
            Some(wanted) => {
                let found = nfsmw_data::car::pick_folder(&names, wanted)
                    .ok_or_else(|| anyhow!("no asset {wanted:?}, or several start with it"))?;
                names.iter().position(|n| n == found).unwrap_or(0)
            }
            None => 0,
        };
        let model = assets.model(selected);
        let mut car = CarScene::new(model, START_YAW);
        car.solid_shading = Shading::Prelit;
        car.texture_anims = assets.anims(selected);
        Ok(Self::start(Source::Scenery(assets), "assets", names, selected, car))
    }

    fn start(source: Source, noun: &'static str, names: Vec<String>, selected: usize, mut car: CarScene) -> Self {
        car.view_offset = sidebar::PANEL_FRACTION;
        Self {
            source,
            noun,
            names,
            selected,
            requested: None,
            panel: sidebar::Panel::default(),
            over_panel: false,
            collapsed: false,
            error: None,
            car,
            clock: 0.0,
        }
    }

    /// The texture animations of the model at `index` (none for a car).
    fn anims(&self, index: usize) -> Vec<blackbox_tpk::TextureAnim> {
        match &self.source {
            Source::Cars { .. } => Vec::new(),
            Source::Scenery(assets) => assets.anims(index),
        }
    }

    /// The model at `index`, as the car viewer draws it.
    fn load(&self, index: usize) -> Result<CarModel> {
        match &self.source {
            Source::Cars { dir, options } => Ok(nfsmw_data::car::load(dir, &self.names[index], options)?),
            Source::Scenery(assets) => Ok(assets.model(index)),
        }
    }

    /// Put the model at `index` on the floor. One that does not load keeps the one on the floor and says why.
    fn show(&mut self, renderer: &mut dyn RenderBackend, index: usize) {
        match self.load(index) {
            Ok(model) => {
                if matches!(self.source, Source::Scenery(_)) {
                    self.car.solid_shading = Shading::Prelit;
                }
                self.car.replace_model(renderer, model);
                self.car.texture_anims = self.anims(index);
                self.selected = index;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("{}: {e:#}", self.names[index])),
        }
    }
}

/// The wheels' roll (radians, wrapped to one turn) and the front steer, `clock` seconds in.
fn wheel_motion(clock: f32) -> (f32, f32) {
    let spin = (clock * SPIN_RATE).rem_euclid(TAU);
    let steer = STEER_LIMIT * (clock * STEER_RATE).sin();
    (spin, steer)
}

impl Scene for AssetScene {
    fn title(&self) -> String {
        self.car.title()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.car.init(renderer)
    }

    fn set_car_shading(&mut self, shading: CarShading) {
        self.car.set_car_shading(shading);
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if let Some(index) = self.requested.take() {
            self.show(renderer, index);
        }
        if !self.over_panel {
            self.car.update(renderer, input, dt);
        }
        // After the car's update, which may re-upload the car with its wheels at rest. A scenery model has no
        // wheels, and the call does nothing for it.
        self.clock += dt;
        self.car.animate_textures(renderer, self.clock);
        let (spin, steer) = wheel_motion(self.clock);
        self.car.pose_wheels(spin, steer);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.car.frame(aspect)
    }

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        if sidebar::toggle_pressed(ctx) {
            self.collapsed = !self.collapsed;
        }
        if self.collapsed {
            // No list to be over, and the model sits in the middle of the window.
            self.over_panel = false;
            self.car.view_offset = 0.0;
            return 0.0;
        }
        self.car.view_offset = sidebar::PANEL_FRACTION;
        let clicked = sidebar::show(ctx, &mut self.panel, &self.names, self.selected, self.error.as_deref(), self.noun);
        if let Some(index) = clicked {
            self.requested = Some(index);
        }
        self.over_panel = ctx.is_pointer_over_egui();
        sidebar::width(ctx)
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        self.car.commands()
    }

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
        self.car.command(renderer, name, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheels_start_at_rest_and_the_steer_stays_within_its_limit() {
        assert_eq!(wheel_motion(0.0), (0.0, 0.0));
        for step in 0..2000 {
            let (spin, steer) = wheel_motion(step as f32 * 0.05);
            assert!((0.0..TAU).contains(&spin), "the roll wraps to one turn");
            assert!(steer.abs() <= STEER_LIMIT + 1e-6);
        }
    }
}
