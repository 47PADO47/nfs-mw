//! A NIS scene's bundle, `NIS/Scene_<name>_BundleB.bun`: its `AnimScene` chunk (the header and the character records,
//! read with `blackbox-nis`) and its EAGL skeletons and animation banks (read with [`AnimObjects`]). The file layout is
//! in `docs/specs/nis-scenes.md`; how a scene plays is in `docs/specs/nis-player.md`.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use blackbox_chunk::ids;
use blackbox_nis::{Scene, find_scene};
use blackbox_sequence::Package;
use blackbox_solid::Solid;
use blackbox_tpk::TexturePack;
use game_install::GameDir;

use crate::anim::AnimObjects;
use crate::read_unwrapped;

/// The install folder of the NIS bundles.
const NIS_DIR: &str = "NIS";

/// One NIS scene's data: the scene name as the file names it, the scene chunk, the EAGL objects and the event sequence.
/// One NIS scene's data: the scene name as the file names it, the scene chunk, the EAGL objects, and the solids and
/// texture packs the characters are drawn with (empty in the bundles without characters).
#[derive(Debug, Clone)]
pub struct NisBundle {
    /// The scene name, as in the bundle's file name (`IntroNis05` for `Scene_IntroNis05_BundleB.bun`).
    pub name: String,
    /// The scene's header and character records.
    pub scene: Scene,
    /// The bundle's skeletons and animation banks.
    pub anim: AnimObjects,
    /// The scene's CARP event sequence (its `0x0003B811` chunk), or `None` when the bundle has none.
    pub sequence: Option<Arc<Package>>,
    /// The bundle's skinned solids (the character models).
    pub solids: Vec<Solid>,
    /// The bundle's texture packs.
    pub textures: Vec<TexturePack>,
}

impl NisBundle {
    /// Reads the bundle of the scene `scene_name` from the install. The name matches the file's scene name without
    /// regard to case, and the file is opened under its name as the install spells it.
    pub fn load(dir: &GameDir, scene_name: &str) -> Result<Self> {
        let file = bundle_file(dir, scene_name)
            .ok_or_else(|| anyhow!("there is no NIS scene named {scene_name:?} in the install"))?;
        let rel = format!("{NIS_DIR}/{file}");
        let data = read_unwrapped(dir, &rel)?;
        let name = scene_name_of_file(&file).unwrap_or(scene_name);
        Self::from_bytes(name, &data).with_context(|| format!("reading {rel}"))
    }

    /// Reads a bundle from its unwrapped bytes. The bytes must hold an `AnimScene` chunk.
    pub fn from_bytes(name: &str, data: &[u8]) -> Result<Self> {
        let scene = find_scene(data)?.ok_or_else(|| anyhow!("the bundle has no AnimScene chunk"))?;
        let anim = AnimObjects::from_bytes(data)?;
        let sequence = read_sequence(data);
        let solids = blackbox_solid::read_solids(data).context("the solids of the bundle")?;
        let textures = blackbox_tpk::read_texture_packs(data).context("the texture packs of the bundle")?;
        Ok(Self { name: name.to_owned(), scene, anim, sequence, solids, textures })
    }

    /// The scene names of every NIS bundle in the install, in file order.
    pub fn list(dir: &GameDir) -> Vec<String> {
        dir.files_in(NIS_DIR).iter().filter_map(|file| scene_name_of_file(file)).map(str::to_owned).collect()
    }
}

/// The bundle's CARP event sequence, parsed. A sequence that does not parse is logged and dropped: the scene still plays,
/// without its events.
fn read_sequence(data: &[u8]) -> Option<Arc<Package>> {
    let chunk = blackbox_chunk::find(data, ids::EVENT_SEQUENCES)?;
    let blob = chunk.aligned_payload(16).get(16..)?;
    match Package::parse(blob) {
        Ok(package) => Some(Arc::new(package)),
        Err(error) => {
            log::warn!("NIS event sequence unreadable, the scene plays without its events: {error}");
            None
        }
    }
}

/// The bundle file name of the scene `scene_name`, as the install spells it.
fn bundle_file(dir: &GameDir, scene_name: &str) -> Option<String> {
    dir.files_in(NIS_DIR)
        .into_iter()
        .find(|file| scene_name_of_file(file).is_some_and(|name| name.eq_ignore_ascii_case(scene_name)))
}

/// The scene name of a bundle file name: the text between the first and second underscore of `Scene_<name>_BundleB.bun`.
/// `None` for any other file name.
pub fn scene_name_of_file(file: &str) -> Option<&str> {
    let mut parts = file.splitn(3, '_');
    let (prefix, name, suffix) = (parts.next()?, parts.next()?, parts.next()?);
    let is_bundle = prefix.eq_ignore_ascii_case("scene") && suffix.eq_ignore_ascii_case("BundleB.bun");
    match is_bundle && !name.is_empty() {
        true => Some(name),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{NisBundle, scene_name_of_file};

    #[test]
    fn the_scene_name_is_the_text_between_the_first_two_underscores() {
        assert_eq!(scene_name_of_file("Scene_IntroNis05_BundleB.bun"), Some("IntroNis05"));
        assert_eq!(scene_name_of_file("scene_ArrestF02_bundleb.bun"), Some("ArrestF02"), "case does not matter");
        assert_eq!(scene_name_of_file("Scene_ArrestF02_BundleB.bun.bak"), None);
        assert_eq!(scene_name_of_file("Scene__BundleB.bun"), None, "an empty name is not a scene");
        assert_eq!(scene_name_of_file("Scene_IntroNis05_BundleA.bun"), None);
        assert_eq!(scene_name_of_file("Other_IntroNis05_BundleB.bun"), None);
        assert_eq!(scene_name_of_file("IntroNis05.bun"), None);
    }

    #[test]
    fn bytes_without_a_scene_chunk_are_not_a_bundle() {
        let err = NisBundle::from_bytes("IntroNis05", &[]).expect_err("no AnimScene chunk");
        assert!(format!("{err:#}").contains("no AnimScene chunk"), "{err:#}");
    }
}
