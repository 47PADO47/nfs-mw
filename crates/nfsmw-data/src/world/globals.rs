//! Texture packs outside the stream that world geometry also uses: the sky
//! (`SKY_MIDDAY_A_*` in `InGameA.bun`) and shared in-game textures.

use anyhow::{Context, Result};
use blackbox_tpk::{Texture, TextureAnim};
use game_install::GameDir;

use crate::read_unwrapped;

/// Searched in order; a texture already found is not replaced.
pub const GLOBAL_TEXTURE_FILES: &[&str] = &["GLOBAL/INGAMEA.BUN", "GLOBAL/GLOBALB.LZC", "GLOBAL/INGAMEB.BUN"];

/// Textures and texture animations from the global packs.
#[derive(Default)]
pub struct GlobalTextures {
    pub textures: Vec<Texture>,
    pub anims: Vec<TextureAnim>,
}

/// Every texture and texture animation in the global packs.
pub fn load_global_textures(dir: &GameDir) -> Result<GlobalTextures> {
    let mut out = GlobalTextures::default();
    for rel in GLOBAL_TEXTURE_FILES.iter().filter(|rel| dir.exists(rel)) {
        let file = read_unwrapped(dir, rel)?;
        for pack in blackbox_tpk::read_texture_packs(&file).with_context(|| format!("texture packs in {rel}"))? {
            out.textures.extend(pack.textures);
        }
        out.anims.extend(blackbox_tpk::read_texture_anims(&file));
    }
    log::info!("{} global textures, {} animations", out.textures.len(), out.anims.len());
    Ok(out)
}
