use chameleon_gfx::{Material, Shader};

/// What a surface is, from its shader's technique name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// Body paint: takes the car's colour.
    Paint,
    /// Other body panels: trim, plastics, chrome, interior.
    Body,
    /// Windows and lamp lenses.
    Glass,
    /// Lit surfaces (lamps, dashboard lights).
    Light,
    Wheel,
    Tyre,
    Other,
}

/// How a mesh looks, reduced to what an older engine can draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    pub kind: SurfaceKind,
    /// The shader's technique name.
    pub technique: String,
    /// The texture resource the shader samples as diffuse colour, if it has one.
    pub diffuse_texture: Option<u64>,
    /// The material's own diffuse colour (linear RGB), if it sets one.
    pub diffuse_colour: Option<[f32; 3]>,
    /// The diffuse texture's alpha cuts the surface out.
    pub alpha_test: bool,
}

const DIFFUSE_SAMPLERS: [&str; 2] = ["DiffuseTextureSampler", "ColourSampler"];

impl Appearance {
    pub fn new(shader: &Shader, material: &Material) -> Self {
        let technique = shader.technique().unwrap_or_default().to_owned();
        let diffuse_texture = DIFFUSE_SAMPLERS
            .iter()
            .find_map(|sampler| shader.sampler_slot(sampler))
            .and_then(|slot| material.textures.get(slot).copied());
        let diffuse_colour = shader.has_name("PbrMaterialDiffuseColour").then(|| material.first_colour()).flatten();
        Self {
            kind: classify(&technique),
            alpha_test: technique.contains("Alpha1bit"),
            technique,
            diffuse_texture,
            diffuse_colour,
        }
    }
}

fn classify(technique: &str) -> SurfaceKind {
    let has = |word: &str| technique.contains(word);
    if has("BodyPaint") {
        return SurfaceKind::Paint;
    }
    if has("Glass") || has("Refraction") {
        return SurfaceKind::Glass;
    }
    if has("Tyre") {
        return SurfaceKind::Tyre;
    }
    if has("Wheel") {
        return SurfaceKind::Wheel;
    }
    if has("Emissive") {
        return SurfaceKind::Light;
    }
    if has("Body") {
        return SurfaceKind::Body;
    }
    SurfaceKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn techniques_classify() {
        assert_eq!(classify("VehicleNFS13_BodyPaint_Lightmap"), SurfaceKind::Paint);
        assert_eq!(classify("VehicleNFS13_Glass_Textured_Lightmap"), SurfaceKind::Glass);
        assert_eq!(classify("VehicleNFS13_Wheel_Tyre_Textured_Normalmap_Blurred"), SurfaceKind::Tyre);
        assert_eq!(classify("VehicleNFS13_Wheel_Alpha1bit_Textured_Normalmap"), SurfaceKind::Wheel);
        assert_eq!(classify("VehicleNFS13_Body_Textured_NormalMap_Emissive_NoDamage"), SurfaceKind::Light);
        assert_eq!(classify("VehicleNFS13_Body"), SurfaceKind::Body);
        assert_eq!(classify("Something"), SurfaceKind::Other);
    }
}
