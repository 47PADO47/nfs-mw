use anyhow::{Context, Result};
use game_install::GameDir;

/// Read a whole file from the install and remove any compression wrapper.
pub fn read_unwrapped(dir: &GameDir, rel: &str) -> Result<Vec<u8>> {
    let raw = dir.read(rel).with_context(|| format!("reading {rel}"))?;
    Ok(ea_compress::unwrap(&raw).with_context(|| format!("decompressing {rel}"))?.into_owned())
}
