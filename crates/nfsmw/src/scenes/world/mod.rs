//! The streamed city: a free-fly camera, or a car to drive with a chase camera.

mod commands;
mod drive;
mod effects;
mod ground;
mod props;
mod residency;
mod resident;
mod road;
mod space;
mod visibility;
mod zone;

use anyhow::{Context, Result};
use blackbox_render::{FrameParams, Instance, Renderer};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::{CarPhysics, PhysicsData};
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, Streamer, WorldIndex, load_global_textures};

use crate::input::{Action, ActionState};
use crate::viewer::{Scene, camera::FlyCamera};
use drive::{CarRig, Drive, DriveScript, SpawnRequest};
use residency::Residency;

pub struct Options {
    pub at: Option<[f32; 2]>,
    pub height: f32,
    pub heading: f32,
    pub pitch: f32,
    /// Where the fog is complete, in metres.
    pub fog_distance: f32,
    pub wait_for_load: bool,
    /// Drive a car from the start.
    pub drive: Option<DriveOptions>,
}

pub struct DriveOptions {
    /// Car folder (a unique prefix is enough).
    pub car: String,
    /// A scripted driver (`--drive-script`).
    pub script: Option<String>,
}

/// Which camera is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The free-fly camera.
    Fly,
    /// The chase camera behind the car.
    Chase,
}

/// A car waiting for the renderer to exist.
struct PendingCar {
    name: String,
    model: CarModel,
    script: Option<DriveScript>,
}

pub struct WorldScene {
    dir: GameDir,
    track: String,
    camera: FlyCamera,
    residency: Residency,
    /// Start position, for the ground estimate.
    start: [f32; 2],
    /// Height above the ground to place the camera at once the start area has loaded.
    start_height: f32,
    grounded: bool,
    wait_for_load: bool,
    fog_distance: f32,
    visible: Vec<Instance>,
    /// Seconds since the scene started, for texture animations.
    clock: f32,
    view: View,
    pending_car: Option<PendingCar>,
    drive: Option<Drive>,
    /// The car last driven, for `drive` without a name.
    last_car: String,
    /// Car physics data, road grips and the gameplay database.
    physics: PhysicsData,
    tire_effects: [bool; 2],
    smoke_quality: crate::settings::SmokeQuality,
}

/// Leaving the free camera more than this far (metres) from the car brings the car to the camera.
const FLOWN_AWAY: f32 = 25.0;

const CLEAR: [f32; 3] = [0.55, 0.63, 0.72];
/// The car `drive` picks when none was named.
pub const DEFAULT_CAR: &str = "BMWM3GTR";

/// Load a car for driving (highest detail, stock parts). `name` may be a unique prefix.
fn load_car(dir: &GameDir, name: &str) -> Result<(String, CarModel)> {
    let cars = nfsmw_data::car::list(dir);
    let folder = nfsmw_data::car::pick_folder(&cars, name)
        .with_context(|| format!("no car {name:?}, or several start with it (`nfsmw list-cars`)"))?
        .to_owned();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(dir, &folder, &options)?;
    Ok((folder, model))
}

impl WorldScene {
    pub fn open(dir: &GameDir, options: Options) -> Result<Self> {
        let index = WorldIndex::open(dir, DEFAULT_TRACK)?;
        let physics = PhysicsData::load(dir)?;
        let props = PropCatalog::new(index.prop_bounds.clone(), physics.database());
        log::info!("{} props have collision bounds", props.len());
        let stream = dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
        let streamer = Streamer::spawn(stream.to_path_buf(), index.sections.clone(), threads);
        let start = options.at.unwrap_or_else(|| index.centre());
        let camera = FlyCamera {
            position: Vec3::new(start[0], start[1], options.height),
            yaw: options.heading.to_radians(),
            pitch: options.pitch.to_radians(),
            speed: 40.0,
        };
        log::info!("camera starts at ({:.0}, {:.0}), {:.0} m above the ground", start[0], start[1], options.height);
        let pending_car = options
            .drive
            .map(|d| {
                let (name, model) = load_car(dir, &d.car)?;
                let script = d.script.as_deref().map(DriveScript::parse).transpose().map_err(anyhow::Error::msg)?;
                Ok::<_, anyhow::Error>(PendingCar { name, model, script })
            })
            .transpose()?;
        Ok(Self {
            dir: dir.clone(),
            track: index.track.clone(),
            camera,
            residency: Residency::new(
                index.sections,
                index.visible,
                streamer,
                load_global_textures(dir)?,
                index.collision_grid,
                props,
            ),
            start,
            start_height: options.height,
            // The car is placed on a road, not the camera on the ground.
            grounded: pending_car.is_some(),
            wait_for_load: options.wait_for_load,
            fog_distance: options.fog_distance,
            visible: Vec::new(),
            clock: 0.0,
            view: if pending_car.is_some() { View::Chase } else { View::Fly },
            last_car: pending_car.as_ref().map_or_else(|| DEFAULT_CAR.to_owned(), |c| c.name.clone()),
            pending_car,
            drive: None,
            physics,
            tire_effects: [true; 2],
            smoke_quality: crate::settings::SmokeQuality::Standard,
        })
    }

    /// Map position (x, y) the streaming follows.
    fn focus(&self) -> [f32; 2] {
        match (&self.drive, self.view) {
            (Some(drive), View::Chase) => match drive.waiting_for_road() {
                Some(request) => request.near,
                None => {
                    let p = drive.position();
                    [p.x, p.y]
                }
            },
            _ => [self.camera.position.x, self.camera.position.y],
        }
    }

    /// The physics of the car `model` was assembled as.
    fn physics_of(&mut self, model: &CarModel) -> Result<CarPhysics> {
        let type_name = model.car_type.as_deref().context("this car is not in the car tables, so it has no physics")?;
        self.physics.car(type_name)
    }

    /// Start (or restart) driving `model` from the camera's place.
    fn start_driving(
        &mut self,
        renderer: &mut Renderer,
        name: String,
        model: CarModel,
        script: Option<DriveScript>,
    ) -> Result<()> {
        let physics = self.physics_of(&model)?;
        let rig = CarRig::upload(renderer, model);
        let [x, y] = self.focus();
        match self.drive.as_mut() {
            Some(drive) => {
                drive.set_car(renderer, name.clone(), rig, physics);
                drive.respawn_near([x, y], None);
            }
            None => {
                let request =
                    SpawnRequest { near: [self.camera.position.x, self.camera.position.y], heading: None, exact: None };
                self.drive = Some(Drive::new(name.clone(), rig, physics, request, script));
            }
        }
        self.last_car = name;
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_enabled(self.tire_effects[0], self.tire_effects[1]);
            drive.effects.set_quality(self.smoke_quality);
        }
        self.view = View::Chase;
        Ok(())
    }

    /// Put a waiting car on the road once the area around it has loaded, then run its physics.
    fn update_drive(&mut self, input: &ActionState, dt: f32) {
        let (Some(drive), physics) = (self.drive.as_mut(), &self.physics) else { return };
        if let Some(request) = drive.waiting_for_road()
            && self.residency.complete()
        {
            let collision = self.residency.collision();
            let surfaces = &physics.surfaces;
            let estimate = ground::height_near(self.residency.placed(), request.near[0], request.near[1], 150.0);
            let top = estimate.unwrap_or(0.0) + 40.0;
            let found = match request.exact {
                Some(spawn) if drive.spawn(spawn, collision, surfaces) => Some(spawn),
                _ => road::find(request.near, |x, y| road::probe(collision, x, y, top)).filter(|&spawn| {
                    // `spawn` consumed the request when it failed above; ask again only on success.
                    drive.spawn(spawn, collision, surfaces)
                }),
            };
            match found {
                Some(spawn) => log::info!(
                    "{} on the road at ({:.0}, {:.0}, {:.1}), heading {:.0} degrees",
                    drive.car_name,
                    spawn.position.x,
                    spawn.position.y,
                    spawn.position.z,
                    spawn.heading.to_degrees()
                ),
                // No road in reach: the last place the car stood on one, if there is any.
                None if drive.last_good().is_some_and(|good| drive.spawn(good, collision, surfaces)) => {
                    log::info!(
                        "no road near ({:.0}, {:.0}); the car is back where it last was on a road",
                        request.near[0],
                        request.near[1]
                    );
                }
                None => {
                    log::warn!(
                        "no road within 400 m of ({:.0}, {:.0}) and none to go back to; switching to the free camera",
                        request.near[0],
                        request.near[1]
                    );
                    drive.cancel_request();
                    self.view = View::Fly;
                    self.camera.position = Vec3::new(request.near[0], request.near[1], estimate.unwrap_or(0.0) + 40.0);
                }
            }
        }
        if self.view == View::Chase {
            let loaded = self.residency.complete();
            let (collision, props) = self.residency.world_parts();
            drive.step(collision, props, &physics.surfaces, input, dt, loaded);
            drive.follow(self.residency.collision(), (input.value(Action::LookX), input.value(Action::LookY)), dt);
            return;
        }
        drive.age_effects(dt);
    }

    /// Switch between the chase camera and the free camera (the car waits while you fly).
    fn toggle_view(&mut self) -> &'static str {
        let Some(drive) = self.drive.as_mut() else { return "not driving (use the drive command)" };
        match self.view {
            View::Chase => {
                let camera = drive.camera();
                let look = (drive.position() - camera.position).normalize_or_zero();
                self.camera.position = camera.position;
                self.camera.yaw = look.y.atan2(look.x);
                self.camera.pitch = look.z.clamp(-1.0, 1.0).asin();
                self.view = View::Fly;
                "free camera (the car waits); F or `freecam` to go back"
            }
            View::Fly => {
                // Back in the car: if the camera has been flown away from it, the car joins the
                // street nearest to where the camera is, facing the way the camera looks.
                let cam = self.camera.position;
                let at = drive.position();
                if (cam.x - at.x).hypot(cam.y - at.y) > FLOWN_AWAY {
                    drive.respawn_near([cam.x, cam.y], Some(self.camera.yaw));
                }
                self.view = View::Chase;
                "chase camera"
            }
        }
    }
}

impl Scene for WorldScene {
    fn captures_mouse(&self) -> bool {
        // The mouse looks round the car too, on the chase camera.
        true
    }

    fn title(&self) -> String {
        format!("nfsmw — {}", self.track)
    }

    fn init(&mut self, renderer: &mut Renderer) -> Result<()> {
        if let Some(PendingCar { name, model, script }) = self.pending_car.take() {
            self.start_driving(renderer, name, model, script)?;
        }
        Ok(())
    }

    fn update(&mut self, renderer: &mut Renderer, input: &ActionState, dt: f32) {
        if input.just_pressed(Action::ToggleCamera) && self.drive.is_some() {
            log::info!("{}", self.toggle_view());
        }
        let [x, y] = self.focus();
        self.residency.update(renderer, x, y);
        self.clock += dt;
        self.residency.world_parts().1.tick(dt);
        self.residency.animate(renderer, self.clock);
        if self.view == View::Fly {
            self.camera.update(input, dt);
        }
        self.update_drive(input, dt);
        if !self.grounded && self.residency.complete() {
            self.grounded = true;
            let estimate = ground::height_near(self.residency.placed(), self.start[0], self.start[1], 150.0);
            // The collision surface just under the estimate is the real road; the estimate is only a guess.
            let top = estimate.unwrap_or(0.0) + 30.0;
            let road = space::ground_below(self.residency.collision(), self.start[0], self.start[1], top, top - 120.0);
            self.camera.position.z = road.or(estimate).unwrap_or(0.0) + self.start_height;
            log::info!(
                "ground near the start: scenery estimate {estimate:?}, collision {road:?}; camera at z = {:.0}",
                self.camera.position.z
            );
        }
        self.upload_effects(renderer);
    }

    fn set_tire_effects(&mut self, smoke: bool, skid_marks: bool) {
        self.tire_effects = [smoke, skid_marks];
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_enabled(smoke, skid_marks);
        }
    }

    fn refresh_effects(&mut self, renderer: &mut Renderer) {
        self.upload_effects(renderer);
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.smoke_quality = quality;
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_quality(quality);
        }
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let chase = self.drive.as_ref().filter(|_| self.view == View::Chase);
        let (view_proj, position, forward, fov_degrees) = match chase {
            Some(drive) => {
                let c = drive.camera();
                (c.view_proj(aspect), c.position, c.forward(), c.fov_degrees())
            }
            None => {
                (self.camera.view_proj(aspect), self.camera.position, self.camera.forward(), FlyCamera::FOV_Y_DEGREES)
            }
        };
        let fog_end = self.fog_distance;
        let camera = visibility::Camera {
            view_proj,
            position: position.to_array(),
            forward: forward.to_array(),
            fov_y_radians: fov_degrees.to_radians(),
        };
        let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
        visibility::collect(self.residency.placed(), &camera, rules, self.residency.props(), &mut self.visible);
        if let Some(drive) = &self.drive {
            drive.instances(&mut self.visible);
        }
        let params = FrameParams {
            view_proj,
            camera_position: position,
            light_dir: Vec3::new(-0.35, -0.45, -1.0),
            clear_color: CLEAR,
            fog_start: fog_end * 0.5,
            fog_end,
        };
        (params, &self.visible)
    }

    fn ready(&self) -> bool {
        match &self.drive {
            Some(drive) => self.residency.complete() && drive.waiting_for_road().is_none() && drive.script_finished(),
            None if self.wait_for_load => self.grounded && self.residency.complete(),
            None => self.residency.counts().2,
        }
    }

    fn status(&self) -> Option<String> {
        let (resident, loading, shared) = self.residency.counts();
        let p = match (&self.drive, self.view) {
            (Some(drive), View::Chase) => drive.position(),
            _ => self.camera.position,
        };
        let zone = self.residency.zone().unwrap_or_else(|| "-".into());
        Some(format!(
            "{}zone {zone}, tiles {resident} (+{loading} loading), {} drawn, {} props ({} knocked over), at ({:.0}, {:.0}, {:.0})",
            if shared { "" } else { "loading shared sets… " },
            self.visible.len(),
            self.residency.props().len(),
            self.residency.props().knocked_count(),
            p.x,
            p.y,
            p.z
        ))
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        if self.view == View::Fly {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        self.drive.as_ref()?.hud_state()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        self.drive.as_mut()?.car_sound()
    }

    fn hud(&self) -> Option<String> {
        let drive = self.drive.as_ref()?;
        Some(match self.view {
            View::Chase => drive.hud(),
            View::Fly => format!("free camera, the car waits (F to return)\n{}", drive.hud()),
        })
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        commands::LIST
    }

    fn command(&mut self, renderer: &mut Renderer, name: &str, args: &[&str]) -> Option<Result<String, String>> {
        commands::run(self, renderer, name, args)
    }
}
