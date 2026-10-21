//! The menu room: a front-end stage file (`FRONTEND/PLATFORMS/PlatformCrib.BIN`, five solids and their textures, or the
//! safe house's `CAREER_SAFEHOUSE.BIN`), drawn behind the menus. Spec: `docs/specs/menu-backdrop.md` and
//! `docs/specs/menu-backdrop-looks.md`.

use std::collections::HashMap;

use anyhow::{Context, Result};
use blackbox_gfx::{BlendMode, Instance, InstanceKey, MeshHandle, RenderBackend, Shading, TextureHandle};
use blackbox_scene::{MaterialLookup, blend_mode, upload_solid, upload_texture};
use game_install::GameDir;
use glam::Mat4;

use super::stage::Stage;

/// The instance key of the room's parts. The car viewer keys its parts from zero, so this one stays apart.
const ROOM_ID: u32 = u32::MAX - 2;

/// The room's GPU resources and its placed parts.
pub struct Room {
    meshes: Vec<MeshHandle>,
    textures: Vec<TextureHandle>,
    instances: Vec<Instance>,
}

/// Textures of the room by name hash.
struct Materials(HashMap<u32, (TextureHandle, BlendMode)>);

impl MaterialLookup for Materials {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        self.0.get(&texture_hash).copied()
    }
}

impl Room {
    /// Uploads the textures and the drawn solids of `stage`. The room sits where the stage file puts it (identity
    /// transform).
    pub fn load(renderer: &mut dyn RenderBackend, dir: &GameDir, stage: Stage) -> Result<Self> {
        let path = stage.file();
        let file = dir.read(path).with_context(|| format!("reading {path}"))?;
        let solids = blackbox_solid::read_solids(&file).with_context(|| format!("solids in {path}"))?;
        let packs = blackbox_tpk::read_texture_packs(&file).with_context(|| format!("textures in {path}"))?;
        let mut textures = Vec::new();
        let mut map = HashMap::new();
        for pack in &packs {
            for t in &pack.textures {
                let Some(handle) = upload_texture(renderer, t) else { continue };
                map.insert(t.name_hash, (handle, blend_mode(Some(t))));
                textures.push(handle);
            }
        }
        let materials = Materials(map);
        let mut meshes = Vec::new();
        let mut instances = Vec::new();
        for solid in solids.iter().filter(|solid| stage.draws(&solid.name)) {
            // The stage is pre-lit world geometry (its vertex colours carry the light), as the world's scenery is.
            let Some(mesh) = upload_solid(renderer, solid, &materials, Shading::Prelit) else { continue };
            meshes.push(mesh);
            let key = InstanceKey::new(ROOM_ID, instances.len() as u32);
            instances.push(Instance::keyed(mesh, Mat4::IDENTITY, key));
        }
        // The renderer wants instances of one mesh next to each other.
        instances.sort_by_key(|i| i.mesh);
        Ok(Self { meshes, textures, instances })
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// Frees the room's meshes and textures.
    pub fn destroy(self, renderer: &mut dyn RenderBackend) {
        for mesh in self.meshes {
            renderer.destroy_mesh(mesh);
        }
        for texture in self.textures {
            renderer.destroy_texture(texture);
        }
    }
}
