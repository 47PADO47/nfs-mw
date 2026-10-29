//! Baking a RIDE 6 material into 2005 car textures.
//!
//! The base colour picture is the diffuse texture, darkened by the occlusion channel of the packed picture (the car shader
//! has no ambient term, so the occlusion an Unreal renderer applies to indirect light is applied to the colour instead,
//! down to a floor). With normal maps on, the same material also gets a normal texture and a specular texture for the
//! tangent-space surface shader. See [the material model](../../../docs/formats/ride6-materials.md).

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;
use blackbox_tpk::{AlphaUsage, Level, PixelFormat, TextureSource, full_mip_count};

use crate::glass;
use crate::partfile::{Kind, Material};
use crate::textures::{Palette, Picture, TexStore};

/// Bit of the texture info `Flags` byte that makes a texture's surfaces two-sided.
const TWO_SIDED: u8 = 0x02;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// The longest side of the body's textures, pixels; the wheels' are half of it and the discs' a quarter.
    pub max_texture: u32,
    /// The occlusion factor of a fully occluded texel (0 to 1): the colour is scaled by `floor + (1 - floor) * occlusion`.
    pub ao_floor: f32,
    /// Also write normal and specular textures and tangent vertices for the tangent-space surface shader.
    pub normal_maps: bool,
    /// Invert the green channel of the normal maps (Unreal's convention against the shader's).
    pub flip_green: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_texture: 2048, ao_floor: 0.3, normal_maps: false, flip_green: false }
    }
}

/// A material as the car files hold it.
pub struct Surface {
    /// Texture names in the car's pack.
    pub diffuse: String,
    pub normal: Option<String>,
    pub specular: Option<String>,
    /// The packed specular, roughness, metallic and occlusion picture, for classifying triangles.
    pub pbr: Option<Rc<Picture>>,
    /// The base colour picture as the exporter wrote it (before the occlusion is baked in), for classifying triangles.
    pub base: Option<Rc<Picture>>,
    /// The floor of the occlusion factor baked into the diffuse (see [`Options::ao_floor`]); `None` when the diffuse carries
    /// no occlusion.
    pub ao_floor: Option<f32>,
    /// For a translucent material: the transmittance picture, from which triangles take their glass light material.
    pub glass: Option<Glass>,
}

/// The transmittance of a translucent material.
#[derive(Clone)]
pub struct Glass {
    pub picture: Rc<Picture>,
    /// `Modulation - Transmittance - Max`.
    pub max: f32,
}

/// How a baked texture's alpha is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alpha {
    Opaque,
    /// Cut out at half (punch-through), two-sided.
    CutOut,
    /// Smooth, alpha-blended and sorted like the stock windows.
    Blended,
}

pub struct Surfaces {
    store: TexStore,
    pub palette: Palette,
    options: Options,
    cache: HashMap<(String, Kind), Rc<Surface>>,
}

/// The longest side a texture of a part of `kind` may have.
pub fn size_cap(kind: Kind, max_texture: u32) -> u32 {
    let divisor = match kind {
        Kind::Body => 1,
        Kind::Tyre | Kind::Rim => 2,
        Kind::Brake => 4,
    };
    (max_texture / divisor).max(4)
}

pub(crate) fn srgb_to_linear(byte: u8) -> f32 {
    let v = f32::from(byte) / 255.0;
    if v <= 0.040_45 {
        return v / 12.92;
    }
    ((v + 0.055) / 1.055).powf(2.4)
}

fn linear_to_srgb(linear: f32) -> u8 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        return (v * 12.92 * 255.0 + 0.5) as u8;
    }
    ((1.055 * v.powf(1.0 / 2.4) - 0.055) * 255.0 + 0.5) as u8
}

/// The base colour with the occlusion applied in linear light and the opacity cut at `clip` (alpha 0 or 255), or
/// opaque when `clip` is `None`. The packed picture is sampled by texture coordinate, so it may have another size.
pub fn bake_diffuse(base: Level, pbr: Option<&Picture>, occlusion: bool, ao_floor: f32, clip: Option<f32>) -> Level {
    let Level { width, height, mut rgba } = base;
    let threshold = clip.map(|c| (c * 255.0).round() as u8);
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            if occlusion && let Some(pbr) = pbr {
                let uv = [(x as f32 + 0.5) / width as f32, (y as f32 + 0.5) / height as f32];
                let ao = f32::from(pbr.sample(uv)[3]) / 255.0;
                let factor = ao_floor + (1.0 - ao_floor) * ao;
                for c in 0..3 {
                    rgba[at + c] = linear_to_srgb(srgb_to_linear(rgba[at + c]) * factor);
                }
            }
            rgba[at + 3] = match threshold {
                Some(t) if rgba[at + 3] >= t => 255,
                Some(_) => 0,
                None => 255,
            };
        }
    }
    Level { width, height, rgba }
}

/// The specular colour for the tangent-space shader: the base colour for metal and a grey for the rest, scaled by how
/// smooth the surface is (`(1 - roughness)^2`).
pub fn bake_specular(like: &Level, base: &Picture, pbr: &Picture) -> Level {
    let mut rgba = Vec::with_capacity(like.rgba.len());
    for y in 0..like.height {
        for x in 0..like.width {
            let uv = [(x as f32 + 0.5) / like.width as f32, (y as f32 + 0.5) / like.height as f32];
            let packed = pbr.sample(uv);
            let (rough, metal) = (f32::from(packed[1]) / 255.0, f32::from(packed[2]) / 255.0);
            let smooth = (1.0 - rough) * (1.0 - rough);
            let colour = base.sample(uv);
            for &channel in &colour[..3] {
                let tint = 0.5 + (f32::from(channel) / 255.0 - 0.5) * metal;
                rgba.push((smooth * tint * 255.0 + 0.5) as u8);
            }
            rgba.push(255);
        }
    }
    Level { width: like.width, height: like.height, rgba }
}

/// A normal map as the shader reads it: the picture, its alpha made opaque, and the green channel inverted on request.
pub fn bake_normal(picture: Picture, limit: u32, flip_green: bool) -> Level {
    let mut level = picture.shrunk(limit);
    for texel in level.rgba.as_chunks_mut::<4>().0.iter_mut() {
        if flip_green {
            texel[1] = 255 - texel[1];
        }
        texel[3] = 255;
    }
    level
}

fn upper(stem: &str) -> String {
    stem.to_ascii_uppercase()
}

impl Surfaces {
    pub fn new(store: TexStore, options: Options) -> Self {
        Self { store, palette: Palette::default(), options, cache: HashMap::new() }
    }

    /// The textures of `material` for a part of `kind`, baking them the first time.
    pub fn surface(&mut self, car: &str, material: &Material, kind: Kind) -> Result<Rc<Surface>> {
        let key = (material.name.to_ascii_lowercase(), kind);
        if let Some(done) = self.cache.get(&key) {
            return Ok(done.clone());
        }
        let made = Rc::new(self.bake(car, material, kind)?);
        self.cache.insert(key, made.clone());
        Ok(made)
    }

    fn bake(&mut self, car: &str, material: &Material, kind: Kind) -> Result<Surface> {
        let cap = size_cap(kind, self.options.max_texture);
        let pbr = match &material.pbr {
            Some(stem) => self.store.picture(stem)?,
            None => None,
        };
        let base = match &material.base_color {
            Some(stem) => self.store.picture(stem)?,
            None => None,
        };
        if material.is_translucent()
            && let Some(surface) = self.bake_glass(car, material, cap)?
        {
            return Ok(surface);
        }
        let (Some(base_stem), Some(base)) = (&material.base_color, base) else {
            let grey = self.palette.add_colour([128, 128, 128]);
            return Ok(Surface {
                diffuse: grey,
                normal: None,
                specular: None,
                pbr,
                base: None,
                ao_floor: None,
                glass: None,
            });
        };
        let diffuse = format!("{car}_{}", upper(base_stem));
        let clip = material.cutout();
        let level = Picture { width: base.width, height: base.height, rgba: base.rgba.clone() }.shrunk(cap);
        let level = bake_diffuse(level, pbr.as_deref(), material.pbr_has_occlusion, self.options.ao_floor, clip);
        let cut_out = level.rgba.as_chunks::<4>().0.iter().any(|t| t[3] < 255);
        if !self.palette.contains(&diffuse) {
            let (format, alpha) =
                if cut_out { (PixelFormat::Dxt5, Alpha::CutOut) } else { (PixelFormat::Dxt1, Alpha::Opaque) };
            self.add_level(&diffuse, level.clone(), format, alpha);
        }
        let (mut normal, mut specular) = (None, None);
        if self.options.normal_maps {
            if let Some(stem) = &material.normal
                && let Some(picture) = self.store.picture(stem)?
            {
                let name = format!("{car}_{}", upper(stem));
                let copy = Picture { width: picture.width, height: picture.height, rgba: picture.rgba.clone() };
                if !self.palette.contains(&name) {
                    let level = bake_normal(copy, cap, self.options.flip_green);
                    self.add_level(&name, level, PixelFormat::Dxt1, Alpha::Opaque);
                }
                normal = Some(name);
            }
            if let Some(pbr) = &pbr {
                let name = format!("{car}_{}_S", upper(base_stem));
                if !self.palette.contains(&name) {
                    let level = bake_specular(&level, &base, pbr);
                    self.add_level(&name, level, PixelFormat::Dxt1, Alpha::Opaque);
                }
                specular = Some(name);
            }
        }
        let ao_floor = (material.pbr_has_occlusion && pbr.is_some()).then_some(self.options.ao_floor);
        Ok(Surface { diffuse, normal, specular, pbr, base: Some(base), ao_floor, glass: None })
    }

    /// The glass texture of a translucent material, or `None` when it has no transmittance picture (it is then baked as an
    /// ordinary surface).
    fn bake_glass(&mut self, car: &str, material: &Material, cap: u32) -> Result<Option<Surface>> {
        let Some(stem) = material.transmittance_picture() else { return Ok(None) };
        let Some(picture) = self.store.picture(stem)? else { return Ok(None) };
        let max = material.transmittance_max.unwrap_or(1.0);
        let diffuse = format!("{car}_{}", upper(stem));
        if !self.palette.contains(&diffuse) {
            let copy = Picture { width: picture.width, height: picture.height, rgba: picture.rgba.clone() };
            let level = glass::bake(copy.shrunk(cap), max);
            self.add_level(&diffuse, level, PixelFormat::Dxt5, Alpha::Blended);
        }
        let glass = Some(Glass { picture, max });
        Ok(Some(Surface { diffuse, normal: None, specular: None, pbr: None, base: None, ao_floor: None, glass }))
    }

    fn add_level(&mut self, name: &str, level: Level, format: PixelFormat, alpha: Alpha) {
        let mips = full_mip_count(level.width, level.height);
        let Some(mut source) = TextureSource::from_rgba(name, level, format, mips) else { return };
        match alpha {
            Alpha::Opaque => {}
            Alpha::CutOut => {
                source.alpha_usage = AlphaUsage::PunchThrough;
                source.flags |= TWO_SIDED;
            }
            Alpha::Blended => {
                source.alpha_usage = AlphaUsage::Modulated;
                source.alpha_sorting = true;
                source.alpha_blend = 1;
            }
        }
        self.palette.add(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(width: u32, height: u32, texel: [u8; 4]) -> Level {
        Level { width, height, rgba: texel.repeat((width * height) as usize) }
    }

    #[test]
    fn occlusion_darkens_down_to_the_floor_and_no_occlusion_keeps_the_colour() {
        let pbr = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 0] };
        let dark = bake_diffuse(level(2, 2, [200, 200, 200, 255]), Some(&pbr), true, 0.3, None);
        // Fully occluded: 0.3 of the linear colour.
        let expected = linear_to_srgb(srgb_to_linear(200) * 0.3);
        assert_eq!(dark.rgba[0], expected);
        assert!(expected < 200 && expected > 60);
        let kept = bake_diffuse(level(2, 2, [200, 200, 200, 255]), Some(&pbr), false, 0.3, None);
        assert_eq!(kept.rgba[0], 200);
        let open = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 255] };
        let lit = bake_diffuse(level(2, 2, [200, 200, 200, 255]), Some(&open), true, 0.3, None);
        assert_eq!(lit.rgba[0], 200, "an unoccluded texel keeps its colour");
    }

    #[test]
    fn the_opacity_is_cut_at_the_clip_value() {
        let mut base = level(2, 1, [10, 20, 30, 0]);
        base.rgba[3] = 200;
        base.rgba[7] = 40; // 40 / 255 is below the clip of 1/3
        let cut = bake_diffuse(base, None, false, 0.3, Some(0.3333));
        assert_eq!((cut.rgba[3], cut.rgba[7]), (255, 0));
        let opaque = bake_diffuse(level(1, 1, [1, 2, 3, 9]), None, false, 0.3, None);
        assert_eq!(opaque.rgba[3], 255, "no clip value means opaque");
    }

    #[test]
    fn srgb_round_trips() {
        for v in [0u8, 1, 17, 128, 200, 255] {
            assert_eq!(linear_to_srgb(srgb_to_linear(v)), v);
        }
    }

    #[test]
    fn the_specular_is_bright_for_smooth_metal_and_dark_for_rough_dielectric() {
        let like = level(1, 1, [0; 4]);
        let base = Picture { width: 1, height: 1, rgba: vec![255, 255, 255, 255] };
        let smooth_metal = Picture { width: 1, height: 1, rgba: vec![128, 0, 255, 255] };
        let rough_plastic = Picture { width: 1, height: 1, rgba: vec![128, 255, 0, 255] };
        assert_eq!(bake_specular(&like, &base, &smooth_metal).rgba[0], 255);
        assert_eq!(bake_specular(&like, &base, &rough_plastic).rgba[0], 0);
    }

    #[test]
    fn a_normal_map_can_have_its_green_flipped() {
        let picture = Picture { width: 1, height: 1, rgba: vec![10, 20, 250, 7] };
        let flipped = bake_normal(picture, 8, true);
        assert_eq!(flipped.rgba, vec![10, 235, 250, 255]);
    }

    fn write_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(rgba).unwrap();
    }

    #[test]
    fn a_translucent_material_bakes_a_blended_texture_with_the_opacity_in_the_alpha() {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-glass-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Two texels of a 8 x 4 picture: clear on the left half, a red lens on the right.
        let texel = |x: u32| if x < 4 { [255, 255, 255, 0] } else { [173, 0, 5, 36] };
        let rgba: Vec<u8> = (0..32).flat_map(|i| texel(i % 8)).collect();
        write_png(&dir.join("glass_d.png"), 8, 4, &rgba);
        let material = Material {
            name: "mat_glass".into(),
            blend_mode: Some("BLEND_TranslucentGreyTransmittance".into()),
            base_color: Some("glass_d".into()),
            transmittance: Some("glass_d".into()),
            transmittance_max: Some(0.95),
            ..Material::default()
        };
        let mut surfaces = Surfaces::new(TexStore::open(&dir).unwrap(), Options::default());
        let surface = surfaces.surface("CAR", &material, Kind::Body).unwrap();
        assert_eq!(surface.diffuse, "CAR_GLASS_D");
        assert!(surface.normal.is_none() && surface.pbr.is_none());
        assert!((surface.glass.as_ref().unwrap().max - 0.95).abs() < 1e-6);
        let textures = std::mem::take(&mut surfaces.palette).into_textures();
        std::fs::remove_dir_all(&dir).unwrap();
        let [glass] = textures.as_slice() else { panic!("{} textures", textures.len()) };
        assert_eq!(glass.format, PixelFormat::Dxt5);
        assert!(glass.alpha_sorting && glass.alpha_blend == 1);
        assert_eq!(glass.alpha_usage, AlphaUsage::Modulated);
        assert_eq!(glass.flags & TWO_SIDED, 0, "a blended surface is drawn one-sided like the stock windows");
    }

    #[test]
    fn the_size_cap_follows_the_part() {
        assert_eq!(size_cap(Kind::Body, 2048), 2048);
        assert_eq!(size_cap(Kind::Rim, 2048), 1024);
        assert_eq!(size_cap(Kind::Brake, 2048), 512);
        assert_eq!(size_cap(Kind::Brake, 8), 4);
    }
}
