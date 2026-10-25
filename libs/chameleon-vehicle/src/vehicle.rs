use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chameleon_bundle::{Pool, ResourceKind};
use chameleon_gfx::{Material, Model, Renderable, Shader, Texture, Vertex, VertexDescriptor};

use crate::appearance::Appearance;
use crate::error::{Error, Result};
use crate::spec::GraphicsSpec;

/// One draw of a car part: its geometry and how it looks.
#[derive(Debug, Clone, PartialEq)]
pub struct PartMesh {
    pub vertices: Vec<Vertex>,
    /// Counter-clockwise outward in the engine's right-handed space.
    pub triangles: Vec<[u32; 3]>,
    pub appearance: Appearance,
}

/// The meshes of one level of detail.
pub type Lod = Vec<PartMesh>;

/// A wheel with the meshes of all its models, around the wheel's own origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Wheel {
    pub name: String,
    pub position: [f32; 3],
    /// Level 0 is the most detailed. A level holds the meshes of every model of the wheel at that level (a model that
    /// has fewer levels contributes its last one).
    pub lods: Vec<Lod>,
}

/// A car at every level of detail.
#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    /// Level 0 is the most detailed.
    pub body: Vec<Lod>,
    pub wheels: Vec<Wheel>,
}

impl Vehicle {
    /// Every mesh at every level, body first.
    pub fn all_meshes(&self) -> impl Iterator<Item = &PartMesh> {
        let wheels = self.wheels.iter().flat_map(|wheel| wheel.lods.iter().flatten());
        self.body.iter().flatten().chain(wheels)
    }
}

/// Assemble the vehicle whose graphics spec is `spec`, resolving every resource through `pool`. The pool must hold the
/// vehicle bundle, the shared texture bundle and the shader bundle.
pub fn load(pool: &Pool, spec: u64) -> Result<Vehicle> {
    let spec = expect(pool, spec, ResourceKind::VehicleGraphicsSpec)?;
    let spec = GraphicsSpec::parse(&spec.blocks[0], &spec.imports)?;
    let surfaces = Surfaces::default();
    let body = model_lods(pool, &surfaces, spec.body_model)?;
    let mut wheels = Vec::with_capacity(spec.wheels.len());
    for wheel in &spec.wheels {
        let models =
            wheel.models.iter().map(|&model| model_lods(pool, &surfaces, model)).collect::<Result<Vec<_>>>()?;
        let levels = models.iter().map(Vec::len).max().unwrap_or(0);
        let lods = (0..levels)
            .map(|level| models.iter().filter_map(|lods| lods.get(level).or(lods.last())).flatten().cloned().collect())
            .collect();
        wheels.push(Wheel { name: wheel.name.clone(), position: wheel.position, lods });
    }
    Ok(Vehicle { body, wheels })
}

/// The texture resource `id`, decoded.
pub fn load_texture(pool: &Pool, id: u64) -> Result<Texture> {
    let resource = expect(pool, id, ResourceKind::Texture)?;
    Ok(Texture::parse(&resource.blocks[0], &resource.blocks[1])?)
}

fn expect(pool: &Pool, id: u64, kind: ResourceKind) -> Result<chameleon_bundle::Resource> {
    let resource = pool.resource(id)?;
    if resource.entry.kind != kind.id() {
        return Err(Error::WrongKind { id, found: resource.entry.kind, expected: kind.id() });
    }
    Ok(resource)
}

/// The meshes of every level of a model, most detailed first.
fn model_lods(pool: &Pool, surfaces: &Surfaces, model: u64) -> Result<Vec<Lod>> {
    let resource = expect(pool, model, ResourceKind::Model)?;
    let model = Model::parse(&resource.blocks[0], &resource.imports)?;
    model.lods.iter().map(|&lod| renderable_meshes(pool, surfaces, lod)).collect()
}

fn renderable_meshes(pool: &Pool, surfaces: &Surfaces, lod: u64) -> Result<Lod> {
    let resource = expect(pool, lod, ResourceKind::Renderable)?;
    let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
    let graphics = &resource.blocks[1];
    let mut meshes = Vec::with_capacity(renderable.meshes.len());
    for mesh in &renderable.meshes {
        let Some(surface) = surfaces.get(pool, mesh.material)? else { continue };
        let (descriptor, appearance) = (&surface.0, surface.1.clone());
        let vertices = descriptor.decode(mesh.vertex_bytes(graphics)?)?;
        let triangles = mesh.triangles(graphics)?;
        let in_range = triangles.iter().flatten().all(|&index| (index as usize) < vertices.len());
        if !in_range {
            return Err(Error::Spec(format!("a mesh of renderable {lod:016X} indexes past its vertices")));
        }
        meshes.push(PartMesh { vertices, triangles, appearance });
    }
    Ok(meshes)
}

/// The vertex layout and the look of each material seen so far: a car's meshes share a few dozen materials across its
/// levels of detail and its wheels, and each costs three inflates and a parse the first time.
/// A material's vertex layout and look.
type Surface = Rc<(VertexDescriptor, Appearance)>;

#[derive(Default)]
struct Surfaces {
    known: RefCell<HashMap<u64, Option<Surface>>>,
}

impl Surfaces {
    /// The surface of `material`, or `None` when the material's shader has no layout we can read.
    fn get(&self, pool: &Pool, material: u64) -> Result<Option<Surface>> {
        if let Some(found) = self.known.borrow().get(&material) {
            return Ok(found.clone());
        }
        let made = surface(pool, material)?.map(Rc::new);
        self.known.borrow_mut().insert(material, made.clone());
        Ok(made)
    }
}

/// The vertex layout and the look of a material, or `None` when the material's shader has no layout we can read.
fn surface(pool: &Pool, material: u64) -> Result<Option<(VertexDescriptor, Appearance)>> {
    let resource = expect(pool, material, ResourceKind::Material)?;
    let material = Material::parse(&resource.blocks[0], &resource.imports)?;
    let shader_resource = expect(pool, material.shader, ResourceKind::Shader)?;
    let is_descriptor = |id: u64| pool.kind(id) == Some(ResourceKind::VertexDescriptor.id());
    let shader = Shader::parse(&shader_resource.blocks[0], &shader_resource.imports, is_descriptor)?;
    let Some(descriptor_id) = shader.vertex_descriptor else { return Ok(None) };
    let descriptor = expect(pool, descriptor_id, ResourceKind::VertexDescriptor)?;
    let descriptor = VertexDescriptor::parse(&descriptor.blocks[0])?;
    Ok(Some((descriptor, Appearance::new(&shader, &material))))
}
