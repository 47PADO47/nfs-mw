//! Texture upload: DXT straight to the GPU when possible, else decoded to RGBA8.

use std::borrow::Cow;

use blackbox_gfx::{BlendMode, PixelFormat, RenderBackend, TextureDesc, TextureHandle};
use blackbox_tpk::{AlphaUsage, PixelFormat as TpkFormat, Texture};

/// Upload `t`, or `None` (with a warning) for formats that can't be decoded yet.
pub fn upload_texture(backend: &mut dyn RenderBackend, t: &Texture) -> Option<TextureHandle> {
    upload_texture_predecoded(backend, t, None)
}

/// Upload `t` like [`upload_texture`]. `decoded` is the RGBA8 of `t` when a loader thread already decoded it
/// (`SectionData::decode_for_upload`, the same rule as `Texture::needs_decode`), so the render thread only copies it.
pub fn upload_texture_predecoded(
    backend: &mut dyn RenderBackend,
    t: &Texture,
    decoded: Option<&[u8]>,
) -> Option<TextureHandle> {
    upload_texture_predecoded_mips(backend, t, decoded, 0)
}

/// Discard available top mips before allocation; this changes residency, not sampler bias.
/// Predecoded pixels describe level zero and are used only when that level is retained.
pub fn upload_texture_predecoded_mips(
    backend: &mut dyn RenderBackend,
    t: &Texture,
    decoded: Option<&[u8]>,
    discard: u32,
) -> Option<TextureHandle> {
    let count = (0..t.mip_levels.min(32)).take_while(|&level| t.mip(level).is_some()).count() as u32;
    if count == 0 {
        return None;
    }
    let first = discard.min(count - 1);
    let width = (t.width >> first).max(1);
    let height = (t.height >> first).max(1);
    let bc = match t.format {
        TpkFormat::Dxt1 => Some(PixelFormat::Bc1),
        TpkFormat::Dxt3 => Some(PixelFormat::Bc2),
        TpkFormat::Dxt5 => Some(PixelFormat::Bc3),
        _ => None,
    };
    match bc {
        // wgpu needs block-compressed textures to be a whole number of blocks (`needs_decode` checks that).
        Some(format) if backend.capabilities().compressed_bc && width.is_multiple_of(4) && height.is_multiple_of(4) => {
            let mips: Vec<&[u8]> = (first..count).map_while(|level| t.mip(level)).collect();
            Some(backend.create_texture(&TextureDesc { label: &t.name, width, height, format, mips }))
        }
        _ => match rgba8_mip(t, decoded, first) {
            Some(rgba) => Some(backend.create_texture(&TextureDesc {
                label: &t.name,
                width,
                height,
                format: PixelFormat::Rgba8,
                mips: vec![&rgba[..]],
            })),
            None => {
                log::warn!("{}: {:?} {}x{} is not supported yet", t.name, t.format, t.width, t.height);
                None
            }
        },
    }
}

/// The RGBA8 pixels of `t`: the loader's copy when there is one, else decoded here.
fn rgba8_mip<'a>(t: &Texture, decoded: Option<&'a [u8]>, level: u32) -> Option<Cow<'a, [u8]>> {
    if level == 0
        && let Some(rgba) = decoded
    {
        return Some(Cow::Borrowed(rgba));
    }
    blackbox_tpk::decode_mip_rgba8(t, level).map(Cow::Owned)
}

/// How a texture's alpha should be drawn, from its `AlphaBlendType`
/// (`TEXBLEND_*`: 0 copy, 1 blend, 2 additive, 3 subtractive, 4 overbright).
///
/// Copy-mode textures with "modulated" alpha keep a specular / reflection mask
/// in alpha (it is not transparency), so they draw opaque; punch-through alpha
/// is cut out. Blend-mode textures include glass, leaf cards and the `SHD_`
/// shadow overlays laid over roads and terrain. See `docs/formats/textures.md` ("Alpha").
pub fn blend_mode(t: Option<&Texture>) -> BlendMode {
    let Some(t) = t else { return BlendMode::Opaque };
    match t.alpha_blend {
        1 => BlendMode::AlphaBlend,
        2 => BlendMode::Additive,
        3 => BlendMode::Subtractive,
        // Overbright remains approximate until its render state is decoded.
        4 => BlendMode::AlphaBlend,
        _ if t.alpha_usage == AlphaUsage::PunchThrough => BlendMode::AlphaTest,
        _ => BlendMode::Opaque,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtractive_texture_uses_its_own_mode() {
        let texture = Texture {
            name: String::new(),
            name_hash: 0,
            width: 1,
            height: 1,
            mip_levels: 1,
            format: TpkFormat::Dxt3,
            compression_type: 0,
            alpha_usage: AlphaUsage::Modulated,
            alpha_sorting: true,
            alpha_blend: 3,
            flags: 0,
            data: Vec::new(),
            palette: Vec::new(),
        };
        assert_eq!(blend_mode(Some(&texture)), BlendMode::Subtractive);
    }
}
