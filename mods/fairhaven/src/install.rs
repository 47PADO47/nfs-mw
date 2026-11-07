//! The 2012 install, opened for reading: the bundles every unit may borrow from (shared by id), the zone list, and the shaders.
//! Opening reads the headers and tables of about fifteen files, a few hundred kilobytes in all; resources are read when asked for.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool, Resource};
use chameleon_gfx::{Element, Shader, VertexDescriptor};

use crate::pvs::{self, Zone};
use crate::ranges::FileRanges;
use crate::unit::Unit;

/// The world's folder under the install.
pub const WORLD_FOLDER: &str = "HAWAII";
/// The resource types the reader looks for by kind.
pub const KIND_VERTEX_DESCRIPTOR: u32 = 0x3;
pub const KIND_SHADER: u32 = 0x53;

/// What a mesh needs to know about the shader of its material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderInfo {
    /// The technique name (`WorldPBR_Diffuse_Normal_Specular_Reflective_AO_Singlesided`).
    pub technique: String,
    /// The samplers that take the material's textures, in the material's order.
    pub samplers: Vec<String>,
    /// The vertex layout the shader draws with.
    pub elements: Vec<Element>,
}

/// An opened 2012 install.
pub struct Install {
    root: PathBuf,
    shared: Pool<'static>,
    zones: Vec<Zone>,
    shaders: Mutex<HashMap<u64, Option<Arc<ShaderInfo>>>>,
}

fn open_bundle(path: &Path) -> Result<Bundle<'static>> {
    let ranges = FileRanges::open(path).with_context(|| format!("opening {}", path.display()))?;
    Bundle::parse_ranges(Arc::new(ranges)).with_context(|| format!("reading the table of {}", path.display()))
}

impl Install {
    /// Opens the install in `root` (the folder with `NFS13.exe`, `SHADERS.BNDL` and `HAWAII/`).
    pub fn open(root: &Path) -> Result<Self> {
        let world = root.join(WORLD_FOLDER);
        if !world.join("PVS.BNDL").is_file() {
            bail!("{} has no {WORLD_FOLDER}/PVS.BNDL: it is not the folder of a 2012 install", root.display());
        }
        let zones = read_zones(&world.join("PVS.BNDL"))?;
        let mut shared = Pool::new();
        // The first bundle that holds an id wins. The districts are in file-name order, so the choice is the same on every machine.
        let mut paths = vec![world.join("GLOBALRESOURCES.BNDL")];
        paths.extend(district_files(&world)?);
        paths.extend(
            ["GLOBALTEXTUREDICTIONARY.BNDL", "GLOBALMATERIALDICTIONARY.BNDL", "SHADERS.BNDL"]
                .map(|name| root.join(name)),
        );
        for path in paths {
            shared.add(open_bundle(&path)?);
        }
        Ok(Self { root: root.to_path_buf(), shared, zones, shaders: Mutex::new(HashMap::new()) })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The zone of every unit, in the order of the zone list.
    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    /// The zone of unit `number`.
    pub fn zone(&self, number: u32) -> Option<&Zone> {
        self.zones.iter().find(|zone| zone.unit == number)
    }

    /// The path of a unit's bundle.
    pub fn unit_path(&self, number: u32) -> PathBuf {
        self.root.join(WORLD_FOLDER).join(format!("TRK_UNIT{number}.BNDL"))
    }

    /// Opens unit `number`: reads its table of resources.
    pub fn open_unit(&self, number: u32) -> Result<Unit<'_>> {
        let own = open_bundle(&self.unit_path(number))?;
        Ok(Unit::new(self, number, own))
    }

    /// The resource `id` of the shared bundles.
    pub fn shared_resource(&self, id: u64) -> Result<Resource> {
        Ok(self.shared.resource(id)?)
    }

    /// The type of the shared resource `id`.
    pub fn shared_kind(&self, id: u64) -> Option<u32> {
        self.shared.kind(id)
    }

    /// What the shader `id` tells about a mesh: its technique, samplers and vertex layout. `None` for a shader that is not in
    /// the install or does not read. Each shader is read once.
    pub fn shader(&self, id: u64) -> Option<Arc<ShaderInfo>> {
        let mut cache = self.shaders.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(known) = cache.get(&id) {
            return known.clone();
        }
        let info = self.read_shader(id).map(Arc::new);
        cache.insert(id, info.clone());
        info
    }

    fn read_shader(&self, id: u64) -> Option<ShaderInfo> {
        if self.shared.kind(id) != Some(KIND_SHADER) {
            return None;
        }
        let resource = self.shared.resource(id).ok()?;
        let is_descriptor = |candidate: u64| self.shared.kind(candidate) == Some(KIND_VERTEX_DESCRIPTOR);
        let shader = Shader::parse(&resource.blocks[0], &resource.imports, is_descriptor).ok()?;
        let descriptor = self.shared.resource(shader.vertex_descriptor?).ok()?;
        let layout = VertexDescriptor::parse(&descriptor.blocks[0]).ok()?;
        Some(ShaderInfo {
            technique: shader.technique().unwrap_or_default().to_owned(),
            samplers: shader.material_samplers.clone(),
            elements: layout.elements,
        })
    }
}

/// The zone list of the install in `root`, without opening the rest of it: a few kilobytes.
pub fn read_zone_list(root: &Path) -> Result<Vec<Zone>> {
    read_zones(&root.join(WORLD_FOLDER).join("PVS.BNDL"))
}

fn read_zones(path: &Path) -> Result<Vec<Zone>> {
    let bundle = open_bundle(path)?;
    let Some(index) = bundle.entries().iter().position(|entry| entry.kind == pvs::ZONE_LIST) else {
        bail!("{} holds no zone list", path.display());
    };
    let resource = bundle.resource(index)?;
    pvs::parse(&resource.blocks[0]).with_context(|| format!("reading the zone list of {}", path.display()))
}

/// The district bundles of the world folder, in file-name order.
fn district_files(world: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(world)
        .with_context(|| format!("listing {}", world.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_ascii_uppercase();
            name.starts_with("DISTRICT_") && name.ends_with(".BNDL")
        })
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_that_is_not_an_install_is_refused_with_its_name() {
        let error = Install::open(&std::env::temp_dir()).err().expect("a temp folder is no install");
        assert!(error.to_string().contains("PVS.BNDL"), "{error}");
    }
}
