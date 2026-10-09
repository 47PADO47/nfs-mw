//! The uniform buffer shared by all passes, and the per-frame state it is computed from.
//!
//! The first 148 bytes follow AMD's `cbFSR3Upscaler` field for field; the tail adds what this port needs.

use bytemuck::{Pod, Zeroable};
use glam::{UVec2, Vec2};

use crate::{
    config::{DepthConvention, Fsr3Config},
    error::Fsr3Error,
    inputs::Fsr3Inputs,
    jitter,
};

/// `FLT_EPSILON`, which AMD adds to the infinite-depth projection terms.
const FLT_EPSILON: f32 = 1.192_092_9e-7;

/// The uniform block, 160 bytes, mirrored by `Constants` in `shaders/common.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Constants {
    pub render_size: [i32; 2],
    pub previous_render_size: [i32; 2],
    pub upscale_size: [i32; 2],
    pub previous_upscale_size: [i32; 2],
    pub max_render_size: [i32; 2],
    pub max_upscale_size: [i32; 2],
    /// `[a, b, x, y]`: view depth is `b / (device_depth - a)`; `x` and `y` turn NDC into view rays.
    pub device_to_view_depth: [f32; 4],
    pub jitter: [f32; 2],
    pub previous_jitter: [f32; 2],
    pub motion_vector_scale: [f32; 2],
    pub downscale_factor: [f32; 2],
    pub motion_vector_jitter_cancellation: [f32; 2],
    pub tan_half_fov: f32,
    pub jitter_phase_count: f32,
    pub delta_time: f32,
    pub delta_pre_exposure: f32,
    pub view_space_to_meters: f32,
    pub frame_index: f32,
    pub velocity_factor: f32,
    pub reactiveness_scale: f32,
    pub shading_change_scale: f32,
    pub accumulation_added_per_frame: f32,
    pub min_disocclusion_accumulation: f32,
    /// RCAS: `exp2(-stops)`, 1 for the sharpest.
    pub rcas_lobe: f32,
    pub _pad: [f32; 2],
}

/// The size of [`Constants`] in bytes, as the WGSL uniform block lays it out.
pub const CONSTANTS_SIZE: usize = 160;

/// What carries over from frame to frame on the CPU.
#[derive(Debug, Default)]
pub(crate) struct FrameState {
    /// Frames since the last reset; 0 on the reset frame.
    frame_index: f32,
    has_previous: bool,
    previous_render: UVec2,
    previous_upscale: UVec2,
    jitter: Vec2,
    jitter_cancel_previous: Vec2,
    jitter_phase_count: f32,
    pre_exposure: f32,
}

/// The result of starting a frame.
pub(crate) struct Frame {
    pub constants: Constants,
    /// This frame starts from nothing (first frame, a reset, or new sizes).
    pub reset: bool,
}

impl FrameState {
    /// Works out this frame's constants. `resized` is true when the GPU resources were just recreated and
    /// `forced` when the application asked to drop the history outside the inputs.
    pub fn begin(
        &mut self,
        config: &Fsr3Config,
        inputs: &Fsr3Inputs<'_>,
        output_size: UVec2,
        resized: bool,
        forced: bool,
    ) -> Result<Frame, Fsr3Error> {
        let render = inputs.render_size;
        let reset = inputs.reset || resized || forced || !self.has_previous;
        let device_to_view =
            device_to_view_depth(config.depth, inputs.camera_near, inputs.camera_far, inputs.camera_fov_y, render)?;
        let previous_render = if reset { render } else { self.previous_render };
        let previous_upscale = if reset { output_size } else { self.previous_upscale };
        let previous_jitter = if reset { inputs.jitter } else { self.jitter };

        // Pre-exposure changes are compensated in the history.
        let pre_exposure = if inputs.pre_exposure != 0.0 { inputs.pre_exposure } else { 1.0 };
        let delta_pre_exposure = if self.pre_exposure > 0.0 { pre_exposure / self.pre_exposure } else { 1.0 };

        let mv_target = if config.motion_vectors.display_resolution { output_size } else { render };
        let mv_target_f = Vec2::new(mv_target.x as f32, mv_target.y as f32);
        // The jitter is in render pixels, so it is divided by the render size (AMD divides by the size of
        // the motion vectors, which is only right when they are at the render resolution).
        let cancellation = if config.motion_vectors.jittered {
            let c = (self.jitter_cancel_previous - inputs.jitter) / Vec2::new(render.x as f32, render.y as f32);
            self.jitter_cancel_previous = inputs.jitter;
            c
        } else {
            Vec2::ZERO
        };

        // The jitter cycle length follows the ratio, changing by one phase per frame.
        let target_phases = jitter::phase_count(render.x, output_size.x) as f32;
        if reset || self.jitter_phase_count == 0.0 {
            self.jitter_phase_count = target_phases;
        } else if target_phases > self.jitter_phase_count {
            self.jitter_phase_count += 1.0;
        } else if target_phases < self.jitter_phase_count {
            self.jitter_phase_count -= 1.0;
        }

        self.frame_index = if reset { 0.0 } else { self.frame_index + 1.0 };
        let aspect = render.x as f32 / render.y as f32;
        let tan_half_fov = (inputs.camera_fov_y * 0.5).tan() * aspect;
        let tuning = config.tuning.clamped();
        let sharpness = inputs.sharpness.unwrap_or(0.0).clamp(0.0, 1.0);

        let constants = Constants {
            render_size: ivec(render),
            previous_render_size: ivec(previous_render),
            upscale_size: ivec(output_size),
            previous_upscale_size: ivec(previous_upscale),
            max_render_size: ivec(render),
            max_upscale_size: ivec(output_size),
            device_to_view_depth: device_to_view,
            jitter: inputs.jitter.to_array(),
            previous_jitter: previous_jitter.to_array(),
            motion_vector_scale: (inputs.motion_vector_scale / mv_target_f).to_array(),
            downscale_factor: [render.x as f32 / output_size.x as f32, render.y as f32 / output_size.y as f32],
            motion_vector_jitter_cancellation: cancellation.to_array(),
            tan_half_fov,
            jitter_phase_count: self.jitter_phase_count,
            delta_time: inputs.delta_time.clamp(0.0, 1.0),
            delta_pre_exposure,
            view_space_to_meters: if inputs.view_space_to_meters > 0.0 { inputs.view_space_to_meters } else { 1.0 },
            frame_index: self.frame_index,
            velocity_factor: tuning.velocity_factor,
            reactiveness_scale: tuning.reactiveness_scale,
            shading_change_scale: tuning.shading_change_scale,
            accumulation_added_per_frame: tuning.accumulation_added_per_frame,
            min_disocclusion_accumulation: tuning.min_disocclusion_accumulation,
            rcas_lobe: (-(2.0 - 2.0 * sharpness)).exp2(),
            _pad: [0.0; 2],
        };

        self.has_previous = true;
        self.previous_render = render;
        self.previous_upscale = output_size;
        self.jitter = inputs.jitter;
        self.pre_exposure = pre_exposure;
        Ok(Frame { constants, reset })
    }
}

fn ivec(v: UVec2) -> [i32; 2] {
    [v.x as i32, v.y as i32]
}

/// The factors `[a, b, x, y]` that turn a depth buffer value `d` into a view-space distance
/// `b / (d - a)` (AMD's `fDeviceToViewDepth`), and an NDC position into a view ray with `x` and `y`.
pub fn device_to_view_depth(
    depth: DepthConvention,
    near: f32,
    far: f32,
    fov_y: f32,
    render_size: UVec2,
) -> Result<[f32; 4], Fsr3Error> {
    if !(near.is_finite() && near > 0.0) {
        return Err(Fsr3Error::InvalidCamera("near must be positive"));
    }
    if !(fov_y.is_finite() && fov_y > 0.0 && fov_y < std::f32::consts::PI) {
        return Err(Fsr3Error::InvalidCamera("the vertical field of view must be within (0, pi) radians"));
    }
    if !depth.infinite && !(far.is_finite() && far > 0.0 && far != near) {
        return Err(Fsr3Error::InvalidCamera("far must be positive and differ from near"));
    }
    // Only the nearer and farther distance matter, not which is called which; infinite planes ignore far.
    let far = if depth.infinite { f32::MAX } else { far };
    let (mut min, mut max) = (near.min(far), near.max(far));
    if depth.inverted {
        std::mem::swap(&mut min, &mut max);
    }
    // The projection's depth row: [c d; e 0] with d = -1.
    let q = max / (min - max);
    let (c, e) = match (depth.inverted, depth.infinite) {
        (false, false) => (q, q * min),
        (false, true) => (-1.0 - FLT_EPSILON, -min - FLT_EPSILON),
        (true, false) => (q, q * min),
        (true, true) => (FLT_EPSILON, max),
    };
    let aspect = render_size.x as f32 / render_size.y as f32;
    let cot_half_fov = 1.0 / (fov_y * 0.5).tan();
    Ok([-c, e, aspect / cot_half_fov, 1.0 / cot_half_fov])
}

/// The view-space distance a depth buffer value stands for (the CPU twin of the WGSL helper).
pub fn view_depth(factors: [f32; 4], device_depth: f32) -> f32 {
    factors[1] / (device_depth - factors[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: UVec2 = UVec2::new(1280, 720);

    fn factors(depth: DepthConvention) -> [f32; 4] {
        device_to_view_depth(depth, 0.1, 1000.0, 1.0, SIZE).unwrap()
    }

    #[test]
    fn constants_are_160_bytes_with_the_offsets_the_shader_declares() {
        assert_eq!(size_of::<Constants>(), CONSTANTS_SIZE);
        assert_eq!(std::mem::offset_of!(Constants, device_to_view_depth), 48);
        assert_eq!(std::mem::offset_of!(Constants, jitter), 64);
        assert_eq!(std::mem::offset_of!(Constants, motion_vector_jitter_cancellation), 96);
        assert_eq!(std::mem::offset_of!(Constants, tan_half_fov), 104);
        assert_eq!(std::mem::offset_of!(Constants, delta_time), 112);
        assert_eq!(std::mem::offset_of!(Constants, velocity_factor), 128);
        assert_eq!(std::mem::offset_of!(Constants, min_disocclusion_accumulation), 144);
        assert_eq!(std::mem::offset_of!(Constants, rcas_lobe), 148);
    }

    #[test]
    fn reverse_infinite_depth_is_near_over_distance() {
        let f = factors(DepthConvention::REVERSE_INFINITE);
        // AMD adds FLT_EPSILON to the divisor, which matters only beyond a few thousand units.
        for z in [0.2_f32, 1.0, 10.0, 500.0, 2_000.0] {
            let d = 0.1 / z;
            assert!((view_depth(f, d) / z - 1.0).abs() < 1e-2, "z {z}: {}", view_depth(f, d));
        }
    }

    #[test]
    fn reverse_finite_depth_maps_one_to_near_and_zero_to_far() {
        let f = factors(DepthConvention::REVERSE);
        assert!((view_depth(f, 1.0) / 0.1 - 1.0).abs() < 1e-4);
        assert!((view_depth(f, 0.0) / 1000.0 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn standard_finite_depth_maps_zero_to_near_and_one_to_far() {
        let f = factors(DepthConvention::STANDARD);
        assert!((view_depth(f, 0.0) / 0.1 - 1.0).abs() < 1e-4);
        assert!((view_depth(f, 1.0) / 1000.0 - 1.0).abs() < 1e-4);
        // D3D-style depth of a point at z = 10: far (z - near) / (z (far - near)).
        let d = 1000.0 * (10.0 - 0.1) / (10.0 * (1000.0 - 0.1));
        assert!((view_depth(f, d) / 10.0 - 1.0).abs() < 1e-3);
    }

    #[test]
    fn standard_infinite_depth_matches_one_minus_near_over_distance() {
        let f = factors(DepthConvention { inverted: false, infinite: true });
        let z = 20.0_f32;
        assert!((view_depth(f, 1.0 - 0.1 / z) / z - 1.0).abs() < 1e-2);
    }

    #[test]
    fn near_and_far_may_be_given_in_either_order() {
        let a = device_to_view_depth(DepthConvention::REVERSE, 0.1, 1000.0, 1.0, SIZE).unwrap();
        let b = device_to_view_depth(DepthConvention::REVERSE, 1000.0, 0.1, 1.0, SIZE).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn view_ray_factors_follow_the_field_of_view() {
        let f = factors(DepthConvention::REVERSE_INFINITE);
        assert!((f[3] - 0.5_f32.tan()).abs() < 1e-6);
        assert!((f[2] - 0.5_f32.tan() * 1280.0 / 720.0).abs() < 1e-5);
    }

    #[test]
    fn invalid_cameras_are_refused() {
        let d = DepthConvention::REVERSE;
        assert!(device_to_view_depth(d, 0.0, 10.0, 1.0, SIZE).is_err());
        assert!(device_to_view_depth(d, 1.0, 1.0, 1.0, SIZE).is_err());
        assert!(device_to_view_depth(d, 1.0, 10.0, 0.0, SIZE).is_err());
        assert!(device_to_view_depth(d, 1.0, 10.0, 4.0, SIZE).is_err());
        assert!(device_to_view_depth(d, 1.0, f32::NAN, 1.0, SIZE).is_err());
        // An infinite far plane ignores far.
        assert!(device_to_view_depth(DepthConvention::REVERSE_INFINITE, 1.0, 0.0, 1.0, SIZE).is_ok());
    }
}
