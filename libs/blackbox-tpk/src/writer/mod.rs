//! Writing TPK texture packs (version 5, NFS: Most Wanted PC). Spec: `docs/formats/textures.md`.
//!
//! The output parses back with [`read_texture_packs`](crate::read_texture_packs) to the textures that went in.
//! Two forms are produced (see [`PackForm`]); both lay the chunks out the way the game's own packs do: info
//! container at 0x40, data container and array chunk at 0x80, array payload at 0x80, key and entry tables
//! sorted by name hash.

mod chunk_buf;
mod error;
mod jdlz;
mod plain;
mod records;
mod skeleton;
mod source;
mod streamed;

#[cfg(test)]
mod tests;

pub use error::{WriteError, WriteResult};
pub use source::{DEFAULT_RENDERING_ORDER, MAX_DIMENSION, MAX_STORED_NAME, TextureSource, default_compression_type};

use std::collections::HashMap;

use crate::layout::{self, PackLayout};

/// How the pixels are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackForm {
    /// The form of the car `TEXTURES.BIN`: one JDLZ blob per texture in the data array, addressed by file offset
    /// from 24-byte streaming entries. The pack must be saved at offset 0 of its file.
    #[default]
    Streamed,
    /// Records in the info chunk, raw pixels in the data array (the form of `GlobalB.lzc` and the world stream).
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriteOptions {
    pub form: PackForm,
    /// The pipeline path stored in the header (`Global\Pipeline\CarTemplateTextures_X.tpk`). Defaults to
    /// `<pack name>.tpk`.
    pub filename: Option<String>,
}

/// The version 5 layout everything here encodes.
fn v5() -> &'static PackLayout {
    layout::for_version(5).expect("the version 5 layout is built in")
}

/// A pack named `name` holding `textures`, in the streamed form of the car `TEXTURES.BIN`.
pub fn write_tpk(name: &str, textures: &[TextureSource]) -> WriteResult<Vec<u8>> {
    write_tpk_with(name, textures, &WriteOptions::default())
}

/// Like [`write_tpk`], choosing the form and the stored file name.
pub fn write_tpk_with(name: &str, textures: &[TextureSource], options: &WriteOptions) -> WriteResult<Vec<u8>> {
    let layout = v5();
    let filename = options.filename.clone().unwrap_or_else(|| format!("{name}.tpk"));
    let header = records::header_record(layout, name, &filename)?;
    for t in textures {
        t.validate()?;
    }
    let sorted = sorted_by_hash(textures)?;
    match options.form {
        PackForm::Streamed => streamed::build(layout, &header, &filename, &sorted),
        PackForm::Plain => plain::build(layout, &header, &filename, &sorted),
    }
}

/// The textures ordered by name hash (the engine looks them up by binary search); duplicates are an error.
fn sorted_by_hash(textures: &[TextureSource]) -> WriteResult<Vec<&TextureSource>> {
    let mut seen: HashMap<u32, &str> = HashMap::new();
    for t in textures {
        if let Some(first) = seen.insert(t.hash(), &t.name) {
            return Err(WriteError::Duplicate { first: first.to_owned(), second: t.name.clone(), hash: t.hash() });
        }
    }
    let mut sorted: Vec<&TextureSource> = textures.iter().collect();
    sorted.sort_by_key(|t| t.hash());
    Ok(sorted)
}
