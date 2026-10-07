//! Texture upload: DXT straight to the GPU when possible, else decoded to RGBA8.

use blackbox_render::{BlendMode, PixelFormat, Renderer, TextureDesc, TextureHandle};
use blackbox_tpk::{AlphaUsage, PixelFormat as TpkFormat, Texture};

/// Upload `t`, or `None` (with a warning) for formats that can't be decoded yet.
pub fn upload_texture(renderer: &mut Renderer, t: &Texture) -> Option<TextureHandle> {
    let bc = match t.format {
        TpkFormat::Dxt1 => Some(PixelFormat::Bc1),
        TpkFormat::Dxt3 => Some(PixelFormat::Bc2),
        TpkFormat::Dxt5 => Some(PixelFormat::Bc3),
        _ => None,
    };
    match bc {
        // wgpu needs block-compressed textures to be a whole number of blocks.
        Some(format) if renderer.supports_bc() && t.width.is_multiple_of(4) && t.height.is_multiple_of(4) => {
            let mips: Vec<&[u8]> = (0..t.mip_levels).map_while(|level| t.mip(level)).collect();
            Some(renderer.create_texture(&TextureDesc {
                label: &t.name,
                width: t.width,
                height: t.height,
                format,
                mips,
            }))
        }
        _ => match blackbox_tpk::decode_rgba8(t) {
            Some(rgba) => Some(renderer.create_texture(&TextureDesc {
                label: &t.name,
                width: t.width,
                height: t.height,
                format: PixelFormat::Rgba8,
                mips: vec![&rgba],
            })),
            None => {
                log::warn!("{}: {:?} {}x{} is not supported yet", t.name, t.format, t.width, t.height);
                None
            }
        },
    }
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
