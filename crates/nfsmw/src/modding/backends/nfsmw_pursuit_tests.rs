//! The `nfsmw.pursuit` port: reads come from the published chase, changes are queued for the world or the console.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_NOT_READY, BB_ERR_UNSUPPORTED};
use blackbox_pursuit::Fines;
use nfsmw_modding::abi::{CopTable, EndMode};
use nfsmw_modding::host::PursuitBackend;

use super::bridge::Bridge;
use super::nfsmw_pursuit::PursuitPort;
use super::support::MemoryStore;
use crate::modding::outbox;
use crate::modding::pursuit_feed::{PursuitRequest, PursuitView};

fn port() -> (PursuitPort, Rc<Bridge>) {
    let bridge = Rc::new(Bridge::with_store(Box::new(MemoryStore::default())));
    (PursuitPort(bridge.clone()), bridge)
}

fn fines() -> Fines {
    Fines::new([100, 110, 120, 130, 140, 150, 160, 170])
}

/// Publish a chase at heat `heat`, with `cops` cop cars and the busted meter `meter`.
fn publish_chase(heat: f32, cops: u32, meter: f32) {
    outbox::publish(PursuitView {
        pursuing: true,
        heat,
        level: heat as u32,
        cops,
        busted_meter: meter,
        ..PursuitView::idle(fines())
    });
}

#[test]
fn without_a_chase_the_heat_calls_are_not_ready_and_the_reads_are_empty() {
    outbox::publish(PursuitView::idle(fines()));
    let (port, _bridge) = port();
    assert_eq!(port.heat_get("m"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.heat_set("m", 4.0), Err(BB_ERR_NOT_READY));
    assert_eq!(port.end("m", EndMode::Abort), Err(BB_ERR_NOT_READY));
    assert_eq!(port.is_pursuing("m"), Ok(false));
    assert_eq!(port.cops_in_pursuit("m"), Ok(0));
    assert_eq!(port.busted_get("m"), Ok(0.0));
    assert!(outbox::take_requests().is_empty(), "nothing is queued without a chase");
}

#[test]
fn a_running_chase_reads_from_its_snapshot() {
    publish_chase(3.5, 6, 0.25);
    let (port, _bridge) = port();
    assert_eq!(port.heat_get("m"), Ok(3.5));
    assert_eq!(port.is_pursuing("m"), Ok(true));
    assert_eq!(port.cops_in_pursuit("m"), Ok(6));
    assert_eq!(port.busted_get("m"), Ok(0.25));
    outbox::take_requests();
}

#[test]
fn a_heat_set_and_an_end_are_queued_for_the_world_in_order() {
    publish_chase(2.0, 2, 0.0);
    let (port, _bridge) = port();
    assert_eq!(port.heat_set("m", 7.5), Ok(()));
    assert_eq!(port.end("m", EndMode::Bust), Ok(()));
    assert_eq!(outbox::take_requests(), [PursuitRequest::SetHeat(7.5), PursuitRequest::End(EndMode::Bust)]);
}

#[test]
fn a_start_runs_the_console_pursuit_command_next_frame() {
    let (port, bridge) = port();
    assert_eq!(port.start("m", 4, None), Ok(()));
    assert_eq!(bridge.queued_lines.borrow().as_slice(), ["pursuit 4".to_owned()]);
}

#[test]
fn the_cop_tables_are_not_wired_yet() {
    let (port, _bridge) = port();
    assert_eq!(port.cop_table_get("m", CopTable::Roam, 1).err(), Some(BB_ERR_UNSUPPORTED));
}
