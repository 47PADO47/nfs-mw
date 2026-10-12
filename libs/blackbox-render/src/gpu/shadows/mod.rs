//! Prototype raster sun visibility. Separate caster lists never enter the colour pass.
mod fit;
pub(in crate::gpu) mod maps;
mod pipelines;
#[cfg(test)]
mod stability_tests;
#[cfg(test)]
mod tests;

use super::{Renderer, instances::InstanceBuffer};
use blackbox_gfx::{BlendMode, ExperimentStats, FrameParams, Instance, Shading};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};
use std::time::Instant;

pub(super) struct Shadows {
    opaque: wgpu::RenderPipeline,
    cutout: wgpu::RenderPipeline,
    buffers: Vec<wgpu::Buffer>,
    groups: Vec<wgpu::BindGroup>,
    instances: Vec<InstanceBuffer>,
    bounds: Vec<(usize, Vec3, f32)>,
    selected: Vec<usize>,
    matrices: Vec<[f32; 16]>,
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
        let fit = (settings.sun_shadows && stats.supported).then(|| fit::fit(frame, settings)).flatten();
        let resolution = fit.as_ref().map_or(1, |_| settings.shadow_resolution);
        if self.shared.shadows.resolution != resolution {
            self.shared.shadows = maps::Maps::new(
                &self.device,
                &self.shared.glossy.material,
                resolution,
                Some(self.shared.shadows.contact.clone()),
                Some(self.shared.shadows.ambient.clone()),
            );
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        stats.map_mib = resolution * resolution * 16 / (1024 * 1024);
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
        let shadows =
            self.shadows.get_or_insert_with(|| Shadows::new(&self.device, &self.shared.bindings.texture_layout));
        let casters = self.shadow_casters.as_deref().unwrap_or(visible);
        stats.cascades = fit.count as u32;
        shadows.bounds.clear();
        for (index, instance) in casters.iter().enumerate() {
            let Some(mesh) = self.meshes.get(instance.mesh.raw()) else { continue };
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
        for cascade in 0..fit.count {
            let matrix = Mat4::from_cols_array(&fit.params.matrices[cascade]);
            let rows = matrix.transpose();
            let axes = Vec3::new(
                rows.x_axis.truncate().length(),
                rows.y_axis.truncate().length(),
                rows.z_axis.truncate().length(),
            );
            shadows.selected.clear();
            shadows.matrices.clear();
            for &(index, center, radius) in &shadows.bounds {
                if !intersects(matrix, axes, center, radius) {
                    continue;
                }
                shadows.selected.push(index);
                shadows.matrices.push(casters[index].transform.to_cols_array());
            }
            let selected = &shadows.selected;
            shadows.instances[cascade].upload(&self.device, &self.queue, &shadows.matrices);
            self.queue.write_buffer(&shadows.buffers[cascade], 0, bytemuck::cast_slice(&fit.params.matrices[cascade]));
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
            let mut first = 0;
            while first < selected.len() {
                let mesh_handle = casters[selected[first]].mesh;
                let end =
                    first + selected[first..].iter().take_while(|&&index| casters[index].mesh == mesh_handle).count();
                let mesh = self.meshes.get(mesh_handle.raw()).expect("selected resident mesh");
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                for draw in &mesh.draws {
                    if draw.shading == Shading::Sky || !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest) {
                        continue;
                    }
                    let pipeline = match draw.blend {
                        BlendMode::AlphaTest => &shadows.cutout,
                        _ => &shadows.opaque,
                    };
                    pass.set_pipeline(pipeline);
                    let slot = draw.texture.map(|t| self.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                    let texture = slot.and_then(|s| self.textures.get(s)).or(self.textures.get(0));
                    let Some(texture) = texture else { continue };
                    pass.set_bind_group(1, texture, &[]);
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

fn intersects(matrix: Mat4, axes: Vec3, center: Vec3, radius: f32) -> bool {
    let p = matrix.transform_point3(center);
    let r = axes * radius;
    p.x + r.x >= -1.0
        && p.x - r.x <= 1.0
        && p.y + r.y >= -1.0
        && p.y - r.y <= 1.0
        && p.z + r.z >= 0.0
        && p.z - r.z <= 1.0
}
