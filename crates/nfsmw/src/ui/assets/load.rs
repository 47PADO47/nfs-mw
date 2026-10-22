//! The reads behind [`UiAssets::load_with`](super::UiAssets::load_with). Each returns the textures of one group of
//! sources (first name wins inside the group), so the groups can be read side by side and merged in priority order.

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_feng::fe_hash_upper;
use blackbox_tpk::{Texture, read_texture_packs, read_texture_packs_filtered, read_texture_packs_ranged};
use game_install::{GameDir, Provider};
use nfsmw_data::{read_unwrapped, shared_unwrapped};

use super::{MAP_VARIANTS, RIVALS_FILE, TEXTURE_FILES, TIPS_FILE};
use crate::ui::tips;

/// Textures by name hash.
pub(super) type Textures = HashMap<u32, Texture>;

/// Packs the session memo holds anyway, because the catalog, the fonts or the world read the same file through it:
/// reading them through the memo costs nothing extra. Every other pack is read once and freed after its textures are
/// copied out.
const MEMOISED: [&str; 5] =
    ["GLOBAL/InGameA.bun", "GLOBAL/GlobalB.lzc", "FRONTEND/FrontB.lzc", "GLOBAL/GLOBALA.BUN", "GLOBAL/INGAMEC.BUN"];

/// A pack file's bytes with any compression wrapper removed.
fn pack_bytes(dir: &GameDir, rel: &str) -> Result<Arc<Vec<u8>>> {
    if MEMOISED.iter().any(|memoised| memoised.eq_ignore_ascii_case(rel)) {
        return shared_unwrapped(dir, rel);
    }
    read_unwrapped(dir, rel).map(Arc::new)
}

/// Adds `incoming` to `textures`; a name already there wins.
pub(super) fn merge(textures: &mut Textures, incoming: Textures) {
    for (hash, texture) in incoming {
        textures.entry(hash).or_insert(texture);
    }
}

/// The textures of [`TEXTURE_FILES`], first match wins, and the stock sizes of those among them that `font_textures`
/// names (only a pack the install itself serves counts, not an overlay's).
pub(super) fn texture_files(dir: &GameDir, font_textures: &HashSet<u32>) -> (Textures, HashMap<u32, (u32, u32)>) {
    let mut stock = HashMap::new();
    let mut textures = Textures::new();
    // The ten custom tachometer skins' packs follow the fixed files.
    let gauge_packs: Vec<_> = (1..=10).map(|i| format!("GLOBAL/HUDS_Custom_{i:02}.bin")).collect();
    for rel in TEXTURE_FILES.iter().copied().chain(gauge_packs.iter().map(String::as_str)) {
        let Ok(data) = pack_bytes(dir, rel) else { continue };
        let from_install = matches!(dir.provider(rel), Some(Provider::Install));
        match read_texture_packs(&data) {
            Ok(packs) => {
                for t in packs.into_iter().flat_map(|p| p.textures) {
                    if from_install && font_textures.contains(&t.name_hash) {
                        stock.entry(t.name_hash).or_insert((t.width, t.height));
                    }
                    textures.entry(t.name_hash).or_insert(t);
                }
            }
            Err(e) => log::warn!("{rel}: texture packs do not read: {e}"),
        }
    }
    (textures, stock)
}

/// The wanted textures of the pack file `rel`, read through byte ranges when the file has no compression wrapper (only
/// the headers and the wanted textures are read, however big the file is) and through a whole read otherwise.
pub(super) fn read_wanted(dir: &GameDir, rel: &str, keep: impl Fn(u32) -> bool) -> Result<Vec<Texture>> {
    let mut file = dir.open_file(rel).with_context(|| format!("opening {rel}"))?;
    let mut magic = [0u8; 4];
    let wrapped = match file.read_exact(&mut magic) {
        Ok(()) => matches!(&magic, b"JDLZ" | b"HUFF" | b"RAWW"),
        Err(_) => false,
    };
    let packs = match wrapped {
        true => read_texture_packs_filtered(&read_unwrapped(dir, rel)?, keep),
        false => {
            file.seek(SeekFrom::Start(0))?;
            read_texture_packs_ranged(&mut BufReader::new(file), keep)
        }
    };
    let packs = packs.with_context(|| format!("{rel}: the texture packs do not read"))?;
    Ok(packs.into_iter().flat_map(|p| p.textures).collect())
}

/// Reads the wanted textures of `rel` into a map. A missing file is skipped quietly if `quiet_if_missing`, as the other
/// packs are; any other failure is logged.
fn wanted_into(dir: &GameDir, rel: &str, quiet_if_missing: bool, keep: impl Fn(u32) -> bool) -> Textures {
    if quiet_if_missing && !dir.exists(rel) {
        return Textures::new();
    }
    match read_wanted(dir, rel, keep) {
        Ok(found) => {
            let mut textures = Textures::new();
            for t in found {
                textures.entry(t.name_hash).or_insert(t);
            }
            textures
        }
        Err(e) => {
            log::warn!("{rel}: {e:#}");
            Textures::new()
        }
    }
}

/// The textures of `GLOBAL/RIVALS.BIN` (90 pictures, about 60 MiB once decoded), then of the tips' pack
/// `GLOBAL/DYNTEX.BIN`, first name wins. Neither file is read whole: the 21 MB and 10 MB packs are read for the
/// wanted textures only. A missing file is skipped quietly, as the other packs are.
pub(super) fn rivals_and_tips(dir: &GameDir) -> Textures {
    // Only the menu pictures (`RIVAL_NN`, `_BG`, `_GRAF`) are decoded: the `_IG` variants would double the 30 MB they
    // take.
    let rivals: HashSet<u32> = (1..=15)
        .flat_map(|n| ["", "_BG", "_GRAF"].map(|suffix| fe_hash_upper(&format!("RIVAL_{n:02}{suffix}"))))
        .collect();
    let mut textures = wanted_into(dir, RIVALS_FILE, true, |hash| rivals.contains(&hash));
    // Only the tips' pictures are decoded: the pack holds 278 other textures.
    let pictures: HashSet<u32> = tips::TIPS.iter().map(|key| fe_hash_upper(&tips::picture_name(key))).collect();
    merge(&mut textures, wanted_into(dir, TIPS_FILE, true, |hash| pictures.contains(&hash)));
    textures
}

/// The minimap's tiles: the HUD swaps them into its pieces by name. Every variant's map is read, since the career stage
/// picks one (`nfsmw_career::regions::MapVariant`).
pub(super) fn minimap_tiles(dir: &GameDir) -> Textures {
    let mut textures = Textures::new();
    for variant in MAP_VARIANTS {
        let stem = variant.minimap_file().trim_end_matches(".BIN");
        match nfsmw_data::minimap::read_tiles(dir, stem) {
            Ok(tiles) => {
                for t in tiles.tiles {
                    textures.entry(t.name_hash).or_insert(t);
                }
            }
            Err(e) => log::warn!("the minimap tiles {stem} do not read: {e:#}"),
        }
    }
    textures
}

/// The world map's picture: one of the `TRACK_MAP*` textures of `TrackMaps.bin` (63 MB, no wrapper), one per variant.
/// Only those textures are read, by their byte ranges, and the file is not kept in the session memo.
pub(super) fn world_maps(dir: &GameDir) -> Textures {
    let wanted = MAP_VARIANTS.map(|v| fe_hash_upper(v.world_map_texture()));
    wanted_into(dir, "TRACKS/L2RA/TrackMaps.bin", false, |hash| wanted.contains(&hash))
}
