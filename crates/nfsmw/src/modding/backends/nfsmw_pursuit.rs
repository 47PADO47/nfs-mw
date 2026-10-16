//! `nfsmw.pursuit` on the game side. Reads come from the world's last published snapshot (`outbox::view`); changes are
//! queued as requests the world applies in its police update, or, for a chase start, as a console line that runs the
//! `pursuit` command the player has (the same path as typing it). A call that needs a chase answers `BB_ERR_NOT_READY`
//! while none is on. The cop waves are not wired yet: they answer `BB_ERR_UNSUPPORTED`.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_NOT_READY, BB_ERR_UNSUPPORTED, BbStatus};
use nfsmw_modding::abi::EndMode;
use nfsmw_modding::host::{BackendResult, HeatRowSpec, PursuitBackend};

use super::bridge::Bridge;
use crate::modding::outbox;
use crate::modding::pursuit_feed::{PursuitRequest, PursuitView};

/// The `nfsmw.pursuit` port of the game: one per mod table, sharing the bridge.
pub struct PursuitPort(pub(super) Rc<Bridge>);

/// The snapshot of a chase that is on, or `BB_ERR_NOT_READY` while none is.
fn running() -> BackendResult<PursuitView> {
    match outbox::view() {
        Some(view) if view.pursuing => Ok(view),
        _ => Err(BB_ERR_NOT_READY),
    }
}

/// The status of a cop table call: the game does not expose the install's waves to mods yet.
fn no_cop_tables() -> BbStatus {
    BB_ERR_UNSUPPORTED
}

impl PursuitBackend for PursuitPort {
    fn heat_get(&self, _owner: &str) -> BackendResult<f32> {
        Ok(running()?.heat)
    }

    fn heat_set(&self, _owner: &str, heat: f32) -> BackendResult<()> {
        running()?;
        outbox::request(PursuitRequest::SetHeat(heat));
        Ok(())
    }

    fn cop_table_get(
        &self,
        _owner: &str,
        _table: nfsmw_modding::abi::CopTable,
        _level: u32,
    ) -> BackendResult<HeatRowSpec> {
        Err(no_cop_tables())
    }

    fn cop_table_set(
        &self,
        _owner: &str,
        _table: nfsmw_modding::abi::CopTable,
        _level: u32,
        _spec: &HeatRowSpec,
    ) -> BackendResult<()> {
        Err(no_cop_tables())
    }

    fn start(&self, _owner: &str, level: u32, _near: Option<u64>) -> BackendResult<()> {
        // `near` is not used yet: the console's `pursuit` command places the cops by the player's road.
        self.0.queued_lines.borrow_mut().push(format!("pursuit {level}"));
        Ok(())
    }

    fn end(&self, _owner: &str, mode: EndMode) -> BackendResult<()> {
        running()?;
        outbox::request(PursuitRequest::End(mode));
        Ok(())
    }

    fn is_pursuing(&self, _owner: &str) -> BackendResult<bool> {
        Ok(outbox::view().is_some_and(|view| view.pursuing))
    }

    fn busted_get(&self, _owner: &str) -> BackendResult<f32> {
        // The meter keeps its value after a chase, until the next one starts (the game's own rule).
        Ok(outbox::view().map_or(0.0, |view| view.busted_meter))
    }

    fn cops_in_pursuit(&self, _owner: &str) -> BackendResult<u32> {
        Ok(outbox::view().map_or(0, |view| view.cops))
    }
}
