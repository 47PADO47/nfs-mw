//! From the Unreal material model to the 2005 car shader's: which light material (a named set of diffuse, specular and
//! environment ramps in the install's `GlobalB.lzc`) a surface takes.
//!
//! The car shader has no per-texel gloss: one light material per shading group. RIDE 6 packs specular, roughness,
//! metallic and occlusion into one picture ([the material model](../../../docs/formats/ride6-materials.md)), so a
//! surface is classified from that picture and from its base colour, triangle by triangle, and a material's triangles are
//! split into one group per light material. The thresholds are modelling choices (the names and the split are data of the
//! install and the bike).
//!
//! **Why the base colour takes part.** A metal reflects with its base colour as the reflectance (RIDE 6 shades a bare metal
//! so), so a black metal reflects almost nothing and a gold one reflects gold. The car materials of the bare-metal kind
//! (`CHROME`, `MAGSILVER`, `ALUMINUM`) draw the reflection of the environment alone, tinted by the texture only a third of the
//! way and never darker than a polished steel: a black or coloured metal drawn with them turns silver. They are therefore
//! given to the bright, neutral metals only; a dark or coloured metal keeps its texture under a material that draws the
//! texture and adds a gloss.

use crate::partfile::Kind;
use crate::textures::{Picture, interior_points};

/// Light materials of the install this importer picks (all present in `GlobalB.lzc` of the PC install).
pub const RUBBER: &str = "RUBBER";
pub const BRAKE_DISC: &str = "BRAKEDISC";
pub const CHROME: &str = "CHROME";
pub const POLISHED_METAL: &str = "MAGSILVER";
pub const ROUGH_METAL: &str = "ALUMINUM";
/// A metal of middle brightness: the texture under a painted-metal response with a weak reflection.
pub const DARK_METAL: &str = "MAGGUNMETAL";
pub const GLOSS: &str = "SHINYPLASTIC";
pub const MATTE: &str = "DULLPLASTIC";

/// Metallic value at and above which a surface is bare metal.
pub const METAL_THRESHOLD: f32 = 0.5;
/// Roughness below which bare metal is chrome, and below which a dielectric is glossy.
pub const CHROME_ROUGHNESS: f32 = 0.2;
pub const POLISHED_ROUGHNESS: f32 = 0.4;
pub const GLOSS_ROUGHNESS: f32 = 0.4;
/// Linear luminance of the base colour below which a metal reflects too little to be told from a dark paint.
pub const DARK_METAL_LUMINANCE: f32 = 0.08;
/// Linear luminance of the base colour at and above which a metal is bright enough for the environment-only materials
/// (the bare steel and aluminium of the bikes measure 0.4 to 0.6).
pub const BRIGHT_METAL_LUMINANCE: f32 = 0.30;
/// Saturation (`(max - min) / max` of the linear colour) at and above which a metal is a coloured one (gold, anodised).
pub const COLOURED_SATURATION: f32 = 0.30;

/// What a triangle of a surface is made of: the means of the pictures over its interior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Response {
    /// Metallic value, 0 to 1.
    pub metal: f32,
    /// Roughness, 0 to 1.
    pub rough: f32,
    /// Linear luminance of the base colour, 0 to 1.
    pub luminance: f32,
    /// Saturation of the linear base colour, 0 to 1.
    pub saturation: f32,
}

impl Response {
    /// A surface without pictures: a smooth dielectric of the brightest colour (it takes the part's own default).
    pub const NONE: Self = Self { metal: 0.0, rough: 0.0, luminance: 1.0, saturation: 0.0 };
}

/// The light material of a dielectric (paint, plastic, rubber) of roughness `rough`.
fn dielectric(rough: f32) -> &'static str {
    if rough < GLOSS_ROUGHNESS {
        return GLOSS;
    }
    MATTE
}

/// The light material of a surface of `response` on a part of `kind`.
pub fn light_material(kind: Kind, response: Response) -> &'static str {
    if kind == Kind::Tyre {
        return RUBBER;
    }
    if response.metal < METAL_THRESHOLD {
        return dielectric(response.rough);
    }
    if kind == Kind::Brake {
        return BRAKE_DISC;
    }
    // A dark or coloured metal: the environment-only materials would make it silver.
    if response.luminance < DARK_METAL_LUMINANCE || response.saturation >= COLOURED_SATURATION {
        return dielectric(response.rough);
    }
    if response.luminance < BRIGHT_METAL_LUMINANCE {
        return DARK_METAL;
    }
    if response.rough < CHROME_ROUGHNESS {
        return CHROME;
    }
    if response.rough < POLISHED_ROUGHNESS {
        return POLISHED_METAL;
    }
    ROUGH_METAL
}

/// The pictures of a surface a triangle is classified from.
#[derive(Clone, Copy, Default)]
pub struct Pictures<'a> {
    /// The base colour, as the exporter wrote it (without the occlusion baked in).
    pub base: Option<&'a Picture>,
    /// The packed specular, roughness, metallic and occlusion picture.
    pub pbr: Option<&'a Picture>,
}

/// The mean of `response` over the interior of a triangle: the packed picture (specular, roughness, metallic, occlusion) and
/// the base colour picture sampled at [`interior_points`]. `None` without the packed picture; without a base colour the
/// metal counts as bright and neutral.
///
/// With an `ao_floor` the colour is the one the diffuse picture is baked to (`floor + (1 - floor) * occlusion` of it, in
/// linear light, `bake::bake_diffuse`): a metal in a crevice reflects less than an exposed one (RIDE 6 applies the occlusion
/// to the specular light too), and the environment-only materials cannot be darkened per texel, so the metal that the
/// occlusion darkens takes a material with a body.
pub fn triangle_response(pictures: Pictures<'_>, ao_floor: Option<f32>, uvs: [[f32; 2]; 3]) -> Option<Response> {
    let pbr = pictures.pbr?;
    let points = interior_points(uvs);
    let count = points.len() as f32;
    let packed_mean =
        |channel: usize| points.iter().map(|&uv| f32::from(pbr.sample(uv)[channel])).sum::<f32>() / (count * 255.0);
    // Channels: specular, roughness, metallic, occlusion.
    let (metal, rough) = (packed_mean(2), packed_mean(1));
    let Some(base) = pictures.base else {
        return Some(Response { metal, rough, ..Response::NONE });
    };
    let mut colour = [0.0f32; 3];
    for &uv in &points {
        let texel = base.sample(uv);
        let factor = ao_floor.map_or(1.0, |floor| floor + (1.0 - floor) * f32::from(pbr.sample(uv)[3]) / 255.0);
        for (sum, &byte) in colour.iter_mut().zip(&texel[..3]) {
            *sum += crate::bake::srgb_to_linear(byte) * factor / count;
        }
    }
    let [r, g, b] = colour;
    let top = r.max(g).max(b);
    let saturation = if top > 1e-6 { (top - r.min(g).min(b)) / top } else { 0.0 };
    Some(Response { metal, rough, luminance: 0.2126 * r + 0.7152 * g + 0.0722 * b, saturation })
}

/// The light material of a triangle of a part of `kind`: from its pictures, else the part's own default.
pub fn classify(kind: Kind, pictures: Pictures<'_>, ao_floor: Option<f32>, uvs: [[f32; 2]; 3]) -> &'static str {
    light_material(kind, triangle_response(pictures, ao_floor, uvs).unwrap_or(Response::NONE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(rgba: [u8; 4]) -> Picture {
        Picture { width: 2, height: 2, rgba: rgba.repeat(4) }
    }

    fn both<'a>(base: &'a Picture, pbr: &'a Picture) -> Pictures<'a> {
        Pictures { base: Some(base), pbr: Some(pbr) }
    }

    fn response(metal: f32, rough: f32, luminance: f32, saturation: f32) -> Response {
        Response { metal, rough, luminance, saturation }
    }

    /// A bright neutral metal, as the steel and aluminium of the bikes.
    fn steel(rough: f32) -> Response {
        response(1.0, rough, 0.5, 0.02)
    }

    #[test]
    fn tyres_are_rubber_and_discs_are_brake_discs_whatever_the_picture_says() {
        assert_eq!(light_material(Kind::Tyre, steel(0.0)), RUBBER);
        assert_eq!(light_material(Kind::Brake, steel(0.9)), BRAKE_DISC);
        assert_eq!(
            light_material(Kind::Brake, response(0.0, 0.9, 0.0, 0.0)),
            MATTE,
            "a painted part of a disc is paint"
        );
    }

    #[test]
    fn bright_neutral_metal_steps_from_chrome_to_rough_and_dielectrics_from_gloss_to_matte() {
        assert_eq!(light_material(Kind::Body, steel(0.1)), CHROME);
        assert_eq!(light_material(Kind::Rim, steel(0.3)), POLISHED_METAL);
        assert_eq!(light_material(Kind::Body, steel(0.8)), ROUGH_METAL);
        assert_eq!(light_material(Kind::Body, response(0.0, 0.2, 0.5, 0.0)), GLOSS);
        assert_eq!(light_material(Kind::Body, response(0.2, 0.9, 0.5, 0.0)), MATTE);
    }

    #[test]
    fn a_dark_or_coloured_metal_keeps_its_texture_instead_of_turning_silver() {
        // Black anodised metal: base colour 0.02 linear, metallic 1, smooth.
        assert_eq!(light_material(Kind::Body, response(1.0, 0.35, 0.02, 0.0)), GLOSS);
        assert_eq!(light_material(Kind::Body, response(1.0, 0.7, 0.02, 0.0)), MATTE);
        // Gold: bright but saturated.
        assert_eq!(light_material(Kind::Body, response(1.0, 0.2, 0.45, 0.8)), GLOSS);
        // Dark steel (a chain): between the two limits.
        assert_eq!(light_material(Kind::Body, response(1.0, 0.2, 0.12, 0.0)), DARK_METAL);
    }

    #[test]
    fn the_limits_are_inclusive_where_they_say_at_and_above() {
        assert_eq!(light_material(Kind::Body, response(METAL_THRESHOLD, 0.1, 0.5, 0.0)), CHROME);
        assert_eq!(light_material(Kind::Body, response(1.0, 0.1, BRIGHT_METAL_LUMINANCE, 0.0)), CHROME);
        assert_eq!(light_material(Kind::Body, response(1.0, 0.1, DARK_METAL_LUMINANCE, 0.0)), DARK_METAL);
        assert_eq!(light_material(Kind::Body, response(1.0, 0.1, 0.5, COLOURED_SATURATION)), GLOSS);
    }

    #[test]
    fn the_packed_picture_is_read_as_specular_roughness_metallic() {
        // R = specular, G = roughness (64 of 255), B = metallic (1.0); a mid-grey base colour (sRGB 188 is about 0.5).
        let pbr = flat([128, 64, 255, 255]);
        let base = flat([188, 188, 188, 255]);
        let uv = [[0.1, 0.1], [0.9, 0.1], [0.5, 0.9]];
        let found = triangle_response(both(&base, &pbr), None, uv).unwrap();
        assert!((found.metal - 1.0).abs() < 1e-5 && (found.rough - 64.0 / 255.0).abs() < 1e-5);
        assert!((found.luminance - 0.5).abs() < 0.02 && found.saturation < 1e-3, "{found:?}");
        assert_eq!(classify(Kind::Body, both(&base, &pbr), None, uv), POLISHED_METAL);
        assert_eq!(classify(Kind::Body, Pictures::default(), None, uv), GLOSS);
        assert!(triangle_response(Pictures { base: Some(&base), pbr: None }, None, uv).is_none());
    }

    #[test]
    fn the_base_colour_decides_whether_a_metal_texel_reflects() {
        let pbr = flat([128, 64, 255, 255]);
        let uv = [[0.1, 0.1], [0.9, 0.1], [0.5, 0.9]];
        let black = flat([30, 30, 30, 255]);
        assert_eq!(classify(Kind::Body, both(&black, &pbr), None, uv), GLOSS);
        let gold = flat([220, 170, 60, 255]);
        assert_eq!(classify(Kind::Body, both(&gold, &pbr), None, uv), GLOSS);
        let bare = Pictures { base: None, pbr: Some(&pbr) };
        assert_eq!(classify(Kind::Body, bare, None, uv), POLISHED_METAL, "without a colour a metal is silver");
    }

    #[test]
    fn a_corner_on_the_edge_of_the_picture_is_not_read_from_the_far_edge() {
        // A 2 x 2 picture: metal in the top row, plain below. A triangle in the bottom row with two corners at v = 1.0
        // (which wraps to the top row) is plain.
        let pbr = Picture {
            width: 2,
            height: 2,
            rgba: vec![128, 64, 255, 255, 128, 64, 255, 255, 128, 64, 0, 255, 128, 64, 0, 255],
        };
        let uv = [[0.1, 1.0], [0.9, 1.0], [0.5, 0.55]];
        let found = triangle_response(Pictures { base: None, pbr: Some(&pbr) }, None, uv).unwrap();
        assert!(found.metal < 0.1, "{found:?}");
    }

    #[test]
    fn the_occlusion_darkens_a_metal_the_way_the_baked_picture_is() {
        // Steel (sRGB 188, 0.5 linear) smooth metal; the occlusion channel is 0 (fully occluded) or 255.
        let base = flat([188, 188, 188, 255]);
        let uv = [[0.1, 0.1], [0.9, 0.1], [0.5, 0.9]];
        let open = flat([128, 20, 255, 255]);
        let shut = flat([128, 20, 255, 0]);
        assert_eq!(classify(Kind::Body, both(&base, &open), Some(0.3), uv), CHROME);
        // 0.5 * 0.3 = 0.15: between the dark and the bright limit.
        assert_eq!(classify(Kind::Body, both(&base, &shut), Some(0.3), uv), DARK_METAL);
        assert_eq!(
            classify(Kind::Body, both(&base, &shut), None, uv),
            CHROME,
            "no occlusion in the diffuse, none here"
        );
    }
}
