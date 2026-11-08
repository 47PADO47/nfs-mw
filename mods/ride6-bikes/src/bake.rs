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

use crate::dash;
use crate::glass;
use crate::grade::{
    DEFAULT_AO_FLOOR, DEFAULT_GAIN, DEFAULT_REFLECTED, Grade, MAX_GAIN, PAINT_AO_FLOOR, lift_colour, metal_floor,
};
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
    /// The lift of the base colour (1 to 2, 1 is none), see [`lift`](crate::grade::lift).
    pub gain: f32,
    /// The environment radiance a dielectric reflects, baked into its colour (0 to 1, 0 is none), see [`DEFAULT_REFLECTED`](crate::grade::DEFAULT_REFLECTED).
    pub reflected: f32,
    /// Also write normal and specular textures and tangent vertices for the tangent-space surface shader.
    pub normal_maps: bool,
    /// Invert the green channel of the normal maps (Unreal's convention against the shader's).
    pub flip_green: bool,
    /// The stock parts written as solids of their own so that the shop can swap them (`docs/modding/ride6-bike-upgrades.md`): bit
    /// `n` is `PIECES[n]` of [`crate::build_pieces`] (the exhaust, the bar pad, the rims, the brake discs); 0, none: they are
    /// part of the body, bars and wheel solids.
    pub split_parts: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_texture: 2048,
            ao_floor: DEFAULT_AO_FLOOR,
            gain: DEFAULT_GAIN,
            reflected: DEFAULT_REFLECTED,
            normal_maps: false,
            flip_green: false,
            split_parts: 0,
        }
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
    /// The base colour picture (file stem) of the livery surfaces of a bike that has liveries to offer: its diffuse texture is the
    /// car's paint placeholder `<CAR>_SKIN1`, which the game draws a chosen livery over ([`Surfaces::with_paint`]).
    paint: Option<String>,
    /// The live screen of the bike's instrument cluster, when its picture has a display ([`crate::dash::Screen`]): the first baked.
    pub dash: Option<dash::Screen>,
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

pub(crate) fn linear_to_srgb(linear: f32) -> u8 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        return (v * 12.92 * 255.0 + 0.5) as u8;
    }
    ((1.055 * v.powf(1.0 / 2.4) - 0.055) * 255.0 + 0.5) as u8
}

/// The base colour with the occlusion applied in linear light, lifted by `gain` and the opacity cut at `clip` (alpha 0 or
/// 255), or opaque when `clip` is `None`. The packed picture is sampled by texture coordinate, so it may have another size.
pub fn bake_diffuse(base: Level, pbr: Option<&Picture>, occlusion: bool, grade: Grade, clip: Option<f32>) -> Level {
    let Grade { ao_floor, gain, reflected } = grade;
    let Level { width, height, mut rgba } = base;
    let threshold = clip.map(|c| (c * 255.0).round() as u8);
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            if let Some(pbr) = pbr {
                let uv = [(x as f32 + 0.5) / width as f32, (y as f32 + 0.5) / height as f32];
                let packed = pbr.sample(uv);
                let ao = if occlusion { f32::from(packed[3]) / 255.0 } else { 1.0 };
                let metal = f32::from(packed[2]) / 255.0;
                let floor = metal_floor(ao_floor, metal);
                let factor = floor + (1.0 - floor) * ao;
                // The reflection of a dielectric: `F0 = 0.08 * specular` of the environment, none for a metal.
                let reflection = (1.0 - metal) * 0.08 * (f32::from(packed[0]) / 255.0) * reflected;
                let linear = [0, 1, 2].map(|c| srgb_to_linear(rgba[at + c]));
                // A veil of one grey on every texel washes a saturated paint out (a red of (184, 0, 26) turned pink), and what
                // the veil stands for, the reflection of the sky, is white where the paint is white and the paint's own tint
                // where it is coloured: it is therefore applied in proportion to how little saturated the texel is.
                let reflection = reflection * (1.0 - saturation(linear));
                for c in 0..3 {
                    rgba[at + c] = linear_to_srgb(linear[c] * factor + reflection);
                }
            }
            let lifted = lift_colour([rgba[at], rgba[at + 1], rgba[at + 2]], gain);
            rgba[at..at + 3].copy_from_slice(&lifted);
            rgba[at + 3] = match threshold {
                Some(t) if rgba[at + 3] >= t => 255,
                Some(_) => 0,
                None => 255,
            };
        }
    }
    Level { width, height, rgba }
}

/// The saturation of a linear colour, `(max - min) / max` (0 for black and for greys, 1 for a pure hue).
fn saturation(linear: [f32; 3]) -> f32 {
    let max = linear.iter().copied().fold(0.0, f32::max);
    if max <= 1e-4 {
        return 0.0;
    }
    let min = linear.iter().copied().fold(f32::MAX, f32::min);
    (max - min) / max
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
        Self { store, palette: Palette::default(), options, cache: HashMap::new(), paint: None, dash: None }
    }

    /// Name the diffuse texture of every surface whose base colour picture is `stem` `<CAR>_SKIN1`, the placeholder the game
    /// repaints for the livery a player picks (`docs/modding/car-customization.md#liveries`). The picture itself is baked into
    /// that texture, so the car wears its stock livery whether or not the game draws another over it.
    pub fn with_paint(mut self, stem: Option<String>) -> Self {
        self.paint = stem;
        self
    }

    /// How the base colour of `material` is graded: the options, with the occlusion of a livery (paint) at least [`PAINT_AO_FLOOR`].
    fn grade(&self, material: &Material) -> Grade {
        let name = material.name.to_ascii_lowercase();
        let ao_floor =
            if name.contains("livery") { self.options.ao_floor.max(PAINT_AO_FLOOR) } else { self.options.ao_floor };
        // A cluster's screen is emissive in RIDE 6 and the car shader lights it like paint (about half its brightness): the
        // strongest lift the grade allows. A choice, made against the RC 390 and Panigale V4 screens in the helmet camera.
        let gain = if name.starts_with("mat_gauge") { MAX_GAIN } else { self.options.gain };
        Grade { ao_floor, gain, reflected: self.options.reflected }
    }

    /// The texture of a livery picture: the base colour of `material` graded like a body texture, opaque, one level (the game
    /// reads the first level only), named `name`. Returns false when the material has no picture to bake.
    pub fn add_livery_picture(&mut self, name: &str, material: &Material) -> Result<bool> {
        let Some(base_stem) = &material.base_color else { return Ok(false) };
        let Some(base) = self.store.picture(base_stem)? else { return Ok(false) };
        let pbr = match &material.pbr {
            Some(stem) => self.store.picture(stem)?,
            None => None,
        };
        let cap = size_cap(Kind::Body, self.options.max_texture);
        let level = Picture { width: base.width, height: base.height, rgba: base.rgba.clone() }.shrunk(cap);
        let level = bake_diffuse(level, pbr.as_deref(), material.pbr_has_occlusion, self.grade(material), None);
        if !self.palette.contains(name) {
            self.add_level_mips(name, level, PixelFormat::Dxt1, Alpha::Opaque, 1);
        }
        Ok(true)
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
        let diffuse = match &self.paint {
            Some(paint) if paint.eq_ignore_ascii_case(base_stem) => format!("{}_SKIN1", car.to_ascii_uppercase()),
            _ => format!("{car}_{}", upper(base_stem)),
        };
        let clip = material.cutout();
        let mut picture = Picture { width: base.width, height: base.height, rgba: base.rgba.clone() };
        // The display of a cluster, found in the static picture before the widgets are composed over it: a cluster that has one is
        // stored as 32-bit colour at `dash::LIVE_SIZE` so the host can paint the speed, gear and revs into it.
        let mut live = None;
        if material.name.to_ascii_lowercase().starts_with("mat_gauge") {
            live = dash::find_window(&picture).map(|window| (window, (picture.width, picture.height)));
            // The screen of an instrument cluster is drawn by widgets at run time: take the exporter's composition of them at rest,
            // or else draw a small panel into the black window of the static picture.
            match dash::composed(&mut self.store, base_stem)? {
                Some(screen) => picture = screen,
                None => {
                    dash::fill_screen(&mut picture);
                }
            }
        }
        let level = picture.shrunk(if live.is_some() { cap.min(dash::LIVE_SIZE) } else { cap });
        let grade = self.grade(material);
        let level = bake_diffuse(level, pbr.as_deref(), material.pbr_has_occlusion, grade, clip);
        let cut_out = level.rgba.as_chunks::<4>().0.iter().any(|t| t[3] < 255);
        if !self.palette.contains(&diffuse) {
            if let Some((window, from)) = live {
                let screen = dash::Screen::new(&diffuse, window, from, (level.width, level.height), grade.gain);
                self.dash.get_or_insert(screen);
                self.add_level_mips(&diffuse, level.clone(), PixelFormat::Argb8888, Alpha::Opaque, 1);
            } else {
                let (format, alpha) =
                    if cut_out { (PixelFormat::Dxt5, Alpha::CutOut) } else { (PixelFormat::Dxt1, Alpha::Opaque) };
                self.add_level(&diffuse, level.clone(), format, alpha);
            }
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
        let ao_floor = (material.pbr_has_occlusion && pbr.is_some()).then_some(self.grade(material).ao_floor);
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
        self.add_level_mips(name, level, format, alpha, mips);
    }

    fn add_level_mips(&mut self, name: &str, level: Level, format: PixelFormat, alpha: Alpha, mips: u32) {
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
#[path = "bake_tests.rs"]
mod tests;
