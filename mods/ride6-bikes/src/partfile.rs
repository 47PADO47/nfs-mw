//! The part table of one bike: `parts.json`, written by the exporter script (`docs/modding/ride6-bikes.md`).
//!
//! The file lists the stock parts of a bike (the chassis, tyres, rims, brake discs, exhaust, bar pad, hand guards), where
//! each sits on the chassis skeleton, which material instance each of its surfaces uses, and the effective parameters of
//! every material (textures, shading model, blend mode). The importer reads nothing about a bike from anywhere else, so a
//! new bike is a new `parts.json` and its exported meshes and textures.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::front_end::Rules;

/// Where a part's frame sits on the chassis: its rigid transform into the chassis frame (glTF axes, metres).
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub bone: Option<String>,
    pub translation: [f32; 3],
    /// Unit quaternion x, y, z, w.
    pub rotation: [f32; 4],
}

impl Anchor {
    /// Move a point of the part's frame into the chassis frame.
    pub fn point(&self, p: [f32; 3]) -> [f32; 3] {
        let r = rotate(self.rotation, p);
        [r[0] + self.translation[0], r[1] + self.translation[1], r[2] + self.translation[2]]
    }

    /// Turn a direction of the part's frame into the chassis frame.
    pub fn direction(&self, d: [f32; 3]) -> [f32; 3] {
        rotate(self.rotation, d)
    }
}

/// Rotate `v` by the unit quaternion `q` (x, y, z, w).
fn rotate([x, y, z, w]: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    // v' = v + 2w (u x v) + 2 u x (u x v), u = (x, y, z)
    let u = [x, y, z];
    let cross =
        |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let t = cross(u, v).map(|c| 2.0 * c);
    let t2 = cross(u, t);
    [v[0] + w * t[0] + t2[0], v[1] + w * t[1] + t2[1], v[2] + w * t[2] + t2[2]]
}

/// What a part is, which picks the shading of its surfaces and whether it turns with a wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Body,
    Tyre,
    Rim,
    Brake,
}

/// Which solid a part is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Body,
    FrontWheel,
    BackWheel,
}

#[derive(Debug, Clone)]
pub struct Part {
    pub id: String,
    pub kind: Kind,
    pub role: Role,
    /// The exported mesh's file stem (`chassis369` is `chassis369.glb`, its levels of detail `chassis369_LOD2.glb`).
    pub export: String,
    pub anchor: Anchor,
    /// Material name in the exported mesh to the material instance it stands for.
    pub materials: HashMap<String, String>,
}

/// One material instance with its parameters resolved through its parents.
#[derive(Debug, Clone, Default)]
pub struct Material {
    pub name: String,
    pub shading_model: Option<String>,
    pub blend_mode: Option<String>,
    /// The opacity mask's clip value of a masked material.
    pub clip: Option<f32>,
    /// Texture of the base colour (file stem in `tex/`), and whether its alpha is the opacity mask.
    pub base_color: Option<String>,
    pub opacity_in_base: bool,
    pub normal: Option<String>,
    pub pbr: Option<String>,
    /// The packed texture holds specular, roughness, metallic and occlusion (else the first three only).
    pub pbr_has_occlusion: bool,
}

pub struct PartTable {
    pub vehicle: String,
    /// The physics asset the game's `PHYSICS` table names for the bike (`/PhysicsRide6/.../KTM_450SMR_2022.KTM_450SMR_2022`):
    /// the physics dump's file is found by its name, which does not always repeat the vehicle id. Left out by older files.
    pub physics_asset: Option<String>,
    pub parts: Vec<Part>,
    pub materials: HashMap<String, Material>,
    /// Tyre size codes (`120-60-17`) of the `front` and `back` wheel.
    pub tyre_sizes: HashMap<String, String>,
    /// Bone world positions of the chassis skeleton in the chassis frame (glTF axes, metres).
    pub bones: HashMap<String, [f32; 3]>,
    /// Which bones steer and which slide (`front_end.rs`): the optional `front_end` object of the file (`steer_roots`,
    /// `slide_roots`, `slide_followers`), else RIDE 6's bone names.
    pub front_end: Rules,
}

fn string(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn floats<const N: usize>(v: Option<&Value>) -> Option<[f32; N]> {
    let list = v?.as_array()?;
    if list.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, value) in out.iter_mut().zip(list) {
        *slot = value.as_f64()? as f32;
    }
    Some(out)
}

/// The last path component of an Unreal object path without its `.N` suffix, lower case (`/Game/a/b.b.0` is `b`).
pub fn asset_stem(path: &str) -> String {
    let last = path.rsplit('/').next().unwrap_or(path);
    last.split('.').next().unwrap_or(last).to_ascii_lowercase()
}

fn kind_of(id: &str) -> Kind {
    match id.rsplit('_').next().unwrap_or(id) {
        "tyre" => Kind::Tyre,
        "rim" => Kind::Rim,
        "brake" => Kind::Brake,
        _ => Kind::Body,
    }
}

fn role_of(text: &str) -> Option<Role> {
    match text {
        "body" => Some(Role::Body),
        "front_wheel" => Some(Role::FrontWheel),
        "back_wheel" => Some(Role::BackWheel),
        _ => None,
    }
}

fn read_part(value: &Value) -> Result<Part> {
    let id = string(value, "id").context("a part has no id")?;
    let role_text = string(value, "role").with_context(|| format!("part {id} has no role"))?;
    let role = role_of(&role_text).with_context(|| format!("part {id}: unknown role {role_text}"))?;
    let export = string(value, "export").with_context(|| format!("part {id} has no export"))?;
    let anchor = value.get("anchor").with_context(|| format!("part {id} has no anchor"))?;
    let anchor = Anchor {
        bone: string(anchor, "bone"),
        translation: floats(anchor.get("translation")).with_context(|| format!("part {id}: bad anchor translation"))?,
        rotation: floats(anchor.get("rotation")).with_context(|| format!("part {id}: bad anchor rotation"))?,
    };
    let materials = value
        .get("materials")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(name, path)| Some((name.to_ascii_lowercase(), asset_stem(path.as_str()?))))
                .collect()
        })
        .unwrap_or_default();
    Ok(Part { kind: kind_of(&id), id, role, export, anchor, materials })
}

/// The steering rules of `front_end` (`{"steer_roots": [..], "slide_roots": [..], "slide_followers": [..]}`), RIDE 6's where
/// the file has none; a list the object leaves out is empty.
fn read_rules(value: Option<&Value>) -> Rules {
    let Some(value) = value else { return Rules::ride6() };
    let names = |key: &str| -> Vec<String> {
        let list = value.get(key).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
        list.iter().filter_map(|name| name.as_str().map(str::to_owned)).collect()
    };
    Rules {
        steer_roots: names("steer_roots"),
        slide_roots: names("slide_roots"),
        slide_followers: names("slide_followers"),
    }
}

/// The texture of a material parameter, unless it is one of the engine's flat default pictures. The tyre master
/// material names its parameters `BaseColor`, `CleanNormal` and `Pbr`; the vehicle master `... - Texture`.
fn texture(tex: &Value, key: &str) -> Option<String> {
    let path = tex.get(key)?.as_str()?;
    let stem = asset_stem(path);
    // The engine's flat default pictures carry no data of the bike.
    (!stem.starts_with("t_default") && !stem.starts_with("t_white") && !stem.starts_with("t_black")).then_some(stem)
}

fn read_material(name: &str, value: &Value) -> Material {
    let tex = value.get("tex").cloned().unwrap_or(Value::Null);
    let switch = |key: &str| value.get("switch").and_then(|s| s.get(key)).and_then(Value::as_bool).unwrap_or(false);
    let opacity_in_base = switch("BaseColorOpacity - Source");
    let base_key = if opacity_in_base { "BaseColorOpacity - Texture" } else { "BaseColor - Texture" };
    Material {
        name: name.to_owned(),
        shading_model: string(value, "shading_model"),
        blend_mode: string(value, "blend_mode"),
        clip: value.get("clip").and_then(Value::as_f64).map(|c| c as f32),
        base_color: texture(&tex, base_key).or_else(|| texture(&tex, "BaseColor")),
        opacity_in_base,
        normal: texture(&tex, "Normal - Texture").or_else(|| texture(&tex, "CleanNormal")),
        pbr: texture(&tex, "PBR - Texture").or_else(|| texture(&tex, "Pbr")),
        pbr_has_occlusion: switch("LegacyTexturesFormats - PBR - S_R_M_O"),
    }
}

impl Material {
    /// Whether the base colour's alpha cuts the surface out (a masked material that reads its opacity from the picture).
    pub fn cutout(&self) -> Option<f32> {
        if self.blend_mode.as_deref() != Some("BLEND_Masked") || !self.opacity_in_base {
            return None;
        }
        self.clip
    }

    /// A decal laid over glass: not part of the bike's body.
    pub fn is_glass_decal(&self) -> bool {
        self.name.to_ascii_lowercase().contains("glassdecal")
    }
}

impl PartTable {
    /// The name the physics dump's file for this bike is looked up by: the physics asset's own name when the file has it, else
    /// the vehicle id.
    pub fn physics_name(&self) -> String {
        let Some(asset) = &self.physics_asset else { return self.vehicle.clone() };
        let last = asset.rsplit('/').next().unwrap_or(asset);
        last.split('.').next().unwrap_or(last).to_owned()
    }

    pub fn parse(text: &str) -> Result<Self> {
        let doc: Value = serde_json::from_str(text).context("parts.json is not JSON")?;
        let schema = doc.get("schema").and_then(Value::as_u64).unwrap_or(0);
        if schema != 1 {
            bail!("parts.json schema {schema} is not supported (this importer reads schema 1)");
        }
        let vehicle = string(&doc, "vehicle").context("no vehicle")?;
        let physics_asset = string(&doc, "physics_asset");
        let parts = doc
            .get("parts")
            .and_then(Value::as_array)
            .context("no parts")?
            .iter()
            .map(read_part)
            .collect::<Result<Vec<_>>>()?;
        let materials = doc
            .get("materials")
            .and_then(Value::as_object)
            .map(|map| map.iter().map(|(name, v)| (name.to_ascii_lowercase(), read_material(name, v))).collect())
            .unwrap_or_default();
        let bones = doc
            .get("bones")
            .and_then(Value::as_object)
            .map(|map| map.iter().filter_map(|(name, v)| Some((name.clone(), floats(v.get("position"))?))).collect())
            .unwrap_or_default();
        let tyre_sizes = doc
            .get("tyre_sizes")
            .and_then(Value::as_object)
            .map(|map| map.iter().filter_map(|(side, size)| Some((side.clone(), size.as_str()?.to_owned()))).collect())
            .unwrap_or_default();
        let front_end = read_rules(doc.get("front_end"));
        Ok(Self { vehicle, physics_asset, parts, materials, tyre_sizes, bones, front_end })
    }

    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join("parts.json");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// The material instance a surface of `part` named `glb_name` stands for.
    pub fn material(&self, part: &Part, glb_name: &str) -> Option<&Material> {
        let instance =
            part.materials.get(&glb_name.to_ascii_lowercase()).cloned().unwrap_or_else(|| asset_stem(glb_name));
        self.materials.get(&instance)
    }

    /// A bone's position in the chassis frame.
    pub fn bone(&self, name: &str) -> Result<[f32; 3]> {
        self.bones.get(name).copied().with_context(|| format!("the chassis skeleton has no bone {name}"))
    }

    pub fn parts_of(&self, role: Role) -> impl Iterator<Item = &Part> {
        self.parts.iter().filter(move |p| p.role == role)
    }

    pub fn chassis(&self) -> Result<&Part> {
        self.parts.iter().find(|p| p.id == "chassis").context("the part table has no chassis")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const SAMPLE: &str = r#"{
      "schema": 1, "vehicle": "TEST_BIKE", "tyre_sizes": {"front": "120-60-17", "back": "160-60-17"},
      "bones": {"BONE_F_VIS_WHEEL": {"position": [0.7, 0.2, 0.0], "rotation": [0,0,0,1]},
                "BONE_B_VIS_WHEEL": {"position": [-0.7, 0.2, 0.0], "rotation": [0,0,0,1]}},
      "parts": [
        {"id": "chassis", "role": "body", "export": "chassis1", "anchor": {"bone": "BONE_CHASSIS", "translation": [0,0,0], "rotation": [0,0,0,1]},
         "materials": {"mat_a": "/Game/x/mat_a.0"}},
        {"id": "front_tyre", "role": "front_wheel", "export": "f_tyre1", "anchor": {"bone": "BONE_F_VIS_WHEEL", "translation": [0.7,0.2,0], "rotation": [0,0,0,1]},
         "materials": {"mat_t": "/Game/x/Mat_T.0"}}
      ],
      "materials": {
        "mat_a": {"shading_model": "MSM_ClearCoat", "blend_mode": "BLEND_Masked",
                  "tex": {"BaseColor - Texture": "/Game/x/livery_d.0", "PBR - Texture": "/Game/x/livery_l.0", "Normal - Texture": "/Engine/T_Default_N.0"},
                  "switch": {"LegacyTexturesFormats - PBR - S_R_M_O": true}},
        "mat_t": {"blend_mode": "BLEND_Masked", "clip": 0.3333, "tex": {"BaseColorOpacity - Texture": "/Game/x/tyre_d.0"}, "switch": {"BaseColorOpacity - Source": true}}
      }
    }"#;

    #[test]
    fn the_physics_file_is_looked_up_by_the_assets_name_when_the_table_has_one() {
        let table = PartTable::parse(SAMPLE).unwrap();
        assert_eq!(table.physics_name(), "TEST_BIKE");
        let text = SAMPLE.replacen(
            "\"vehicle\": \"TEST_BIKE\",",
            "\"vehicle\": \"TEST_BIKE\", \"physics_asset\": \"/PhysicsRide6/Physics/V/KTM/KTM_450SMR_2022.KTM_450SMR_2022\",",
            1,
        );
        assert_eq!(PartTable::parse(&text).unwrap().physics_name(), "KTM_450SMR_2022");
    }

    #[test]
    fn a_table_reads_parts_materials_and_bones() {
        let table = PartTable::parse(SAMPLE).unwrap();
        assert_eq!(table.vehicle, "TEST_BIKE");
        assert_eq!(table.parts.len(), 2);
        assert_eq!(table.parts[1].kind, Kind::Tyre);
        assert_eq!(table.parts[1].role, Role::FrontWheel);
        assert_eq!(table.bone("BONE_B_VIS_WHEEL").unwrap(), [-0.7, 0.2, 0.0]);
        assert!(table.bone("NOPE").is_err());
        assert_eq!(table.chassis().unwrap().export, "chassis1");
        assert_eq!(table.tyre_sizes.get("back").map(String::as_str), Some("160-60-17"));
    }

    #[test]
    fn a_material_takes_the_textures_its_switches_name_and_ignores_engine_defaults() {
        let table = PartTable::parse(SAMPLE).unwrap();
        let a = table.material(&table.parts[0], "mat_a").unwrap();
        assert_eq!(a.base_color.as_deref(), Some("livery_d"));
        assert_eq!(a.pbr.as_deref(), Some("livery_l"));
        assert_eq!(a.normal, None, "the engine's default normal map is no data");
        assert!(a.pbr_has_occlusion && !a.opacity_in_base);
        let t = table.material(&table.parts[1], "mat_t").unwrap();
        assert_eq!(t.base_color.as_deref(), Some("tyre_d"));
        assert!(t.opacity_in_base);
        assert_eq!(t.clip, Some(0.3333));
        assert!(table.material(&table.parts[1], "unknown").is_none());
    }

    #[test]
    fn the_steering_rules_are_ride6s_unless_the_file_names_its_own() {
        let table = PartTable::parse(SAMPLE).unwrap();
        assert_eq!(table.front_end, Rules::ride6());
        let own = SAMPLE.replacen("\"schema\": 1,", "\"schema\": 1, \"front_end\": {\"steer_roots\": [\"FORK\"]},", 1);
        let table = PartTable::parse(&own).unwrap();
        assert_eq!(
            table.front_end,
            Rules { steer_roots: vec!["FORK".to_owned()], slide_roots: Vec::new(), slide_followers: Vec::new() }
        );
    }

    #[test]
    fn schemas_other_than_one_are_refused() {
        assert!(PartTable::parse(r#"{"schema": 2}"#).is_err());
        assert!(PartTable::parse("[").is_err());
    }

    #[test]
    fn the_anchor_rotates_then_moves() {
        // A quarter turn about z takes x to y.
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let anchor = Anchor { bone: None, translation: [1.0, 0.0, 0.0], rotation: [0.0, 0.0, half, half] };
        let p = anchor.point([1.0, 0.0, 0.0]);
        assert!((p[0] - 1.0).abs() < 1e-5 && (p[1] - 1.0).abs() < 1e-5 && p[2].abs() < 1e-5, "{p:?}");
        let d = anchor.direction([0.0, 1.0, 0.0]);
        assert!((d[0] + 1.0).abs() < 1e-5 && d[1].abs() < 1e-5, "{d:?}");
    }
}
