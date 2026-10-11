//! The upscaler context: pipelines, resources and the state that carries over from frame to frame.

use glam::UVec2;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Device, Queue};

use crate::{
    config::Fsr3Config,
    constants::{CONSTANTS_SIZE, FrameState},
    error::Fsr3Error,
    inputs::{Fsr3Inputs, Fsr3Outputs},
    passes::Recording,
    pipelines::Pipelines,
    resources::Resources,
};

/// How many uniform blocks to rotate through, so that dispatches recorded before one submit do not
/// overwrite each other's constants.
const UNIFORM_RING: usize = 4;
/// The smallest render or output side.
const MIN_SIDE: u32 = 2;

/// A temporal upscaler for one camera.
///
/// Create it once, then call [`dispatch`](Self::dispatch) every frame. The history lives in the
/// context, so a second camera (a mirror, a split screen) needs a context of its own.
pub struct Fsr3Context {
    config: Fsr3Config,
    pipelines: Pipelines,
    resources: Option<Resources>,
    state: FrameState,
    uniforms: Vec<Buffer>,
    uniform_index: usize,
    /// Frames since the resources were created: picks which textures swap roles.
    frame: u64,
    force_reset: bool,
}

impl Fsr3Context {
    /// Builds the pipelines for `config`. GPU resources for the images are made by the first
    /// [`dispatch`](Self::dispatch), and again whenever the render or output size changes.
    pub fn new(device: &Device, config: Fsr3Config) -> Result<Self, Fsr3Error> {
        config.validate()?;
        let pipelines = Pipelines::new(device, &config);
        let uniforms = (0..UNIFORM_RING)
            .map(|_| {
                device.create_buffer(&BufferDescriptor {
                    label: Some("fsr3 constants"),
                    size: CONSTANTS_SIZE as u64,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Ok(Self {
            config,
            pipelines,
            resources: None,
            state: FrameState::default(),
            uniforms,
            uniform_index: 0,
            frame: 0,
            force_reset: true,
        })
    }

    pub(crate) fn resources(&self) -> Option<&Resources> {
        self.resources.as_ref()
    }

    /// Frames dispatched since the resources were created.
    pub(crate) fn frames_dispatched(&self) -> u64 {
        self.frame
    }

    /// The configuration the context was created with.
    pub fn config(&self) -> &Fsr3Config {
        &self.config
    }

    /// Drops the history at the next dispatch, as [`Fsr3Inputs::reset`] does.
    pub fn reset(&mut self) {
        self.force_reset = true;
    }

    /// The GPU memory of the internal images and buffers, in bytes; 0 before the first dispatch.
    pub fn gpu_memory_bytes(&self) -> u64 {
        self.resources.as_ref().map_or(0, |r| r.bytes)
    }

    /// Records one upscale into `encoder`: the passes read the textures of `inputs` and write the
    /// upscaled image to `outputs`. The commands must be submitted before the next dispatch's results are
    /// used, and in order (the history is a chain).
    ///
    /// If `inputs.render_size` or `outputs.size` differ from the previous call, the internal images are
    /// recreated and the history is dropped.
    pub fn dispatch(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        inputs: &Fsr3Inputs<'_>,
        outputs: &Fsr3Outputs<'_>,
    ) -> Result<(), Fsr3Error> {
        check_sizes(inputs.render_size, outputs.size)?;
        let resized = !self.resources.as_ref().is_some_and(|r| r.fits(inputs.render_size, outputs.size));
        if resized {
            self.resources = Some(Resources::new(device, inputs.render_size, outputs.size));
            self.frame = 0;
        }
        let frame = self.state.begin(&self.config, inputs, outputs.size, resized, self.force_reset)?;
        let reset = frame.reset;
        self.force_reset = false;

        let uniform = &self.uniforms[self.uniform_index];
        self.uniform_index = (self.uniform_index + 1) % UNIFORM_RING;
        queue.write_buffer(uniform, 0, bytemuck::bytes_of(&frame.constants));

        let resources = self.resources.as_ref().expect("resources are created above");
        let sharpen = inputs.sharpness.is_some();
        Recording { device, pipelines: &self.pipelines, resources, uniform, frame: self.frame, sharpen, reset }
            .encode(encoder, inputs, outputs);
        self.frame += 1;
        Ok(())
    }
}

fn check_sizes(render: UVec2, output: UVec2) -> Result<(), Fsr3Error> {
    for (what, size) in [("render", render), ("output", output)] {
        if size.x < MIN_SIDE || size.y < MIN_SIDE {
            return Err(Fsr3Error::SizeTooSmall { what, width: size.x, height: size.y });
        }
    }
    if render.x > output.x || render.y > output.y {
        return Err(Fsr3Error::RenderLargerThanOutput { render: render.to_array(), output: output.to_array() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_checked() {
        assert!(check_sizes(UVec2::new(1280, 720), UVec2::new(1920, 1080)).is_ok());
        assert!(check_sizes(UVec2::new(1920, 1080), UVec2::new(1920, 1080)).is_ok());
        assert!(matches!(
            check_sizes(UVec2::new(0, 720), UVec2::new(1920, 1080)),
            Err(Fsr3Error::SizeTooSmall { what: "render", .. })
        ));
        assert!(matches!(
            check_sizes(UVec2::new(1280, 720), UVec2::new(1, 1)),
            Err(Fsr3Error::SizeTooSmall { what: "output", .. })
        ));
        assert!(matches!(
            check_sizes(UVec2::new(2560, 720), UVec2::new(1920, 1080)),
            Err(Fsr3Error::RenderLargerThanOutput { .. })
        ));
    }
}
