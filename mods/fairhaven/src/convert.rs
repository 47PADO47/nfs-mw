//! One unit of the world as one section file of the game (`docs/modding/fairhaven.md`): its models become solids, their pictures
//! a texture pack, the unit's collision a collision pack, and where each model stands a scenery section. Everything is read from
//! the install by ranges and made in memory; nothing is kept between units except the shaders.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_scenery::build::{MAX_COMPONENT, Placement, section as scenery_section};
use blackbox_solid::SolidSource;
use blackbox_tpk::{PixelFormat, TextureSource};
use chameleon_gfx::{Material, Model, Texture};

use crate::collision;
use crate::install::{Install, ShaderInfo};
use crate::instances::{self, COMPOUND_INSTANCE_LIST, DYNAMIC_INSTANCE_LIST, INSTANCE_LIST, MODEL, PROP_INSTANCE_LIST};
use crate::material::{Layers, Surface, surface};
use crate::mesh::{self, MeshData, RenderableData};
use crate::pvs::Zone;
use crate::sea::{WATER_BLOCK, WATER_NAME};
use crate::section;
use crate::solid::{self, BuiltSolid, Group};
use crate::space;
use crate::terrain::{self, Mask};
use crate::textures::{Look, name_of, texture_source};
use crate::unit::Unit;

/// Instances whose bounding sphere is wider than this are the panorama every unit repeats (the ring of hills, the sea): left
/// out, the game has a sky of its own.
pub const MAX_RADIUS: f32 = 800.0;
/// How far outside its zone's rectangle a unit keeps an instance, metres: units list the instances of their borders twice.
pub const CLIP_MARGIN: f32 = 25.0;
/// The game's rule for an object's exclude flags: drawn in every main view (`modding/content/scenery.rs`).
const EXCLUDE_FLAGS: u32 = 0x60;
/// The name of the flat grey picture a surface with no picture of its own is drawn with.
const FLAT_NAME: &str = "FLATGREY";
const RESOURCE_TEXTURE: u32 = 0x1;

/// What the conversion of a unit is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The longest side of a picture, pixels.
    pub max_texture: u32,
    /// How bright the pictures are, percent of their stored colour (100 leaves them as they are).
    pub brightness: u32,
}

/// What a conversion did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    /// Instances in the unit's lists, those left out because they lie outside the zone or are the panorama, and those whose
    /// model is missing or whose turn does not fit the game's records.
    pub instances: usize,
    /// Props, compound and dynamic objects in the unit's lists.
    pub objects: usize,
    pub clipped: usize,
    pub missing: usize,
    /// The ids of the models (or objects) that are missing, each once: in no bundle the reader opens.
    pub missing_ids: Vec<u64>,
    pub too_large: usize,
    /// Placements written, solids and triangles in them (a coarse level counts as well).
    pub placements: usize,
    pub solids: usize,
    pub triangles: usize,
    /// Pictures written, and the ones that could not be read (drawn flat).
    pub textures: usize,
    pub textures_failed: usize,
    /// Collision triangles written and left out.
    pub ground: usize,
    pub ground_skipped: usize,
    /// Bytes of the section file and the time the conversion took, and where the time went: the pictures (read and made), the
    /// collision, and writing the file.
    pub bytes: usize,
    pub millis: u128,
    pub picture_millis: u128,
    pub ground_millis: u128,
    pub write_millis: u128,
}

/// A converted unit.
pub struct Converted {
    pub bytes: Vec<u8>,
    pub stats: Stats,
}

/// A model read from the unit: what is needed to decide whether to place it and to build it.
struct ModelInfo {
    lods: Vec<u64>,
    sphere: [f32; 4],
    fine: RenderableData,
}

/// What a built solid is placed with.
#[derive(Clone)]
struct PartPlacement {
    name: String,
    hashes: [u32; 4],
    bounds: ([f32; 3], [f32; 3]),
    radius: f32,
    centre: [f32; 3],
}

struct MaterialInfo {
    shader: Arc<ShaderInfo>,
    surface: Surface,
}

struct Converter<'a> {
    unit: &'a Unit<'a>,
    options: &'a Options,
    stats: Stats,
    picture_micros: u128,
    materials: HashMap<u64, Option<Rc<MaterialInfo>>>,
    textures: Vec<TextureSource>,
    texture_names: HashMap<String, bool>,
    models: HashMap<u64, Option<Rc<ModelInfo>>>,
    object_models: HashMap<u64, Option<u64>>,
    masks: HashMap<u64, Option<Rc<Mask>>>,
    built: HashMap<u64, Rc<Vec<PartPlacement>>>,
    sources: Vec<SolidSource>,
}

/// Converts unit `number` into the section file of section `section`.
pub fn convert_unit(install: &Install, number: u32, section_number: u32, options: &Options) -> Result<Converted> {
    let started = Instant::now();
    let Some(&zone) = install.zone(number) else { bail!("unit {number} is not in the zone list") };
    let unit = install.open_unit(number)?;
    let mut converter = Converter {
        unit: &unit,
        options,
        stats: Stats::default(),
        picture_micros: 0,
        materials: HashMap::new(),
        textures: Vec::new(),
        texture_names: HashMap::new(),
        models: HashMap::new(),
        object_models: HashMap::new(),
        masks: HashMap::new(),
        built: HashMap::new(),
        sources: Vec::new(),
    };
    let placements = converter.place_all(&zone)?;
    let scenery =
        scenery_section(section_number, &placements).map_err(|error| anyhow::anyhow!("placements: {error:?}"))?;
    let ground_started = Instant::now();
    let ground = collision::ground(&unit).with_context(|| format!("the collision of unit {number}"))?;
    let ground_millis = ground_started.elapsed().as_millis();
    let name = format!("U{number}");
    let write_started = Instant::now();
    let made = section::build(&name, section_number, &converter.sources, &converter.textures, &ground, &scenery)?;
    let write_millis = write_started.elapsed().as_millis();
    let mut stats = converter.stats;
    stats.picture_millis = converter.picture_micros / 1000;
    stats.ground_millis = ground_millis;
    stats.write_millis = write_millis;
    stats.placements = placements.len();
    stats.solids = converter.sources.len();
    stats.triangles = converter.sources.iter().flat_map(|s| &s.groups).map(|g| g.indices.len() / 3).sum();
    stats.textures = converter.textures.len();
    stats.ground = made.collision.triangles;
    stats.ground_skipped = made.collision.skipped;
    stats.bytes = made.bytes.len();
    stats.millis = started.elapsed().as_millis();
    Ok(Converted { bytes: made.bytes, stats })
}

impl Converter<'_> {
    /// The placements of every instance of the unit that is kept: the models, then the props, compound and dynamic objects.
    fn place_all(&mut self, zone: &Zone) -> Result<Vec<Placement>> {
        let mut placements = Vec::new();
        for list in self.unit.ids_of(INSTANCE_LIST) {
            for instance in instances::parse(&self.unit.resource(list)?)? {
                self.stats.instances += 1;
                self.place(zone, &instance, &mut placements);
            }
        }
        for kind in [PROP_INSTANCE_LIST, COMPOUND_INSTANCE_LIST, DYNAMIC_INSTANCE_LIST] {
            for list in self.unit.ids_of(kind) {
                for object in instances::parse_with(&self.unit.resource(list)?, instances::OBJECTS)? {
                    self.stats.objects += 1;
                    let Some(model) = self.model_of_object(object.model) else {
                        self.note_missing(object.model);
                        continue;
                    };
                    self.place(zone, &instances::Instance { model, ..object }, &mut placements);
                }
            }
        }
        placements.truncate(i16::MAX as usize);
        Ok(placements)
    }

    fn note_missing(&mut self, id: u64) {
        self.stats.missing += 1;
        if !self.stats.missing_ids.contains(&id) {
            self.stats.missing_ids.push(id);
        }
    }

    /// The model an object (a prop, a compound or a dynamic object) is drawn with: the first model it imports.
    fn model_of_object(&mut self, object: u64) -> Option<u64> {
        if let Some(&known) = self.object_models.get(&object) {
            return known;
        }
        let found = self.unit.resource(object).ok().and_then(|resource| {
            let mut imports: Vec<_> = resource.imports.iter().collect();
            imports.sort_by_key(|import| import.slot());
            imports.into_iter().map(|import| import.id).find(|&id| self.unit.kind(id) == Some(MODEL))
        });
        self.object_models.insert(object, found);
        found
    }

    fn place(&mut self, zone: &Zone, instance: &instances::Instance, placements: &mut Vec<Placement>) {
        let Some(model) = self.model(instance.model) else {
            self.note_missing(instance.model);
            return;
        };
        let m = &instance.matrix;
        let [sx, sy, sz, radius] = model.sphere;
        let centre = [0, 1, 2].map(|c| sx * m[c] + sy * m[4 + c] + sz * m[8 + c] + m[12 + c]);
        // The panorama is left out, except the water: the sea is one huge mesh that every unit by the coast lists, and it is all
        // there is of the sea.
        let panorama = radius > MAX_RADIUS && !self.is_water(&model);
        if panorama || (radius <= MAX_RADIUS && !zone.contains(centre[0], centre[2], CLIP_MARGIN)) {
            self.stats.clipped += 1;
            return;
        }
        let (rows, position) = space::instance_rows(m);
        if !rows.iter().flatten().all(|v| v.is_finite() && v.abs() <= MAX_COMPONENT)
            || !position.iter().all(|v| v.is_finite())
        {
            self.stats.too_large += 1;
            return;
        }
        for part in self.parts_of(instance.model).iter() {
            placements.push(Placement {
                name: part.name.clone(),
                solids: part.hashes,
                local_min: part.bounds.0,
                local_max: part.bounds.1,
                radius: part.radius,
                position: space::recentred(position, &rows, part.centre),
                rows,
                exclude_flags: EXCLUDE_FLAGS,
            });
        }
    }

    /// Whether every mesh of the model's first level is water.
    fn is_water(&mut self, model: &ModelInfo) -> bool {
        let mut meshes = model.fine.meshes.iter().peekable();
        meshes.peek().is_some()
            && meshes.all(|mesh| self.material(mesh.material).is_some_and(|info| info.surface.water))
    }

    /// The model `id`: its levels and the first level's meshes, read once. `None` for a model that is missing or does not read.
    fn model(&mut self, id: u64) -> Option<Rc<ModelInfo>> {
        if let Some(known) = self.models.get(&id) {
            return known.clone();
        }
        let read = self.read_model(id).map_err(|error| log::debug!("model {id:016X}: {error:#}")).ok().map(Rc::new);
        self.models.insert(id, read.clone());
        read
    }

    fn read_model(&mut self, id: u64) -> Result<ModelInfo> {
        let resource = self.unit.resource(id)?;
        let model = Model::parse(&resource.blocks[0], &resource.imports)?;
        let Some(&first) = model.lods.first() else { bail!("the model has no level of detail") };
        let fine = self.renderable(first)?;
        Ok(ModelInfo { lods: model.lods, sphere: fine.sphere, fine })
    }

    fn renderable(&mut self, id: u64) -> Result<RenderableData> {
        let resource = self.unit.resource(id)?;
        mesh::read_renderable(&resource, |material| self.material(material).map(|info| info.shader.clone()))
    }

    /// The solids of model `id`, built the first time they are asked for: the placements they are placed with.
    fn parts_of(&mut self, id: u64) -> Rc<Vec<PartPlacement>> {
        if let Some(known) = self.built.get(&id) {
            return known.clone();
        }
        let parts = Rc::new(self.build_model(id));
        self.built.insert(id, parts.clone());
        parts
    }

    fn build_model(&mut self, id: u64) -> Vec<PartPlacement> {
        let Some(model) = self.model(id) else { return Vec::new() };
        let number = self.built.len();
        let fine = self.solids_of(&model.fine, &format!("M{number}A"));
        // The level of the middle of the chain stands for the coarse model of the game's two-level choice.
        let coarse_id = (model.lods.len() > 1)
            .then(|| model.lods[model.lods.len() / 2])
            .filter(|lod| Some(lod) != model.lods.first());
        let coarse = coarse_id
            .and_then(|lod| self.renderable(lod).ok())
            .map(|data| self.solids_of(&data, &format!("M{number}C")));
        let coarse = coarse.filter(|solids| solids.len() == fine.len());
        let mut parts = Vec::with_capacity(fine.len());
        for (n, built) in fine.iter().enumerate() {
            let coarse_hash = coarse.as_ref().map_or(0, |solids| bstring_hash(&solids[n].source.name));
            let hash = bstring_hash(&built.source.name);
            parts.push(PartPlacement {
                name: built.source.name.clone(),
                hashes: [hash, hash, coarse_hash, 0],
                bounds: (built.source.bounds_min, built.source.bounds_max),
                radius: built.radius,
                centre: built.centre,
            });
        }
        self.sources.extend(fine.into_iter().map(|built| built.source));
        if let Some(coarse) = coarse {
            self.sources.extend(coarse.into_iter().map(|built| built.source));
        }
        parts
    }

    /// The solids of a renderable's meshes, each group drawn with the picture of its material.
    fn solids_of(&mut self, data: &RenderableData, prefix: &str) -> Vec<BuiltSolid> {
        let mut groups = Vec::new();
        for mesh in &data.meshes {
            let Some(material) = self.material(mesh.material) else { continue };
            if material.surface.hidden {
                continue;
            }
            let look = Look { cut_out: material.surface.cut_out, two_sided: material.surface.two_sided };
            if let Some(layers) = &material.surface.layers
                && let Some(mask) = self.mask(layers.mask)
            {
                groups.extend(self.terrain_groups(mesh, layers, &mask, look));
                continue;
            }
            let texture = match material.surface.water {
                true => self.water(),
                false => self.texture(material.surface.diffuse, look),
            };
            groups.extend(self.groups(mesh, &texture, material.surface.uv_scale));
        }
        solid::build(prefix, groups)
    }

    /// The parts of `mesh` drawn with `texture`, its coordinates repeated `scale` times.
    fn groups(&self, mesh: &MeshData, texture: &str, [across, down]: [f32; 2]) -> Vec<Group> {
        mesh::parts(mesh)
            .into_iter()
            .map(|mut part| {
                part.uvs.iter_mut().for_each(|uv| *uv = [uv[0] * across, uv[1] * down]);
                Group { texture: texture.to_owned(), part }
            })
            .collect()
    }

    /// A terrain mesh cut into the triangles of each of its layers (the layer the mask is strongest in at the triangle's middle),
    /// each drawn with its layer's picture at its own repeat.
    fn terrain_groups(&mut self, mesh: &MeshData, layers: &Layers, mask: &Mask, look: Look) -> Vec<Group> {
        let mut triangles: [Vec<[u32; 3]>; 3] = Default::default();
        for triangle in &mesh.triangles {
            let corner = |n: usize| mesh.uvs[triangle[n] as usize];
            let middle = [0, 1].map(|axis| (corner(0)[axis] + corner(1)[axis] + corner(2)[axis]) / 3.0);
            triangles[terrain::layer_of(mask, middle)].push(*triangle);
        }
        let mut groups = Vec::new();
        for (layer, kept) in triangles.iter().enumerate().filter(|(_, kept)| !kept.is_empty()) {
            let texture = self.texture(Some(layers.textures[layer]), look);
            groups.extend(self.groups(&mesh::subset(mesh, kept), &texture, layers.ratios[layer]));
        }
        groups
    }

    /// The mask of a terrain: a quarter of its size at most 256 pixels, read once.
    fn mask(&mut self, id: u64) -> Option<Rc<Mask>> {
        if let Some(known) = self.masks.get(&id) {
            return known.clone();
        }
        let made = self.read_texture(id).ok().and_then(|texture| terrain::mask_of(&texture)).map(Rc::new);
        self.masks.insert(id, made.clone());
        made
    }

    fn material(&mut self, id: u64) -> Option<Rc<MaterialInfo>> {
        if let Some(known) = self.materials.get(&id) {
            return known.clone();
        }
        let info = self.read_material(id).map(Rc::new);
        self.materials.insert(id, info.clone());
        info
    }

    fn read_material(&self, id: u64) -> Option<MaterialInfo> {
        let resource = self.unit.resource(id).ok()?;
        let material = Material::parse(&resource.blocks[0], &resource.imports).ok()?;
        let shader = self.unit.install().shader(material.shader)?;
        let surface = surface(&material, &shader);
        Some(MaterialInfo { shader, surface })
    }

    /// The name of the picture a surface is drawn with, made the first time it is used. A picture that is missing or does not read
    /// is the flat grey one.
    fn texture(&mut self, diffuse: Option<u64>, look: Look) -> String {
        let Some(id) = diffuse else { return self.flat() };
        let name = name_of(id, look);
        if let Some(&ok) = self.texture_names.get(&name) {
            return if ok { name } else { self.flat() };
        }
        let started = Instant::now();
        let made = self.read_texture(id).and_then(|texture| {
            texture_source(&name, &texture, self.options.max_texture, self.options.brightness, look)
        });
        self.picture_micros += started.elapsed().as_micros();
        match made {
            Ok(source) => {
                self.textures.push(source);
                self.texture_names.insert(name.clone(), true);
                name
            }
            Err(error) => {
                log::debug!("picture {id:016X}: {error:#}");
                self.stats.textures_failed += 1;
                self.texture_names.insert(name, false);
                self.flat()
            }
        }
    }

    fn read_texture(&self, id: u64) -> Result<Texture> {
        if self.unit.kind(id) != Some(RESOURCE_TEXTURE) {
            bail!("it is not a texture");
        }
        let resource = self.unit.resource(id)?;
        Ok(Texture::parse(&resource.blocks[0], &resource.blocks[1])?)
    }

    fn water(&mut self) -> String {
        if !self.texture_names.contains_key(WATER_NAME) {
            self.textures.push(TextureSource::new(WATER_NAME, 4, 4, PixelFormat::Dxt1, 1, WATER_BLOCK.to_vec()));
            self.texture_names.insert(WATER_NAME.to_owned(), true);
        }
        WATER_NAME.to_owned()
    }

    fn flat(&mut self) -> String {
        if !self.texture_names.contains_key(FLAT_NAME) {
            let block = vec![0x10, 0x84, 0x10, 0x84, 0, 0, 0, 0];
            self.textures.push(TextureSource::new(FLAT_NAME, 4, 4, PixelFormat::Dxt1, 1, block));
            self.texture_names.insert(FLAT_NAME.to_owned(), true);
        }
        FLAT_NAME.to_owned()
    }
}
