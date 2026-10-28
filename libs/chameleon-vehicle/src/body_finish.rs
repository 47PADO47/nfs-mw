//! The finish of a car's plain body surfaces: what their material says about the colour and the shine. Spec:
//! `docs/formats/chameleon-customization.md`.
//!
//! The untextured `VehicleNFS13_Body` technique has seven float4 parameters in the order of their names: clearcoat fresnel,
//! clearcoat specular, diffuse colour, roughness, scuff colour, scuff settings, specular colour **[verified]**: the same
//! seven slots on every such material of the Ariel Atom, with the colour slot where `chameleon_gfx::Material::first_colour` finds
//! it. A material with another number of slots is left out.

use chameleon_bundle::{Pool, ResourceKind};
use chameleon_gfx::{Material, Model, Renderable, Shader};

use crate::error::Result;
use crate::spec::GraphicsSpec;
use crate::vehicle::expect;

/// The technique of the untextured body surface.
pub const PLAIN: &str = "VehicleNFS13_Body";
/// The number of parameters of that technique, and the slots of the three this reads.
const SLOTS: usize = 7;
const DIFFUSE: usize = 2;
const ROUGHNESS: usize = 3;
const SPECULAR: usize = 6;

/// One untextured body surface of a body level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlainSurface {
    pub triangles: usize,
    /// The material's diffuse colour, linear RGB.
    pub diffuse: [f32; 3],
    /// The material's specular colour: bright on a metal, dim on a plastic or a paint.
    pub specular: [f32; 3],
    pub roughness: f32,
}

/// The untextured body surfaces of every body level of the vehicle whose graphics spec is `spec`, most detailed level first.
pub fn plain_surfaces(pool: &Pool, spec: u64) -> Result<Vec<Vec<PlainSurface>>> {
    let spec = expect(pool, spec, ResourceKind::VehicleGraphicsSpec)?;
    let spec = GraphicsSpec::parse(&spec.blocks[0], &spec.imports)?;
    let model = expect(pool, spec.body_model, ResourceKind::Model)?;
    let model = Model::parse(&model.blocks[0], &model.imports)?;
    model.lods.iter().map(|&lod| level_surfaces(pool, lod)).collect()
}

fn level_surfaces(pool: &Pool, lod: u64) -> Result<Vec<PlainSurface>> {
    let resource = expect(pool, lod, ResourceKind::Renderable)?;
    let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
    let is_descriptor = |id: u64| pool.kind(id) == Some(ResourceKind::VertexDescriptor.id());
    let mut out = Vec::new();
    for mesh in &renderable.meshes {
        let material = expect(pool, mesh.material, ResourceKind::Material)?;
        let material = Material::parse(&material.blocks[0], &material.imports)?;
        let shader = expect(pool, material.shader, ResourceKind::Shader)?;
        let shader = Shader::parse(&shader.blocks[0], &shader.imports, is_descriptor)?;
        if shader.vertex_descriptor.is_none() || shader.technique() != Some(PLAIN) {
            continue;
        }
        let Some(surface) = surface(&material, mesh.triangles(&resource.blocks[1])?.len()) else { continue };
        out.push(surface);
    }
    Ok(out)
}

/// The finish a material of the plain technique gives; `None` for a material with another layout.
fn surface(material: &Material, triangles: usize) -> Option<PlainSurface> {
    if material.constants.len() != SLOTS * 16 {
        return None;
    }
    let rgb = |slot: usize| material.float4(slot).map(|[r, g, b, _]| [r, g, b]);
    Some(PlainSurface {
        triangles,
        diffuse: rgb(DIFFUSE)?,
        specular: rgb(SPECULAR)?,
        roughness: material.float4(ROUGHNESS)?[0],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(slots: &[[f32; 4]]) -> Material {
        let constants = slots.iter().flatten().flat_map(|value| value.to_le_bytes()).collect();
        Material { shader: 0, textures: Vec::new(), constants }
    }

    #[test]
    fn a_material_of_seven_slots_gives_its_colours_and_roughness() {
        let slots = [
            [0.014, 0.0, 0.0, 0.0],
            [0.0; 4],
            [0.006, 0.006, 0.006, 1.0],
            [0.518, 0.0, 0.0, 0.0],
            [0.18, 0.18, 0.18, 1.0],
            [0.7, 0.3, 0.0, 0.0],
            [0.057, 0.0625, 0.057, 1.0],
        ];
        let found = surface(&material(&slots), 5071).unwrap();
        assert_eq!(found.triangles, 5071);
        assert_eq!((found.diffuse, found.roughness, found.specular), ([0.006; 3], 0.518, [0.057, 0.0625, 0.057]));
    }

    #[test]
    fn a_material_with_another_layout_is_left_out() {
        assert_eq!(surface(&material(&[[0.0; 4]; 3]), 10), None);
        assert_eq!(surface(&material(&[]), 10), None);
    }
}
