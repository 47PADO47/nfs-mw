//! The environment cube map: face layout and the procedural sky, ported from native's
//! `libs/blackbox-render/src/gpu/glossy/env.rs` so the same bytes land in the same face order
//! (+X, -X, +Y, -Y, +Z, -Z).

use bevy_asset::RenderAssetUsages;
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use blackbox_gfx::{Environment, SkyGradient};
use glam::Vec3;
use wgpu::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};

const FACES: u32 = 6;

/// The square size of a generated sky face, like native's `SKY_FACE_SIZE`.
const SKY_FACE_SIZE: u32 = 64;

/// The direction a texel of cube `face` looks at; `s` and `t` run 0 to 1 across the face (left to
/// right, top to bottom in the image). Standard cube-map addressing.
fn face_direction(face: usize, s: f32, t: f32) -> Vec3 {
    let (sc, tc) = (2.0 * s - 1.0, 2.0 * t - 1.0);
    let d = match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    };
    d.normalize()
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The sky colour looking along unit `dir` (+z is up).
fn sky_color(sky: &SkyGradient, dir: Vec3) -> Vec3 {
    let [horizon, zenith, ground] = [sky.horizon, sky.zenith, sky.ground].map(Vec3::from);
    if dir.z >= 0.0 {
        return horizon.lerp(zenith, dir.z.powf(0.6));
    }
    horizon.lerp(ground, smoothstep(0.0, 0.35, -dir.z))
}

/// The six RGBA8 faces of `sky`, `size` by `size` texels each, concatenated face after face.
fn sky_faces(sky: &SkyGradient, size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((FACES * size * size * 4) as usize);
    for face in 0..FACES as usize {
        for y in 0..size {
            for x in 0..size {
                let (s, t) = ((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
                let rgb = sky_color(sky, face_direction(face, s, t));
                data.extend_from_slice(&rgb.to_array().map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8));
                data.push(255);
            }
        }
    }
    data
}

/// The clamped, linear sampler native's environment cube map uses.
fn sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..ImageSamplerDescriptor::default()
    })
}

/// The environment cube map image for `environment`, or `None` if `Environment::Faces` gave too
/// little data (logged, like native's `set_environment_faces`).
pub fn build(environment: Environment<'_>) -> Option<Image> {
    let (size, data) = match environment {
        Environment::Sky(sky) => (SKY_FACE_SIZE, sky_faces(&sky, SKY_FACE_SIZE)),
        Environment::Faces { size, faces } => {
            let needed = (size * size * 4) as usize;
            for (face, bytes) in faces.iter().enumerate() {
                if bytes.len() < needed {
                    log::warn!(
                        "bevy renderer: environment face {face} is {} bytes, expected {needed}; keeping the old environment",
                        bytes.len()
                    );
                    return None;
                }
            }
            (size, faces.iter().flat_map(|f| f[..needed].iter().copied()).collect())
        }
    };
    let extent = Extent3d { width: size, height: size, depth_or_array_layers: FACES };
    let mut image =
        Image::new_uninit(extent, TextureDimension::D2, TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD);
    image.data = Some(data);
    image.texture_descriptor.label = None;
    image.texture_view_descriptor =
        Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..Default::default() });
    image.sampler = sampler();
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_centres_look_along_the_axes() {
        let axes = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
        for (face, axis) in axes.into_iter().enumerate() {
            assert!((face_direction(face, 0.5, 0.5) - axis).length() < 1e-6, "face {face}");
        }
    }

    #[test]
    fn gradient_runs_from_ground_through_horizon_to_zenith() {
        let sky = SkyGradient::default();
        assert_eq!(sky_color(&sky, Vec3::Z), Vec3::from(sky.zenith));
        assert_eq!(sky_color(&sky, Vec3::X), Vec3::from(sky.horizon));
        assert_eq!(sky_color(&sky, Vec3::NEG_Z), Vec3::from(sky.ground));
    }

    #[test]
    fn a_sky_builds_a_six_layer_cube_image() {
        let image = build(Environment::Sky(SkyGradient::default())).unwrap();
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, FACES);
        assert_eq!(image.data.as_ref().unwrap().len(), (FACES * SKY_FACE_SIZE * SKY_FACE_SIZE * 4) as usize);
        assert_eq!(image.texture_view_descriptor.unwrap().dimension, Some(TextureViewDimension::Cube));
    }

    #[test]
    fn faces_too_short_are_rejected() {
        let short = [0u8; 4];
        let faces = [short.as_slice(); 6];
        assert!(build(Environment::Faces { size: 2, faces }).is_none());
    }

    #[test]
    fn faces_big_enough_are_concatenated_in_order() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        let faces = [a.as_slice(), b.as_slice(), a.as_slice(), a.as_slice(), a.as_slice(), a.as_slice()];
        let image = build(Environment::Faces { size: 2, faces }).unwrap();
        let data = image.data.unwrap();
        assert_eq!(&data[0..16], &a);
        assert_eq!(&data[16..32], &b);
    }
}
