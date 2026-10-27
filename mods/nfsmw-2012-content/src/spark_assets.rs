//! The collision sparks of the 2012 game, as the files of the `SPARKS` and `XENON` effect variants (`profile.toml` and PNG
//! pictures under `files/SPARKS/MW2012/` and `files/XENON/MW2012/`). Spec of the profiles:
//! `docs/modding/effect-variants-sparks-xenon.md`; what the effects hold: `docs/formats/chameleon-collision-effects.md`.
//!
//! `GLOBALEFFECTS.BNDL` has a `VfxVehicleCollisions` object (twice: one for impacts, one for scrapes) that lists the effects the
//! game plays when a car hits something or slides along it. The effect most listed in each is the default surface's. Their
//! emitters spawn on the collision mesh of the object that was hit (`EmitterCollisionMesh`); the ones that draw a spark are
//! stretched sprites (`RenderSpriteVelocity`), the flash and the dust are flipbook sprites. This module reads those emitters with
//! `chameleon-effects`, takes the numbers at four ages of the particle's life and keeps them in a [`Set`]; [`profile`] turns the
//! set into the two profiles. The debris (meshes) and the 3D ground sprites are left out: the host draws neither.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Resource, ResourceKind};
use chameleon_effects::collision::{self, Source as Input};
use chameleon_effects::{Effect, ops};
use chameleon_gfx::{Material, Texture};

use crate::source::Source;
use crate::vfx::{self, GLOBAL_EFFECTS, Picture};

mod profile;
#[cfg(test)]
mod tests;

/// The id of the variant in the `collision_spark_variant` and `xenon_spark_variant` options, and what the menu shows for it.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folders of the variants under the mod's `files/`.
pub const SPARKS_FOLDER: &str = "SPARKS/MW2012";
pub const XENON_FOLDER: &str = "XENON/MW2012";

const COLLISIONS_TYPE: &str = "Genesys.Gen.VfxVehicleCollisions";
const TYPE_FLAG: u32 = 0x8000_0000;
/// The ages (0 to 1 of the life) the colour, alpha and size curves are sampled at. The last is just short of the end, where the
/// 2012 curves have already dropped to zero.
pub const KEYS: [f32; 4] = [0.0, 0.33, 0.66, 0.99];

/// How a spark is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A segment stretched along the velocity (`RenderSpriteVelocity`), with no picture.
    Streak,
    /// A camera-facing sprite or flipbook (`RenderSprite`, `RenderSpriteSubUV`).
    Sprite,
}

/// The picture of a sprite emitter and the cells per side of its sheet (1: the whole picture).
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub picture: Picture,
    pub grid: u32,
}

/// One emitter of a collision effect, with the numbers at the four [`KEYS`].
#[derive(Debug, Clone, PartialEq)]
pub struct Spark {
    pub kind: Kind,
    /// The shortest and longest life, seconds.
    pub life: [f32; 2],
    /// Hue (the gradient's red, green and blue), the brightness multiplier (above 1: the 2012 game blooms it) and the opacity.
    pub rgb: [[f32; 3]; 4],
    pub brightness: [f32; 4],
    pub alpha: [f32; 4],
    /// Full width in metres: twice the mean starting radius times the radius curve.
    pub width: [f32; 4],
    /// The speed range in metres a second at full strength, the largest angle in degrees from the surface normal, and the
    /// share of the owner's velocity a particle takes (range).
    pub speed: [f32; 2],
    pub tilt: f32,
    pub inherit: [f32; 2],
    /// Particles per square metre of the surface: all at once for an impact, a second for a scrape. The radius in metres of the
    /// patch of the surface that is used.
    pub density: f32,
    pub patch_radius: f32,
    pub bounce: Option<collision::Bounce>,
    /// The seconds of travel a streak shows (`length_per_speed`; the executable's own factor is taken as 1).
    pub stretch: Option<f32>,
    pub sheet: Option<Sheet>,
}

/// What strength the effect is played at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gate {
    /// The impact's closing speed scales the count and the speed: from `floor` at rest to 1 at `full_speed` metres a second.
    Impact { floor: f32, full_speed: f32 },
    /// The scrape's sliding speed scales the rate: nothing below `start` metres a second, full from `full`.
    Scrape { start: f32, full: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub gate: Gate,
    pub sparks: Vec<Spark>,
}

/// The impact effect and the scrape effect of the install.
#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub hit: Channel,
    pub scrape: Channel,
}

/// Read the install's collision effects from `GLOBALEFFECTS.BNDL`.
pub fn read(bundle: &Bundle) -> Result<Set> {
    let (mut hit, mut scrape) = (None, None);
    for id in default_effects(bundle) {
        let channel = read_channel(bundle, id).with_context(|| format!("collision effect {id:#x}"))?;
        let gate = channel.gate;
        match gate {
            Gate::Impact { .. } => hit = hit.or(Some(channel)),
            Gate::Scrape { .. } => scrape = scrape.or(Some(channel)),
        }
    }
    match (hit, scrape) {
        (Some(hit), Some(scrape)) => Ok(Set { hit, scrape }),
        _ => bail!("the bundle has no impact effect and scrape effect in a VfxVehicleCollisions object"),
    }
}

/// The effect each `VfxVehicleCollisions` object lists most often (the default surface's).
fn default_effects(bundle: &Bundle) -> Vec<u64> {
    let names = type_names(bundle);
    let object_kind = ResourceKind::GenesysObject.id();
    let mut out = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object_kind || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        let is_collisions = resource.imports.iter().any(|import| {
            import.offset & TYPE_FLAG != 0 && names.get(&import.id).is_some_and(|n| n == COLLISIONS_TYPE)
        });
        if !is_collisions {
            continue;
        }
        let mut counts: HashMap<u64, usize> = HashMap::new();
        let mut order = Vec::new();
        let mut imports: Vec<_> = resource.imports.iter().collect();
        imports.sort_by_key(|import| import.offset);
        for import in imports {
            let is_effect = bundle.find(import.id).is_some_and(|at| bundle.entries()[at].kind == EFFECT_KIND);
            if !is_effect {
                continue;
            }
            if !counts.contains_key(&import.id) {
                order.push(import.id);
            }
            *counts.entry(import.id).or_default() += 1;
        }
        let most =
            order.iter().copied().max_by_key(|id| (counts[id], std::cmp::Reverse(order.iter().position(|o| o == id))));
        out.extend(most);
    }
    out
}

const EFFECT_KIND: u32 = 0x301;

/// The names of the Genesys types of the bundle, by resource id (the name sits after the header the word at 0x20 points to).
fn type_names(bundle: &Bundle) -> HashMap<u64, String> {
    let kind = ResourceKind::GenesysType.id();
    let mut names = HashMap::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if let Some(name) = type_name(&resource.blocks[0]) {
            names.insert(entry.id, name);
        }
    }
    names
}

fn type_name(main: &[u8]) -> Option<String> {
    let header = u32::from_le_bytes(main.get(0x20..0x24)?.try_into().ok()?) as usize;
    let tail = main.get(header.checked_add(8)?..)?;
    let end = tail.iter().position(|&byte| byte == 0)?;
    let name = std::str::from_utf8(&tail[..end]).ok()?;
    (name.len() >= 5 && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')))
        .then(|| name.to_owned())
}

/// The sparks of one effect and the gate that scales it.
fn read_channel(bundle: &Bundle, id: u64) -> Result<Channel> {
    let resource = bundle.resource(bundle.find(id).context("not in the bundle")?)?;
    let effect = Effect::parse(&resource.blocks[0]).map_err(|error| anyhow::anyhow!("{error}"))?;
    let offsets = collision::emitter_offsets(&resource.blocks[0]).map_err(|error| anyhow::anyhow!("{error}"))?;
    let mut sparks = Vec::new();
    let mut gate = None;
    for (n, emitter) in effect.emitters.iter().enumerate() {
        let Some(mesh) = emitter.collision_mesh() else { continue };
        let kind = match emitter.render_op() {
            Some(ops::RENDER_SPRITE_SUB_UV | ops::RENDER_SPRITE) => Kind::Sprite,
            Some(collision::RENDER_SPRITE_VELOCITY) => Kind::Streak,
            _ => continue,
        };
        // A scrape spawns for ever at a rate; an impact once, as a burst.
        let (density, this_gate) = match (mesh.burst, mesh.rate) {
            (burst, 0) if burst > 0 => (burst as f32, impact_gate(emitter)),
            (_, rate) if rate > 0 => (rate as f32, scrape_gate(emitter)),
            _ => continue,
        };
        gate = gate.or(this_gate);
        let end = offsets.get(n + 1).copied().unwrap_or(usize::MAX);
        let span = offsets[n]..end;
        let sheet = match kind {
            Kind::Streak => None,
            Kind::Sprite => Some(sheet_of(bundle, &resource, &span, id, n)?),
        };
        sparks.push(sample(emitter, kind, mesh, density, sheet).context("an emitter with a missing block")?);
    }
    let gate = gate.context("no emitter has a binding that gates it")?;
    Ok(Channel { gate, sparks })
}

/// The strength curve of an impact: `ImpactClosingVelocity` over a scale, through a response curve that starts at a floor and
/// is flat from its second point.
fn impact_gate(emitter: &chameleon_effects::Emitter) -> Option<Gate> {
    let bindings = emitter.bindings()?;
    let binding = bindings.bindings.iter().find(|b| b.source == Input::InputBOver)?;
    let (first, second) = (binding.curve[0], binding.curve[1]);
    Some(Gate::Impact { floor: first.1 + binding.bias, full_speed: second.0 * binding.scale })
}

/// The strength curve of a scrape: the speed times a scale through a curve that is 0 up to its first point and 1 from its
/// second.
fn scrape_gate(emitter: &chameleon_effects::Emitter) -> Option<Gate> {
    let bindings = emitter.bindings()?;
    let binding = bindings.bindings.iter().find(|b| {
        b.source == Input::Speed && b.curve[0].1 == 0.0 && b.curve[1].1 >= 0.99 && b.curve[0].0 > 0.0 && b.scale > 0.0
    })?;
    Some(Gate::Scrape { start: binding.curve[0].0 / binding.scale, full: binding.curve[1].0 / binding.scale })
}

fn sample(
    emitter: &chameleon_effects::Emitter,
    kind: Kind,
    mesh: collision::CollisionMesh,
    density: f32,
    sheet: Option<Sheet>,
) -> Option<Spark> {
    let life = emitter.lifetime()?;
    let radius = emitter.radius()?;
    let curve = emitter.radius_curve()?;
    let gradient = emitter.gradient()?;
    let inherit = emitter.inherit()?;
    let mean_radius = (radius.min + radius.max) * 0.5;
    let colours = KEYS.map(|t| gradient.colour_at(t));
    Some(Spark {
        kind,
        life: [life.min, life.max],
        rgb: colours.map(|c| [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]),
        brightness: colours.map(|c| c[3]),
        alpha: KEYS.map(|t| gradient.alpha_at(t)),
        width: KEYS.map(|t| 2.0 * mean_radius * curve.at(t)),
        speed: mesh.speed,
        tilt: mesh.angle[0][1],
        inherit: [inherit.min, inherit.max],
        density,
        patch_radius: mesh.radius,
        bounce: emitter.bounce(),
        stretch: emitter.velocity_streak().filter(|s| s.opacity > 0.0).map(|s| s.length_per_speed),
        sheet,
    })
}

/// The picture of the sprite emitter `n` (the first material among the imports in its part of the block) and the cells of its
/// sheet (the first of the material's second constant).
fn sheet_of(
    bundle: &Bundle,
    resource: &Resource,
    span: &std::ops::Range<usize>,
    effect: u64,
    n: usize,
) -> Result<Sheet> {
    let import = resource
        .imports
        .iter()
        .filter(|import| span.contains(&((import.offset & !TYPE_FLAG) as usize)))
        .find(|import| {
            bundle.find(import.id).is_some_and(|at| bundle.entries()[at].kind == ResourceKind::Material.id())
        })
        .context("a sprite emitter without a material")?;
    let material = bundle.resource(bundle.find(import.id).context("no material")?)?;
    let parsed = Material::parse(&material.blocks[0], &material.imports)?;
    let texture_id = *parsed.textures.first().context("a sprite material without a texture")?;
    let texture = bundle.resource(bundle.find(texture_id).context("no texture")?)?;
    let texture = Texture::parse(&texture.blocks[0], &texture.blocks[1])?;
    let grid = parsed.float4(1).map_or(1, |cells| cells[0].round().max(1.0) as u32);
    Ok(Sheet { name: format!("E{:X}_{n}.PNG", effect & 0xFFFF_FFFF), picture: vfx::decode(&texture)?, grid })
}

/// Write the profiles and the pictures of the sparks under `out/files/`. A failure only costs the variants.
pub fn write(source: &Source, out: &Path) {
    let set = match read_set(source) {
        Ok(set) => set,
        Err(error) => return log::warn!("the 2012 collision sparks are not read: {error:#}"),
    };
    if let Err(error) = write_set(out, &set) {
        log::warn!("the 2012 collision spark variants are not written: {error:#}");
    }
}

fn read_set(source: &Source) -> Result<Set> {
    let bytes = source.read_file(GLOBAL_EFFECTS)?;
    let bundle = Bundle::parse(&bytes)?;
    read(&bundle)
}

fn write_set(out: &Path, set: &Set) -> Result<()> {
    let sparks = out.join("files").join(SPARKS_FOLDER);
    let xenon = out.join("files").join(XENON_FOLDER);
    for folder in [&sparks, &xenon] {
        std::fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    let mut written: Vec<&str> = Vec::new();
    for spark in set.hit.sparks.iter().chain(&set.scrape.sparks) {
        let Some(sheet) = &spark.sheet else { continue };
        if written.contains(&sheet.name.as_str()) {
            continue;
        }
        written.push(&sheet.name);
        std::fs::write(sparks.join(&sheet.name), vfx::png_bytes(&sheet.picture)?)
            .with_context(|| format!("writing {}", sheet.name))?;
    }
    std::fs::write(sparks.join("profile.toml"), profile::sparks(set)).context("writing the sparks profile")?;
    std::fs::write(xenon.join("profile.toml"), profile::xenon(set)).context("writing the xenon profile")
}
