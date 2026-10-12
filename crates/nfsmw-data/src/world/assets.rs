//! The shared scenery models of a track (poles, cones, bins...): the models the map tiles place, kept in the
//! track's non-spatial sections (docs/formats/maps.md). The asset viewer lists them and shows one at a time.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_solid::Solid;
use blackbox_tpk::{Texture, TextureAnim};
use game_install::GameDir;
use glam::Mat4;

use super::{WorldIndex, parse_section};
use crate::car::{CarModel, Placement, TextureSwaps};

/// One shared model: its solid, the textures it draws (animation frames included) and the texture
/// animations of the textures it draws.
struct Asset {
    solid: Solid,
    textures: Vec<Texture>,
    anims: Vec<TextureAnim>,
}

/// Every shared model of a track, sorted by name, each name once.
pub struct SharedAssets {
    assets: Vec<Asset>,
}

impl SharedAssets {
    /// Read the track's shared sections and keep their models.
    pub fn load(dir: &GameDir, track: &str) -> Result<Self> {
        let index = WorldIndex::open(dir, track)?;
        let mut solids = Vec::new();
        let mut anims = Vec::new();
        let mut textures: HashMap<u32, Texture> = HashMap::new();
        for i in index.shared() {
            let section = &index.sections[i];
            let bytes = dir
                .read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize)
                .with_context(|| format!("reading section {}", section.name))?;
            let data = parse_section(i, &bytes).with_context(|| format!("parsing section {}", section.name))?;
            solids.extend(data.solids);
            anims.extend(data.anims);
            for texture in data.textures {
                textures.entry(texture.name_hash).or_insert(texture);
            }
        }
        let mut seen = HashSet::new();
        let mut assets: Vec<Asset> = solids
            .into_iter()
            .filter(|solid| seen.insert(solid.name_hash))
            .map(|solid| {
                let anims: Vec<TextureAnim> =
                    anims.iter().filter(|a| solid.texture_hashes.contains(&a.name_hash)).cloned().collect();
                // The frames of an animation are drawn in its place, so they are uploaded with the model too.
                let hashes: HashSet<u32> = solid
                    .texture_hashes
                    .iter()
                    .copied()
                    .chain(anims.iter().flat_map(|a| a.frames.iter().copied()))
                    .collect();
                let textures = hashes.iter().filter_map(|h| textures.get(h).cloned()).collect();
                Asset { solid, textures, anims }
            })
            .collect();
        assets.sort_by_key(|a| a.solid.name.to_ascii_lowercase());
        Ok(Self { assets })
    }

    /// The name of each model, in the order [`model`](Self::model) takes indices.
    pub fn names(&self) -> Vec<String> {
        self.assets.iter().map(|a| a.solid.name.clone()).collect()
    }

    /// The texture animations of model `index`.
    pub fn anims(&self, index: usize) -> Vec<TextureAnim> {
        self.assets[index].anims.clone()
    }

    /// Model `index` as a car-viewer model: its one solid at the origin, with its textures and no car around it.
    pub fn model(&self, index: usize) -> CarModel {
        let asset = &self.assets[index];
        let hash = asset.solid.name_hash;
        let mut solid = asset.solid.clone();
        // Some models carry no vertex colour at all; drawn as they are they come out black, so their texture shows
        // at full brightness instead.
        for v in &mut solid.vertices {
            if v.color_bgra == [0; 4] {
                v.color_bgra = [255; 4];
            }
        }
        CarModel {
            name: asset.solid.name.clone(),
            car_type: None,
            solids: HashMap::from([(hash, solid)]),
            placements: vec![Placement {
                solid: hash,
                transform: Mat4::IDENTITY,
                left_brake: false,
                slot: usize::MAX,
                corner: None,
            }],
            textures: asset.textures.iter().map(|t| (t.name_hash, t.clone())).collect(),
            swaps: TextureSwaps::default(),
            paint: None,
            light_materials: HashMap::new(),
            floor_height: 0.0,
            corners: None,
            wheel_setup: None,
        }
    }
}
