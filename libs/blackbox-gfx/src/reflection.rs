//! Local environment captures, in the same game axes as glossy reflection sampling.

use glam::Vec3;

use crate::{FrameParams, Instance, Projection};

/// Capture resolution. All six faces are refreshed together; no temporal accumulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReflectionQuality {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ReflectionQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub const fn size(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 64,
            Self::Medium => 128,
            Self::High => 256,
        }
    }
}

impl std::str::FromStr for ReflectionQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|q| q.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected off, low, medium or high, got {s:?}"))
    }
}

impl std::fmt::Display for ReflectionQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Smallest cube face the port captures, in pixels.
pub const CUBE_FACE_MIN: u32 = 16;
/// Largest cube face the port captures, in pixels.
pub const CUBE_FACE_MAX: u32 = 1024;
/// Largest planar target edge, in pixels: the `max_texture_dimension_2d` every wgpu downlevel device offers.
/// Pass it as the device limit when the renderer does not query its own.
pub const PLANAR_MAX: u32 = 2048;
/// Smallest planar target height, in pixels.
pub const PLANAR_MIN_HEIGHT: u32 = 64;
/// The automatic mirror height is the output height divided by this.
pub const AUTO_MIRROR_DIVISOR: u32 = 6;
/// Smallest and largest scale factor of a reflection size: 25 to 400 percent.
pub const MIN_SCALE: f32 = 0.25;
pub const MAX_SCALE: f32 = 4.0;

/// How big one reflection target is. `Auto` follows the output, `Fixed` is in pixels and `Scale` multiplies the
/// target's base size (for the mirror, the base height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReflectionSize {
    /// The cube face is the output height. The mirror is the output height over [`AUTO_MIRROR_DIVISOR`].
    Auto,
    /// Pixels: the cube face, or the mirror picture's height (its width follows the bezel's aspect).
    Fixed(u32),
    /// A factor of the base size; 1.0 is the base.
    Scale(f32),
}

impl Default for ReflectionSize {
    fn default() -> Self {
        Self::Scale(1.0)
    }
}

impl ReflectionSize {
    /// The scale factor clamped into [`MIN_SCALE`]..=[`MAX_SCALE`]; non-finite factors become 1.0.
    pub fn sanitized(self) -> Self {
        match self {
            Self::Scale(factor) => Self::Scale(clamp_scale(factor)),
            other => other,
        }
    }
}

impl std::str::FromStr for ReflectionSize {
    type Err = String;

    /// `auto`, a percentage such as `150%` (25% to 400%), or a size in pixels such as `256`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim().to_ascii_lowercase();
        let bad = || format!("expected auto, a percentage from 25% to 400%, or pixels, got {s:?}");
        if text == "auto" {
            return Ok(Self::Auto);
        }
        let Some(percent) = text.strip_suffix('%') else {
            return text.parse::<u32>().map(Self::Fixed).map_err(|_| bad());
        };
        let percent: f32 = percent.trim().parse().map_err(|_| bad())?;
        if !(MIN_SCALE * 100.0..=MAX_SCALE * 100.0).contains(&percent) {
            return Err(bad());
        }
        Ok(Self::Scale(percent / 100.0))
    }
}

impl std::fmt::Display for ReflectionSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Fixed(px) => write!(f, "{px}"),
            Self::Scale(factor) => write!(f, "{}%", (factor * 100.0).round() as u32),
        }
    }
}

/// The sizes of the environment captures and the rear-view mirror, and the power-of-two rule for cube faces.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReflectionResolution {
    /// Size of each car cube face. The quality preset gives the base.
    pub cube: ReflectionSize,
    /// Size of the rear-view mirror picture. The bezel gives the base.
    pub mirror: ReflectionSize,
    /// Round each cube face down to a power of two. Planar targets are not rounded.
    pub pow2: bool,
}

impl ReflectionResolution {
    /// The scale factors clamped into range.
    pub fn sanitized(self) -> Self {
        Self { cube: self.cube.sanitized(), mirror: self.mirror.sanitized(), ..self }
    }

    /// The edge of one cube face in pixels. `base` is the quality preset's face size, `output_height` the output's
    /// height and `max_dimension` the largest texture edge the device takes.
    pub fn cube_face(self, base: u32, output_height: u32, max_dimension: u32) -> u32 {
        let face = clamp_round(
            wanted(self.cube, base as f32, output_height as f32),
            CUBE_FACE_MIN,
            CUBE_FACE_MAX.min(max_dimension),
        );
        if self.pow2 {
            return floor_pow2(face);
        }
        face
    }

    /// The mirror picture's size in pixels as `[width, height]`. `base` is the bezel's proportions and `output`
    /// the output size. The width always follows `base`'s aspect, so the bezel's shape does not change.
    pub fn mirror_size(self, base: [u32; 2], output: [u32; 2], max_dimension: u32) -> [u32; 2] {
        let aspect = base[0].max(1) as f32 / base[1].max(1) as f32;
        let auto = output[1] as f32 / AUTO_MIRROR_DIVISOR as f32;
        let longest = max_dimension.min(PLANAR_MAX);
        let tallest = (longest as f32 / aspect).floor() as u32;
        let height = clamp_round(wanted(self.mirror, base[1] as f32, auto), PLANAR_MIN_HEIGHT, tallest);
        let width = clamp_round(height as f32 * aspect, 1, longest);
        [width, height]
    }
}

/// The colour the rear-view mirror picture is multiplied by, per channel. [`MirrorTint::NONE`] leaves it as captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorTint(pub [u8; 3]);

impl MirrorTint {
    /// White: every channel is multiplied by one.
    pub const NONE: Self = Self([255; 3]);
    /// The stock look of the mirror tint: 150 per channel, as the sniten mod sets it. The original's own tint is
    /// not confirmed (docs/specs/hd-reflections.md, section 10).
    pub const STOCK: Self = Self([150; 3]);

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

impl Default for MirrorTint {
    fn default() -> Self {
        Self::NONE
    }
}

impl std::str::FromStr for MirrorTint {
    type Err = String;

    /// `off`, `on` (the stock tint), `#rrggbb`, or three channels `r g b` or `r,g,b` from 0 to 255.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim().to_ascii_lowercase();
        let bad = || format!("expected off, on, #rrggbb or three channels from 0 to 255, got {s:?}");
        if text == "off" {
            return Ok(Self::NONE);
        }
        if text == "on" {
            return Ok(Self::STOCK);
        }
        if let Some(hex) = text.strip_prefix('#') {
            let value = u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6).ok_or_else(bad)?;
            return Ok(Self([(value >> 16) as u8, (value >> 8) as u8, value as u8]));
        }
        let parts: Vec<&str> = text.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()).collect();
        match parts.as_slice() {
            [r, g, b] => {
                Ok(Self([r.parse().map_err(|_| bad())?, g.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?]))
            }
            _ => Err(bad()),
        }
    }
}

impl std::fmt::Display for MirrorTint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            return f.write_str("off");
        }
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

/// Multiplies the colour channels of a packed RGBA picture by `tint`. Alpha is left as it is.
pub fn tint_rgba(rgba: &mut [u8], tint: MirrorTint) {
    if tint.is_none() {
        return;
    }
    for pixel in rgba.as_chunks_mut::<4>().0 {
        for (channel, factor) in pixel[..3].iter_mut().zip(tint.0) {
            *channel = ((u16::from(*channel) * u16::from(factor) + 127) / 255) as u8;
        }
    }
}

/// The largest power of two not above `value`; zero stays zero.
pub fn floor_pow2(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    1 << (u32::BITS - 1 - value.leading_zeros())
}

fn clamp_scale(factor: f32) -> f32 {
    if !factor.is_finite() {
        return 1.0;
    }
    factor.clamp(MIN_SCALE, MAX_SCALE)
}

/// The size before clamping, in pixels: `auto` for [`ReflectionSize::Auto`], the pixel count, or the base times the
/// factor.
fn wanted(size: ReflectionSize, base: f32, auto: f32) -> f32 {
    match size {
        ReflectionSize::Auto => auto,
        ReflectionSize::Fixed(px) => px as f32,
        ReflectionSize::Scale(factor) => base * clamp_scale(factor),
    }
}

/// Rounds `value` to whole pixels and clamps it into `low..=high` (`high` is raised to `low` if it is below).
fn clamp_round(value: f32, low: u32, high: u32) -> u32 {
    let high = high.max(low);
    value.round().clamp(low as f32, high as f32) as u32
}

/// One surroundings snapshot. The caller supplies scenery outside the main view too, and excludes
/// the reflective object itself. Effects and UI are deliberately absent from this pass.
pub struct ReflectionScene<'a> {
    pub origin: Vec3,
    pub quality: ReflectionQuality,
    pub frame: FrameParams,
    /// Frustum/LOD-selected scenery for each face, including objects behind the main camera.
    pub faces: [&'a [Instance]; 6],
}

/// RH face cameras (+X, -X, +Y, -Y, +Z, -Z). A capture must flip the image vertically
/// when storing it: standard cube addressing has the opposite handedness to a RH camera.
pub fn face_frame(origin: Vec3, face: usize, template: &FrameParams) -> FrameParams {
    let directions = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let ups = [Vec3::NEG_Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z, Vec3::NEG_Y, Vec3::NEG_Y];
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(origin, origin + directions[face], ups[face]),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 0.25,
        },
        camera_position: origin,
        camera_cut: true,
        ..*template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_cameras_agree_with_cube_texel_addressing() {
        let frame = FrameParams {
            view: glam::Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
        };
        for (face, direction) in
            [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z].into_iter().enumerate()
        {
            let f = face_frame(Vec3::new(4.0, 8.0, 2.0), face, &frame);
            let camera = f.view.inverse();
            assert!(camera.transform_vector3(Vec3::NEG_Z).abs_diff_eq(direction, 1e-6));
            // Cube sampling at (s=.75,t=.25) must project to that pixel on every face.
            let expected = match face {
                0 => Vec3::new(1.0, 0.5, -0.5),
                1 => Vec3::new(-1.0, 0.5, 0.5),
                2 => Vec3::new(0.5, 1.0, -0.5),
                3 => Vec3::new(0.5, -1.0, 0.5),
                4 => Vec3::new(0.5, 0.5, 1.0),
                _ => Vec3::new(-0.5, 0.5, -1.0),
            };
            let clip = f.view_proj() * (f.camera_position + expected).extend(1.0);
            assert!((clip.x / clip.w - 0.5).abs() < 1e-5, "face {face}");
            assert!((clip.y / clip.w + 0.5).abs() < 1e-5, "face {face}");
        }
    }

    const BEZEL: [u32; 2] = [384, 131];
    const DEVICE: u32 = PLANAR_MAX;

    fn scaled(cube: f32, mirror: f32) -> ReflectionResolution {
        ReflectionResolution { cube: ReflectionSize::Scale(cube), mirror: ReflectionSize::Scale(mirror), pow2: false }
    }

    #[test]
    fn the_default_keeps_the_quality_face_and_the_bezel_picture() {
        let r = ReflectionResolution::default();
        for base in [64, 128, 256] {
            assert_eq!(r.cube_face(base, 1080, DEVICE), base);
        }
        assert_eq!(r.mirror_size(BEZEL, [1920, 1080], DEVICE), BEZEL);
    }

    #[test]
    fn size_text_round_trips_and_rejects_bad_values() {
        for text in ["auto", "256", "150%", "25%", "400%"] {
            let size: ReflectionSize = text.parse().unwrap();
            assert_eq!(size.to_string(), text);
        }
        assert_eq!("100%".parse::<ReflectionSize>().unwrap(), ReflectionSize::Scale(1.0));
        assert_eq!(" AUTO ".parse::<ReflectionSize>().unwrap(), ReflectionSize::Auto);
        for text in ["10%", "401%", "-5", "1.5x", "", "nan%"] {
            assert!(text.parse::<ReflectionSize>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn scale_multiplies_the_quality_face_and_clamps_to_the_range() {
        assert_eq!(scaled(1.5, 1.0).cube_face(128, 1080, DEVICE), 192);
        assert_eq!(scaled(0.25, 1.0).cube_face(64, 1080, DEVICE), CUBE_FACE_MIN);
        assert_eq!(scaled(4.0, 1.0).cube_face(256, 1080, DEVICE), CUBE_FACE_MAX);
        assert_eq!(scaled(f32::NAN, 1.0).cube_face(128, 1080, DEVICE), 128);
    }

    #[test]
    fn auto_follows_the_output_height() {
        let auto = ReflectionResolution { cube: ReflectionSize::Auto, ..Default::default() };
        assert_eq!(auto.cube_face(128, 720, DEVICE), 720);
        assert_eq!(auto.cube_face(128, 2160, DEVICE), CUBE_FACE_MAX, "the top of the cube range");
        assert_eq!(auto.cube_face(128, 0, DEVICE), CUBE_FACE_MIN);
    }

    #[test]
    fn fixed_faces_ignore_the_quality_base() {
        let fixed = ReflectionResolution { cube: ReflectionSize::Fixed(300), ..Default::default() };
        assert_eq!(fixed.cube_face(64, 1080, DEVICE), 300);
        let tiny = ReflectionResolution { cube: ReflectionSize::Fixed(4), ..Default::default() };
        assert_eq!(tiny.cube_face(64, 1080, DEVICE), CUBE_FACE_MIN);
    }

    #[test]
    fn power_of_two_rounds_cube_faces_down_only() {
        let pow2 = |cube| ReflectionResolution { cube, mirror: ReflectionSize::Auto, pow2: true };
        assert_eq!(pow2(ReflectionSize::Auto).cube_face(128, 720, DEVICE), 512);
        assert_eq!(pow2(ReflectionSize::Fixed(256)).cube_face(128, 720, DEVICE), 256);
        assert_eq!(pow2(ReflectionSize::Fixed(255)).cube_face(128, 720, DEVICE), 128);
        assert_eq!(floor_pow2(0), 0);
        assert_eq!(floor_pow2(1), 1);
        assert_eq!(floor_pow2(1000), 512);
        // The mirror keeps its size.
        let mirror = ReflectionResolution { mirror: ReflectionSize::Fixed(100), pow2: true, ..Default::default() };
        assert_eq!(mirror.mirror_size(BEZEL, [1920, 1080], DEVICE)[1], 100);
    }

    #[test]
    fn auto_mirror_is_a_sixth_of_the_height_at_the_bezel_aspect() {
        let auto = ReflectionResolution { mirror: ReflectionSize::Auto, ..Default::default() };
        assert_eq!(auto.mirror_size(BEZEL, [1920, 1080], DEVICE), [528, 180]);
    }

    #[test]
    fn scaled_mirror_keeps_the_bezel_aspect() {
        let [w, h] = scaled(1.0, 2.0).mirror_size(BEZEL, [1920, 1080], DEVICE);
        assert_eq!((w, h), (768, 262));
        let [w, h] = scaled(1.0, 4.0).mirror_size(BEZEL, [1920, 1080], DEVICE);
        assert_eq!(h, 524);
        assert!((w as f32 / h as f32 - 384.0 / 131.0).abs() < 0.01);
    }

    #[test]
    fn mirror_clamps_to_the_minimum_height_and_the_device_width() {
        let [w, h] = scaled(1.0, 0.25).mirror_size(BEZEL, [1920, 1080], DEVICE);
        assert_eq!((w, h), (188, PLANAR_MIN_HEIGHT));
        let [w, h] = ReflectionResolution { mirror: ReflectionSize::Auto, ..Default::default() }.mirror_size(
            BEZEL,
            [3840, 4320],
            DEVICE,
        );
        assert!(w <= DEVICE && h <= DEVICE, "{w}x{h}");
        let small = ReflectionResolution { mirror: ReflectionSize::Fixed(4000), ..Default::default() };
        assert!(small.mirror_size(BEZEL, [1920, 1080], 512)[0] <= 512);
    }

    #[test]
    fn the_device_limit_caps_the_cube_face() {
        let auto = ReflectionResolution { cube: ReflectionSize::Auto, ..Default::default() };
        assert_eq!(auto.cube_face(128, 2160, 512), 512);
    }

    #[test]
    fn mirror_tint_text_round_trips() {
        assert_eq!("off".parse::<MirrorTint>().unwrap(), MirrorTint::NONE);
        assert_eq!("on".parse::<MirrorTint>().unwrap(), MirrorTint::STOCK);
        assert_eq!("#96a0ff".parse::<MirrorTint>().unwrap(), MirrorTint([0x96, 0xa0, 0xff]));
        assert_eq!("10 20 30".parse::<MirrorTint>().unwrap(), MirrorTint([10, 20, 30]));
        assert_eq!("10,20,30".parse::<MirrorTint>().unwrap(), MirrorTint([10, 20, 30]));
        for tint in [MirrorTint::NONE, MirrorTint([1, 2, 3])] {
            assert_eq!(tint.to_string().parse::<MirrorTint>().unwrap(), tint);
        }
        for text in ["256 0 0", "#12345", "#zzzzzz", "1 2", "red"] {
            assert!(text.parse::<MirrorTint>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn tint_multiplies_colour_and_keeps_alpha() {
        let mut white = vec![200, 200, 200, 90];
        tint_rgba(&mut white, MirrorTint::NONE);
        assert_eq!(white, [200, 200, 200, 90], "no tint leaves the picture alone");
        tint_rgba(&mut white, MirrorTint::STOCK);
        assert_eq!(white, [118, 118, 118, 90]);
    }

    #[test]
    fn sanitizing_clamps_the_scale_factors() {
        let wild = ReflectionResolution {
            cube: ReflectionSize::Scale(9.0),
            mirror: ReflectionSize::Scale(f32::NAN),
            pow2: true,
        };
        assert_eq!(
            wild.sanitized(),
            ReflectionResolution {
                cube: ReflectionSize::Scale(MAX_SCALE),
                mirror: ReflectionSize::Scale(1.0),
                pow2: true,
            }
        );
    }
}
