//! Stage-local images take precedence over shared front-end material references.
use std::collections::HashSet;

use anyhow::{Context, Result};
use blackbox_solid::Solid;
use blackbox_tpk::TexturePack;
use game_install::GameDir;

pub(super) const SOURCES: &[&str] =
    &["GLOBAL/GLOBALB.LZC", "GLOBAL/GLOBALA.BUN", "FRONTEND/FRONTA.LZC", "FRONTEND/FRONTB.LZC"];

pub(super) fn complete(dir: &GameDir, solids: &[Solid], packs: &mut Vec<TexturePack>) -> Result<()> {
    let mut missing: HashSet<_> =
        solids.iter().flat_map(|solid| solid.groups.iter().filter_map(|group| group.diffuse_texture(solid))).collect();
    for pack in packs.iter() {
        for texture in &pack.textures {
            missing.remove(&texture.name_hash);
        }
    }
    for source in SOURCES.iter().filter(|source| dir.exists(source)) {
        if missing.is_empty() {
            break;
        }
        let bytes = nfsmw_data::shared_unwrapped(dir, source)?;
        let supplemental = blackbox_tpk::read_texture_packs_filtered(&bytes, |hash| missing.contains(&hash))
            .with_context(|| format!("shared menu materials in {source}"))?;
        for pack in supplemental {
            for texture in &pack.textures {
                missing.remove(&texture.name_hash);
            }
            if !pack.textures.is_empty() {
                packs.push(pack);
            }
        }
    }
    if !missing.is_empty() {
        log::warn!("unresolved menu material references: {missing:x?}");
    }
    Ok(())
}
