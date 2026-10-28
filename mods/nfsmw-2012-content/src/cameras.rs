//! The 2012 game's cameras as extra camera views of the converted cars (`bb.camera` 1.1).
//!
//! The camera data and the behaviour model are `chameleon-camera`'s (docs/formats/chameleon-camera.md). This module only
//! keeps what the model needs next to each converted car, `files/CARS/<ID>/CAMERAS.TOML`, written while the car is
//! converted, and hands the host three views: the 2012 chase camera, the bonnet camera and the bumper camera. A car the
//! mod did not convert has no file, so the views do not fit it and the game draws its own cameras.
//!
//! The mounted cameras' numbers are offsets from an anchor the 2012 data does not name (the format doc); the mod anchors the
//! bonnet camera at the car's origin and the bumper camera at the nose, which it measures on the converted body.
//!
//! The file is written by the conversion, so cars converted before the cameras were kept have none until the conversion
//! runs again (`stamp.rs` bumps its version for that). The views fit a car only while its file is there.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use blackbox_modding::sdk::{Camera, CameraInput, CameraPose, CameraView as HostView, Ctx, Level};
use chameleon_bundle::Bundle;
use chameleon_camera::{CameraOptions, CameraState, CameraView, CarCameras, CarMotion, read_cameras};
use serde::{Deserialize, Serialize};

use crate::emit::ModFolder;

/// The file next to a converted car's other files.
pub const FILE: &str = "CAMERAS.TOML";

/// What the mod keeps of a car's cameras.
#[derive(Serialize, Deserialize)]
struct Saved {
    /// Where the bumper camera's offsets start: the nose of the converted body, `[forward, left, up]` in car space.
    bumper_anchor: [f32; 3],
    cameras: CarCameras,
}

/// Write the cameras of the 2012 car in `bundle` for the converted car `id`. `nose` is how far in front of the car's origin
/// the converted body ends and `middle` how far ahead of the 2012 origin the converted car's origin is, metres: the
/// conversion centres the car on its wheelbase, and the cameras are measured from the origin the game drives.
pub fn write(folder: &ModFolder, id: &str, bundle: &[u8], nose: f32, middle: f32) -> Result<()> {
    let mut cameras = read_cameras(&Bundle::parse(bundle)?).context("the cameras of the car could not be read")?;
    cameras.move_origin_forward(middle);
    let text = toml::to_string(&Saved { bumper_anchor: [nose, 0.0, 0.0], cameras })?;
    let path = folder.root().join("files").join("CARS").join(id).join(FILE);
    std::fs::write(&path, text).with_context(|| format!("{} could not be written", path.display()))
}

/// How long a car without a readable file is left alone before the folder is looked at again: the conversion may be
/// writing the cars while the game runs.
const RECHECK: Duration = Duration::from_secs(5);

/// What was found for a car.
enum Known {
    Found(Rc<Saved>),
    /// No readable file, as of this moment.
    Missing(Instant),
}

/// The cameras of the converted cars, read from the mod's folder when a car asks.
struct Cars {
    cars_dir: PathBuf,
    loaded: HashMap<String, Known>,
}

impl Cars {
    fn get(&mut self, car: &str) -> Option<Rc<Saved>> {
        match self.loaded.get(car) {
            Some(Known::Found(saved)) => return Some(saved.clone()),
            Some(Known::Missing(at)) if at.elapsed() < RECHECK => return None,
            _ => {}
        }
        let first = !self.loaded.contains_key(car);
        let path = self.cars_dir.join(car).join(FILE);
        let read = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| toml::from_str::<Saved>(&text).map_err(|error| format!("it is not readable: {error}")));
        let saved = match read {
            Ok(saved) => Rc::new(saved),
            Err(reason) => {
                if first {
                    let place = path.display();
                    log::info!(
                        "{car}: no 2012 cameras in {place} ({reason}); a car converted by an older mod has none"
                    );
                }
                self.loaded.insert(car.to_owned(), Known::Missing(Instant::now()));
                return None;
            }
        };
        self.loaded.insert(car.to_owned(), Known::Found(saved.clone()));
        Some(saved)
    }
}

/// One of the three views. Each holds the camera model of the car the player drives, rebuilt when the car changes.
struct View2012 {
    which: CameraView,
    cars: Rc<RefCell<Cars>>,
    state: Option<(String, CameraState)>,
}

impl HostView for View2012 {
    fn available(&mut self, car: &str) -> bool {
        self.cars.borrow_mut().get(car).is_some()
    }

    fn update(&mut self, input: &CameraInput<'_>) -> Option<CameraPose> {
        let motion = motion_of(input);
        let rebuilt = self.state.as_ref().is_none_or(|(car, _)| car != input.car);
        if rebuilt {
            let saved = self.cars.borrow_mut().get(input.car)?;
            let options = CameraOptions { bumper_anchor: saved.bumper_anchor, ..CameraOptions::default() };
            let mut state = CameraState::new(&saved.cameras, &options);
            state.set_view(self.which);
            self.state = Some((input.car.to_owned(), state));
        }
        let (_, state) = self.state.as_mut()?;
        let pose = match rebuilt || input.reset {
            true => state.reset(&motion),
            false => state.step(&motion),
        };
        Some(CameraPose {
            position: pose.position,
            forward: pose.forward,
            up: pose.up,
            fov: pose.vertical_fov_deg,
            // A camera on the bonnet or the bumper is inside or against the car's own body.
            hide_car: self.which != CameraView::Chase,
        })
    }
}

fn motion_of(input: &CameraInput<'_>) -> CarMotion {
    CarMotion {
        position: input.position,
        forward: input.forward,
        up: input.up,
        velocity: input.velocity,
        speed: input.speed,
        steer: input.steer,
        throttle: input.throttle,
        brake: input.brake,
        yaw_rate: input.angular_velocity[2],
        grounded: input.grounded,
        impact: input.impact,
        dt: input.dt,
    }
}

/// The views, with the id the setting saves them under and the name it shows.
const VIEWS: [(&str, &str, CameraView); 3] = [
    ("chase", "2012 Chase", CameraView::Chase),
    ("bonnet", "2012 Bonnet", CameraView::Bonnet),
    ("bumper", "2012 Bumper", CameraView::Bumper),
];

/// Offer the three views to the host, when it has `bb.camera` 1.1. A host without it only loses the views.
pub fn register(ctx: &Ctx, mod_dir: &Path) {
    let Some(camera) = ctx.camera() else {
        ctx.log(Level::Info, "nfsmw-2012-content: the game has no bb.camera, so the 2012 cameras are not offered");
        return;
    };
    register_with(ctx, &camera, mod_dir);
}

fn register_with(ctx: &Ctx, camera: &Camera, mod_dir: &Path) {
    let cars = Rc::new(RefCell::new(Cars { cars_dir: mod_dir.join("files").join("CARS"), loaded: HashMap::new() }));
    for (id, name, which) in VIEWS {
        let view = View2012 { which, cars: cars.clone(), state: None };
        if let Err(error) = camera.register_view(id, name, view) {
            ctx.log(Level::Warn, &format!("nfsmw-2012-content: the {id} camera is not added: {error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(car: &'a str, position: [f32; 3]) -> CameraInput<'a> {
        CameraInput {
            reset: false,
            dt: 1.0 / 60.0,
            speed: 0.0,
            steer: 0.0,
            throttle: 0.0,
            brake: 0.0,
            impact: 0.0,
            grounded: true,
            position,
            forward: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, 1.0],
            velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            car,
        }
    }

    #[test]
    fn a_car_without_a_cameras_file_has_no_2012_views() {
        let dir = std::env::temp_dir().join(format!("nfsmw2012-cameras-{}", std::process::id()));
        let cars = Rc::new(RefCell::new(Cars { cars_dir: dir, loaded: HashMap::new() }));
        let mut view = View2012 { which: CameraView::Chase, cars, state: None };
        assert!(!view.available("NOCAR"));
        assert_eq!(view.update(&input("NOCAR", [0.0; 3])), None);
    }

    #[test]
    fn an_unreadable_file_is_no_cameras_and_is_looked_at_again_later() {
        let dir = std::env::temp_dir().join(format!("nfsmw2012-cameras-bad-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("BADCAR")).unwrap();
        std::fs::write(dir.join("BADCAR").join(FILE), "this is not a camera file").unwrap();
        let mut cars = Cars { cars_dir: dir.clone(), loaded: HashMap::new() };
        assert!(cars.get("BADCAR").is_none());
        assert!(matches!(cars.loaded.get("BADCAR"), Some(Known::Missing(_))), "the miss is remembered");
        // A miss older than the recheck time is read again; the file is still bad, so it is a miss again.
        let long_ago = Instant::now().checked_sub(RECHECK * 2).unwrap_or_else(Instant::now);
        cars.loaded.insert("BADCAR".into(), Known::Missing(long_ago));
        assert!(cars.get("BADCAR").is_none());
        let Some(Known::Missing(at)) = cars.loaded.get("BADCAR") else { panic!("still a miss") };
        assert!(at.elapsed() < RECHECK, "the check was made again");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_motion_keeps_the_hosts_axes_and_yaw_rate() {
        let mut host = input("C", [1.0, 2.0, 3.0]);
        host.angular_velocity = [0.1, 0.2, 0.3];
        host.steer = -0.5;
        let motion = motion_of(&host);
        assert_eq!((motion.position, motion.yaw_rate, motion.steer), ([1.0, 2.0, 3.0], 0.3, -0.5));
    }

    #[test]
    fn three_views_with_unique_ids_are_offered() {
        let ids: std::collections::HashSet<&str> = VIEWS.iter().map(|view| view.0).collect();
        assert_eq!(ids.len(), 3);
        assert!(VIEWS.iter().all(|view| !view.1.is_empty()));
    }
}
