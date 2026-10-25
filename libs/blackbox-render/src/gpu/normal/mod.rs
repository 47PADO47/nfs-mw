//! Retained tangent-space scenery materials; per-frame light data is shared.

mod bindings;
pub(super) mod pipeline;
mod ray_pass;
#[cfg(test)]
mod reflect_tests;
#[cfg(test)]
mod surface_boundary_tests;
#[cfg(test)]
mod tests;
mod water;

use super::draw_list::{instance_store, instance_view};
use super::{Renderer, cull::Face, draw_list::LayerCmd, meshes::GpuMesh, pipelines::BLEND_ORDER, slots::Slots};
use bindings::material_group;
use blackbox_gfx::{BlendMode, FrameParams, NormalMaterialDesc, NormalMaterialHandle};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

struct Material {
    bind_group: wgpu::BindGroup,
    water_group: Option<wgpu::BindGroup>,
    model: blackbox_gfx::NormalModel,
    views: [wgpu::TextureView; 4],
    offset: wgpu::Buffer,
    uses_projected: bool,
    diffuse: blackbox_gfx::TextureHandle,
    hit_surface: NormalMaterialDesc,
}

pub(super) struct Normal {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    lighting: wgpu::Buffer,
    /// What `lighting` holds as of the last queued write, so views of one frame with the same data write it once.
    written_lighting: Option<[[f32; 4]; 4]>,
    black_reflection: wgpu::TextureView,
    projected: Option<wgpu::TextureView>,
    materials: Slots<Material>,
    /// Per target format and whether the variant discards fragments behind the clip plane (`(format, clip)`).
    pipelines: HashMap<(wgpu::TextureFormat, bool), pipeline::FaceSets>,
    /// Pick and build the clip-plane variants (a planar-reflection capture); the main view leaves it off.
    clip: bool,
    /// Instances are read from the instance store, bound at group 3.
    store: bool,
    water: water::Water,
    ray: Option<ray_pass::RayPass>,
}

impl Normal {
    pub(super) fn set_ray_bindings(
        &mut self,
        device: &wgpu::Device,
        rt: super::ray_reflections::Bindings,
        globals: &wgpu::Buffer,
    ) {
        if let Some(ray) = &mut self.ray {
            ray.set_scene(device, &rt, globals);
        }
    }
    fn new(
        device: &wgpu::Device,
        globals: &wgpu::BindGroupLayout,
        shadows: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let material_layout = pipeline::material_layout(device);
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("normal scenery"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), Some(shadows), instances.as_ref()]
                [..3 + usize::from(store)],
            immediate_size: 0,
        });
        let ray = ray_pass::RayPass::new(device, &material_layout, shadows, instances.as_ref());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("normal scenery shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}",
                    instance_view::instance_source(include_str!("../../shaders/normal.wgsl"), store.then_some(3)),
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2"),
                    include_str!("../../shaders/road_reflection.wgsl")
                )
                .into(),
            ),
        });
        let sampler = sampler.clone();
        let lighting = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("normal scenery shared specular"),
            contents: bytemuck::cast_slice(&[[0.0_f32; 4]; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // WebGPU initializes new textures to zero. Absence of a projected view adds no light.
        let black_reflection = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("absent projected reflection"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Self {
            shader,
            layout,
            material_layout,
            sampler,
            lighting,
            written_lighting: None,
            black_reflection,
            projected: None,
            materials: Slots::new(),
            pipelines: HashMap::new(),
            clip: false,
            store,
            water: water::Water::new(device),
            ray,
        }
    }

    /// Pick (and from the next [`Self::prepare`] on, build) the variants that discard fragments behind the clip plane.
    pub(super) fn use_clip(&mut self, clip: bool) {
        self.clip = clip;
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (format, culled): (wgpu::TextureFormat, bool),
        frame: &FrameParams,
        cache: Option<&wgpu::PipelineCache>,
        water: (blackbox_gfx::SurfaceWater, blackbox_gfx::RenderExperiments),
    ) {
        let (store, shader, layout, clip) = (self.store, &self.shader, &self.layout, self.clip);
        pipeline::fill_faces(self.pipelines.entry((format, clip)).or_default(), culled, |mode, face| {
            pipeline::create(device, shader, layout, format, (mode, face), true, store, clip, cache)
        });
        if let Some(ray) = &mut self.ray {
            ray.prepare(device, format, culled, store, cache);
        }
        let specular = frame
            .surface_light
            .map_or([0.0, 0.0, 0.0, 1.0], |l| [l.specular[0], l.specular[1], l.specular[2], l.specular_power]);
        let reflection = frame.surface_light.map_or(0.0, |l| l.reflection_scale);
        let highlight_scale = frame.surface_light.map_or(1.0, |l| l.reflective_specular_scale);
        let lighting = [
            specular,
            [
                reflection,
                highlight_scale,
                u8::from(
                    water.1.enhanced_wet_surfaces
                        && matches!(frame.projection, blackbox_gfx::Projection::PerspectiveInfiniteReverse { .. }),
                ) as f32,
                0.0,
            ],
            [water.0.dampness, water.0.pooling, water.0.time, water.0.rain],
            [
                u8::from(water.1.enhanced_wet_surfaces) as f32,
                water.1.puddle_coverage,
                water.1.puddle_scale,
                water.1.water_ripples,
            ],
        ];
        if self.written_lighting == Some(lighting) {
            return;
        }
        queue.write_buffer(&self.lighting, 0, bytemuck::cast_slice(&lighting));
        self.written_lighting = Some(lighting);
    }

    /// Draw the normal-map scenery of blend mode `mode`: `list` holds only those draws, in the order they were
    /// given. The material and the mesh's buffers are bound only when they change. `instances` is the instance
    /// store's bind group, when the renderer has one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        (mode, mirrored_view, water_pass): (BlendMode, bool, bool),
        meshes: &'a Slots<GpuMesh>,
        list: &[LayerCmd],
        shadows: &'a wgpu::BindGroup,
        instances: Option<&wgpu::BindGroup>,
    ) {
        if list.is_empty() {
            return;
        }
        let pipelines = match (water_pass, &self.ray) {
            (true, Some(ray)) if ray.enabled && ray.group.is_some() => ray.pipelines.get(&format),
            _ => self.pipelines.get(&(format, self.clip)),
        };
        let Some(pipelines) = pipelines else { return };
        let index = BLEND_ORDER.iter().position(|m| *m == mode).expect("known blend mode");
        let Some(two_sided) = pipeline::pick(pipelines, index, Face::Both) else { return };
        pass.set_pipeline(two_sided);
        let mut bound_face = Face::Both;
        pass.set_bind_group(2, shadows, &[]);
        if let Some(instances) = instances.filter(|_| self.store) {
            pass.set_bind_group(3, instances, &[]);
        }
        let mut bound_mesh = None;
        let mut bound_material = None;
        for cmd in list {
            let Some(mesh) = meshes.get(cmd.mesh) else { continue };
            let Some(draw) = mesh.draws.get(cmd.draw as usize) else { continue };
            let blackbox_gfx::Shading::WorldNormal(handle) = draw.shading else { continue };
            let Some(material) = self.materials.get(handle.raw()) else { continue };
            if water_pass && !material.uses_projected {
                continue;
            }
            let Some(tangents) = &mesh.tangents else { continue };
            if bound_material != Some(handle.raw()) {
                pass.set_bind_group(
                    1,
                    match water_pass {
                        true => material.water_group.as_ref().unwrap_or(&material.bind_group),
                        false => &material.bind_group,
                    },
                    &[],
                );
                bound_material = Some(handle.raw());
            }
            let face = cmd.face.seen(mirrored_view);
            if face != bound_face
                && let Some(p) = pipeline::pick(pipelines, index, face)
            {
                pass.set_pipeline(p);
                bound_face = face;
            }
            if bound_mesh != Some(cmd.mesh) {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(2, tangents.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                bound_mesh = Some(cmd.mesh);
            }
            pass.draw_indexed(
                draw.first_index..draw.first_index + draw.index_count,
                draw.base_vertex,
                cmd.instances.0..cmd.instances.1,
            );
        }
    }

    pub(super) fn reflective_material(&self, handle: NormalMaterialHandle) -> bool {
        self.materials.get(handle.raw()).is_some_and(|m| m.model == blackbox_gfx::NormalModel::Reflective)
    }
}

impl Renderer {
    pub(super) fn prepare_normal(&mut self, frame: &FrameParams, format: wgpu::TextureFormat, culled: bool) {
        if self.experiments.enhanced_wet_surfaces {
            self.ensure_glossy();
        }
        let environment = self.glossy.as_ref().map(|g| g.road_environment());
        let Some(normal) = &mut self.normal else { return };
        if let Some((environment, base, filtered)) = environment {
            if normal.water.base_environment != base {
                normal.water.base_environment = base;
                normal.rebind_water(&self.device);
            }
            if normal.water.environment != environment || normal.water.filtered_environment != filtered {
                normal.water.environment = environment;
                normal.water.filtered_environment = filtered;
                for material in normal.materials.iter_mut() {
                    material.water_group = None;
                }
            }
        }
        normal.prepare(
            &self.device,
            &self.queue,
            (format, culled),
            frame,
            self.pipeline_cache.as_ref(),
            (self.surface_water, self.experiments),
        );
    }

    pub(super) fn reflective_surface(&self, shading: blackbox_gfx::Shading) -> bool {
        if !self.experiments.original_world_materials {
            return false;
        }
        let blackbox_gfx::Shading::WorldNormal(handle) = shading else { return false };
        self.normal.as_ref().is_some_and(|normal| normal.reflective_material(handle))
    }

    pub fn create_normal_material(&mut self, desc: &NormalMaterialDesc) -> Option<NormalMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let normal_map = self.texture_views.get(&desc.normal.raw())?;
        let specular = self.texture_views.get(&desc.specular.raw())?;
        let normal = self.normal.get_or_insert_with(|| {
            Normal::new(
                &self.device,
                &self.shared.bindings.globals_layout,
                &self.shared.glossy.material,
                &self.shared.bindings.sampler,
            )
        });
        if normal.ray.as_ref().is_some_and(|ray| ray.group.is_none())
            && let Some(rt) = &self.ray_reflections
        {
            normal.set_ray_bindings(&self.device, rt.bindings.clone(), &self.shared.bindings.globals);
        }
        normal.set_projected(&self.device, self.projected.as_ref().map(|p| p.output()));
        let reflection = match desc.reflection {
            Some(texture) => self.texture_views.get(&texture.raw())?,
            None if desc.model == blackbox_gfx::NormalModel::Reflective => {
                normal.projected.as_ref().unwrap_or(&normal.black_reflection)
            }
            None => &normal.black_reflection,
        };
        let offset = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("normal scenery UV offset"),
            contents: bytemuck::cast_slice(&[
                desc.uv_offset[0],
                desc.uv_offset[1],
                u8::from(desc.model == blackbox_gfx::NormalModel::Reflective) as f32,
                u8::from(desc.reflection.is_none()) as f32,
            ]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let views = [diffuse.clone(), normal_map.clone(), specular.clone(), reflection.clone()];
        let bind_group = material_group(
            &self.device,
            &normal.material_layout,
            &normal.sampler,
            &normal.lighting,
            &views,
            &offset,
            &normal.water,
            false,
        );
        Some(NormalMaterialHandle::from_raw(normal.materials.insert(Material {
            bind_group,
            water_group: None,
            model: desc.model,
            uses_projected: desc.reflection.is_none() && desc.model == blackbox_gfx::NormalModel::Reflective,
            diffuse: desc.diffuse,
            hit_surface: *desc,
            views,
            offset,
        })))
    }

    pub fn destroy_normal_material(&mut self, handle: NormalMaterialHandle) {
        if let Some(normal) = &mut self.normal {
            normal.materials.remove(handle.raw());
        }
    }
}
