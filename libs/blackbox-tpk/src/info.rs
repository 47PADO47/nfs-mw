//! Reading one `TextureInfo` + platform record pair into a [`Texture`].

use crate::layout::PackLayout;
use crate::{AlphaUsage, PixelFormat, Texture, mip_level_size};

/// A `TextureInfo` record viewed through a version's layout.
#[derive(Clone, Copy)]
pub(crate) struct InfoRecord<'a> {
    pub raw: &'a [u8],
    pub layout: &'static PackLayout,
}

impl InfoRecord<'_> {
    fn u32(&self, o: usize) -> u32 {
        u32::from_le_bytes(self.raw[o..o + 4].try_into().unwrap())
    }

    fn u16(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.raw[o], self.raw[o + 1]])
    }

    pub fn name(&self) -> String {
        let n = &self.raw[self.layout.info.name.clone()];
        let len = n.iter().position(|&b| b == 0).unwrap_or(n.len());
        String::from_utf8_lossy(&n[..len]).into_owned()
    }

    pub fn name_hash(&self) -> u32 {
        self.u32(self.layout.info.name_hash)
    }

    pub fn image_placement(&self) -> usize {
        self.u32(self.layout.info.image_placement) as usize
    }

    pub fn palette_placement(&self) -> usize {
        self.u32(self.layout.info.palette_placement) as usize
    }

    pub fn image_size(&self) -> usize {
        self.u32(self.layout.info.image_size) as usize
    }

    pub fn palette_size(&self) -> usize {
        self.u32(self.layout.info.palette_size) as usize
    }

    /// Build the texture from this record, its platform record and its pixels.
    pub fn build(&self, plat: &[u8], data: Vec<u8>, palette: Vec<u8>) -> Texture {
        let l = &self.layout.info;
        let fo = self.layout.plat_format;
        let format = PixelFormat::from_d3d(u32::from_le_bytes(plat[fo..fo + 4].try_into().unwrap()));
        let width = u32::from(self.u16(l.width));
        let height = u32::from(self.u16(l.height));
        let declared_mips = u32::from(self.raw[l.mip_levels]).max(1);
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
            name: self.name(),
            name_hash: self.name_hash(),
            width,
            height,
            mip_levels: mip_levels.max(1),
            format,
            compression_type: self.raw[l.compression_type],
            alpha_usage: AlphaUsage::from(self.raw[l.alpha_usage]),
            alpha_sorting: self.raw[l.alpha_sorting] != 0,
            alpha_blend: self.raw[l.alpha_blend],
            data,
            palette,
        }
    }
}
