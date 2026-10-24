// Ported from the smaa crate 0.21.0 (src/lib.rs, the SmaaTarget type and its resize and start_frame):
// Copyright (c) 2018 Jonathan Behrens
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::SmaaMode;
use crate::bindings::{BindGroupLayouts, BindGroups};
use crate::frame::SmaaFrame;
use crate::pipelines::Pipelines;
use crate::resources::Resources;
use crate::targets::Targets;

/// The state of an SMAA 1x target: the three pipelines, the area and search tables and the sized render targets.
/// Create one per output format and size, and call [`SmaaTarget::resize`] when the size changes.
pub struct SmaaTarget {
    pub(crate) inner: Option<Inner>,
}

/// The state of an enabled target. `None` in [`SmaaTarget`] when the mode is [`SmaaMode::Disabled`].
pub(crate) struct Inner {
    pub(crate) layouts: BindGroupLayouts,
    pub(crate) pipelines: Pipelines,
    pub(crate) resources: Resources,
    pub(crate) targets: Targets,
    pub(crate) bind_groups: BindGroups,
    pub(crate) format: wgpu::TextureFormat,
}

impl SmaaTarget {
    /// Create a target whose pipelines compile from scratch. Same as [`SmaaTarget::new_cached`] with no cache.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        mode: SmaaMode,
    ) -> Self {
        Self::new_cached(device, queue, width, height, format, mode, None)
    }

    /// Create a target whose three pipelines are built with `cache` (for example a persistent
    /// [`wgpu::PipelineCache`]). `format` is the output view's format; `width` and `height` the frame size.
    /// [`SmaaMode::Disabled`] builds nothing: the frame then renders straight into the output view.
    pub fn new_cached(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        mode: SmaaMode,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        if mode == SmaaMode::Disabled {
            return Self { inner: None };
        }
        let layouts = BindGroupLayouts::new(device);
        let pipelines = Pipelines::new(device, format, &layouts, cache);
        let resources = Resources::new(device, queue);
        let targets = Targets::new(device, width, height, format);
        let bind_groups = BindGroups::new(device, &layouts, &resources, &targets);
        Self { inner: Some(Inner { layouts, pipelines, resources, targets, bind_groups, format }) }
    }

    /// Resize the intermediate targets to `width` by `height`. The pipelines and tables are kept.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let Some(inner) = self.inner.as_mut() else {
            return;
        };
        inner.targets = Targets::new(device, width, height, inner.format);
        inner.bind_groups = BindGroups::new(device, &inner.layouts, &inner.resources, &inner.targets);
    }

    /// Start a frame. Render the scene into the returned [`SmaaFrame`] (it derefs to a view). Calling
    /// [`SmaaFrame::resolve`], or dropping it, runs the three passes into `output_view`.
    pub fn start_frame<'a>(
        &'a mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        output_view: &'a wgpu::TextureView,
    ) -> SmaaFrame<'a> {
        SmaaFrame { target: self, device, queue, output_view, encoded: false }
    }

    /// The view the scene renders into, for a caller that knows the output view only after the scene is recorded
    /// (a swapchain image acquired late). `None` when the mode is [`SmaaMode::Disabled`]: the scene then renders
    /// straight into the output view.
    pub fn input_view(&self) -> Option<&wgpu::TextureView> {
        self.inner.as_ref().map(|inner| &inner.targets.color)
    }

    /// Record the three passes from [`Self::input_view`] into `output_view`, like [`SmaaFrame::encode_into`].
    /// Does nothing when the mode is [`SmaaMode::Disabled`].
    pub fn encode_to(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let Some(inner) = self.inner.as_ref() else {
            return;
        };
        crate::frame::encode_passes(encoder, inner, output_view);
    }
}
