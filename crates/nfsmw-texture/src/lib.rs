//! TPK texture packs. Spec: `docs/formats/textures.md`.
//!
//! A pack comes in one of two forms:
//!
//! - **Plain:** `TexturePackInfoTextures` holds one 0x7C-byte `TextureInfo` per
//!   texture, `TexturePackInfoComps` one 0x20-byte platform record, and the pixels
//!   live in `TexturePackDataArray` at `ImagePlacement`.
//! - **Compressed:** `TexturePackInfoEntries` holds 24-byte streaming entries that
//!   point (by file offset) at one JDLZ or HUFF blob per texture. Each blob inflates
//!   to the pixels followed by the 0x7C `TextureInfo` and the 0x20 platform record.

mod decode;

pub use decode::{decode_rgba8, mip_level_size};

use nfsmw_bchunk::{Chunk, ids};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] nfsmw_bchunk::Error),
    #[error(transparent)]
    Compress(#[from] nfsmw_compress::Error),
    #[error("texture pack at 0x{offset:X}: {detail}")]
    Pack { offset: usize, detail: String },
    #[error("texture 0x{hash:08X}: {detail}")]
    Texture { hash: u32, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Size of an on-disk `TextureInfo` record.
pub const TEXTURE_INFO_LEN: usize = 0x7C;
/// Size of an on-disk platform (`Comps`) record.
pub const PLAT_INFO_LEN: usize = 0x20;
/// Size of one `TexturePackInfoEntries` record.
pub const ENTRY_LEN: usize = 24;

/// Pixel formats seen in the PC install (see `docs/formats/textures.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    /// D3DFMT_A8R8G8B8 (21): bytes B, G, R, A.
    Argb8888,
    /// D3DFMT_P8 (41): one palette index per pixel.
    P8,
    /// Anything else, as the raw D3DFORMAT / FourCC value.
    Other(u32),
}

impl PixelFormat {
    pub fn from_d3d(format: u32) -> Self {
        match &format.to_le_bytes() {
            b"DXT1" => Self::Dxt1,
            b"DXT3" => Self::Dxt3,
            b"DXT5" => Self::Dxt5,
            _ => match format {
                21 => Self::Argb8888,
                41 => Self::P8,
                other => Self::Other(other),
            },
        }
    }

    /// Bytes per 4×4 block for block-compressed formats.
    pub fn block_bytes(self) -> Option<usize> {
        match self {
            Self::Dxt1 => Some(8),
            Self::Dxt3 | Self::Dxt5 => Some(16),
            _ => None,
        }
    }
}

/// How the engine treats a texture's alpha channel (`TextureAlphaUsageType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaUsage {
    None,
    PunchThrough,
    Modulated,
    Other(u8),
}

impl From<u8> for AlphaUsage {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::None,
            1 => Self::PunchThrough,
            2 => Self::Modulated,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub name: String,
    pub name_hash: u32,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub format: PixelFormat,
    /// `TextureCompressionType` byte from the info record (e.g. 0x22 = DXT1).
    pub compression_type: u8,
    pub alpha_usage: AlphaUsage,
    pub alpha_blend: u8,
    /// All mip levels, largest first, as stored.
    pub data: Vec<u8>,
    /// Palette for [`PixelFormat::P8`], as stored.
    pub palette: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TexturePack {
    pub name: String,
    pub filename: String,
    pub textures: Vec<Texture>,
    /// Textures that could not be read (e.g. an unsupported compressor), with the reason.
    pub failed: Vec<(u32, String)>,
}

/// Fields of the 0x7C `TextureInfo` record that the reader needs.
#[derive(Debug, Clone, Copy)]
struct InfoRecord<'a> {
    raw: &'a [u8],
}

impl InfoRecord<'_> {
    fn u32(&self, o: usize) -> u32 {
        u32::from_le_bytes(self.raw[o..o + 4].try_into().unwrap())
    }
    fn u16(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.raw[o], self.raw[o + 1]])
    }
    fn name(&self) -> String {
        let n = &self.raw[0x0C..0x24];
        let len = n.iter().position(|&b| b == 0).unwrap_or(n.len());
        String::from_utf8_lossy(&n[..len]).into_owned()
    }
    fn name_hash(&self) -> u32 {
        self.u32(0x24)
    }
    fn image_placement(&self) -> usize {
        self.u32(0x30) as usize
    }
    fn palette_placement(&self) -> usize {
        self.u32(0x34) as usize
    }
    fn image_size(&self) -> usize {
        self.u32(0x38) as usize
    }
    fn palette_size(&self) -> usize {
        self.u32(0x3C) as usize
    }
}

fn build_texture(info: InfoRecord<'_>, plat: &[u8], data: Vec<u8>, palette: Vec<u8>) -> Texture {
    let format = PixelFormat::from_d3d(u32::from_le_bytes(plat[0x14..0x18].try_into().unwrap()));
    let width = u32::from(info.u16(0x44));
    let height = u32::from(info.u16(0x46));
    let declared_mips = u32::from(info.raw[0x4E]).max(1);
    // Never trust the mip count past the data we actually have.
    let mut mip_levels = 0;
    let mut used = 0;
    while mip_levels < declared_mips {
        let size = mip_level_size(format, width, height, mip_levels);
        if size == 0 || used + size > data.len() {
            break;
        }
        used += size;
        mip_levels += 1;
    }
    Texture {
        name: info.name(),
        name_hash: info.name_hash(),
        width,
        height,
        mip_levels: mip_levels.max(1),
        format,
        compression_type: info.raw[0x4A],
        alpha_usage: AlphaUsage::from(info.raw[0x55]),
        alpha_blend: info.raw[0x56],
        data,
        palette,
    }
}

fn cstr(b: &[u8]) -> String {
    let len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..len]).into_owned()
}

/// Every texture pack in `file` (a whole file with any wrapper removed).
///
/// Compressed packs address their blobs by offset into `file`, so pass the whole
/// file, not just the pack chunk.
pub fn read_texture_packs(file: &[u8]) -> Result<Vec<TexturePack>> {
    nfsmw_bchunk::find_all(file, ids::TEXTURE_PACK).into_iter().map(|pack| read_pack(file, pack)).collect()
}

fn read_pack(file: &[u8], pack: Chunk<'_>) -> Result<TexturePack> {
    let err = |detail: &str| Error::Pack { offset: pack.offset, detail: detail.to_owned() };
    let info = pack.child(ids::TEXTURE_PACK_INFO).ok_or_else(|| err("no TexturePackInfo"))?;
    let header = info.child(ids::TEXTURE_PACK_INFO_HEADER).ok_or_else(|| err("no TexturePackInfoHeader"))?;
    let header = header.payload;
    if header.len() < 0x60 {
        return Err(err("TexturePackInfoHeader too short"));
    }
    let mut out = TexturePack {
        name: cstr(&header[0x04..0x20]),
        filename: cstr(&header[0x20..0x60]),
        textures: Vec::new(),
        failed: Vec::new(),
    };

    if let Some(entries) = info.child(ids::TEXTURE_PACK_INFO_ENTRIES) {
        for e in entries.payload.as_chunks::<ENTRY_LEN>().0 {
            let hash = u32::from_le_bytes(e[0..4].try_into().unwrap());
            match read_compressed_texture(file, e) {
                Ok(t) => out.textures.push(t),
                Err(why) => out.failed.push((hash, why.to_string())),
            }
        }
        return Ok(out);
    }

    let infos = info.child(ids::TEXTURE_PACK_INFO_TEXTURES).ok_or_else(|| err("no TexturePackInfoTextures"))?;
    let plats = info.child(ids::TEXTURE_PACK_INFO_COMPS).ok_or_else(|| err("no TexturePackInfoComps"))?;
    let array = pack
        .child(ids::TEXTURE_PACK_DATA)
        .and_then(|d| d.child(ids::TEXTURE_PACK_DATA_ARRAY))
        .ok_or_else(|| err("no TexturePackDataArray"))?
        .aligned_payload(0x80);

    for (raw, plat) in
        infos.payload.as_chunks::<TEXTURE_INFO_LEN>().0.iter().zip(plats.payload.as_chunks::<PLAT_INFO_LEN>().0)
    {
        let rec = InfoRecord { raw };
        let image = array.get(rec.image_placement()..rec.image_placement() + rec.image_size());
        let palette = if rec.palette_size() > 0 {
            array.get(rec.palette_placement()..rec.palette_placement() + rec.palette_size())
        } else {
            Some(&[][..])
        };
        match (image, palette) {
            (Some(image), Some(palette)) => {
                out.textures.push(build_texture(rec, plat, image.to_vec(), palette.to_vec()));
            }
            _ => out.failed.push((rec.name_hash(), "pixel data lies outside TexturePackDataArray".into())),
        }
    }
    Ok(out)
}

fn read_compressed_texture(file: &[u8], entry: &[u8]) -> Result<Texture> {
    let hash = u32::from_le_bytes(entry[0..4].try_into().unwrap());
    let offset = u32::from_le_bytes(entry[4..8].try_into().unwrap()) as usize;
    let csize = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
    let usize_ = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
    let err = |detail: String| Error::Texture { hash, detail };

    let blob = file
        .get(offset..offset + csize)
        .ok_or_else(|| err(format!("blob 0x{offset:X}+{csize} lies outside the file")))?;
    let block = nfsmw_compress::unwrap(blob)?;
    let trailer = TEXTURE_INFO_LEN + PLAT_INFO_LEN;
    if block.len() != usize_ || block.len() < trailer {
        return Err(err(format!("inflated to {} bytes, expected {usize_}", block.len())));
    }
    let split = block.len() - trailer;
    let rec = InfoRecord { raw: &block[split..split + TEXTURE_INFO_LEN] };
    let plat = &block[split + TEXTURE_INFO_LEN..];
    if rec.name_hash() != hash {
        return Err(err(format!("trailer names 0x{:08X}", rec.name_hash())));
    }
    let image_end = rec.image_size().min(split);
    // Unconfirmed: a palette, if any, follows the image inside the block.
    let palette = block.get(image_end..(image_end + rec.palette_size()).min(split)).unwrap_or_default().to_vec();
    Ok(build_texture(rec, plat, block[..image_end].to_vec(), palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_formats() {
        assert_eq!(PixelFormat::from_d3d(u32::from_le_bytes(*b"DXT1")), PixelFormat::Dxt1);
        assert_eq!(PixelFormat::from_d3d(u32::from_le_bytes(*b"DXT5")), PixelFormat::Dxt5);
        assert_eq!(PixelFormat::from_d3d(21), PixelFormat::Argb8888);
        assert_eq!(PixelFormat::from_d3d(41), PixelFormat::P8);
        assert_eq!(PixelFormat::from_d3d(7), PixelFormat::Other(7));
    }

    fn info_record(name: &str, hash: u32, w: u16, h: u16, image_size: u32, mips: u8) -> Vec<u8> {
        let mut r = vec![0u8; TEXTURE_INFO_LEN];
        r[0x0C..0x0C + name.len()].copy_from_slice(name.as_bytes());
        r[0x24..0x28].copy_from_slice(&hash.to_le_bytes());
        r[0x38..0x3C].copy_from_slice(&image_size.to_le_bytes());
        r[0x44..0x46].copy_from_slice(&w.to_le_bytes());
        r[0x46..0x48].copy_from_slice(&h.to_le_bytes());
        r[0x4A] = 0x22;
        r[0x4E] = mips;
        r[0x55] = 1;
        r
    }

    fn plat_record(fourcc: &[u8; 4]) -> Vec<u8> {
        let mut p = vec![0u8; PLAT_INFO_LEN];
        p[0x14..0x18].copy_from_slice(fourcc);
        p
    }

    #[test]
    fn compressed_entry_with_raww_blob() {
        // 8x8 DXT1 with 2 mips: 32 + 8 bytes of pixels, then the trailer.
        let mut block = vec![0xAB; 40];
        block.extend(info_record("TEST", 0x1234_5678, 8, 8, 40, 2));
        block.extend(plat_record(b"DXT1"));
        let mut blob = b"RAWW".to_vec();
        blob.extend_from_slice(&[1, 0x10, 0, 0]);
        blob.extend_from_slice(&(block.len() as u32).to_le_bytes());
        blob.extend_from_slice(&((block.len() + 16) as u32).to_le_bytes());
        blob.extend_from_slice(&block);

        let mut file = vec![0u8; 32];
        let offset = file.len() as u32;
        file.extend_from_slice(&blob);
        let mut entry = Vec::new();
        entry.extend_from_slice(&0x1234_5678u32.to_le_bytes());
        entry.extend_from_slice(&offset.to_le_bytes());
        entry.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        entry.extend_from_slice(&(block.len() as u32).to_le_bytes());
        entry.extend_from_slice(&[0; 8]);

        let t = read_compressed_texture(&file, &entry).unwrap();
        assert_eq!(t.name, "TEST");
        assert_eq!((t.width, t.height, t.mip_levels), (8, 8, 2));
        assert_eq!(t.format, PixelFormat::Dxt1);
        assert_eq!(t.alpha_usage, AlphaUsage::PunchThrough);
        assert_eq!(t.data.len(), 40);
    }

    #[test]
    fn mip_count_is_clamped_to_data() {
        let info = info_record("X", 1, 8, 8, 32, 4);
        let t = build_texture(InfoRecord { raw: &info }, &plat_record(b"DXT1"), vec![0; 32], Vec::new());
        assert_eq!(t.mip_levels, 1);
    }
}
