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

use crate::fallback::{self, Fallback};
use crate::front_end::Rules;

/// Where a part's frame sits on the chassis: its transform into the chassis frame (glTF axes, metres): the scale of each axis,
/// then the rotation, then the translation. A stock part is rigid (scale 1); a silencer takes the scale of the bone of its
/// header pipes that it is fitted to (0.8 to 1.1, `docs/formats/ride6-bike-parts.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub bone: Option<String>,
    pub translation: [f32; 3],
    /// Unit quaternion x, y, z, w.
    pub rotation: [f32; 4],
    /// The scale of each axis of the part's frame; 1 for a rigid anchor.
    pub scale: [f32; 3],
}

impl Anchor {
    /// A rigid anchor: no scale.
    pub fn rigid(bone: Option<String>, translation: [f32; 3], rotation: [f32; 4]) -> Self {
        Self { bone, translation, rotation, scale: [1.0; 3] }
    }

    /// Move a point of the part's frame into the chassis frame.
    pub fn point(&self, p: [f32; 3]) -> [f32; 3] {
        let r = rotate(self.rotation, [p[0] * self.scale[0], p[1] * self.scale[1], p[2] * self.scale[2]]);
        [r[0] + self.translation[0], r[1] + self.translation[1], r[2] + self.translation[2]]
    }

    /// Turn a direction of the part's frame into the chassis frame: a normal of a scaled part is divided by the scale of its axis
    /// (the inverse transpose), so it stays square to the surface; the caller normalises it. For a rigid anchor, a turn.
    pub fn direction(&self, d: [f32; 3]) -> [f32; 3] {
        let unscaled = [d[0] / self.scale[0], d[1] / self.scale[1], d[2] / self.scale[2]];
        rotate(self.rotation, unscaled)
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
    /// The transmittance picture of a translucent material (`Transmittance - Texture`) and its `Modulation - Transmittance -
    /// Max` (the red channel of the vector; a material that has none transmits all).
    pub transmittance: Option<String>,
    pub transmittance_max: Option<f32>,
}

/// One livery of a bike (`liveries[]` of `parts.json`, absent from older files): the chassis material instances it replaces.
/// The meshes are the stock ones; a livery is a set of other pictures on the same surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Livery {
    /// The number of the `CH_<id>_Livery-NN` row, from 1.
    pub index: u32,
    /// Material name in the exported chassis mesh (lower case) to the instance that replaces it in this livery (the file stem
    /// of the instance, lower case). Only the materials the livery changes are listed.
    pub materials: HashMap<String, String>,
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
    /// The liveries of the bike in the order of their rows (the stock one is the first); empty in files that have none.
    pub liveries: Vec<Livery>,
    /// Bone world positions of the chassis skeleton in the chassis frame (glTF axes, metres).
    pub bones: HashMap<String, [f32; 3]>,
    /// Which bones steer and which slide (`front_end.rs`): the optional `front_end` object of the file (`steer_roots`,
    /// `slide_roots`, `slide_followers`), else RIDE 6's bone names.
    pub front_end: Rules,
    /// The surfaces that wear another model's material, with where it came from (`fallbacks[]`; the manifest keeps the record).
    pub fallbacks: Vec<Fallback>,
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

/// The kind of a chassis surface by its name without the numbers (`mat_livery_0_482_003` and `mat_livery_1` are `mat_livery`):
/// only the kinds that carry a picture of the bike's own.
fn material_kind(name: &str) -> Option<&str> {
    let end = name.find(|c: char| c.is_ascii_digit()).unwrap_or(name.len());
    let kind = name[..end].trim_end_matches('_');
    ["mat_livery", "mat_mechanics", "mat_cockpit", "mat_gear", "mat_light", "mat_lights"]
        .contains(&kind)
        .then_some(kind)
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

fn read_livery(value: &Value) -> Option<Livery> {
    let index = u32::try_from(value.get("index")?.as_u64()?).ok()?;
    let materials = value
        .get("materials")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(name, path)| Some((name.to_ascii_lowercase(), asset_stem(path.as_str()?))))
                .collect()
        })
        .unwrap_or_default();
    Some(Livery { index, materials })
}

fn read_part(value: &Value) -> Result<Part> {
    let id = string(value, "id").context("a part has no id")?;
    let role_text = string(value, "role").with_context(|| format!("part {id} has no role"))?;
    let role = role_of(&role_text).with_context(|| format!("part {id}: unknown role {role_text}"))?;
    let export = string(value, "export").with_context(|| format!("part {id} has no export"))?;
    let anchor = value.get("anchor").with_context(|| format!("part {id} has no anchor"))?;
    let anchor = Anchor::rigid(
        string(anchor, "bone"),
        floats(anchor.get("translation")).with_context(|| format!("part {id}: bad anchor translation"))?,
        floats(anchor.get("rotation")).with_context(|| format!("part {id}: bad anchor rotation"))?,
    );
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
    // Which of the two parameters holds the picture is not always what the switch says: a silencer, a header and a cockpit have the
    // opacity switch on and the picture in `BaseColor - Texture`; the old rims' master names it `BaseColor0_Map`.
    let (first, second) = match opacity_in_base {
        true => ("BaseColorOpacity - Texture", "BaseColor - Texture"),
        false => ("BaseColor - Texture", "BaseColorOpacity - Texture"),
    };
    let base_color = [first, second, "BaseColor", "BaseColor0_Map"].iter().find_map(|key| texture(&tex, key));
    Material {
        name: name.to_owned(),
        shading_model: string(value, "shading_model"),
        blend_mode: string(value, "blend_mode"),
        clip: value.get("clip").and_then(Value::as_f64).map(|c| c as f32),
        base_color,
        opacity_in_base,
        normal: ["Normal - Texture", "CleanNormal", "Normal0_Map"].iter().find_map(|key| texture(&tex, key)),
        pbr: ["PBR - Texture", "Pbr", "Pbr0_Map"].iter().find_map(|key| texture(&tex, key)),
        pbr_has_occlusion: switch("LegacyTexturesFormats - PBR - S_R_M_O"),
        transmittance: texture(&tex, "Transmittance - Texture"),
        transmittance_max: value
            .get("vector")
            .and_then(|v| v.get("Modulation - Transmittance - Max"))
            .and_then(|v| v.get(0))
            .and_then(Value::as_f64)
            .map(|m| m as f32),
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

    /// Whether the surface lets the scene behind it through (the screen, the lamp lenses): `BLEND_TranslucentGreyTransmittance`,
    /// the blend mode of the thin translucent shading model.
    pub fn is_translucent(&self) -> bool {
        self.blend_mode.as_deref() == Some("BLEND_TranslucentGreyTransmittance")
    }

    /// The picture whose colour is the transmittance of a translucent material: its own transmittance picture, else the base
    /// colour (the material overrides the base colour with the transmittance colour).
    pub fn transmittance_picture(&self) -> Option<&str> {
        self.transmittance.as_deref().or(self.base_color.as_deref())
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
        let mut liveries: Vec<Livery> = doc
            .get("liveries")
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(read_livery).collect())
            .unwrap_or_default();
        liveries.sort_by_key(|livery| livery.index);
        let fallbacks = fallback::read_all(&doc);
        Ok(Self { vehicle, physics_asset, parts, materials, tyre_sizes, liveries, bones, front_end, fallbacks })
    }

    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join("parts.json");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// The material instance a surface of `part` named `glb_name` stands for.
    ///
    /// A slot the data asset leaves without an instance (`mat_livery_1` of the Brutale 1000 RR, `mat_mechanics_1` of a mirror, the far
    /// levels' `mat_bike_lods` of a rim) takes [`Self::sibling_material`] instead of the grey stand-in.
    pub fn material(&self, part: &Part, glb_name: &str) -> Option<&Material> {
        let lower = glb_name.to_ascii_lowercase();
        let instance = part.materials.get(&lower).cloned().unwrap_or_else(|| asset_stem(glb_name));
        self.materials.get(&instance).or_else(|| self.sibling_material(part, &lower))
    }

    /// The instance a surface with none of its own takes: the far levels' composite slot (`mat_bike_lods`, `base_livery_lod`) draws
    /// the part's own first picture; any other slot the chassis' instance of the same kind (the name without its numbers).
    fn sibling_material(&self, part: &Part, name: &str) -> Option<&Material> {
        let pictured = |instance: &String| self.materials.get(instance).filter(|m| m.base_color.is_some());
        if name.contains("bike_lods") || name.contains("livery_lod") {
            let mut own: Vec<(&String, &String)> = part.materials.iter().collect();
            own.sort();
            return own.into_iter().find_map(|(_, instance)| pictured(instance));
        }
        let kind = material_kind(name)?;
        let mut chassis: Vec<(&String, &String)> = self.chassis().ok()?.materials.iter().collect();
        chassis.sort();
        chassis
            .into_iter()
            .filter(|(slot, _)| material_kind(slot) == Some(kind))
            .find_map(|(_, instance)| pictured(instance))
    }

    /// The livery material of the chassis: the name it has in the exported mesh and its parameters. The first material whose name
    /// contains `livery` (so `mat_livery_0_..` before `mat_livery_1_..`) that has a base colour picture.
    pub fn livery_material(&self) -> Option<(String, &Material)> {
        let chassis = self.chassis().ok()?;
        let mut names: Vec<&String> = chassis.materials.keys().filter(|name| name.contains("livery")).collect();
        names.sort();
        names.into_iter().find_map(|name| {
            let material = self.materials.get(chassis.materials.get(name)?)?;
            material.base_color.as_ref()?;
            Some((name.clone(), material))
        })
    }

    /// The livery pictures of the bike: the number of each livery and its livery material (the stock livery's own, for a livery
    /// that does not replace it), when at least two of them have different base colour pictures; else none (a bike with one
    /// look has no liveries to offer).
    pub fn livery_pictures(&self) -> Vec<(u32, Material)> {
        let Some((name, stock)) = self.livery_material() else { return Vec::new() };
        let mut out = Vec::new();
        for livery in &self.liveries {
            let material =
                livery.materials.get(&name).and_then(|instance| self.materials.get(instance)).unwrap_or(stock);
            if material.base_color.is_none() {
                continue;
            }
            out.push((livery.index, material.clone()));
        }
        let mut pictures: Vec<&Option<String>> = out.iter().map(|(_, m)| &m.base_color).collect();
        pictures.sort();
        pictures.dedup();
        if pictures.len() < 2 {
            return Vec::new();
        }
        out
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
#[path = "partfile_tests.rs"]
mod tests;
