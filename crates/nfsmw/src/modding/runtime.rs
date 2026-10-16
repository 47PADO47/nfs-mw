//! The mods the game runs: the runtime, the frame counter, and the sending of the events.

use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;

use blackbox_modding::abi::{BB_EVENT_FRAME_START, BB_EVENT_MOD_UNLOADING, BB_EVENT_SCENE_TICK, BbEvent};
use blackbox_modding::host::{DispatchReport, HostOffer, ModRuntime, ModSummary, Policy, Rejected, RunMode};

use super::events::{frame_start_payload, scene_tick_payload, unloading_payload};

/// The loaded mods, with the frame counter the events carry. It is a `NonSend` resource: the mods run on the main
/// thread, and the runtime holds `Rc`s.
pub struct Mods {
    runtime: ModRuntime,
    frame: u64,
}

impl Mods {
    /// Discover, order and load the mods in `roots` (see [`ModRuntime::start`]). A mod that fails is recorded, and the
    /// game starts without it.
    pub fn start(offer: HostOffer, policy: &Policy, roots: &[PathBuf], mode: RunMode) -> Self {
        Self { runtime: ModRuntime::start(offer, policy, roots, mode), frame: 0 }
    }

    /// The loaded mods, in load order.
    pub fn loaded(&self) -> Vec<ModSummary> {
        self.runtime.loaded()
    }

    /// The mods refused, with their reasons.
    pub fn rejected(&self) -> &[Rejected] {
        self.runtime.rejected()
    }

    /// A frame begins (the `ModsPre` set). Counts the frame, then sends `FRAME_START`.
    pub fn frame_start(&mut self, dt_real: f32) -> DispatchReport {
        self.frame += 1;
        self.deliver(BB_EVENT_FRAME_START, &frame_start_payload(dt_real))
    }

    /// The scene has updated (the `ModsPost` set). Sends `SCENE_TICK`.
    pub fn scene_tick(&mut self, dt_real: f32, game_speed: f32, paused: bool) -> DispatchReport {
        self.deliver(BB_EVENT_SCENE_TICK, &scene_tick_payload(dt_real, game_speed, paused))
    }

    /// Send one event with a payload to every loaded mod, in load order.
    fn deliver<T>(&mut self, kind: u32, payload: &T) -> DispatchReport {
        let event = BbEvent {
            size: size_of::<BbEvent>() as u32,
            kind,
            frame: self.frame,
            payload: (payload as *const T).cast::<c_void>(),
        };
        self.runtime.dispatch_event(&event)
    }
}

impl Drop for Mods {
    /// Before the libraries close, every mod hears `MOD_UNLOADING` for each mod still loaded, last loaded first. The
    /// runtime then destroys them and closes their libraries.
    fn drop(&mut self) {
        let ids: Vec<String> = self.runtime.loaded().into_iter().rev().map(|summary| summary.id).collect();
        for id in &ids {
            self.deliver(BB_EVENT_MOD_UNLOADING, &unloading_payload(id));
        }
    }
}
