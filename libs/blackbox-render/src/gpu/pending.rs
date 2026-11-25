//! The command buffers waiting for the frame's submission, some still being finished on worker threads.
//!
//! wgpu validates and translates a command encoder's commands when it is finished, not when they are recorded, so
//! finishing a frame's encoders one after another on the frame thread is most of what recording a frame costs.
//! A frame therefore closes its encoder at a few points ([`PendingCommands::finish_later`]) and goes on recording
//! into a fresh one while a worker finishes the closed part. The parts keep their order: whoever submits takes them
//! in the order they were queued, waiting for the ones not finished yet, so the GPU runs the same commands in the
//! same order as one encoder would give it.
//!
//! An entry is a capture (see `queued.rs`) or a part of the frame being recorded. A part's commands would still sit
//! in the frame's open encoder if it had not been closed, so a submission that only sends the captures on their way
//! ([`PendingCommands::take_captures`]) leaves the parts queued after the last capture where they are.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, sync_channel};

enum Commands {
    Ready(wgpu::CommandBuffer),
    Finishing(Receiver<wgpu::CommandBuffer>),
}

struct Entry {
    commands: Commands,
    capture: bool,
}

/// The queued command buffers, in submission order. Shared by reference with the pass that submits in the middle
/// of the post chain (native DLSS), hence the lock; only the frame thread takes it.
#[derive(Default)]
pub(in crate::gpu) struct PendingCommands {
    entries: Mutex<Vec<Entry>>,
}

impl PendingCommands {
    fn entries(&self) -> std::sync::MutexGuard<'_, Vec<Entry>> {
        self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether a capture waits for the next submission.
    pub(in crate::gpu) fn has_captures(&self) -> bool {
        self.entries().iter().any(|entry| entry.capture)
    }

    /// Queue a capture's finished commands.
    pub(in crate::gpu) fn push_capture(&self, commands: wgpu::CommandBuffer) {
        self.entries().push(Entry { commands: Commands::Ready(commands), capture: true });
    }

    /// Queue `encoder` and finish it on a worker thread. `capture` says its commands hold a capture, which later
    /// writes to what the capture reads must send on its way first.
    pub(in crate::gpu) fn finish_later(&self, encoder: wgpu::CommandEncoder, capture: bool) {
        let (sender, receiver) = sync_channel(1);
        rayon::spawn(move || {
            let _trace = blackbox_trace::scope("GPU API: finish command encoder");
            // The receiver is gone only when the renderer was dropped in the meantime.
            let _ = sender.send(encoder.finish());
        });
        self.entries().push(Entry { commands: Commands::Finishing(receiver), capture });
    }

    /// Every queued command buffer up to the last capture, in order, waiting for the ones still being finished.
    pub(in crate::gpu) fn take_captures(&self) -> Vec<wgpu::CommandBuffer> {
        let mut entries = self.entries();
        let Some(last) = entries.iter().rposition(|entry| entry.capture) else { return Vec::new() };
        let taken: Vec<Entry> = entries.drain(..=last).collect();
        drop(entries);
        resolve(taken)
    }

    /// Every queued command buffer, in order, waiting for the ones still being finished.
    pub(in crate::gpu) fn take_all(&self) -> Vec<wgpu::CommandBuffer> {
        let taken = std::mem::take(&mut *self.entries());
        resolve(taken)
    }
}

fn resolve(entries: Vec<Entry>) -> Vec<wgpu::CommandBuffer> {
    if entries.is_empty() {
        return Vec::new();
    }
    let _trace = blackbox_trace::scope("GPU API: wait for finished command encoders");
    entries
        .into_iter()
        .map(|entry| match entry.commands {
            Commands::Ready(commands) => commands,
            Commands::Finishing(receiver) => receiver.recv().expect("a worker finishes every queued command encoder"),
        })
        .collect()
}
