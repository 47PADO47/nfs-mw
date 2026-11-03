//! The menu room: a front-end stage file (`FRONTEND/PLATFORMS/PlatformCrib.BIN`, its selected solids and textures, or the
//! safe house's `CAREER_SAFEHOUSE.BIN`), drawn behind the menus. Spec: `docs/specs/menu-backdrop.md` and
//! `docs/specs/menu-backdrop-looks.md`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_gfx::{
    BlendMode, Instance, InstanceKey, MeshHandle, NormalMaterialHandle, RenderBackend, Shading, TextureHandle,
};
use blackbox_scene::{MaterialLookup, blend_mode, upload_solid, upload_texture};
use blackbox_solid::Solid;
use blackbox_tpk::TexturePack;
use game_install::GameDir;
use glam::Mat4;

use super::stage::Stage;
use crate::scenes::world::materials::{Lookup, normal_materials};

/// The instance key of the room's parts. The car viewer keys its parts from zero, so this one stays apart.
const ROOM_ID: u32 = u32::MAX - 2;

/// The room's GPU resources and its placed parts.
pub struct Room {
    meshes: Vec<MeshHandle>,
    textures: Vec<TextureHandle>,
    instances: Vec<Instance>,
    casters: Vec<Instance>,
    floor_texture: Option<TextureHandle>,
    normal_materials: Vec<NormalMaterialHandle>,
    light: Option<super::backdrop_light::BackdropLight>,
}

/// Textures of the room by name hash.
struct Materials(HashMap<u32, (TextureHandle, BlendMode)>);

impl MaterialLookup for Materials {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        self.0.get(&texture_hash).copied()
    }
}

/// What a stage file holds that the room draws, parsed and ready to upload: the drawn solids and the texture packs.
/// Plain CPU data, so it is built off the frame thread (`MenuScene`'s worker) and kept for the session: the menus
/// change look often and each look on the same stage would read and parse the file again.
pub struct RoomData {
    solids: Vec<Solid>,
    casters: Vec<Solid>,
    packs: Vec<TexturePack>,
    /// The stage's turn of the room in degrees (`Stage::room_yaw`) and its shift along +y (`Stage::room_shift`).
    yaw: f32,
    shift: f32,
    floor: Option<u32>,
    /// The scenery light (`backdrop_light.rs`), `None` when the install's presets are unusable.
    light: Option<super::backdrop_light::BackdropLight>,
    /// The file's particle placements (`room_fx.rs`).
    placements: Vec<nfsmw_data::world::emitter_placements::Placement>,
}

impl RoomData {
    /// The stage file's particle placements, in the unturned stage frame.
    pub fn placements(&self) -> &[nfsmw_data::world::emitter_placements::Placement] {
        &self.placements
    }

    /// Reads and parses the stage file of `stage` (once per session, see `nfsmw_data::memo`).
    pub fn read(dir: &GameDir, stage: Stage) -> Result<Arc<Self>> {
        let _trace = blackbox_trace::scope("Frontend: read the menu room");
        let path = stage.file();
        let sources: Vec<_> = std::iter::once(path).chain(super::room_materials::SOURCES.iter().copied()).collect();
        nfsmw_data::memo(dir, &sources, || {
            let file = dir.read(path).with_context(|| format!("reading {path}"))?;
            let mut solids = blackbox_solid::read_solids(&file).with_context(|| format!("solids in {path}"))?;
            for solid in &mut solids {
                solid.name = Stage::canonical_name(&solid.name).to_owned();
            }
            let casters = solids.iter().filter(|solid| stage.shadow_source(&solid.name)).cloned().collect();
            solids.retain(|solid| stage.draws(&solid.name));
            let mut packs = blackbox_tpk::read_texture_packs(&file).with_context(|| format!("textures in {path}"))?;
            super::room_materials::complete(dir, &solids, &mut packs)?;
            let floor = super::room_floor::floor_texture(&solids, stage);
            let light = super::backdrop_light::BackdropLight::read(dir);
            let placements = nfsmw_data::world::emitter_placements::read(&file).unwrap_or_else(|error| {
                log::warn!("{path}: no particle placements: {error:#}");
                Vec::new()
            });
            let (yaw, shift) = (stage.room_yaw(), stage.room_shift());
            Ok(Self { solids, casters, packs, yaw, shift, floor, light, placements })
        })
    }
}

impl Room {
    /// Reads the stage file and uploads it in one go (`RoomData::read`, `Room::upload`).
    #[cfg(test)]
    pub fn load(renderer: &mut dyn RenderBackend, dir: &GameDir, stage: Stage) -> Result<Self> {
        let data = RoomData::read(dir, stage)?;
        Ok(Self::upload(renderer, &data))
    }

    /// Uploads the textures and the drawn solids of a stage. The original's platform draw hands the model renderer the
    /// stage's turn and shift alone (`Stage::room_yaw`, `Stage::room_shift`); a solid's own pivot matrix is not applied.
    /// The safe house's pivots negate x, so applying them showed that room mirrored.
    pub fn upload(renderer: &mut dyn RenderBackend, data: &RoomData) -> Self {
        let _trace = blackbox_trace::scope("Frontend: upload the menu room");
        let mut textures = Vec::new();
        let mut map = HashMap::new();
        for pack in &data.packs {
            for t in &pack.textures {
                let Some(handle) = upload_texture(renderer, t) else { continue };
                map.insert(t.name_hash, (handle, blend_mode(Some(t))));
                textures.push(handle);
            }
        }
        let floor_texture = data
            .floor
            .and_then(|hash| map.get(&hash))
            .filter(|(_, blend)| *blend == BlendMode::AlphaBlend)
            .map(|(handle, _)| *handle);
        let turn = Mat4::from_translation(glam::Vec3::new(0.0, data.shift, 0.0))
            * Mat4::from_rotation_z(data.yaw.to_radians());
        let mut meshes = Vec::new();
        let mut instances = Vec::new();
        let mut normals = HashMap::new();
        let find = |hash: u32| map.get(&hash).copied();
        for solid in &data.solids {
            // The original draws platform groups with the world shaders their effect ids select: the vertex colour is
            // the ambient term and the scene light supplies the rest (`docs/specs/menu-backdrop-light.md`). Groups whose
            // normal-map inputs are missing fall back to the ordinary world shader.
            let routed = normal_materials(renderer, solid, &find, &mut normals);
            let lookup = Lookup(find, false, routed, |_: u32| -> Option<blackbox_scene::Sidedness> { None });
            let Some(mesh) = upload_solid(renderer, solid, &lookup, Shading::World) else { continue };
            meshes.push(mesh);
            let key = InstanceKey::new(ROOM_ID, instances.len() as u32);
            instances.push(Instance::keyed(mesh, turn, key));
        }
        let materials = Materials(map);
        let mut casters = Vec::new();
        for solid in &data.casters {
            let Some(mesh) = upload_solid(renderer, solid, &materials, Shading::Prelit) else { continue };
            meshes.push(mesh);
            let key = InstanceKey::new(ROOM_ID, (instances.len() + casters.len()) as u32);
            casters.push(Instance::keyed(mesh, turn, key));
        }
        let normal_materials = normals.into_values().collect();
        Self { meshes, textures, instances, casters, floor_texture, normal_materials, light: data.light }
    }

    /// The scenery light the room was authored for, when the install's presets are usable.
    pub fn light(&self) -> Option<super::backdrop_light::BackdropLight> {
        self.light
    }

    /// The room's parts in the order the stage file lists its solids, one instance per mesh. The renderer draws blended
    /// parts in the order it is given them, so this order is the picture: the mirrored garage and the sky come before
    /// the floor that is see-through over them. It must not be sorted by mesh handle: the handles come from a free list
    /// that reuses what the last backdrop freed, so their order says nothing about the file's.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub(super) fn shadow_casters(&self) -> &[Instance] {
        &self.casters
    }

    pub(super) fn floor_texture(&self) -> Option<TextureHandle> {
        self.floor_texture
    }

    /// Frees the room's meshes and textures.
    pub fn destroy(self, renderer: &mut dyn RenderBackend) {
        for mesh in self.meshes {
            renderer.destroy_mesh(mesh);
        }
        for texture in self.textures {
            renderer.destroy_texture(texture);
        }
        for material in self.normal_materials {
            renderer.destroy_normal_material(material);
        }
    }
}

#[cfg(test)]
#[path = "room_materials_tests.rs"]
mod material_tests;
