//! Captures recorded before the frame (the projected road reflection) that reach the GPU in the frame's own
//! submission, ahead of its commands, instead of in a submission of their own.
//!
//! A `Queue::write_buffer` lands before every command buffer of the next submission, so a queued capture would read
//! whatever the frame writes later into a buffer the capture reads too. What a projected capture reads of the shared
//! state, and how each write keeps what the capture was recorded with:
//! - the instance store: the capture opens the store's epoch and the frame joins it, so their slots never overlap;
//! - the local probe origin: the frame writes it as a copy recorded in its encoder ([`write_ordered`]);
//! - the environment lanes of the shadow receiver data, the glossy rig and materials, the environment and probe
//!   settings and the sky uniforms: a write that changes them first submits the queued captures
//!   ([`Renderer::flush_captures`]).
//!
//! The planar capture reads the sun shadow data and the normal-scenery lighting, which every frame rewrites, and the
//! rear-view mirror writes the shared globals and post uniforms, so both keep a submission of their own.
//!
//! The reflection cube's faces, when several are drawn, wait here too: the commands their encoder held before them,
//! then one command buffer per face (see `reflections.rs`), each finished on a worker thread. They are queued in the
//! middle of recording that encoder, which goes out right after them in the same submission, so they see the writes
//! they saw when they were recorded into it. Whoever submits that encoder does so with
//! [`Renderer::submit_after_captures`]. Native DLSS submits its scene prefix internally, so it takes everything
//! queued ahead of that prefix (`PassContext::prefix`).
//!
//! The frame closes its own encoder at a few points as well ([`Renderer::split_frame`]), so the parts are finished
//! at the same time on worker threads; `pending.rs` has how they keep their place.

use wgpu::util::DeviceExt;

use super::Renderer;

/// The captures waiting for the frame's submission.
#[derive(Default)]
pub(super) struct QueuedCaptures {
    /// The captures, and the parts of the frame closed so far (see `pending.rs`).
    pub(super) commands: super::pending::PendingCommands,
    /// The environment lanes of the shadow receiver data as last written (as bits), which glossy draws read in
    /// every view.
    shadow_environment: Option<[u32; 4]>,
}

impl Renderer {
    /// Whether a capture waits for the next submission.
    pub(super) fn captures_queued(&self) -> bool {
        self.queued_captures.commands.has_captures()
    }

    /// Send a capture's commands with the next submission, after the captures already queued.
    pub(super) fn queue_capture(&mut self, commands: wgpu::CommandBuffer) {
        self.queued_captures.commands.push_capture(commands);
    }

    /// Close the part of the frame `encoder` holds: a worker finishes it while the frame goes on recording into a
    /// fresh encoder. The part is submitted in its place, ahead of what is recorded next (see `pending.rs`).
    pub(super) fn split_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let next = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        let part = std::mem::replace(encoder, next);
        self.queued_captures.commands.finish_later(part, false);
    }

    /// Submit the queued captures now: before a write to something they read, or a submission they must precede.
    pub(super) fn flush_captures(&mut self) {
        let captures = self.queued_captures.commands.take_captures();
        if captures.is_empty() {
            return;
        }
        let _trace = blackbox_trace::scope("GPU API: queue submit");
        self.queue.submit(captures);
    }

    /// Submit `commands` after the queued captures, in one submission.
    pub(super) fn submit_after_captures(&mut self, commands: wgpu::CommandBuffer) {
        let queued = self.queued_captures.commands.take_all();
        self.queue.submit(queued.into_iter().chain(std::iter::once(commands)));
    }

    /// Open a new epoch of the instance store, unless a queued capture was recorded in the open one: the frame then
    /// joins it, as the views of one encoder do, so the capture keeps the slots it wrote.
    pub(super) fn begin_store_epoch(&mut self) {
        if self.captures_queued() {
            return;
        }
        self.draw_list.store.begin_frame();
    }

    /// Submit the queued captures before the shadow receiver data is written with other `environment` lanes.
    pub(super) fn guard_shadow_environment(&mut self, environment: [f32; 4]) {
        let bits = environment.map(f32::to_bits);
        if self.queued_captures.shadow_environment == Some(bits) {
            return;
        }
        self.queued_captures.shadow_environment = Some(bits);
        self.flush_captures();
    }
}

/// Write `bytes` at `offset` of `buffer`: through the queue, or, given the frame's `encoder` while captures are
/// queued, as a copy recorded at this point of it, so the captures still read what the buffer held. `offset` and
/// the length of `bytes` are multiples of four.
pub(in crate::gpu) fn write_ordered(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: Option<&mut wgpu::CommandEncoder>,
    buffer: &wgpu::Buffer,
    offset: u64,
    bytes: &[u8],
) {
    let Some(encoder) = encoder else {
        queue.write_buffer(buffer, offset, bytes);
        return;
    };
    let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ordered write"),
        contents: bytes,
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    encoder.copy_buffer_to_buffer(&staging, 0, buffer, offset, bytes.len() as u64);
}

#[cfg(test)]
mod tests {
    use super::QueuedCaptures;

    #[test]
    fn nothing_is_queued_at_first() {
        let queued = QueuedCaptures::default();
        assert!(!queued.commands.has_captures());
        assert!(queued.commands.take_all().is_empty());
        assert_eq!(queued.shadow_environment, None);
    }
}
