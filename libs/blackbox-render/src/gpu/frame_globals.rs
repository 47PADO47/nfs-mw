//! Uniforms for main and reduced views share an explicit clipping contract.
use super::{
    Renderer,
    reflections::{CaptureKind, CaptureView},
    resources::Globals,
};
use crate::FrameParams;

impl Renderer {
    pub(super) fn scene_globals(
        &self,
        frame: &FrameParams,
        capture: Option<&CaptureView>,
        reduced: bool,
        offscreen: bool,
    ) -> Globals {
        let reflection = capture.is_some();
        let projected = capture.is_some_and(|v| v.kind == CaptureKind::Projected);
        let planar = capture.is_some_and(|v| v.kind == CaptureKind::Planar);
        let lit_cube = capture.is_some_and(|v| v.kind == CaptureKind::LitCube);
        let clip_plane = capture.map_or([0.0; 4], |v| v.clip_plane);
        let [r, g, b] = frame.clear_color;
        let [fog_start, fog_end] = frame.fog_range();
        // A reflection, and a capture while a temporal upscaler is chosen, are drawn at their own size with no
        // upscale after, so the bias that matches texel density to the output would only sharpen them too much.
        let mip_bias = match reflection || (offscreen && self.post.has_temporal()) {
            true => 0.0,
            false => self.upscale.texture_lod_bias,
        };
        let (scatter_rayleigh, scatter_mie, to_sun) = match frame.scattering {
            Some(s) => (
                [s.rayleigh[0], s.rayleigh[1], s.rayleigh[2], 1.0],
                [s.mie[0], s.mie[1], s.mie[2], s.g],
                s.to_sun.normalize_or_zero().extend(s.distance_scale).to_array(),
            ),
            None => ([0.0; 4], [0.0; 4], [0.0; 4]),
        };
        let (surface_ambient, surface_diffuse) = match frame.surface_light {
            Some(l) => {
                ([l.ambient[0], l.ambient[1], l.ambient[2], 1.0], [l.diffuse[0], l.diffuse[1], l.diffuse[2], 0.0])
            }
            None => ([0.0; 4], [0.0; 4]),
        };
        Globals {
            view_proj: match reflection {
                true => frame.view_proj(),
                false => self.world_to_clip(frame),
            }
            .to_cols_array_2d(),
            // w is reserved for the HDR extension; reflection contracts stay SDR.
            camera_pos: frame
                .camera_position
                .extend(match !reflection && self.post.display.is_some() {
                    true => 2.0,
                    false => 1.0,
                })
                .to_array(),
            light_dir: frame
                .light_dir
                .normalize_or_zero()
                .extend(u8::from(self.experiments.original_world_materials || projected) as f32)
                .to_array(),
            fog_color: [r, g, b, frame.surface_light.map_or(1.0, |s| s.ambient_alpha)],
            fog_range: [
                fog_start,
                fog_end,
                mip_bias,
                match (projected, reflection || reduced) {
                    _ if planar => 4.0,
                    _ if lit_cube => 3.0,
                    (true, _) => 2.0,
                    (_, true) => 1.0,
                    _ => 0.0,
                },
            ],
            scatter_rayleigh,
            scatter_mie,
            to_sun,
            surface_ambient,
            surface_diffuse: [
                surface_diffuse[0],
                surface_diffuse[1],
                surface_diffuse[2],
                frame.scattering.map_or(1.0, |s| s.in_scatter_scale),
            ],
            camera_right: [frame.view.x_axis.x, frame.view.y_axis.x, frame.view.z_axis.x, 0.0],
            clip_plane,
        }
    }
}
