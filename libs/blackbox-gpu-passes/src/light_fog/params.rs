//! The uniform block shared by the fog volume and its per-pixel integration.
use blackbox_gfx::{FogShelter, LocalLight};
use glam::Vec3;

/// Lights one volume holds: up to 64 current lights plus as many fading out.
pub(super) const MAX_LIGHTS: usize = 128;
/// Weather shelters one volume tests.
pub(super) const MAX_SHELTERS: usize = 24;
/// Weight of the reprojected history when it is valid.
const HISTORY_WEIGHT: f32 = 0.65;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Params {
    inverse: [[f32; 4]; 4],
    options: [f32; 4],
    grid: [u32; 4],
    previous_view_proj: [[f32; 4]; 4],
    previous_camera: [f32; 4],
    temporal: [f32; 4],
    shadow_indices: [[u32; 4]; 32],
    positions: [[f32; 4]; MAX_LIGHTS],
    directions: [[f32; 4]; MAX_LIGHTS],
    colors: [[f32; 4]; MAX_LIGHTS],
    uppers: [[f32; 4]; MAX_LIGHTS],
    shelters: [GpuShelter; MAX_SHELTERS],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShelter {
    bounds: [f32; 4],
    info: [f32; 4],
    points: [[f32; 4]; 8],
}

/// The previous frame of one view, when its cells may be reprojected.
pub(super) struct Previous {
    pub view_proj: glam::Mat4,
    pub camera: Vec3,
    pub distance: f32,
}

/// What one draw of the volume describes.
pub(super) struct Inputs<'a> {
    pub inverse_view_proj: glam::Mat4,
    /// Strength and weather.
    pub options: [f32; 2],
    pub distance: f32,
    pub dimensions: [u32; 3],
    pub lights: &'a [LocalLight],
    /// The surface lights in shadow-slot order, to find each fog light's shadow map.
    pub source_lights: &'a [LocalLight],
    pub shelters: &'a [FogShelter],
    pub previous: Option<Previous>,
}

impl Params {
    pub(super) fn new(inputs: &Inputs<'_>) -> Self {
        let mut params = <Self as bytemuck::Zeroable>::zeroed();
        params.shadow_indices = [[u32::MAX; 4]; 32];
        params.inverse = inputs.inverse_view_proj.to_cols_array_2d();
        let lights = &inputs.lights[..inputs.lights.len().min(MAX_LIGHTS)];
        let shelters = &inputs.shelters[..inputs.shelters.len().min(MAX_SHELTERS)];
        params.options = [
            inputs.options[0].clamp(0.0, 4.0),
            inputs.options[1].clamp(0.0, 1.0),
            lights.len() as f32,
            inputs.distance,
        ];
        let [x, y, z] = inputs.dimensions;
        params.grid = [x, y, z, shelters.len() as u32];
        if let Some(previous) = &inputs.previous {
            params.previous_view_proj = previous.view_proj.to_cols_array_2d();
            params.previous_camera = previous.camera.extend(0.0).to_array();
            params.temporal = [HISTORY_WEIGHT, previous.distance, 0.0, 0.0];
        }
        for (i, light) in lights.iter().enumerate() {
            if let Some(slot) = inputs.source_lights.iter().position(|source| {
                source.position == light.position
                    && source.direction == light.direction
                    && source.upper_plane == light.upper_plane
            }) {
                params.shadow_indices[i / 4][i % 4] = slot as u32;
            }
            // Directions and upper planes are normalized here once, not per cell.
            let direction = Vec3::from_array(light.direction).normalize_or_zero();
            params.positions[i] = [light.position[0], light.position[1], light.position[2], light.range];
            params.directions[i] = direction.extend(light.outer_cos).to_array();
            params.colors[i] = [light.color[0], light.color[1], light.color[2], light.inner_cos];
            if let Some(up) = light.upper_plane {
                params.uppers[i] = Vec3::from_array(up).normalize_or_zero().extend(1.0).to_array();
            }
        }
        for (out, shelter) in params.shelters.iter_mut().zip(shelters) {
            out.bounds = shelter.bounds;
            out.info = [shelter.ceiling, shelter.polygon.len().min(16) as f32, 0.0, 0.0];
            for (i, point) in shelter.polygon.iter().take(16).enumerate() {
                out.points[i / 2][(i % 2) * 2..(i % 2) * 2 + 2].copy_from_slice(point);
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_are_normalized_on_the_cpu() {
        let light = LocalLight {
            position: [0.0; 3],
            range: 10.0,
            direction: [0.0, 0.0, -4.0],
            outer_cos: 0.2,
            inner_cos: 0.7,
            color: [1.0; 3],
            upper_plane: Some([0.0, 3.0, 0.0]),
            casts_shadow: false,
            shadow_exclude: None,
        };
        let params = Params::new(&Inputs {
            inverse_view_proj: glam::Mat4::IDENTITY,
            options: [1.0, 1.0],
            distance: 200.0,
            dimensions: [16, 16, 64],
            lights: &[light],
            source_lights: &[light],
            shelters: &[],
            previous: None,
        });
        assert_eq!(params.directions[0], [0.0, 0.0, -1.0, 0.2]);
        assert_eq!(params.uppers[0], [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(params.shadow_indices[0][0], 0);
        assert_eq!(params.temporal[0], 0.0, "no history without a previous frame");
    }
}
