//! Prototype raster sun visibility. Separate caster lists never enter the colour pass.
#[cfg(test)]
mod coverage_tests;
mod fit;
#[cfg(test)]
mod foliage_tests;
pub(in crate::gpu) mod maps;
mod pipelines;
mod projected;
mod projected_fit;
#[cfg(test)]
mod stability_tests;
#[cfg(test)]
mod surface_tests;
#[cfg(test)]
mod tests;

use super::{Renderer, instances::InstanceBuffer};
use blackbox_gfx::{BlendMode, ExperimentStats, FrameParams, Instance, Shading};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3, Vec4};
use std::time::Instant;

pub(super) struct Shadows {
    bias: [f32; 2],
    opaque: wgpu::RenderPipeline,
    cutout: wgpu::RenderPipeline,
    tree_cutout: wgpu::RenderPipeline,
    prelit_cutout: wgpu::RenderPipeline,
    buffers: Vec<wgpu::Buffer>,
    groups: Vec<wgpu::BindGroup>,
    instances: Vec<InstanceBuffer>,
    bounds: Vec<(usize, Vec3, f32)>,
    selected: Vec<usize>,
    matrices: Vec<[f32; 16]>,
    fit_points: Vec<Vec3>,
}

impl Renderer {
    pub(super) fn encode_shadows(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameParams,
        visible: &[Instance],
    ) {
        let start = Instant::now();
        let mut stats =
            ExperimentStats { supported: self.info.api != blackbox_gfx::GraphicsApi::Gl, ..Default::default() };
        let settings = self.experiments;
        if settings.sun_shadows && stats.supported {
            let bias = settings.shadow_projection.map_or([1.0, 1.0], |projection| projection.depth_bias);
            if self.shadows.as_ref().is_none_or(|shadows| shadows.bias != bias) {
                self.shadows = Some(Shadows::new(
                    &self.device,
                    &self.shared.bindings.texture_layout,
                    bias,
                    self.pipeline_cache.as_ref(),
                ));
            }
            let shadows = self.shadows.as_mut().expect("prepared shadow pipelines");
            shadows.bounds.clear();
            let casters = self.shadow_casters.as_deref().unwrap_or(visible);
            for (index, instance) in casters.iter().enumerate() {
                let Some(mesh) = self.meshes.get(instance.mesh.raw()) else { continue };
                if !mesh.draws.iter().any(|draw| draw.index_count > 0 && casts(draw.blend, draw.shading)) {
                    continue;
                }
                let center = instance.transform.transform_point3(mesh.center);
                let scale = instance
                    .transform
                    .x_axis
                    .truncate()
                    .length()
                    .max(instance.transform.y_axis.truncate().length())
                    .max(instance.transform.z_axis.truncate().length());
                shadows.bounds.push((index, center, mesh.radius * scale));
            }
        }
        let fit = (settings.sun_shadows && stats.supported)
            .then(|| match settings.shadow_projection {
                Some(_) => {
                    let shadows = self.shadows.as_mut().expect("prepared bounds");
                    projected::fit(frame, settings, &shadows.bounds, &mut shadows.fit_points)
                }
                None => fit::fit(frame, settings, &self.shadows.as_ref().expect("prepared bounds").bounds),
            })
            .flatten();
        let resolution = fit.as_ref().map_or(1, |_| settings.shadow_resolution);
        let layers = match settings.shadow_projection.is_some() {
            true => 1,
            false => 4,
        };
        if self.shared.shadows.resolution != resolution || self.shared.shadows.layer_count != layers {
            self.shared.shadows = maps::Maps::new(
                &self.device,
                &self.shared.glossy.material,
                resolution,
                layers,
                Some(self.shared.shadows.contact.clone()),
                Some(self.shared.shadows.ambient.clone()),
            );
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        stats.map_mib = resolution * resolution * 4 * layers / (1024 * 1024);
        let mut params = fit.as_ref().map_or_else(fit::Params::zeroed, |f| f.params);
        params.options[3] = u8::from(settings.clearcoat) as f32;
        if settings.contact_shadows
            && matches!(frame.projection, blackbox_gfx::Projection::PerspectiveInfiniteReverse { .. })
            && stats.supported
            && let Some(contact) = &self.contact
        {
            let size = contact.size();
            params.contact = [size.0 as f32, size.1 as f32, 1.0, 0.0];
        }
        params.environment = [
            u8::from(settings.car_skylight && self.supports_irradiance()) as f32,
            settings.sky_strength,
            settings.ground_strength,
            0.0,
        ];
        params.world = [
            u8::from(settings.world_ambient && stats.supported) as f32,
            settings.world_ambient_share,
            settings.world_ambient_tint,
            0.0,
        ];
        params.debug[0] = match stats.supported {
            true => settings.lighting_debug as u32 as f32,
            false => 0.0,
        };
        if self.graphics.ao.active()
            && self.screen_lighting_active(frame)
            && let Some(surfaces) = &self.surfaces
        {
            let size = surfaces.targets.size;
            params.ambient = [size.0 as f32, size.1 as f32, 1.0, 0.0];
        }
        if !stats.supported {
            params.options[0] = 0.0;
            params.environment[0] = 0.0;
        }
        self.queue.write_buffer(&self.shared.shadows.buffer, 0, bytemuck::bytes_of(&params));
        let Some(fit) = fit.filter(|_| settings.sun_shadows && stats.supported) else {
            self.experiment_stats = stats;
            return;
        };
        let shadows = self.shadows.as_mut().expect("prepared shadow pipelines");
        let casters = self.shadow_casters.as_deref().unwrap_or(visible);
        stats.cascades = fit.count as u32;
        for cascade in 0..fit.count {
            let mut matrix = Mat4::from_cols_array(&fit.params.matrices[cascade]);
            if settings.shadow_projection.is_some() {
                // Match the integer-center source raster grid, paired with the
                // receiver's half-texel texture transform on modern pixel centers.
                let texel = 1.0 / settings.shadow_resolution as f32;
                matrix = Mat4::from_translation(Vec3::new(texel, -texel, 0.0)) * matrix;
            }
            let planes = clip_planes(matrix);
            shadows.selected.clear();
            shadows.matrices.clear();
            for &(index, center, radius) in &shadows.bounds {
                if !intersects(&planes, center, radius) {
                    continue;
                }
                shadows.selected.push(index);
                shadows.matrices.push(casters[index].transform.to_cols_array());
            }
            let selected = &shadows.selected;
            shadows.instances[cascade].upload(&self.device, &self.queue, &shadows.matrices);
            self.queue.write_buffer(&shadows.buffers[cascade], 0, bytemuck::cast_slice(&matrix.to_cols_array()));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("spatial sun cascade"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shared.shadows.layers[cascade],
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let size = settings.shadow_resolution as f32;
            pass.set_viewport(0.0, 0.0, size, size, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, settings.shadow_resolution, settings.shadow_resolution);
            pass.set_bind_group(0, &shadows.groups[cascade], &[]);
            pass.set_vertex_buffer(1, shadows.instances[cascade].buffer.slice(..));
            let mut bound_pipeline: Option<&wgpu::RenderPipeline> = None;
            let mut bound_texture: Option<&wgpu::BindGroup> = None;
            let mut first = 0;
            while first < selected.len() {
                let mesh_handle = casters[selected[first]].mesh;
                let end =
                    first + selected[first..].iter().take_while(|&&index| casters[index].mesh == mesh_handle).count();
                let mesh = self.meshes.get(mesh_handle.raw()).expect("selected resident mesh");
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                for draw in &mesh.draws {
                    if !casts(draw.blend, draw.shading) {
                        continue;
                    }
                    let pipeline = match draw.blend {
                        BlendMode::AlphaTest
                            if settings.original_world_materials
                                && matches!(draw.shading, Shading::WorldNormal(handle)
                                if self.normal.as_ref().is_some_and(|n| n.reflective_material(handle))) =>
                        {
                            &shadows.opaque
                        }
                        BlendMode::AlphaTest
                            if settings.original_world_materials
                                && matches!(draw.shading, Shading::Tree | Shading::WorldWindow(_)) =>
                        {
                            &shadows.tree_cutout
                        }
                        BlendMode::AlphaTest
                            if settings.original_world_materials && draw.shading == Shading::WorldPrelit =>
                        {
                            &shadows.prelit_cutout
                        }
                        BlendMode::AlphaTest => &shadows.cutout,
                        _ => &shadows.opaque,
                    };
                    if bound_pipeline.is_none_or(|bound| !std::ptr::eq(bound, pipeline)) {
                        pass.set_pipeline(pipeline);
                        bound_pipeline = Some(pipeline);
                    }
                    let slot = draw.texture.map(|t| self.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                    let texture = slot.and_then(|s| self.textures.get(s)).or(self.textures.get(0));
                    let Some(texture) = texture else { continue };
                    if bound_texture.is_none_or(|bound| !std::ptr::eq(bound, texture)) {
                        pass.set_bind_group(1, texture, &[]);
                        bound_texture = Some(texture);
                    }
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        draw.base_vertex,
                        first as u32..end as u32,
                    );
                    stats.shadow_draws += 1;
                    stats.shadow_instances += (end - first) as u32;
                }
                first = end;
            }
        }
        stats.shadow_cpu_ms = start.elapsed().as_secs_f32() * 1000.0;
        self.experiment_stats = stats;
    }
}

fn casts(blend: BlendMode, shading: Shading) -> bool {
    matches!(blend, BlendMode::Opaque | BlendMode::AlphaTest) && !matches!(shading, Shading::Sky | Shading::SkyLayer(_))
}

fn clip_planes(matrix: Mat4) -> [(Vec4, f32); 6] {
    let rows = matrix.transpose();
    [
        rows.w_axis + rows.x_axis,
        rows.w_axis - rows.x_axis,
        rows.w_axis + rows.y_axis,
        rows.w_axis - rows.y_axis,
        rows.z_axis,
        rows.w_axis - rows.z_axis,
    ]
    .map(|plane| (plane, plane.truncate().length()))
}

fn intersects(planes: &[(Vec4, f32); 6], center: Vec3, radius: f32) -> bool {
    planes.iter().all(|(plane, length)| plane.dot(center.extend(1.0)) >= -radius * length)
}
