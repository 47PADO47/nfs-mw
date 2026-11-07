//! The `[bike.camera]` table of a bike's manifest, converted from RIDE 6's camera container of the bike
//! (`resolved/content/assets/data/cameras/Containers/<class>/<Vehicle>.json` of the physics dump): where the two chase cameras,
//! the camera in front and the camera in the helmet sit and look (`docs/specs/ride6-bike-cameras.md`).
//!
//! A bike's container lists only what the bike changes; the rest comes from its class's base container
//! (`GameBaseCAmeraContainer_<Class>.json`, in the same folder) and then from the root one (`Containers/GameBaseCAmeraContainer.json`).
//! A number is looked for in that order. The container is in Unreal's axes (x forward, y right, z up) and centimetres, in the frame of
//! the bike's skeleton; the manifest's table is in the model's frame (y left, metres), which is the skeleton's moved by the
//! difference of the two frames' hubs (`rider::attach`, `frame_offset`).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::physics::toml_text::Toml;

/// Where the containers are below the physics dump.
const CONTAINERS: [&str; 6] = ["resolved", "content", "assets", "data", "cameras", "Containers"];
/// The stem every base container's file name starts with (the dump spells it so).
const BASE_STEM: &str = "GameBaseCAmeraContainer";

/// Whether a container's file stem is a base container's. The install spells the stem two ways (`GameBaseCAmeraContainer_Racing`,
/// `GameBaseCameraContainer_Bagger` and `_Enduro`), so the comparison ignores case: a base container taken for a bike's own
/// would be matched against the bikes by name and its class would lose its base layer.
fn is_base(stem: &str) -> bool {
    stem.get(..BASE_STEM.len()).is_some_and(|head| head.eq_ignore_ascii_case(BASE_STEM))
}

/// The rider's bones whose middle is the eye point of the helmet camera, the head bone for a rider without them, and the meshes the
/// camera leaves out: the helmet and the visor of the stock rider (`docs/formats/ride6-rider.md`, `rider/parts.rs`).
const EYE_BONES: [&str; 2] = ["eye_l", "eye_r"];
const HEAD_BONE: &str = "head";
pub(crate) const HIDDEN_MESHES: [&str; 2] = ["helmetRacing_default", "visorClosedRacing_default"];

/// How far ahead of the front hub the camera in front of the bike is, m. A modelling choice: RIDE 6's own front camera is bolted
/// to the fairing and looks ahead; the game asks for one that looks back at the bike and the road, so it is put this far out
/// that the bike and its rider fill the picture without the front tyre being cut by the near plane.
pub const FRONT_AHEAD_OF_HUB_M: f32 = 1.6;
/// The tilt of that camera from the bike's axis, degrees (positive looking up): a little downwards, so the road is in the picture.
/// A modelling choice.
pub const FRONT_PITCH_DEG: f32 = -8.0;
/// The most the helmet camera is tilted down from the bike's axis, degrees. A modelling choice: the data tilts a sport, naked, bagger
/// and enduro rider's helmet camera by 10 degrees or not at all, but the supermotard class by 50 and 60 (what that tilt is
/// measured from, the data does not say), which would put the road out of the picture and the horizon out of the view;
/// 10 is what every other class's helmet camera looks down (two scooters 20).
pub const MAX_HELMET_DOWN_DEG: f64 = 10.0;
/// The near plane of the cameras that have none in the data, m (the one RIDE 6 gives the helmet and the cockpit is 10 cm).
pub const NEAR_M: f32 = 0.1;

/// The input type of a camera modifier slot that reads the bike's lean angle.
const LEAN_INPUT: &str = "CameraModifierInputType::Lean";

/// The base containers and the bike's own, in the order a number is looked for.
#[derive(Debug, Clone, Default)]
pub struct Container {
    layers: Vec<Vec<Value>>,
}

impl Container {
    /// The container layers of `vehicle` in the physics dump `physics`.
    pub fn read(physics: &Path, vehicle: &str) -> Result<Self> {
        let root = CONTAINERS.iter().fold(physics.to_path_buf(), |dir, part| dir.join(part));
        let own = locate(&root, vehicle)?;
        let mut paths = vec![own.clone()];
        let class_dir = own.parent().context("the container has no folder")?;
        paths.extend(base_in(class_dir).filter(|base| *base != own));
        paths.extend(base_in(&root).filter(|base| *base != own));
        let mut layers = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            layers.push(serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?);
        }
        Ok(Self { layers })
    }

    /// The property `key` of the object `object` (the name without `_GEN_VARIABLE`), from the first layer that has it.
    fn property(&self, object: &str, key: &str) -> Option<&Value> {
        let name = format!("{object}_GEN_VARIABLE");
        self.layers.iter().flat_map(|layer| layer.iter()).filter(|item| item["Name"] == name.as_str()).find_map(
            |item| {
                let value = &item["Properties"][key];
                (!value.is_null()).then_some(value)
            },
        )
    }

    fn number(&self, object: &str, key: &str) -> Option<f64> {
        self.property(object, key)?.as_f64()
    }

    /// A struct of numbers named by `fields`, in that order; a field it lacks is 0.
    fn parts<const N: usize>(&self, object: &str, key: &str, fields: [&str; N]) -> Option<[f64; N]> {
        let value = self.property(object, key)?;
        Some(fields.map(|field| value[field].as_f64().unwrap_or(0.0)))
    }

    fn location(&self, object: &str, key: &str) -> Option<[f64; 3]> {
        self.parts(object, key, ["X", "Y", "Z"])
    }
}

/// The folder's base container, if it has one.
fn base_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    let found = fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path()).find(|path| {
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
        path.extension().is_some_and(|ext| ext == "json") && is_base(stem)
    });
    found.into_iter()
}

/// The letters and digits of `name` without a year at its end: `Norton_V4SV_2022` and `Norton_V4SV-2021` are the same bike.
fn without_year(name: &str) -> (String, Option<i32>) {
    let squashed = crate::bike_table::squash(name);
    let split = squashed.len().saturating_sub(4);
    match squashed.get(split..).and_then(|tail| tail.parse::<i32>().ok()).filter(|year| (1900..2100).contains(year)) {
        Some(year) => (squashed[..split].to_owned(), Some(year)),
        None => (squashed, None),
    }
}

/// The container file of `vehicle`: in one of the class folders, by the letters and digits of its name. The physics dump and the
/// camera containers name some bikes with a model year apart (`Yamaha_MT-09_2021`, `Yamaha_MT09-2020`): when no container has the
/// name exactly, the one that differs only in the year, the nearest in time, is the bike's.
fn locate(root: &Path, vehicle: &str) -> Result<PathBuf> {
    let wanted = crate::bike_table::squash(vehicle);
    let (family, year) = without_year(vehicle);
    let mut nearest: Option<(i32, PathBuf)> = None;
    let classes = fs::read_dir(root).with_context(|| format!("reading {}", root.display()))?;
    for class in classes.flatten().filter(|entry| entry.path().is_dir()) {
        let Ok(files) = fs::read_dir(class.path()) else { continue };
        for file in files.flatten() {
            let path = file.path();
            let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
            if is_base(stem) || !path.extension().is_some_and(|ext| ext == "json") {
                continue;
            }
            if crate::bike_table::squash(stem) == wanted {
                return Ok(path);
            }
            let (candidate, candidate_year) = without_year(stem);
            let (Some(year), Some(candidate_year), true) = (year, candidate_year, candidate == family) else {
                continue;
            };
            let apart = (candidate_year - year).abs();
            if nearest.as_ref().is_none_or(|(best, _)| apart < *best) {
                nearest = Some((apart, path));
            }
        }
    }
    match nearest {
        Some((_, path)) => Ok(path),
        None => bail!("no camera container for `{vehicle}` under {}", root.display()),
    }
}

/// The frames' difference: where the model frame is from the skeleton's (x, z), m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frames {
    pub offset: [f64; 2],
}

impl Frames {
    /// From the hubs (x, z) in the skeleton's frame and in the model's: the mean of the two hubs' differences, as the game's rider
    /// does (`frame_offset`).
    pub fn from_hubs(skeleton: ([f64; 2], [f64; 2]), model: ([f64; 2], [f64; 2])) -> Self {
        let mean = |axis: usize| ((model.0[axis] - skeleton.0[axis]) + (model.1[axis] - skeleton.1[axis])) * 0.5;
        Self { offset: [mean(0), mean(1)] }
    }

    /// A point of the container (centimetres; x forward, y right, z up) in the model's frame (metres; x forward, y left, z up).
    fn point(&self, at: [f64; 3]) -> [f64; 3] {
        [at[0] / 100.0 + self.offset[0], -at[1] / 100.0, at[2] / 100.0 + self.offset[1]]
    }
}

/// One chase camera: the camera's place, the point it looks at and its field of view.
fn chase(t: &mut Toml, name: &str, c: &Container, frames: Frames, fov_object: &str) -> Result<()> {
    let behaviour = format!("Chase{name}ExternalCameraBehavior");
    let place = c.location(&behaviour, "RelativeLocation").with_context(|| format!("{behaviour} has no place"))?;
    let target = c.location(&behaviour, "TargetLocalOffset").with_context(|| format!("{behaviour} has no target"))?;
    let (place, target) = (frames.point(place), frames.point(target));
    let fov = c.number(fov_object, "FieldOfView").with_context(|| format!("{fov_object} has no field of view"))?;
    t.table(&format!("bike.camera.chase_{}", name.to_ascii_lowercase()));
    t.num("distance_m", -place[0]);
    t.num("height_m", place[2]);
    t.num("target_ahead_m", target[0]);
    t.num("target_height_m", target[2]);
    t.num("fov_deg", fov);
    Ok(())
}

/// The camera in front of the bike.
fn front(t: &mut Toml, c: &Container, frames: Frames, front_hub_x: f64) -> Result<()> {
    let mount = c.location("FrontCamInternalCameraBehavior", "RelativeLocation").context("no front camera mount")?;
    let height = frames.point(mount)[2];
    let fov = c.number("FrontCamReplay", "FieldOfView").context("FrontCamReplay has no field of view")?;
    let own = |key: &str| c.number("FrontCamInternalCameraBehavior", key).unwrap_or(0.0);
    t.table("bike.camera.front");
    t.nums("position_m", &[front_hub_x + f64::from(FRONT_AHEAD_OF_HUB_M), 0.0, height]);
    t.flag("look_back", true);
    t.num("pitch_deg", f64::from(FRONT_PITCH_DEG));
    t.num("fov_deg", fov);
    t.num("near_m", f64::from(NEAR_M));
    t.num("pitch_factor", own("PitchFactor"));
    t.num("roll_factor", own("RollFactor"));
    Ok(())
}

/// The camera in the helmet.
fn helmet(t: &mut Toml, c: &Container, frames: Frames) -> Result<()> {
    let object = "HelmetInternalCameraBehavior";
    let place = c.location(object, "RelativeLocation").context("no helmet camera place")?;
    let pitch =
        c.parts(object, "RelativeRotation", ["Pitch"]).map_or(0.0, |angles| angles[0]).max(-MAX_HELMET_DOWN_DEG);
    let fov = c.number("Helmet", "FieldOfView").context("Helmet has no field of view")?;
    let near = c
        .number("HelmetIgnitionNearPlaneCameraBehavior", "OverrideNearPlane")
        .map_or(f64::from(NEAR_M), |cm| cm / 100.0);
    let follows = c.property("GameHelmetCameraBehavior", "FollowCameraRoll").and_then(Value::as_bool).unwrap_or(false);
    let follow = c.number("GameHelmetCameraBehavior", "MaxAngle").filter(|_| follows).unwrap_or(0.0);
    t.table("bike.camera.helmet");
    t.strings("eye_bones", &EYE_BONES);
    t.string("head_bone", HEAD_BONE);
    t.strings("hide_meshes", &HIDDEN_MESHES);
    t.nums("fixed_position_m", &frames.point(place));
    t.num("pitch_deg", pitch);
    t.num("fov_deg", fov);
    t.num("near_m", near);
    t.num("pitch_factor", c.number(object, "PitchFactor").unwrap_or(0.0));
    t.num("roll_factor", c.number(object, "RollFactor").unwrap_or(0.0));
    t.num("follow_roll_max_deg", follow);
    lean(t, c);
    Ok(())
}

/// The `Lean` slots of the helmet camera's modifier behaviour: how the camera moves and turns as the bike leans
/// (`[bike.camera.helmet.lean]`). A slot maps the lean angle (degrees) between its `InputMin` and `InputMax` linearly to a share
/// of its offsets, which are in Unreal's axes and centimetres (y to the right, the inside of a bend to the right). The bikes have
/// two: one that shifts the camera (and tilts it) and one that turns and rolls it, with ranges of their own; the first slot of
/// each kind is read. A bike with no such slot has a table of zeros (the game's default would not be its own).
fn lean(t: &mut Toml, c: &Container) {
    let slots = c.property("HelmetCameraModifierBehaviorVariable", "Slots").and_then(Value::as_array);
    let leans = slots
        .into_iter()
        .flatten()
        .filter(|slot| slot["InputType"] == LEAN_INPUT && slot["Enabled"].as_bool().unwrap_or(true));
    let (shift, turn) = (
        leans.clone().find(|slot| slot_has(slot, "PositionOffset", &["X", "Y", "Z"])),
        leans.clone().find(|slot| slot_has(slot, "RotationOffset", &["Yaw", "Roll"])),
    );
    let range = |slot: Option<&Value>, key: &str| slot.map_or(0.0, |slot| slot[key].as_f64().unwrap_or(0.0));
    let of = |slot: Option<&Value>, key: &str, field: &str| slot.map_or(0.0, |slot| slot_part(slot, key, field));
    t.table("bike.camera.helmet.lean");
    t.num("shift_from_deg", range(shift, "InputMin"));
    t.num("shift_to_deg", range(shift, "InputMax"));
    t.nums("shift_m", &["X", "Y", "Z"].map(|axis| of(shift, "PositionOffset", axis) / 100.0));
    t.num("shift_pitch_deg", of(shift, "RotationOffset", "Pitch"));
    t.num("turn_from_deg", range(turn, "InputMin"));
    t.num("turn_to_deg", range(turn, "InputMax"));
    t.num("turn_yaw_deg", of(turn, "RotationOffset", "Yaw"));
    t.num("turn_roll_deg", of(turn, "RotationOffset", "Roll"));
}

/// The number `field` of the struct `key` of a modifier slot, 0 when it has none.
fn slot_part(slot: &Value, key: &str, field: &str) -> f64 {
    slot[key][field].as_f64().unwrap_or(0.0)
}

/// Whether a modifier slot's struct `key` has a non-zero `fields`.
fn slot_has(slot: &Value, key: &str, fields: &[&str]) -> bool {
    fields.iter().any(|field| slot_part(slot, key, field) != 0.0)
}

/// The text of the `[bike.camera]` tables of `vehicle`, for a model whose front hub is `front_hub_x` ahead of its origin.
pub fn tables(container: &Container, frames: Frames, front_hub_x: f64) -> Result<String> {
    let mut t = Toml::new();
    chase(&mut t, "Near", container, frames, "ThirdPersonNear")?;
    chase(&mut t, "Far", container, frames, "ThirdPersonFar")?;
    front(&mut t, container, frames, front_hub_x)?;
    helmet(&mut t, container, frames)?;
    Ok(t.into_string())
}

/// Write the `[bike.camera]` tables of the bike `vehicle` into its manifest `cars/<car>.toml` in `mod_dir` (replacing the ones it
/// has). The hubs of the model are read from the manifest's `[wheels]`, those of the skeleton from the car's
/// `RIDER/contacts.json`; a bike without them has no camera table, and the game derives its cameras from its size.
pub fn apply(mod_dir: &Path, car: &str, physics: &Path, vehicle: &str) -> Result<()> {
    let manifest = mod_dir.join("cars").join(format!("{}.toml", car.to_ascii_lowercase()));
    let text = fs::read_to_string(&manifest).with_context(|| format!("reading {}", manifest.display()))?;
    let parsed: toml::Table = text.parse().with_context(|| format!("reading {}", manifest.display()))?;
    let hub = |side: &str| -> Result<[f64; 2]> {
        let wheel =
            parsed.get("wheels").and_then(|wheels| wheels.get(side)).with_context(|| format!("no [wheels.{side}]"))?;
        let at = |key: &str| {
            wheel.get(key).and_then(toml::Value::as_float).with_context(|| format!("no {key} in [wheels.{side}]"))
        };
        Ok([at("x_m")?, at("z_m")?])
    };
    let model = (hub("front")?, hub("rear")?);
    let contacts =
        mod_dir.join("files").join("CARS").join(car.to_ascii_uppercase()).join("RIDER").join("contacts.json");
    let json: Value =
        serde_json::from_slice(&fs::read(&contacts).with_context(|| format!("reading {}", contacts.display()))?)?;
    let skeleton_hub = |side: &str| -> Result<[f64; 2]> {
        let at =
            |axis: usize| json["hubs"][side][axis].as_f64().with_context(|| format!("no hubs.{side} in contacts.json"));
        Ok([at(0)?, at(2)?])
    };
    let frames = Frames::from_hubs((skeleton_hub("front")?, skeleton_hub("rear")?), model);
    let tables = tables(&Container::read(physics, vehicle)?, frames, model.0[0])?;
    fs::write(&manifest, replace(&text, &tables)).with_context(|| format!("writing {}", manifest.display()))
}

/// `manifest` without its `[bike.camera]` tables, with `tables` after it.
pub fn replace(manifest: &str, tables: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in manifest.lines() {
        let header = line.trim_start().starts_with('[');
        if header {
            skipping = line.trim_start().starts_with("[bike.camera");
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    let kept = out.trim_end().len();
    out.truncate(kept);
    out.push_str("\n\n");
    out.push_str(tables);
    out
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
