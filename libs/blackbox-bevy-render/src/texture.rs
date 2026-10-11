//! Native textures into Bevy images.
//!
//! The textures are uploaded as plain `Unorm` images, not sRGB ones: the games' art is authored for a renderer
//! that shades and filters gamma-encoded values, and the material shader (see `docs/bevy-backend.md`, "Colour")
//! does the same, so filtering and mip selection match the native renderer.

use bevy_asset::RenderAssetUsages;
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use blackbox_gfx::{PixelFormat, TextureDesc};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Anisotropic filtering of the world textures, like the native sampler.
const ANISOTROPY: u16 = 8;

fn format_of(format: PixelFormat) -> (TextureFormat, u32, u32) {
    // (wgpu format, block edge in pixels, bytes per block)
    match format {
        PixelFormat::Bc1 => (TextureFormat::Bc1RgbaUnorm, 4, 8),
        PixelFormat::Bc2 => (TextureFormat::Bc2RgbaUnorm, 4, 16),
        PixelFormat::Bc3 => (TextureFormat::Bc3RgbaUnorm, 4, 16),
        PixelFormat::Rgba8 => (TextureFormat::Rgba8Unorm, 1, 4),
    }
}

/// The repeating, trilinear, anisotropic sampler every world texture uses.
pub fn world_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: ANISOTROPY,
        ..ImageSamplerDescriptor::default()
    })
}

/// The Bevy image for `desc`: every mip, back to back, in the format the device samples directly.
///
/// A mip shorter than its size needs ends the chain there (with a warning), like the native upload. Block
/// compressed formats need `block_compression`; without it the caller must decode to RGBA8 first.
pub fn image(desc: &TextureDesc<'_>, block_compression: bool) -> Option<Image> {
    let (format, block, block_bytes) = format_of(desc.format);
    if block > 1 && !block_compression {
        log::warn!("{}: {:?} needs block compression, which this device lacks", desc.label, desc.format);
        return None;
    }
    if desc.width == 0 || desc.height == 0 {
        return None;
    }
    let mut data = Vec::new();
    let mut levels = 0;
    for (level, mip) in desc.mips.iter().enumerate() {
        let w = (desc.width >> level).max(1);
        let h = (desc.height >> level).max(1);
        let needed = (w.div_ceil(block) * h.div_ceil(block) * block_bytes) as usize;
        if mip.len() < needed {
            log::warn!("{}: mip {level} is {} bytes, expected {needed}", desc.label, mip.len());
            break;
        }
        data.extend_from_slice(&mip[..needed]);
        levels += 1;
    }
    if levels == 0 {
        return None;
    }
    let size = Extent3d { width: desc.width, height: desc.height, depth_or_array_layers: 1 };
    let mut image = Image::new_uninit(size, TextureDimension::D2, format, RenderAssetUsages::RENDER_WORLD);
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = levels;
    image.texture_descriptor.label = None;
    image.sampler = world_sampler();
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(width: u32, height: u32, mips: &[&[u8]]) -> TextureDesc<'static> {
        // The slices are copied into leaked buffers so the test can build 'static descriptors tersely.
        let mips = mips.iter().map(|m| &*Box::leak(m.to_vec().into_boxed_slice())).collect();
        TextureDesc { label: "t", width, height, format: PixelFormat::Rgba8, mips }
    }

    #[test]
    fn rgba_mips_are_concatenated() {
        let image = image(&rgba(2, 2, &[&[1; 16], &[2; 4]]), false).unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        assert_eq!(image.data.as_ref().unwrap().len(), 20);
        assert_eq!(image.data.as_ref().unwrap()[16], 2);
    }

    #[test]
    fn a_short_mip_ends_the_chain() {
        let image = image(&rgba(2, 2, &[&[1; 16], &[2; 3]]), false).unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        assert_eq!(image.data.as_ref().unwrap().len(), 16);
        assert!(super::image(&rgba(2, 2, &[&[1; 15]]), false).is_none());
    }

    #[test]
    fn block_compressed_mips_use_whole_blocks() {
        let desc = TextureDesc {
            label: "bc",
            width: 8,
            height: 8,
            format: PixelFormat::Bc1,
            // 8x8: 4 blocks; 4x4: 1 block; 2x2: still 1 block of 4x4 texels.
            mips: vec![&[1; 32], &[2; 8], &[3; 8]],
        };
        let image = image(&desc, true).unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
        assert_eq!(image.texture_descriptor.format, TextureFormat::Bc1RgbaUnorm);
        assert_eq!(image.data.as_ref().unwrap().len(), 48);
        assert!(super::image(&desc, false).is_none(), "no BC support, no BC image");
    }

    #[test]
    fn the_formats_map_to_unorm_not_srgb() {
        for (native, expected) in [
            (PixelFormat::Bc2, TextureFormat::Bc2RgbaUnorm),
            (PixelFormat::Bc3, TextureFormat::Bc3RgbaUnorm),
            (PixelFormat::Rgba8, TextureFormat::Rgba8Unorm),
        ] {
            assert_eq!(format_of(native).0, expected);
        }
    }

    #[test]
    fn empty_textures_make_no_image() {
        assert!(image(&rgba(0, 4, &[&[]]), false).is_none());
        assert!(image(&rgba(4, 4, &[]), false).is_none());
    }
}
