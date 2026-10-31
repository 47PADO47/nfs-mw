//! The contact points of one bike: where the rider sits (`SOCKET_RIDER`), where his hands go (`SOCKET_L_HANDLE_POSE`,
//! `SOCKET_R_HANDLE_POSE`) and his feet (`SOCKET_LEFT_FOOT`, `SOCKET_RIGHT_FOOT`), read from the bike's own skeleton and
//! written to `files/CARS/<ID>/RIDER/contacts.json`. The rider rig (skeleton, clips, meshes, `attach.json`) is shared by
//! every bike; the game lays the file's seat, hubs and targets over the shared `attach.json`, so each bike's rider reaches
//! its own grips and pegs (`docs/formats/ride6-rider.md`, "contacts.json").
//!
//! The skeleton comes from the bike's `parts.json` (`bones{}`: the world place and turn of every bone of the chassis
//! skeleton, in glTF axes and metres, as the exporter reads them from the chassis mesh; `sockets{}`: every socket of the
//! skeleton as Unreal stores it, a place in centimetres and a rotator in degrees on a bone). A socket's world place is the
//! bone's world transform composed with the socket's.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use blackbox_skeleton::Transform;
use glam::Vec3;
use serde_json::{Map, Value, json};

use super::attach::{HUB_BONES, SEAT_SOCKET, TARGETS};
use super::convert;
use super::raw::{JsonBone, JsonSkeleton, JsonSocket};
use super::style;

/// The file name, in the car's `RIDER/` folder.
pub const FILE: &str = "contacts.json";
/// The version the game checks.
pub const VERSION: u32 = 1;

/// Where the animation data table of the physics dump is, below the dump's folder (`RiderAnimationCorrection_DT`: a row
/// per vehicle, the struct `BikeCorrection`).
const CORRECTION_TABLE: [&str; 7] =
    ["resolved", "content", "assets", "data", "datatables", "Anim", "RiderAnimationCorrection_DT.json"];

/// The numeric fields of a `BikeCorrection` row the file carries, as `(file key, RIDE 6 field)`.
const CORRECTION_FIELDS: [(&str, &str); 10] = [
    ("rider_max_lean", "RiderMaxLean"),
    ("aero_tank", "AeroTankCorrection"),
    ("lean_knee", "LeanKneeCorrection"),
    ("lean_elbow", "LeanElbowCorrection"),
    ("knee", "KneeCorrection"),
    ("elbow", "ElbowCorrection"),
    ("muffler", "MufflerCorrection"),
    ("spine_1_deg", "SpineCompensation1"),
    ("spine_2_deg", "SpineCompensation2"),
    ("arms_ik_spine_alpha", "ArmsIKSpineCompensationAlpha"),
];

/// A transform as the files write it: translation, then rotation as `x y z w`.
pub fn transform_json(t: &Transform) -> Value {
    json!({
        "translation": t.translation.to_array(),
        "rotation": t.rotation.to_array(),
    })
}

/// The seat, the hubs and the targets of a bike, in the car frame, metres, in the bike skeleton's frame at rest.
#[derive(Debug, Clone, PartialEq)]
pub struct Points {
    pub seat: Transform,
    /// The front and the rear hub.
    pub hubs: (Vec3, Vec3),
    /// `(role, place, steers with the handlebar)` in the order of [`TARGETS`].
    pub targets: Vec<(&'static str, Transform, bool)>,
}

impl Points {
    /// The place of the target `role`.
    pub fn target(&self, role: &str) -> Result<Transform> {
        let found = self.targets.iter().find(|(name, _, _)| *name == role);
        found.map(|(_, at, _)| *at).with_context(|| format!("no target {role}"))
    }

    /// `rider_root_in_bike`, `hubs` and `targets` of `attach.json` and `contacts.json`.
    pub fn placement(&self) -> Map<String, Value> {
        let targets: Vec<Value> = self
            .targets
            .iter()
            .map(|(role, at, steers)| {
                json!({
                    "name": role,
                    "position": at.translation.to_array(),
                    "rotation": at.rotation.to_array(),
                    "steers": steers,
                })
            })
            .collect();
        let mut out = Map::new();
        out.insert("rider_root_in_bike".to_owned(), transform_json(&self.seat));
        out.insert("hubs".to_owned(), json!({ "front": self.hubs.0.to_array(), "rear": self.hubs.1.to_array() }));
        out.insert("targets".to_owned(), Value::Array(targets));
        out
    }
}

/// The points of a bike skeleton: its seat and target sockets and its two hub bones.
pub fn points(bike: &JsonSkeleton) -> Result<Points> {
    let (skeleton, sockets) = convert::skeleton(bike)?;
    let mut component = Vec::new();
    skeleton.component_transforms(&skeleton.rest_pose().local, &mut component)?;
    let world = |name: &str| -> Result<Transform> {
        let socket =
            sockets.iter().find(|s| s.name == name).with_context(|| format!("the bike has no socket {name}"))?;
        Ok(component[socket.bone].compose(socket.transform))
    };
    let hub = |name: &str| -> Result<Vec3> {
        let bone = skeleton.find(name).with_context(|| format!("the bike has no bone {name}"))?;
        Ok(component[bone].translation)
    };
    let mut targets = Vec::new();
    for (role, socket, steers) in TARGETS {
        targets.push((role, world(socket)?, steers));
    }
    Ok(Points { seat: world(SEAT_SOCKET)?, hubs: (hub(HUB_BONES.0)?, hub(HUB_BONES.1)?), targets })
}

fn floats<const N: usize>(value: Option<&Value>, what: &str) -> Result<[f32; N]> {
    let list = value.and_then(Value::as_array).with_context(|| format!("{what} is missing"))?;
    if list.len() != N {
        bail!("{what} has {} numbers, not {N}", list.len());
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(list) {
        *slot = item.as_f64().with_context(|| format!("{what} holds a non-number"))? as f32;
    }
    Ok(out)
}

/// The bike skeleton of a `parts.json` as the skeleton JSON the rider converter reads, in Unreal's axes and centimetres.
///
/// The exporter wrote every bone's *world* transform in glTF axes (x forward, y up, z right, metres), so each bone is made a
/// root with its world transform as its own. glTF to Unreal is the swap of y and z (and so a reflection: the turn about an
/// axis is negated): a place `(x, y, z) m` is `(x, z, y) * 100 cm`, a turn `(x, y, z, w)` is `(-x, -z, -y, w)`. Checked on
/// the KTM 450 SMR against the sockets' places the Unreal skeleton gives (`SOCKET_L_HANDLE_POSE` at (11.76, -33.35, 113.76)
/// cm in the chassis frame).
pub fn bike_skeleton(parts: &Value) -> Result<JsonSkeleton> {
    let bones = parts.get("bones").and_then(Value::as_object).context("parts.json has no bones")?;
    let sockets = parts.get("sockets").and_then(Value::as_object).context("parts.json has no sockets")?;
    let mut out = JsonSkeleton { bones: Vec::new(), sockets: Vec::new() };
    for (name, bone) in bones {
        let p: [f32; 3] = floats(bone.get("position"), &format!("bone {name} position"))?;
        let q: [f32; 4] = floats(bone.get("rotation"), &format!("bone {name} rotation"))?;
        out.bones.push(JsonBone {
            name: name.clone(),
            parent: -1,
            t: [p[0] * 100.0, p[2] * 100.0, p[1] * 100.0],
            q: [-q[0], -q[2], -q[1], q[3]],
            s: [1.0; 3],
        });
    }
    for (name, socket) in sockets {
        let bone = socket.get("bone").and_then(Value::as_str).with_context(|| format!("socket {name} has no bone"))?;
        out.sockets.push(JsonSocket {
            name: name.clone(),
            bone: bone.to_owned(),
            t: floats(socket.get("location_cm"), &format!("socket {name} location_cm"))?,
            r: floats(socket.get("rotation_deg"), &format!("socket {name} rotation_deg"))?,
            s: [1.0; 3],
        });
    }
    Ok(out)
}

/// The row of `table` (the parsed `RiderAnimationCorrection_DT` dump) for `vehicle`, as the file's `correction` object: the
/// numbers by the keys of [`CORRECTION_FIELDS`], `pole_elbow_cm` (the translation of `TransformPVElbowCorrection`), and the
/// two enumerations as text. `None` when the table has no row for the vehicle.
pub fn correction(table: &Value, vehicle: &str) -> Option<Value> {
    let rows = match table {
        Value::Array(items) => items.first()?.get("Rows")?,
        other => other.get("Rows")?,
    };
    let row = rows.get(vehicle)?;
    let mut out = Map::new();
    for (key, field) in CORRECTION_FIELDS {
        out.insert(key.to_owned(), json!(row.get(field)?.as_f64()?));
    }
    let shift = row.get("TransformPVElbowCorrection")?.get("Translation")?;
    let axis = |k: &str| shift.get(k).and_then(Value::as_f64);
    out.insert("pole_elbow_cm".to_owned(), json!([axis("X")?, axis("Y")?, axis("Z")?]));
    let name = |field: &str| row.get(field).and_then(Value::as_str).map(|s| s.rsplit("::").next().unwrap_or(s));
    out.insert("bike_type".to_owned(), json!(name("BikeType")?));
    out.insert("quick_shift".to_owned(), json!(name("QuickShiftType")?));
    Some(Value::Object(out))
}

/// The table in the physics dump folder `physics`, if the dump has it.
pub fn read_correction_table(physics: &Path) -> Option<Value> {
    let path: PathBuf = CORRECTION_TABLE.iter().fold(physics.to_path_buf(), |dir, part| dir.join(part));
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// `contacts.json` for the bike of `parts` (the parsed `parts.json`); `table` is the correction table, if there is one.
pub fn contacts_json(parts: &Value, table: Option<&Value>) -> Result<Value> {
    let points = points(&bike_skeleton(parts)?)?;
    let vehicle = parts.get("vehicle").and_then(Value::as_str).context("parts.json has no vehicle")?;
    let mut out = points.placement();
    out.insert("version".to_owned(), json!(VERSION));
    out.insert("vehicle".to_owned(), json!(vehicle));
    out.insert(
        "frame".to_owned(),
        json!("car: x forward, y left, z up, metres, bike rest pose, origin of the bike skeleton"),
    );
    if let Some(row) = table.and_then(|table| correction(table, vehicle)) {
        out.insert("correction".to_owned(), row);
    }
    Ok(Value::Object(out))
}

/// Adds `style` to `contacts`: the clip set of the bike's type (`style::choose`: the override of its correction row, else its
/// input handler) with where it came from. `handler` is the class name of the bike's input handler, if its physics file is known.
/// A bike whose type neither says gets no `style` and keeps the rig's shared graph.
pub fn add_style(contacts: &mut Value, handler: Option<&str>) {
    let over = contacts.get("correction").and_then(|c| c.get("bike_type")).and_then(Value::as_str);
    let Some((set, source)) = style::choose(over, handler) else { return };
    contacts["style"] = json!({
        "name": set.name,
        "bike_type": set.bike_type,
        "source": source.name(),
        "input_handler": handler,
    });
}

/// The input handler class of `vehicle` in the physics dump `physics`, if the dump has the vehicle's resolved file.
fn handler_of(physics: &Path, vehicle: &str) -> Option<String> {
    let file = crate::bike_table::locate(physics, vehicle).ok()?;
    style::input_handler_of(&file).ok().flatten()
}

/// Adds `seat_surface` to `contacts` (the seat's profile under the rider, measured on the bike's exported chassis mesh), if the
/// folder has the meshes: without them the file has no seat and the game leaves the rider where the skeleton's socket puts him.
fn add_seat_surface(contacts: &mut Value, bike_dir: &Path) {
    let Some(seat) = contacts.get("rider_root_in_bike").and_then(|s| floats::<3>(s.get("translation"), "seat").ok())
    else {
        return;
    };
    if !bike_dir.join("glb").is_dir() {
        return;
    }
    match super::seat_surface::surface(bike_dir, Vec3::from_array(seat)) {
        Ok(points) => contacts["seat_surface"] = json!(points),
        Err(error) => println!("{}: no seat surface: {error:#}", bike_dir.display()),
    }
}

/// Write `RIDER/contacts.json` into `car_dir` (`files/CARS/<ID>/`) from the bike's `parts.json` in `bike_dir`. `physics` is
/// the physics dump folder, for the correction row. Returns whether the vehicle had a correction row.
pub fn write(bike_dir: &Path, car_dir: &Path, physics: &Path) -> Result<bool> {
    let path = bike_dir.join("parts.json");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parts: Value = serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    let table = read_correction_table(physics);
    let mut json =
        contacts_json(&parts, table.as_ref()).with_context(|| format!("contact points of {}", path.display()))?;
    add_seat_surface(&mut json, bike_dir);
    let vehicle = parts.get("vehicle").and_then(Value::as_str).unwrap_or_default();
    add_style(&mut json, handler_of(physics, vehicle).as_deref());
    let folder = car_dir.join("RIDER");
    fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    fs::write(folder.join(FILE), serde_json::to_vec_pretty(&json)?).context("writing contacts.json")?;
    Ok(json.get("correction").is_some())
}

#[cfg(test)]
#[path = "contacts_tests.rs"]
mod tests;
