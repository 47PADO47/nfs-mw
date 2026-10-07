//! Texture packs outside the stream that world geometry also uses: the sky
//! (`SKY_MIDDAY_A_*` in `InGameA.bun`) and shared in-game textures.

use anyhow::{Context, Result};
use blackbox_tpk::Texture;
use game_install::GameDir;

use crate::read_unwrapped;

/// Searched in order; a texture already found is not replaced.
pub const GLOBAL_TEXTURE_FILES: &[&str] = &["GLOBAL/INGAMEA.BUN", "GLOBAL/GLOBALB.LZC"];

/// Every texture in the global packs.
pub fn load_global_textures(dir: &GameDir) -> Result<Vec<Texture>> {
    let mut textures = Vec::new();
    for rel in GLOBAL_TEXTURE_FILES.iter().filter(|rel| dir.exists(rel)) {
        let file = read_unwrapped(dir, rel)?;
        for pack in blackbox_tpk::read_texture_packs(&file).with_context(|| format!("texture packs in {rel}"))? {
            textures.extend(pack.textures);
        }
    }
    log::info!("{} global textures", textures.len());
    Ok(textures)
}
