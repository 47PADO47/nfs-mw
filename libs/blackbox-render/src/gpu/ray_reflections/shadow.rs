//! Shadow-only eligibility and alpha rules alongside the shared reflection geometry.
use super::{Scene, mesh::Mesh};
use blackbox_gfx::{
    BlendMode, FrameParams, Instance, MeshDesc, NormalMaterialDesc, NormalModel, RenderExperiments, Shading,
};
use std::collections::HashMap;

pub(super) fn classify(
    mesh: &mut Mesh,
    desc: &MeshDesc<'_>,
    layers: &[Option<u32>],
    surfaces: &[Option<NormalMaterialDesc>],
) {
    let mut first = HashMap::new();
    let mut copies = HashMap::new();
    let mut at = 0;
    for ((draw, layer), surface) in desc.draws.iter().zip(layers).zip(surfaces) {
        if layer.is_none()
            || !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest)
            || matches!(draw.shading, Shading::Sky | Shading::SkyLayer(_))
        {
            continue;
        }
        // 0 opaque, 1 ordinary cutout, 2 tree/window, 3 doubled prelit, 4 reflective normal, 5 world.
        let mode = match (draw.blend, draw.shading) {
            (BlendMode::AlphaTest, Shading::Tree | Shading::WorldWindow(_)) => 2,
            (BlendMode::AlphaTest, Shading::WorldPrelit) => 3,
            (BlendMode::AlphaTest, Shading::World | Shading::WorldNoFog) => 5,
            (BlendMode::AlphaTest, Shading::WorldNormal(_))
                if surface.is_some_and(|s| s.model == NormalModel::Reflective) =>
            {
                4
            }
            (BlendMode::AlphaTest, _) => 1,
            _ => 0,
        };
        let end = at + draw.index_count as usize / 3;
        for triangle in &mut mesh.triangles[at..end] {
            for index in &mut triangle[..3] {
                let original = *index;
                let previous = *first.entry(original).or_insert(mode);
                if previous == mode {
                    mesh.vertices[original as usize].normal[3] = mode as f32;
                    continue;
                }
                *index = *copies.entry((original, mode)).or_insert_with(|| {
                    let mut vertex = mesh.vertices[original as usize];
                    vertex.normal[3] = mode as f32;
                    let id = mesh.vertices.len() as u32;
                    mesh.vertices.push(vertex);
                    id
                });
            }
        }
        at = end;
    }
}
impl Scene {
    pub(super) fn select_shadow_casters(
        &mut self,
        candidates: &[Instance],
        frame: &FrameParams,
        options: &RenderExperiments,
        active: bool,
    ) {
        self.shadow.clear();
        if !active {
            return;
        }
        let sun = (-frame.light_dir).normalize_or_zero();
        let spread = match options.ray_soft_shadows {
            true => options.ray_shadow_length * options.sun_angle.to_radians().tan(),
            false => 0.0,
        };
        for instance in candidates {
            if !instance.shadow.cast {
                continue;
            }
            let Some(mesh) = self.meshes.get(&instance.mesh.raw()) else { continue };
            let center = instance.transform.transform_point3(mesh.center) - frame.camera_position;
            // Frobenius norm is a conservative scale bound, including sheared instances.
            let scale = (instance.transform.x_axis.truncate().length_squared()
                + instance.transform.y_axis.truncate().length_squared()
                + instance.transform.z_axis.truncate().length_squared())
            .sqrt();
            let radius = mesh.radius * scale;
            let travel = center.dot(sun).clamp(0.0, options.ray_shadow_length);
            if (center - sun * travel).length() > options.ray_shadow_range + radius + spread {
                continue;
            }
            self.shadow.push(*instance);
        }
    }
}
