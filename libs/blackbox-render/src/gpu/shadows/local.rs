//! Bounded spotlights sharing the material-identical raster caster path, without an RT scene.
use super::{Shadows, maps, upload};
use crate::gpu::Renderer;
use blackbox_gfx::{ExperimentStats, FrameParams, Instance, LocalLight, Projection, RenderBackend};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};

mod cache;
mod light_grid;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(in crate::gpu) struct Params {
    matrices: [[f32; 16]; 16],
    positions: [[f32; 4]; 128],
    directions: [[f32; 4]; 128],
    colors: [[f32; 4]; 128],
    uppers: [[f32; 4]; 128],
    weights: [[f32; 4]; 4],
    /// Light count (zero skips all local shading), shadowed count, filter radius, night blend.
    options: [f32; 4],
    /// Specular strength, light-grid origin x and y, light-grid cell edge (`light_grid.rs`).
    tuning: [f32; 4],
    grid: [[u32; 4]; 256],
    /// Car paint at night: reflection level, light strength, RT lamp lighting (0 or 1), lamp radius.
    car: [f32; 4],
}

pub(in crate::gpu) struct Local {
    resolution: u32,
    depth: Shadows,
    cache: cache::LampCache,
    /// Shadowed lights of the last frame that reused their kept static depth.
    pub(in crate::gpu) reused: u32,
    view: wgpu::TextureView,
    layers: Vec<wgpu::TextureView>,
}
impl Renderer {
    pub fn set_local_lights(&mut self, lights: &[LocalLight]) {
        self.local_lights.clear();
        self.local_lights.extend(lights.iter().filter(|l| l.valid()).take(128).copied());
    }
    pub(in crate::gpu) fn encode_local_shadows(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameParams,
        visible: &[Instance],
    ) {
        let options = self.render_experiments().sanitized();
        let mut params = Params::zeroed();
        params.tuning[0] = options.local_specular_strength;
        let night = self.night_blend.unwrap_or(f32::from(u8::from(options.experimental_night)));
        params.options[3] = night;
        params.car = [
            options.car_night_reflections,
            options.car_light_strength,
            f32::from(u8::from(options.car_rt_lamp_lighting)),
            options.car_lamp_size,
        ];
        if let Some(glossy) = self.glossy.as_mut() {
            glossy.sync_sky_night(&self.queue, night);
        }
        let offset = size_of::<super::fit::Params>() as u64;
        // With no lights `options.x` stays zero, which the shader treats as "skip all local lighting".
        if self.local_lights.is_empty() || !matches!(frame.projection, Projection::PerspectiveInfiniteReverse { .. }) {
            self.queue.write_buffer(&self.shared.shadows.buffer, offset, bytemuck::bytes_of(&params));
            return;
        }
        let _trace = blackbox_trace::scope("Renderer CPU: local lamp shadows");
        let resolution = options.local_shadow_resolution;
        let capacity = options.local_shadow_count.max(1) as usize;
        let texture_resolution = match options.local_shadow_count {
            0 => 1,
            _ => resolution,
        };
        let shadow_count = (options.local_shadow_count as usize).min(self.local_lights.len());
        let mut local = self
            .local_shadows
            .take()
            .filter(|l| l.resolution == texture_resolution && l.layers.len() == capacity)
            .unwrap_or_else(|| {
                let (view, layers) = maps::atlas(&self.device, texture_resolution, capacity as u32);
                Local {
                    resolution: texture_resolution,
                    depth: Shadows::with_layers(
                        &self.device,
                        &self.shared.bindings.texture_layout,
                        [1.0, 1.5],
                        self.pipeline_cache.as_ref(),
                        capacity,
                    ),
                    cache: Default::default(),
                    reused: 0,
                    view,
                    layers,
                }
            });
        if self.shared.shadows.local != local.view {
            self.shared.shadows.local = local.view.clone();
            self.shared.shadows.fallback = self.shared.shadows.material_group(
                &self.device,
                &self.shared.glossy.material,
                &self.shared.shadows.dummy,
            );
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        let casters = self.shadow_casters.as_deref().unwrap_or(visible);
        let depth = &mut local.depth;
        if shadow_count > 0 && depth.store {
            upload::caster_slots(
                &mut self.draw_list.store,
                &self.device,
                &self.queue,
                casters,
                self.shadow_casters.is_none(),
                &mut depth.caster_slots,
            );
        }
        depth.bounds.clear();
        for (index, instance) in casters.iter().enumerate().take(if shadow_count > 0 { usize::MAX } else { 0 }) {
            if !instance.shadow.cast {
                continue;
            }
            let Some(mesh) = self.meshes.get(instance.mesh.raw()) else { continue };
            let center = instance.transform.transform_point3(mesh.center);
            let scale = instance
                .transform
                .x_axis
                .truncate()
                .length()
                .max(instance.transform.y_axis.truncate().length())
                .max(instance.transform.z_axis.truncate().length());
            depth.bounds.push((index, center, mesh.radius * scale));
        }
        let grid = light_grid::populate(&mut params.grid, &self.local_lights);
        params.tuning[1] = grid.origin.x;
        params.tuning[2] = grid.origin.y;
        params.tuning[3] = grid.cell;
        // The nearest shadow-eligible light beyond the budget, for fading the last shadowed slots.
        let next_distance = self
            .local_lights
            .iter()
            .skip(shadow_count)
            .find(|l| l.upper_plane.is_none() && l.casts_shadow)
            .map(|l| Vec3::from_array(l.position).distance(frame.camera_position));
        let mut stats = ExperimentStats::default();
        let mut reused = 0u32;
        local.cache.begin(capacity, texture_resolution, casters.len());
        depth.cache.begin(casters.len());
        let mut fixed = std::mem::take(&mut depth.fixed);
        let mut moving = std::mem::take(&mut depth.moving);
        for (slot, light) in self.local_lights.iter().enumerate() {
            let position = Vec3::from_array(light.position);
            let direction = Vec3::from_array(light.direction).normalize();
            params.positions[slot] = position.extend(light.range).to_array();
            params.directions[slot] = direction.extend(light.outer_cos).to_array();
            params.colors[slot] = Vec3::from_array(light.color).extend(light.inner_cos).to_array();
            if let Some(up) = light.upper_plane {
                params.uppers[slot] = Vec3::from_array(up).normalize().extend(1.0).to_array();
            }
            if slot >= shadow_count || !light.casts_shadow {
                continue;
            }
            params.weights[slot / 4][slot % 4] = match (light.upper_plane, next_distance) {
                (None, Some(next)) => ((next - position.distance(frame.camera_position)) / 8.0).clamp(0.0, 1.0),
                _ => 1.0,
            };
            let range_fade =
                ((options.local_shadow_distance - position.distance(frame.camera_position)) / 20.0).clamp(0.0, 1.0);
            params.weights[slot / 4][slot % 4] *= range_fade * range_fade * (3.0 - 2.0 * range_fade);
            if params.weights[slot / 4][slot % 4] <= 0.0 {
                continue;
            }
            let up = match direction.dot(Vec3::Z).abs() > 0.95 {
                true => Vec3::Y,
                false => Vec3::Z,
            };
            let view = glam::camera::rh::view::look_at_mat4(position, position + direction, up);
            // Right-handed perspective, forward depth in [0,1].
            let f = 1.0 / light.outer_cos.acos().tan();
            let a = light.range / (0.15 - light.range);
            let projection = Mat4::from_cols_array(&[
                f,
                0.0,
                0.0,
                0.0,
                0.0,
                f,
                0.0,
                0.0,
                0.0,
                0.0,
                a,
                -1.0,
                0.0,
                0.0,
                0.15 * a,
                0.0,
            ]);
            let matrix = projection * view;
            params.matrices[slot] = matrix.to_cols_array();
            self.queue.write_buffer(&depth.buffers[slot], 0, bytemuck::cast_slice(&matrix.to_cols_array()));
            let planes = super::clip_planes(matrix);
            depth.selected.clear();
            for &(index, center, radius) in &depth.bounds {
                if light
                    .shadow_exclude
                    .is_some_and(|key| key != blackbox_gfx::InstanceKey::TRANSIENT && casters[index].owner_key == key)
                    || center.distance_squared(position) > (light.range + radius).powi(2)
                    || !super::intersects(&planes, center, radius)
                {
                    continue;
                }
                depth.selected.push(index);
            }
            let slots = depth.store.then_some(depth.caster_slots.as_slice());
            // A light that kept its projection since the previous frame keeps the depth of its unmoving casters.
            let projection = matrix.to_cols_array().map(f32::to_bits);
            fixed.clear();
            moving.clear();
            if local.cache.still(projection) && keep_static_depth() {
                for &index in &depth.selected {
                    match local.cache.resting(index, casters) {
                        true => fixed.push(index),
                        false => moving.push(index),
                    }
                }
            }
            if fixed.is_empty() {
                local.cache.set_layer(slot, None);
                depth.scratch.upload(
                    &self.device,
                    &self.queue,
                    &mut depth.instances[slot],
                    casters,
                    &depth.selected,
                    slots,
                );
                depth.draw_selected(
                    self,
                    encoder,
                    slot,
                    &local.layers[slot],
                    resolution as f32,
                    wgpu::LoadOp::Clear(1.0),
                    &depth.instances[slot].buffer,
                    &depth.selected,
                    casters,
                    &mut stats,
                );
                continue;
            }
            let fingerprint = depth.cache.fingerprint(self, matrix, &fixed, casters);
            let kept = local.cache.kept(&self.device, resolution, projection, fingerprint);
            if kept.stale {
                let buffer = &mut depth.static_instances[slot];
                depth.scratch.upload(&self.device, &self.queue, buffer, casters, &fixed, slots);
                depth.draw_selected(
                    self,
                    encoder,
                    slot,
                    &kept.image,
                    resolution as f32,
                    wgpu::LoadOp::Clear(1.0),
                    &depth.static_instances[slot].buffer,
                    &fixed,
                    casters,
                    &mut stats,
                );
            }
            if !kept.stale {
                reused += 1;
            }
            if !kept.stale && moving.is_empty() && local.cache.holds(slot, projection, fingerprint) {
                continue;
            }
            let mut destination = local.layers[slot].texture().as_image_copy();
            destination.origin.z = slot as u32;
            encoder.copy_texture_to_texture(
                kept.image.texture().as_image_copy(),
                destination,
                wgpu::Extent3d { width: resolution, height: resolution, depth_or_array_layers: 1 },
            );
            local.cache.set_layer(slot, moving.is_empty().then_some((projection, fingerprint)));
            if moving.is_empty() {
                continue;
            }
            depth.scratch.upload(&self.device, &self.queue, &mut depth.instances[slot], casters, &moving, slots);
            depth.draw_selected(
                self,
                encoder,
                slot,
                &local.layers[slot],
                resolution as f32,
                wgpu::LoadOp::Load,
                &depth.instances[slot].buffer,
                &moving,
                casters,
                &mut stats,
            );
        }
        depth.fixed = fixed;
        depth.moving = moving;
        local.cache.end();
        local.reused = reused;
        params.options = [self.local_lights.len() as f32, shadow_count as f32, options.local_shadow_filter, night];
        self.queue.write_buffer(&self.shared.shadows.buffer, offset, bytemuck::bytes_of(&params));
        blackbox_trace::global().counter(
            "Local lamp shadows",
            &[
                ("lights", self.local_lights.len() as f64),
                ("caster_draws", f64::from(stats.shadow_draws)),
                ("reused_maps", f64::from(reused)),
                ("kept_mib", f64::from(local.cache.mib())),
            ],
        );
        self.local_shadows = Some(local);
    }
}

/// Whether still lights keep the depth of their resting casters. `BLACKBOX_LAMP_SHADOW_CACHE=0` redraws every map
/// in full each frame, for comparing the two.
fn keep_static_depth() -> bool {
    static KEEP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *KEEP.get_or_init(|| std::env::var("BLACKBOX_LAMP_SHADOW_CACHE").as_deref() != Ok("0"))
}
