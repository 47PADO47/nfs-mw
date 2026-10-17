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
    let bc = match t.format {
        TpkFormat::Dxt1 => Some(PixelFormat::Bc1),
        TpkFormat::Dxt3 => Some(PixelFormat::Bc2),
        TpkFormat::Dxt5 => Some(PixelFormat::Bc3),
        _ => None,
    };
    match bc {
        // wgpu needs block-compressed textures to be a whole number of blocks (`needs_decode` checks that).
        Some(format) if !t.needs_decode(backend.capabilities().compressed_bc) => {
            let mips: Vec<&[u8]> = (0..t.mip_levels).map_while(|level| t.mip(level)).collect();
            Some(backend.create_texture(&TextureDesc {
                label: &t.name,
                width: t.width,
                height: t.height,
                format,
                mips,
            }))
        }
        _ => match rgba8(t, decoded) {
            Some(rgba) => Some(backend.create_texture(&TextureDesc {
                label: &t.name,
                width: t.width,
                height: t.height,
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
fn rgba8<'a>(t: &Texture, decoded: Option<&'a [u8]>) -> Option<Cow<'a, [u8]>> {
    if let Some(rgba) = decoded {
        return Some(Cow::Borrowed(rgba));
    }
    blackbox_tpk::decode_rgba8(t).map(Cow::Owned)
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
        // Subtractive and overbright are rare; approximate until the effects are decoded.
        3 | 4 => BlendMode::AlphaBlend,
        _ if t.alpha_usage == AlphaUsage::PunchThrough => BlendMode::AlphaTest,
        _ => BlendMode::Opaque,
    }
}
