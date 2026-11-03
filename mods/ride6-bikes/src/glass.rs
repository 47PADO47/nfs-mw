//! Translucent surfaces (RIDE 6 `BLEND_TranslucentGreyTransmittance`, shading model `MSM_ThinTranslucent`: the screen, the
//! lamp lenses) as the 2005 car shader's glass.
//!
//! A stock window group of the 2005 cars is drawn alpha-blended: its texture is a blended one (`AlphaBlendType` 1, alpha
//! usage modulated, alpha-sorted) and its light material is `WINDSHIELD`, whose diffuse alpha is a constant 0.4; the shader
//! outputs `texture alpha * light material alpha` (`docs/specs/car-material-shading.md`). So a surface of a given opacity
//! needs a picture whose alpha is that opacity divided by the light material's alpha, and the light material has to be one
//! whose alpha reaches it. See [the material model](../../../docs/formats/ride6-materials.md#translucent-surfaces).

use blackbox_tpk::Level;

use crate::bake::srgb_to_linear;
use crate::textures::{Picture, interior_points};

/// The light material of clear and lightly tinted glass: the stock windows' (`WINDSHIELD`). Its diffuse alpha is the
/// constant [`CLEAR_ALPHA`] and it reflects the environment strongly at grazing angles.
pub const CLEAR_GLASS: &str = "WINDSHIELD";
/// `DiffuseMinAlpha` and `DiffuseMaxAlpha` of `WINDSHIELD`, read from the install's `GlobalB.lzc`: the opacity of the glass
/// for a picture alpha of 1.
pub const CLEAR_ALPHA: f32 = 0.4;
/// The light material of strongly tinted glass: the stock tail lamp lens (`BRAKELIGHTGLASS`): diffuse alpha a constant 1.0
/// (`DiffuseMinAlpha` and `DiffuseMaxAlpha` of the install's `GlobalB.lzc`), no highlight, a strong environment term.
pub const TINTED_GLASS: &str = "BRAKELIGHTGLASS";
/// Its diffuse alpha: the picture's alpha is the opacity itself.
pub const TINTED_ALPHA: f32 = 1.0;

/// Where the texel's own opacity decides which of the two light materials reaches it: up to [`CLEAR_ALPHA`] the clear glass
/// can show it, above it the tinted glass has to.
const CLEAR_LIMIT: f32 = CLEAR_ALPHA;

/// The opacity of a texel (0 to 1): `1 - transmittance`, the transmittance being the mean of the texel's linear colour
/// (the "grey" of the blend mode's name) scaled by the material's `Modulation - Transmittance - Max`.
pub fn opacity(texel: [u8; 4], max: f32) -> f32 {
    let linear = [texel[0], texel[1], texel[2]].map(srgb_to_linear);
    let mean = linear.iter().sum::<f32>() / 3.0;
    (1.0 - mean * max.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

/// The light material that shows `opacity`.
pub fn light_material(opacity: f32) -> &'static str {
    if opacity <= CLEAR_LIMIT {
        return CLEAR_GLASS;
    }
    TINTED_GLASS
}

/// The alpha a picture stores for a texel of `opacity`, given the light material that will draw it.
fn stored_alpha(opacity: f32) -> u8 {
    let scale = if opacity <= CLEAR_LIMIT { CLEAR_ALPHA } else { TINTED_ALPHA };
    ((opacity / scale).clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// The transmittance picture as the car's glass texture: the colour is the picture's (the tint of the glass; where the
/// glass is clear it is white, which the low alpha makes a faint veil), the alpha the opacity divided by the alpha of the
/// light material that will draw the texel.
pub fn bake(picture: Level, max: f32) -> Level {
    let Level { width, height, mut rgba } = picture;
    for texel in rgba.as_chunks_mut::<4>().0.iter_mut() {
        texel[3] = stored_alpha(opacity(*texel, max));
    }
    Level { width, height, rgba }
}

/// The mean opacity of a triangle: the transmittance picture sampled at the interior points of the triangle
/// ([`interior_points`], which never read across the border of an island or the edge of the picture).
pub fn triangle_opacity(picture: &Picture, max: f32, uvs: [[f32; 2]; 3]) -> f32 {
    let samples = interior_points(uvs).map(|uv| opacity(picture.sample(uv), max));
    samples.iter().sum::<f32>() / samples.len() as f32
}

/// The light material of a glass triangle.
pub fn classify(picture: &Picture, max: f32, uvs: [[f32; 2]; 3]) -> &'static str {
    light_material(triangle_opacity(picture, max, uvs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_glass_is_nearly_clear_and_the_modulation_leaves_what_it_does_not_transmit() {
        assert!((opacity([255, 255, 255, 0], 0.95) - 0.05).abs() < 1e-6);
        assert!((opacity([255, 255, 255, 0], 1.0)).abs() < 1e-6);
        assert!((opacity([0, 0, 0, 255], 0.95) - 1.0).abs() < 1e-6, "a black texel blocks the light");
    }

    #[test]
    fn a_red_lens_is_mostly_opaque_and_the_alpha_of_the_picture_plays_no_part() {
        let red = opacity([173, 0, 5, 36], 0.95);
        assert!(red > 0.8 && red < 0.9, "{red}");
        assert_eq!(red, opacity([173, 0, 5, 0], 0.95));
    }

    #[test]
    fn the_stored_alpha_times_the_light_materials_alpha_is_the_opacity() {
        let clear = Level { width: 1, height: 1, rgba: vec![255, 255, 255, 0] };
        let alpha = f32::from(bake(clear, 0.95).rgba[3]) / 255.0;
        assert!((alpha * CLEAR_ALPHA - 0.05).abs() < 0.003, "{alpha}");
        let red = Level { width: 1, height: 1, rgba: vec![173, 0, 5, 36] };
        let alpha = f32::from(bake(red, 0.95).rgba[3]) / 255.0;
        assert!((alpha * TINTED_ALPHA - opacity([173, 0, 5, 0], 0.95)).abs() < 0.003, "{alpha}");
        let black = Level { width: 1, height: 1, rgba: vec![0, 0, 0, 0] };
        assert_eq!(bake(black, 0.95).rgba[3], 255, "the alpha is capped at 1");
    }

    #[test]
    fn the_colour_is_kept_and_the_light_material_follows_the_opacity() {
        let picture = Level { width: 2, height: 1, rgba: vec![255, 250, 255, 3, 173, 0, 5, 36] };
        let baked = bake(picture, 0.95);
        assert_eq!(&baked.rgba[..3], &[255, 250, 255]);
        assert_eq!(&baked.rgba[4..7], &[173, 0, 5]);
        assert_eq!(light_material(0.05), CLEAR_GLASS);
        assert_eq!(light_material(CLEAR_ALPHA), CLEAR_GLASS);
        assert_eq!(light_material(0.87), TINTED_GLASS);
    }

    #[test]
    fn a_triangle_takes_the_light_material_of_its_mean_opacity() {
        let picture = Picture { width: 2, height: 1, rgba: vec![255, 255, 255, 0, 173, 0, 5, 36] };
        let left = [[0.1, 0.5], [0.2, 0.5], [0.3, 0.5]];
        let right = [[0.6, 0.5], [0.7, 0.5], [0.9, 0.5]];
        assert_eq!(classify(&picture, 0.95, left), CLEAR_GLASS);
        assert_eq!(classify(&picture, 0.95, right), TINTED_GLASS);
    }
}
