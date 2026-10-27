//! `bb.engine`: the player's pedals and the engine's switch (`docs/modding/api-engine.md`). The state is the bridge's copy of
//! the scene this frame; a stop and a start are queued as a [`WorldRequest`] and made on the scene after the mods have run.
//! The game remembers which mod stopped the engine, so that it runs again when that mod unloads.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::{BackendResult, EngineBackend, EngineState};

use super::bridge::Bridge;
use super::world::WorldRequest;
use crate::engine_switch::{EngineCommand, StartSound};

/// The `bb.engine` port of the game: one per mod table, sharing the bridge.
pub struct EnginePort(pub(super) Rc<Bridge>);

impl EnginePort {
    /// The car is driven, or `NotReady`.
    fn ready(&self) -> BackendResult<()> {
        if self.0.engine.borrow().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        Ok(())
    }

    fn queue(&self, command: EngineCommand) {
        self.0.world_requests.borrow_mut().push(WorldRequest::Engine(command));
    }
}

impl EngineBackend for EnginePort {
    fn state(&self, _owner: &str) -> BackendResult<EngineState> {
        self.0.engine.borrow().as_ref().map(|readout| readout.state).ok_or(BB_ERR_NOT_READY)
    }

    fn car(&self, _owner: &str) -> BackendResult<String> {
        self.0.engine.borrow().as_ref().map(|readout| readout.car.clone()).ok_or(BB_ERR_NOT_READY)
    }

    fn stop(&self, owner: &str) -> BackendResult<()> {
        self.ready()?;
        *self.0.engine_holder.borrow_mut() = Some(owner.to_owned());
        self.queue(EngineCommand::Stop);
        Ok(())
    }

    fn start(&self, owner: &str, sound: &str, gain: f32, delay: f32) -> BackendResult<()> {
        self.ready()?;
        let mut holder = self.0.engine_holder.borrow_mut();
        if holder.as_deref() == Some(owner) {
            *holder = None;
        }
        let sound = (!sound.is_empty()).then(|| StartSound { file: sound.to_owned(), gain });
        self.queue(EngineCommand::Start { sound, delay });
        Ok(())
    }

    fn release(&self, owner: &str) {
        let mut holder = self.0.engine_holder.borrow_mut();
        if holder.as_deref() != Some(owner) {
            return;
        }
        *holder = None;
        self.queue(EngineCommand::Start { sound: None, delay: 0.0 });
    }
}
