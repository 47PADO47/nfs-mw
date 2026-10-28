//! From the Unreal material model to the 2005 car shader's: which light material (a named set of diffuse, specular and
//! environment ramps in the install's `GlobalB.lzc`) a surface takes.
//!
//! The car shader has no per-texel gloss: one light material per shading group. RIDE 6 packs specular, roughness,
//! metallic and occlusion into one picture ([the material model](../../../docs/formats/ride6-materials.md)), so a
//! surface is classified from that picture, triangle by triangle, and a material's triangles are split into one group per
//! light material. The thresholds are modelling choices (the names and the split are data of the install and the bike).

use crate::partfile::Kind;
use crate::textures::Picture;

/// Light materials of the install this importer picks (all present in `GlobalB.lzc` of the PC install).
pub const RUBBER: &str = "RUBBER";
pub const BRAKE_DISC: &str = "BRAKEDISC";
pub const CHROME: &str = "CHROME";
pub const POLISHED_METAL: &str = "MAGSILVER";
pub const ROUGH_METAL: &str = "ALUMINUM";
pub const GLOSS: &str = "SHINYPLASTIC";
pub const MATTE: &str = "DULLPLASTIC";

/// Metallic value at and above which a surface is bare metal.
pub const METAL_THRESHOLD: f32 = 0.5;
/// Roughness below which bare metal is chrome, and below which a dielectric is glossy.
pub const CHROME_ROUGHNESS: f32 = 0.2;
pub const POLISHED_ROUGHNESS: f32 = 0.4;
pub const GLOSS_ROUGHNESS: f32 = 0.4;

/// The light material of a surface of metallic value `metal` and roughness `rough` (both 0 to 1) on a part of `kind`.
pub fn light_material(kind: Kind, metal: f32, rough: f32) -> &'static str {
    if kind == Kind::Tyre {
        return RUBBER;
    }
    if metal >= METAL_THRESHOLD {
        if kind == Kind::Brake {
            return BRAKE_DISC;
        }
        if rough < CHROME_ROUGHNESS {
            return CHROME;
        }
        if rough < POLISHED_ROUGHNESS {
            return POLISHED_METAL;
        }
        return ROUGH_METAL;
    }
    if rough < GLOSS_ROUGHNESS {
        return GLOSS;
    }
    MATTE
}

/// The mean metallic value and roughness of a triangle: the packed picture sampled at its three corners and its centre.
/// `None` without a picture.
pub fn triangle_response(pbr: Option<&Picture>, uvs: [[f32; 2]; 3]) -> Option<(f32, f32)> {
    let pbr = pbr?;
    let centre = [(uvs[0][0] + uvs[1][0] + uvs[2][0]) / 3.0, (uvs[0][1] + uvs[1][1] + uvs[2][1]) / 3.0];
    let samples = [uvs[0], uvs[1], uvs[2], centre].map(|uv| pbr.sample(uv));
    let mean = |channel: usize| samples.iter().map(|s| f32::from(s[channel])).sum::<f32>() / (4.0 * 255.0);
    // Channels: specular, roughness, metallic, occlusion.
    Some((mean(2), mean(1)))
}

/// The light material of a triangle of a part of `kind`: from its picture, else the part's own default.
pub fn classify(kind: Kind, pbr: Option<&Picture>, uvs: [[f32; 2]; 3]) -> &'static str {
    match triangle_response(pbr, uvs) {
        Some((metal, rough)) => light_material(kind, metal, rough),
        None => light_material(kind, 0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(rgba: [u8; 4]) -> Picture {
        Picture { width: 2, height: 2, rgba: rgba.repeat(4) }
    }

    #[test]
    fn tyres_are_rubber_and_discs_are_brake_discs_whatever_the_picture_says() {
        assert_eq!(light_material(Kind::Tyre, 1.0, 0.0), RUBBER);
        assert_eq!(light_material(Kind::Brake, 1.0, 0.9), BRAKE_DISC);
    }

    #[test]
    fn metal_steps_from_chrome_to_rough_and_dielectrics_from_gloss_to_matte() {
        assert_eq!(light_material(Kind::Body, 1.0, 0.1), CHROME);
        assert_eq!(light_material(Kind::Rim, 1.0, 0.3), POLISHED_METAL);
        assert_eq!(light_material(Kind::Body, 1.0, 0.8), ROUGH_METAL);
        assert_eq!(light_material(Kind::Body, 0.0, 0.2), GLOSS);
        assert_eq!(light_material(Kind::Body, 0.2, 0.9), MATTE);
    }

    #[test]
    fn the_packed_picture_is_read_as_specular_roughness_metallic() {
        // R = specular, G = roughness (64 of 255), B = metallic (1.0).
        let picture = flat([128, 64, 255, 255]);
        let uv = [[0.1, 0.1], [0.9, 0.1], [0.5, 0.9]];
        let (metal, rough) = triangle_response(Some(&picture), uv).unwrap();
        assert!((metal - 1.0).abs() < 1e-5 && (rough - 64.0 / 255.0).abs() < 1e-5);
        assert_eq!(classify(Kind::Body, Some(&picture), uv), POLISHED_METAL);
        assert_eq!(classify(Kind::Body, None, uv), GLOSS);
        assert!(triangle_response(None, uv).is_none());
    }
}
