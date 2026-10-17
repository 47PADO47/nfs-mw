//! Texture and model packs outside the stream that world geometry also uses: the sky
//! (`SKY_MIDDAY_A_*` in `InGameA.bun`), shared in-game textures and the shared models of `InGameA.bun`.
//! The game's region loader opens `Global\InGameA.bun` (`docs/formats/global-databases.md`).

use anyhow::{Context, Result};
use blackbox_solid::Solid;
use blackbox_tpk::{Texture, TextureAnim};
use game_install::GameDir;

use crate::shared_unwrapped;

/// Searched in order; a texture already found is not replaced.
pub const GLOBAL_TEXTURE_FILES: &[&str] = &["GLOBAL/INGAMEA.BUN", "GLOBAL/GLOBALB.LZC", "GLOBAL/INGAMEB.BUN"];

/// The global packs whose solids (shared models) are loaded. `InGameA.bun` holds the sky domes
/// (`SKYDOME`, `SKYDOME_XENON`) and the other `SKY_*` models.
pub const GLOBAL_MODEL_FILES: &[&str] = &["GLOBAL/INGAMEA.BUN"];

/// Textures, texture animations and shared models from the global packs.
#[derive(Default)]
pub struct GlobalTextures {
    pub textures: Vec<Texture>,
    pub anims: Vec<TextureAnim>,
    /// Shared solids (models) from [`GLOBAL_MODEL_FILES`].
    pub solids: Vec<Solid>,
}

/// Every texture, texture animation and shared model in the global packs.
pub fn load_global_textures(dir: &GameDir) -> Result<GlobalTextures> {
    let mut out = GlobalTextures::default();
    for rel in GLOBAL_TEXTURE_FILES.iter().filter(|rel| dir.exists(rel)) {
        let file = shared_unwrapped(dir, rel)?;
        for pack in blackbox_tpk::read_texture_packs(&file).with_context(|| format!("texture packs in {rel}"))? {
            out.textures.extend(pack.textures);
        }
        out.anims.extend(blackbox_tpk::read_texture_anims(&file));
        if GLOBAL_MODEL_FILES.contains(rel) {
            let solids = blackbox_solid::read_solids(&file).with_context(|| format!("shared models in {rel}"))?;
            out.solids.extend(solids);
        }
    }
    log::info!(
        "{} global textures, {} animations, {} shared models",
        out.textures.len(),
        out.anims.len(),
        out.solids.len()
    );
    Ok(out)
}
