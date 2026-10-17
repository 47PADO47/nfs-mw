//! `nfsmw.pursuit` on the game side. Reads come from the world's last published snapshot (`outbox::view`); changes are
//! queued as requests the world applies in its police update, or, for a chase start, as a console line that runs the
//! `pursuit` command the player has (the same path as typing it). A call that needs a chase answers `BB_ERR_NOT_READY`
//! while none is on. The cop waves are read from the install (or the wave a mod set) and set for the next chase
//! (`crate::modding::cop_rows`).

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_NOT_READY};
use nfsmw_modding::abi::{CopTable, EndMode};
use nfsmw_modding::host::{BackendResult, HeatRowSpec, PursuitBackend};

use super::bridge::Bridge;
use crate::modding::cop_rows;
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

impl PursuitBackend for PursuitPort {
    fn heat_get(&self, _owner: &str) -> BackendResult<f32> {
        Ok(running()?.heat)
    }

    fn heat_set(&self, _owner: &str, heat: f32) -> BackendResult<()> {
        running()?;
        outbox::request(PursuitRequest::SetHeat(heat));
        Ok(())
    }

    fn cop_table_get(&self, _owner: &str, table: CopTable, level: u32) -> BackendResult<HeatRowSpec> {
        // A wave a mod set for the session answers first; otherwise the install's wave answers.
        let row = match cop_rows::get(table, level) {
            Some(row) => row,
            None => self.0.install_row(table, level)?,
        };
        Ok(cop_rows::spec_of(&row))
    }

    fn cop_table_set(&self, _owner: &str, table: CopTable, level: u32, spec: &HeatRowSpec) -> BackendResult<()> {
        // The wave applies to the next chase (`TrafficWorld::start_pursuit`); a running one keeps its rows.
        let row = cop_rows::row_of(spec).ok_or(BB_ERR_ARG)?;
        cop_rows::set(table, level, row);
        Ok(())
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
