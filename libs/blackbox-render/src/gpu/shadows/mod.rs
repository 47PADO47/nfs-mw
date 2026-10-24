//! Prototype raster sun visibility. Separate caster lists never enter the colour pass.
mod cache;
#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod coverage_tests;
#[cfg(test)]
mod density_tests;
mod draw;
mod fit;
#[cfg(test)]
mod foliage_tests;
mod grid;
pub(in crate::gpu) mod maps;
#[cfg(test)]
mod motion_tests;
mod pipelines;
mod projected;
mod projected_fit;
#[cfg(test)]
mod retention_tests;
#[cfg(test)]
mod sampling_tests;
#[cfg(test)]
mod stability_tests;
mod stagger;
#[cfg(test)]
mod surface_tests;
#[cfg(test)]
mod tests;
mod upload;
#[cfg(test)]
mod wide_tests;

use super::{Renderer, instances::InstanceBuffer};
use blackbox_gfx::{BlendMode, ExperimentStats, FrameParams, Instance, Shading};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3, Vec4};
use std::time::Instant;

pub(super) struct Shadows {
    grid: grid::Grid,
    cache: cache::Cache,
    static_instances: Vec<InstanceBuffer>,
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
    scratch: upload::CasterScratch,
    /// Each caster's slot in the instance store, when the casters are read from it.
    caster_slots: Vec<u32>,
    /// The casters are read from the instance store (bind group 2) through a slot each.
    store: bool,
    /// Scratch partitions of `selected` into static and moving casters.
    fixed: Vec<usize>,
    moving: Vec<usize>,
    fit_points: Vec<Vec3>,
    rendered: stagger::Rendered,
    /// Far cascades refresh on their `stagger::PERIOD`. Image tests need every cascade fresh each frame.
    stagger: bool,
}

impl Renderer {
    pub(super) fn encode_shadows(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameParams,
        visible: &[Instance],
    ) {
        let start = Instant::now();
        let _trace = blackbox_trace::scope("Renderer CPU: shadows");
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
                if !instance.shadow.cast {
                    continue;
                }
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
                None => {
                    let shadows = self.shadows.as_mut().expect("prepared bounds");
                    fit::fit_with_grid(frame, settings, &shadows.bounds, &mut shadows.grid)
                }
            })
            .flatten();
        let resolution = fit.as_ref().map_or(1, |_| settings.shadow_resolution);
        let layers = match settings.shadow_projection.is_some() {
            true => 1,
            false => 4,
        };
        let detail = (fit.is_some() && settings.shadow_projection.is_none() && settings.wide_shadow_detail)
            .then_some(settings.shadow_detail_resolution);
        // UI/clip-space views cannot fit sun shadows. Keep the world atlas until
        // an explicit disable or a different active configuration replaces it.
        let retain_maps = fit.is_none() && settings.sun_shadows && stats.supported;
        if !retain_maps
            && (self.shared.shadows.resolution != resolution
                || self.shared.shadows.layer_count != layers
                || self.shared.shadows.detail_resolution != detail)
        {
            if let Some(shadows) = &mut self.shadows {
                shadows.rendered.reset();
            }
            self.shared.shadows = maps::Maps::new(
                &self.device,
                &self.shared.glossy.material,
                resolution,
                layers,
                detail,
                Some(self.shared.shadows.contact.clone()),
                Some(self.shared.shadows.ambient.clone()),
            );
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        let maps = &self.shared.shadows;
        let pixels = match maps.detail_resolution {
            Some(size) => size * size * 2 + maps.resolution * maps.resolution * (maps.layer_count - 2),
            None => maps.resolution * maps.resolution * maps.layer_count,
        };
        stats.map_mib = pixels * 4 / (1024 * 1024);
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
            settings.sky_strength * settings.indirect_strength,
            settings.ground_strength * settings.indirect_strength,
            u8::from(settings.paint_response) as f32,
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
        // Cascades that skip this frame keep sampling with the matrix their map was rendered with.
        let mut due = [true; 4];
        if let Some(fitted) = fit.as_ref().filter(|_| settings.sun_shadows && stats.supported)
            && let Some(shadows) = self.shadows.as_mut()
        {
            let force = !shadows.stagger || frame.camera_cut || settings.shadow_projection.is_some();
            due = shadows.rendered.plan(&mut params, fitted.count, force);
        }
        // Glossy draws of a queued capture read the environment lanes: other ones send the capture first.
        self.guard_shadow_environment(params.environment);
        self.queue.write_buffer(&self.shared.shadows.buffer, 0, bytemuck::bytes_of(&params));
        let Some(fit) = fit.filter(|_| settings.sun_shadows && stats.supported) else {
            if let Some(shadows) = &mut self.shadows {
                if !retain_maps || !settings.cached_shadows {
                    shadows.cache = cache::Cache::default();
                }
                stats.map_mib += shadows.cache.mib();
            }
            self.experiment_stats = stats;
            return;
        };
        let mut shadows = self.shadows.take().expect("prepared shadow pipelines");
        let casters = self.shadow_casters.as_deref().unwrap_or(visible);
        if shadows.store {
            let main_view = self.shadow_casters.is_none();
            let store = &mut self.draw_list.store;
            upload::caster_slots(store, &self.device, &self.queue, casters, main_view, &mut shadows.caster_slots);
        }
        if settings.cached_shadows {
            shadows.cache.prepare(casters);
        }
        if !settings.cached_shadows {
            shadows.cache = cache::Cache::default();
        }
        stats.cascades = fit.count as u32;
        let mut fixed = std::mem::take(&mut shadows.fixed);
        let mut moving = std::mem::take(&mut shadows.moving);
        for cascade in 0..fit.count {
            if !due[cascade] {
                continue;
            }
            let _trace = blackbox_trace::scope(
                ["Shadow CPU: cascade 0", "Shadow CPU: cascade 1", "Shadow CPU: cascade 2", "Shadow CPU: cascade 3"]
                    [cascade],
            );
            let mut matrix = Mat4::from_cols_array(&fit.params.matrices[cascade]);
            if settings.shadow_projection.is_some() {
                // Match the integer-center source raster grid, paired with the
                // receiver's half-texel texture transform on modern pixel centers.
                let texel = 1.0 / settings.shadow_resolution as f32;
                matrix = Mat4::from_translation(Vec3::new(texel, -texel, 0.0)) * matrix;
            }
            let planes = clip_planes(matrix);
            shadows.selected.clear();
            for &(index, center, radius) in &shadows.bounds {
                if !intersects(&planes, center, radius) {
                    continue;
                }
                shadows.selected.push(index);
            }
            let selected = &shadows.selected;
            // Each draw below uploads just the casters it draws, into the buffer it binds.
            let slots = shadows.store.then_some(shadows.caster_slots.as_slice());
            self.queue.write_buffer(&shadows.buffers[cascade], 0, bytemuck::cast_slice(&matrix.to_cols_array()));
            let target = &self.shared.shadows.layers[cascade];
            let size = fit.params.resolutions[cascade];
            let projection_stable = shadows.cache.projection(cascade, matrix, frame.camera_cut);
            if settings.cached_shadows && settings.shadow_projection.is_none() && projection_stable {
                fixed.clear();
                moving.clear();
                for &i in selected {
                    match shadows.cache.stable[i] {
                        true => fixed.push(i),
                        false => moving.push(i),
                    }
                }
                if !fixed.is_empty() {
                    let fingerprint = shadows.cache.fingerprint(self, matrix, &fixed, casters);
                    let (view, dirty, reuse_final) = shadows.cache.image(
                        &self.device,
                        cascade,
                        size as u32,
                        fingerprint,
                        target,
                        !moving.is_empty(),
                    );
                    if dirty {
                        let buffer = &mut shadows.static_instances[cascade];
                        shadows.scratch.upload(&self.device, &self.queue, buffer, casters, &fixed, slots);
                        shadows.draw_selected(
                            self,
                            encoder,
                            cascade,
                            &view,
                            size,
                            wgpu::LoadOp::Clear(1.0),
                            &shadows.static_instances[cascade].buffer,
                            &fixed,
                            casters,
                            &mut stats,
                        );
                    }
                    if !dirty {
                        stats.shadow_cached_cascades += 1;
                    }
                    if reuse_final {
                        continue;
                    }
                    let layer = match detail {
                        Some(_) if cascade >= 2 => cascade - 2,
                        _ => cascade,
                    };
                    let mut destination = target.texture().as_image_copy();
                    destination.origin.z = layer as u32;
                    encoder.copy_texture_to_texture(
                        view.texture().as_image_copy(),
                        destination,
                        wgpu::Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 },
                    );
                    let buffer = &mut shadows.instances[cascade];
                    shadows.scratch.upload(&self.device, &self.queue, buffer, casters, &moving, slots);
                    shadows.draw_selected(
                        self,
                        encoder,
                        cascade,
                        target,
                        size,
                        wgpu::LoadOp::Load,
                        &shadows.instances[cascade].buffer,
                        &moving,
                        casters,
                        &mut stats,
                    );
                    continue;
                }
            }
            shadows.cache.invalidate_final(cascade);
            let buffer = &mut shadows.instances[cascade];
            shadows.scratch.upload(&self.device, &self.queue, buffer, casters, selected, slots);
            shadows.draw_selected(
                self,
                encoder,
                cascade,
                target,
                size,
                wgpu::LoadOp::Clear(1.0),
                &shadows.instances[cascade].buffer,
                selected,
                casters,
                &mut stats,
            );
        }
        shadows.fixed = fixed;
        shadows.moving = moving;
        stats.map_mib += shadows.cache.mib();
        self.shadows = Some(shadows);
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
