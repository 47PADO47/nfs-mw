//! Records the compute passes of one upscale into a command encoder.

use glam::UVec2;
use wgpu::{BindGroupDescriptor, BindGroupEntry, Buffer, CommandEncoder, ComputePassDescriptor, Device, TextureView};

use crate::{
    inputs::{Fsr3Inputs, Fsr3Outputs},
    pipelines::{Pipelines, Step},
    resources::Resources,
};

/// The threads along each axis of a workgroup.
const GROUP: u32 = 8;

/// What a recording needs besides the encoder.
pub(crate) struct Recording<'a> {
    pub device: &'a Device,
    pub pipelines: &'a Pipelines,
    pub resources: &'a Resources,
    pub uniform: &'a Buffer,
    /// The frame counter that picks which of the swapped textures is current.
    pub frame: u64,
    /// Run the sharpening pass after the accumulation.
    pub sharpen: bool,
    /// The frame starts from nothing: restore the frame info.
    pub reset: bool,
}

fn view(binding: u32, view: &TextureView) -> BindGroupEntry<'_> {
    BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) }
}

fn buffer(binding: u32, buffer: &Buffer) -> BindGroupEntry<'_> {
    BindGroupEntry { binding, resource: buffer.as_entire_binding() }
}

fn groups(size: UVec2) -> [u32; 2] {
    [size.x.div_ceil(GROUP), size.y.div_ceil(GROUP)]
}

impl Recording<'_> {
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        step: &Step,
        label: &str,
        entries: &[BindGroupEntry<'_>],
        grid: [u32; 2],
    ) {
        let mut all = vec![buffer(0, self.uniform)];
        all.extend_from_slice(entries);
        let group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout: &step.layout,
            entries: &all,
        });
        let mut pass =
            encoder.begin_compute_pass(&ComputePassDescriptor { label: Some(label), timestamp_writes: None });
        pass.set_pipeline(&step.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(grid[0], grid[1], 1);
    }

    /// Records every pass, in order.
    pub fn encode(&self, encoder: &mut CommandEncoder, inputs: &Fsr3Inputs<'_>, outputs: &Fsr3Outputs<'_>) {
        let r = self.resources;
        if self.reset {
            encoder.copy_buffer_to_buffer(&r.frame_info_reset, 0, &r.frame_info, 0, 16);
        }
        encoder.clear_buffer(&r.reconstructed_depth, 0, None);
        self.prepare_inputs(encoder, inputs);
        self.luma_pyramid(encoder, inputs);
        self.shading_change(encoder);
        self.prepare_reactivity(encoder, inputs);
        self.luma_instability(encoder);
        self.accumulate(encoder, inputs, outputs);
    }

    fn prepare_inputs(&self, encoder: &mut CommandEncoder, inputs: &Fsr3Inputs<'_>) {
        let r = self.resources;
        self.run(
            encoder,
            &self.pipelines.prepare_inputs,
            "fsr3 prepare inputs",
            &[
                view(1, inputs.color),
                view(2, inputs.depth),
                view(3, inputs.motion_vectors),
                view(4, &r.dilated_motion_vectors),
                view(5, &r.dilated_depth),
                view(6, &r.farthest_depth),
                view(7, r.luma.current(self.frame)),
                buffer(8, &r.reconstructed_depth),
            ],
            groups(r.render),
        );
    }

    fn luma_pyramid(&self, encoder: &mut CommandEncoder, inputs: &Fsr3Inputs<'_>) {
        let r = self.resources;
        self.run(
            encoder,
            &self.pipelines.luma_reduce,
            "fsr3 luma pyramid reduce",
            &[
                view(1, r.luma.current(self.frame)),
                view(2, &r.farthest_depth),
                view(3, &r.farthest_depth_mip1),
                buffer(4, &r.partials),
            ],
            r.luma_reduce_groups().to_array(),
        );
        self.run(
            encoder,
            &self.pipelines.luma_final,
            "fsr3 luma pyramid final",
            &[buffer(4, &r.partials), buffer(5, &r.frame_info), view(6, inputs.exposure.unwrap_or(&r.placeholder))],
            [1, 1],
        );
    }

    /// The pyramid of luma differences and the shading change estimate from it.
    fn shading_change(&self, encoder: &mut CommandEncoder) {
        let r = self.resources;
        let pyramid = &r.shading_pyramid;
        let sizes = r.shading_pyramid_sizes;
        self.run(
            encoder,
            &self.pipelines.shading_level0,
            "fsr3 shading change level 0",
            &[
                view(1, &r.dilated_motion_vectors),
                view(2, r.luma.current(self.frame)),
                view(3, r.luma.previous(self.frame)),
                buffer(4, &r.frame_info),
                view(5, &pyramid[0]),
            ],
            groups(sizes[0]),
        );
        for level in 1..3 {
            self.run(
                encoder,
                &self.pipelines.shading_down,
                "fsr3 shading change downsample",
                &[view(5, &pyramid[level]), view(6, &pyramid[level - 1])],
                groups(sizes[level]),
            );
        }
        self.run(
            encoder,
            &self.pipelines.shading_change,
            "fsr3 shading change",
            &[view(1, &pyramid[0]), view(2, &pyramid[1]), view(3, &pyramid[2]), view(4, &r.shading_change)],
            groups(sizes[0]),
        );
    }

    fn prepare_reactivity(&self, encoder: &mut CommandEncoder, inputs: &Fsr3Inputs<'_>) {
        let r = self.resources;
        self.run(
            encoder,
            &self.pipelines.prepare_reactivity,
            "fsr3 prepare reactivity",
            &[
                view(1, &r.dilated_motion_vectors),
                view(2, &r.dilated_depth),
                buffer(3, &r.reconstructed_depth),
                view(4, inputs.reactive.unwrap_or(&r.placeholder)),
                view(5, inputs.transparency_and_composition.unwrap_or(&r.placeholder)),
                view(6, r.luma.current(self.frame)),
                view(7, &r.shading_change),
                view(8, r.accumulation.previous(self.frame)),
                buffer(9, &r.frame_info),
                view(10, &r.reactive_masks),
                view(11, r.accumulation.current(self.frame)),
                view(12, &r.new_locks),
            ],
            groups(r.render),
        );
    }

    fn luma_instability(&self, encoder: &mut CommandEncoder) {
        let r = self.resources;
        self.run(
            encoder,
            &self.pipelines.luma_instability,
            "fsr3 luma instability",
            &[
                view(1, &r.dilated_motion_vectors),
                view(2, &r.reactive_masks),
                view(3, r.luma.current(self.frame)),
                view(4, r.luma_history.previous(self.frame)),
                buffer(5, &r.frame_info),
                view(6, r.luma_history.current(self.frame)),
                view(7, &r.luma_instability),
            ],
            groups(r.render),
        );
    }

    fn accumulate(&self, encoder: &mut CommandEncoder, inputs: &Fsr3Inputs<'_>, outputs: &Fsr3Outputs<'_>) {
        let r = self.resources;
        let step = if self.sharpen { &self.pipelines.accumulate_sharpen } else { &self.pipelines.accumulate };
        self.run(
            encoder,
            step,
            "fsr3 accumulate",
            &[
                view(1, inputs.color),
                view(2, inputs.motion_vectors),
                view(3, &r.dilated_motion_vectors),
                view(4, &r.reactive_masks),
                view(5, r.history.previous(self.frame)),
                view(6, &r.farthest_depth_mip1),
                view(7, &r.luma_instability),
                buffer(8, &r.frame_info),
                view(9, r.history.current(self.frame)),
                view(10, outputs.output),
                view(11, &r.new_locks),
            ],
            groups(r.output),
        );
        if !self.sharpen {
            return;
        }
        self.run(
            encoder,
            &self.pipelines.rcas,
            "fsr3 rcas",
            &[view(1, r.history.current(self.frame)), buffer(2, &r.frame_info), view(3, outputs.output)],
            groups(r.output),
        );
    }
}
