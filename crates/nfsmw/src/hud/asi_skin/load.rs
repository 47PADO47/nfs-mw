//! Reading a skin: its ini, and every picture it names, decoded and handed to the UI assets as a texture.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_gauge_skin::{Image, Pictures, Skin, dds};
use blackbox_hash::bstring_hash;

use super::detect::{Files, SkinSource};
use crate::ui::UiAssets;

/// A picture of a skin, loaded: the texture key the presenter draws it by, and its pixels (kept for the sector masks).
pub(super) struct Picture {
    pub key: u32,
    pub image: Arc<Image>,
}

/// A skin with its pictures.
pub(super) struct Loaded {
    pub source: SkinSource,
    pub skin: Skin,
    /// By lower-case file name and whether it is blended additively.
    pictures: HashMap<(String, bool), Picture>,
    sizes: HashMap<String, (u32, u32)>,
}

/// A skin's file name as a game-folder path piece: lower case, `/` for `\`.
pub(super) fn file_key(file: &str) -> String {
    file.trim().replace('\\', "/").to_ascii_lowercase()
}

/// The texture key of a picture of a skin under one blend. Never 0, which means "no texture".
pub(super) fn texture_key(dir: &str, file: &str, additive: bool) -> u32 {
    let key = bstring_hash(&format!("customhud:{}:{}:{}", dir.to_ascii_lowercase(), file_key(file), additive));
    key.max(1)
}

impl Loaded {
    /// Reads the skin `source` names and its pictures. A picture that is missing or in a format that cannot be decoded is
    /// logged and left out: its element does not draw. The textures are added to `assets`.
    pub fn open(files: &dyn Files, source: SkinSource, assets: &UiAssets) -> Option<Self> {
        let text = files.read(&source.ini)?;
        let skin = Skin::parse(&String::from_utf8_lossy(&text));
        if skin.is_empty() {
            log::warn!("CustomHUD skin {}: no element is enabled", source.ini);
            return None;
        }
        let mut pictures = HashMap::new();
        let mut sizes = HashMap::new();
        for used in skin.pictures() {
            let name = file_key(&used.file);
            let path = format!("{}/{name}", source.dir);
            let Some(bytes) = files.read(&path) else {
                log::warn!("CustomHUD skin {}: the picture {} is missing", source.ini, used.file);
                continue;
            };
            let image = match dds::decode(&bytes) {
                Ok(image) => image,
                Err(error) => {
                    log::warn!("CustomHUD skin {}: the picture {} cannot be read: {error}", source.ini, used.file);
                    continue;
                }
            };
            let key = texture_key(&source.dir, &used.file, used.additive);
            assets.add_textures([image.to_texture(&name, key, used.additive)]);
            sizes.insert(name.clone(), (image.width, image.height));
            pictures.insert((name, used.additive), Picture { key, image: Arc::new(image) });
        }
        log::info!("CustomHUD skin {}: {} pictures", source.ini, pictures.len());
        Some(Self { source, skin, pictures, sizes })
    }

    /// The loaded picture `file` under a blend, if it loaded.
    pub fn picture(&self, file: &str, additive: bool) -> Option<&Picture> {
        self.pictures.get(&(file_key(file), additive))
    }
}

impl Pictures for Loaded {
    fn size(&self, file: &str) -> Option<(u32, u32)> {
        self.sizes.get(&file_key(file)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_matched_whatever_their_case_and_slashes() {
        assert_eq!(file_key("Tach-Arrow.DDS"), "tach-arrow.dds");
        assert_eq!(file_key(" sub\\Digits.dds "), "sub/digits.dds");
    }

    #[test]
    fn a_texture_key_depends_on_the_folder_the_file_and_the_blend_and_is_never_zero() {
        let a = texture_key("scripts/CustomHUD/NFS15", "gear.dds", false);
        assert_eq!(a, texture_key("SCRIPTS/customhud/nfs15", "GEAR.dds", false));
        assert_ne!(a, texture_key("scripts/CustomHUD/NFS15", "gear.dds", true));
        assert_ne!(a, texture_key("scripts/CustomHUD/NFS2022", "gear.dds", false));
        assert_ne!(a, texture_key("scripts/CustomHUD/NFS15", "digits.dds", false));
        assert_ne!(a, 0);
    }
}
