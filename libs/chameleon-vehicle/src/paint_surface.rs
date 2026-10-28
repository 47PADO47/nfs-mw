//! The paint surfaces of a car's body and which of the car's two colours each takes. Spec:
//! `docs/formats/chameleon-customization.md`.
//!
//! The 2012 body-paint shaders come in plain and two-colour techniques. A plain surface takes the paint's colour. A
//! `TwoPaint` surface takes the first colour or the second, by the `mPaintColourIndex` constant of its material (0 or 1),
//! so the second colour covers whole meshes. A `TwoPaintMask` surface blends the two by a texture's green channel. Both
//! shaders compute `PbrPaintDiffuseColour + PbrPaintSecondaryDiffuseColour * amount` (and the same for the specular colour),
//! the amount being the material index or the mask **[verified]**: a blend of the two colours when the secondary constant
//! holds the difference of the colours, which is what the packs' second colours (black roofs and stripes) need
//! **[unconfirmed]**.

use chameleon_bundle::{Pool, ResourceKind};
use chameleon_gfx::{Material, Model, Renderable, Shader};

use crate::error::Result;
use crate::spec::GraphicsSpec;
use crate::vehicle::expect;

/// The technique of the engine's paint shaders.
const PAINT: &str = "BodyPaint";
/// The sampler that takes a `TwoPaintMask` surface's mask.
const MASK_SAMPLER: &str = "DiffuseTextureSampler";

/// Which colour a paint surface takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintBlend {
    /// The first colour: a plain paint surface, or a two-colour one whose material index is 0.
    First,
    /// The second colour: a two-colour surface whose material index is 1.
    Second,
    /// Both, by the texture with this resource id: the first colour where its green channel is 0, the second where 1.
    Mask(u64),
    /// A livery picture is laid over the surface (a `Livery` technique other than the two-colour ones).
    Picture,
}

/// One paint surface of a body level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintSurface {
    /// The shader's technique name, as [`crate::Appearance::technique`] holds it.
    pub technique: String,
    /// Triangles of the mesh.
    pub triangles: usize,
    pub blend: PaintBlend,
}

/// The paint surfaces of every body level of the vehicle whose graphics spec is `spec`, most detailed level first, each in
/// the order [`crate::load`] gives the level's paint meshes (the meshes whose [`crate::SurfaceKind`] is `Paint`).
pub fn paint_surfaces(pool: &Pool, spec: u64) -> Result<Vec<Vec<PaintSurface>>> {
    let spec = expect(pool, spec, ResourceKind::VehicleGraphicsSpec)?;
    let spec = GraphicsSpec::parse(&spec.blocks[0], &spec.imports)?;
    let model = expect(pool, spec.body_model, ResourceKind::Model)?;
    let model = Model::parse(&model.blocks[0], &model.imports)?;
    model.lods.iter().map(|&lod| level_surfaces(pool, lod)).collect()
}

fn level_surfaces(pool: &Pool, lod: u64) -> Result<Vec<PaintSurface>> {
    let resource = expect(pool, lod, ResourceKind::Renderable)?;
    let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
    let is_descriptor = |id: u64| pool.kind(id) == Some(ResourceKind::VertexDescriptor.id());
    let mut out = Vec::new();
    for mesh in &renderable.meshes {
        let material = expect(pool, mesh.material, ResourceKind::Material)?;
        let material = Material::parse(&material.blocks[0], &material.imports)?;
        let shader = expect(pool, material.shader, ResourceKind::Shader)?;
        let shader = Shader::parse(&shader.blocks[0], &shader.imports, is_descriptor)?;
        // A material whose shader has no vertex layout is not a mesh of the car (`load` leaves it out).
        let technique = shader.technique().unwrap_or_default();
        if shader.vertex_descriptor.is_none() || !technique.contains(PAINT) {
            continue;
        }
        out.push(PaintSurface {
            technique: technique.to_owned(),
            triangles: mesh.triangles(&resource.blocks[1])?.len(),
            blend: blend_of(technique, &shader, &material),
        });
    }
    Ok(out)
}

/// What a surface of `technique` takes. The `Lightmap` forms of the two-colour techniques are read like the plain ones: no
/// car of the install has a `TwoPaint_Lightmap` surface, and where its index sits in the material is not known.
fn blend_of(technique: &str, shader: &Shader, material: &Material) -> PaintBlend {
    if technique.ends_with("TwoPaintMask") || technique.ends_with("TwoPaintMask_Lightmap") {
        let texture = shader.sampler_slot(MASK_SAMPLER).and_then(|slot| material.textures.get(slot));
        return texture.map_or(PaintBlend::First, |&id| PaintBlend::Mask(id));
    }
    if technique.ends_with("TwoPaint") || technique.ends_with("TwoPaint_Livery") {
        let index = material.float4(0).map(|slot| slot[0]).filter(|index| index.is_finite());
        return match index {
            Some(index) if index >= 0.5 => PaintBlend::Second,
            _ => PaintBlend::First,
        };
    }
    if technique.contains("Livery") {
        return PaintBlend::Picture;
    }
    PaintBlend::First
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(textures: Vec<u64>, constants: &[f32]) -> Material {
        let constants = constants.iter().flat_map(|value| value.to_le_bytes()).collect();
        Material { shader: 0, textures, constants }
    }

    fn shader(samplers: &[&str]) -> Shader {
        Shader {
            vertex_descriptor: None,
            names: Vec::new(),
            material_samplers: samplers.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    #[test]
    fn a_two_paint_surface_takes_the_colour_its_material_index_says() {
        let plain = shader(&["CrumpleTextureSampler"]);
        let technique = "VehicleNFS13_BodyPaint_TwoPaint";
        assert_eq!(blend_of(technique, &plain, &material(vec![5], &[0.0, 0.0, 0.0, 0.0])), PaintBlend::First);
        assert_eq!(blend_of(technique, &plain, &material(vec![5], &[1.0, 0.0, 0.0, 0.0])), PaintBlend::Second);
        assert_eq!(blend_of(technique, &plain, &material(vec![5], &[f32::NAN, 0.0, 0.0, 0.0])), PaintBlend::First);
        assert_eq!(blend_of(technique, &plain, &material(vec![5], &[])), PaintBlend::First, "no constants");
        let livery = "VehicleNFS13_BodyPaint_TwoPaint_Livery";
        assert_eq!(blend_of(livery, &plain, &material(vec![5], &[1.0, 0.0, 0.0, 0.0])), PaintBlend::Second);
    }

    #[test]
    fn a_mask_surface_names_the_texture_of_its_diffuse_sampler() {
        let masked = shader(&["DiffuseTextureSampler", "CrumpleTextureSampler"]);
        let material = material(vec![0xABC, 0xDEF], &[]);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_TwoPaintMask", &masked, &material), PaintBlend::Mask(0xABC));
        assert_eq!(
            blend_of("VehicleNFS13_BodyPaint_TwoPaintMask_Lightmap", &masked, &material),
            PaintBlend::Mask(0xABC)
        );
        let unsampled = shader(&["CrumpleTextureSampler"]);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_TwoPaintMask", &unsampled, &material), PaintBlend::First);
    }

    #[test]
    fn other_techniques_are_a_picture_or_the_first_colour() {
        let plain = shader(&[]);
        let none = material(Vec::new(), &[]);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_Livery", &plain, &none), PaintBlend::Picture);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_Livery_Lightmap", &plain, &none), PaintBlend::Picture);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint", &plain, &none), PaintBlend::First);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_Lightmap", &plain, &none), PaintBlend::First);
        assert_eq!(blend_of("VehicleNFS13_BodyPaint_TwoPaint_Lightmap", &plain, &none), PaintBlend::First);
    }
}
