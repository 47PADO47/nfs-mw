//! `bb.handling` and the car's physical summary (`bb.vehicles` 1.3 `spec`): the player's car takes the tyre hook of a mod.
//!
//! One hook is kept for the whole game (the player's car has one tyre-model slot). It is applied to the lead player's car only:
//! cops, traffic and racers keep the built-in tyres, and so does the second player of a co-op game. The scene puts the model
//! on the car at a frame boundary ([`Bridge::handling_frame`], called after the mods' callbacks and before the physics run), so
//! a register, an unregister, a quarantine, a mod unloading or a new car is reconciled within a frame and never between two
//! wheels of one tick.
//!
//! A multiplayer client refuses the hook: a separately built mod cannot promise identical float results on every peer, and the
//! server never trusts client state (ADR 0005 section 6, ADR 0016 section 4). The refusal is `BB_ERR_DENIED` at registration, and
//! a hook registered before the session began is taken off the car for as long as the session lasts.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use blackbox_modding::abi::{
    BB_DRIVE_ALL, BB_DRIVE_FRONT, BB_DRIVE_REAR, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_STALE, BB_SPEC_GEARS,
};
use blackbox_modding::host::{BackendResult, HandlingBackend, TyreHook, VehicleSpecSheet};
use blackbox_vehicle::{DriveLayout, VehicleSummary};

use super::bridge::Bridge;
use crate::viewer::{HandlingFeed, Scene, TyreMaker};

/// The hook a mod registered and its handle.
struct Registered {
    handle: u64,
    hook: TyreHook,
}

/// The car's summary as the mods read it, with the number that says when it changed.
struct CarSpec {
    car: Option<String>,
    summary: VehicleSummary,
}

/// What `bb.handling` and the spec keep in the bridge.
#[derive(Default)]
pub(super) struct HandlingState {
    hook: RefCell<Option<Registered>>,
    /// The scene is part of a networked session, as of the last frame.
    networked: Cell<bool>,
    /// The summary of the player's car, or `None` while no car is driven.
    spec: RefCell<Option<CarSpec>>,
    /// Changes whenever the car or its summary changes; shared with the adapters, which echo it in each query.
    revision: Rc<Cell<u32>>,
    /// Faults of the hook already counted against the mod in the runtime.
    mirrored: Cell<u32>,
    /// The refusal of a hook in an online session is logged once per session.
    refusal_logged: Cell<bool>,
}

/// The `bb.handling` port of the game: one per mod table, sharing the bridge.
pub struct HandlingPort(pub(super) Rc<Bridge>);

impl HandlingBackend for HandlingPort {
    fn register_tyre(&self, owner: &str, hook: TyreHook) -> BackendResult<u64> {
        let state = &self.0.handling;
        if state.networked.get() {
            log::warn!("mod {owner}: bb.handling refused in an online session; the built-in tyres run");
            return Err(BB_ERR_DENIED);
        }
        if let Some(existing) = state.hook.borrow().as_ref() {
            log::warn!("mod {owner}: bb.handling refused, the tyre hook of mod {} is in place", existing.hook.owner());
            return Err(BB_ERR_EXISTS);
        }
        let handle = self.0.next_handle();
        *state.hook.borrow_mut() = Some(Registered { handle, hook });
        log::info!("mod {owner}: tyre hook registered");
        Ok(handle)
    }

    fn unregister_tyre(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let state = &self.0.handling;
        let mut slot = state.hook.borrow_mut();
        let Some(registered) =
            slot.take_if(|registered| registered.handle == handle && registered.hook.owner() == owner)
        else {
            return Err(BB_ERR_STALE);
        };
        registered.hook.retire();
        state.mirrored.set(0);
        log::info!("mod {owner}: tyre hook removed");
        Ok(())
    }
}

impl Bridge {
    /// Once per mod frame: put the tyre hook on the player's car (or take it off), and read what the scene reports for
    /// `bb.handling` and `bb.vehicles` `spec`. Called after [`Bridge::sync`] and after [`Bridge::apply`].
    pub(super) fn handling_frame(&self, scene: &mut dyn Scene) {
        let state = &self.handling;
        let live = state.live_hook();
        let revision = state.revision.clone();
        let maker = live.filter(|_| !state.networked.get()).map(|hook| move || hook.model(revision.clone()));
        let mut feed = HandlingFeed::default();
        scene.handling_feed(maker.as_ref().map(|make| make as &TyreMaker), &mut feed);
        state.networked.set(feed.networked);
        if feed.networked && maker.is_some() {
            scene.handling_feed(None, &mut feed);
            if !state.refusal_logged.replace(true) {
                log::warn!("bb.handling: online session, the mod's tyre hook is off while it lasts");
            }
        }
        state.refusal_logged.set(state.refusal_logged.get() && feed.networked);
        let car = self.engine.borrow().as_ref().map(|readout| readout.car.clone());
        state.note_car(car, feed.summary);
    }

    /// The faults the tyre hook has had since the last call, with the id of its mod, for the runtime's counter.
    pub(crate) fn take_hook_faults(&self) -> Option<(String, u32)> {
        let state = &self.handling;
        let hook = state.hook.borrow();
        let hook = &hook.as_ref()?.hook;
        let faults = hook.stats().faults();
        let new = faults.saturating_sub(state.mirrored.replace(faults));
        (new > 0).then(|| (hook.owner().to_owned(), new))
    }

    /// The mod `owner` is quarantined: its tyre hook is dropped and retired (the car loses the model at the next frame boundary).
    pub(crate) fn drop_hook_of(&self, owner: &str) {
        let state = &self.handling;
        let Some(registered) = state.hook.borrow_mut().take_if(|registered| registered.hook.owner() == owner) else {
            return;
        };
        registered.hook.retire();
        state.mirrored.set(0);
        log::warn!("mod {owner}: tyre hook dropped, the mod is quarantined");
    }

    /// The owner of the tyre hook in place, if any.
    pub(crate) fn tyre_hook_owner(&self) -> Option<String> {
        self.handling.hook.borrow().as_ref().map(|registered| registered.hook.owner().to_owned())
    }

    /// The physical summary of the player's car as the mods read it.
    pub(super) fn vehicle_sheet(&self) -> Option<VehicleSpecSheet> {
        let spec = self.handling.spec.borrow();
        Some(sheet_of(&spec.as_ref()?.summary, self.handling.revision.get()))
    }
}

impl HandlingState {
    /// The hook, unless it has been quarantined or retired, in which case it is dropped.
    fn live_hook(&self) -> Option<TyreHook> {
        let mut slot = self.hook.borrow_mut();
        let dead = slot.as_ref().is_some_and(|registered| !registered.hook.stats().is_live());
        if dead {
            let registered = slot.take()?;
            log::warn!(
                "mod {}: tyre hook dropped after {} faults",
                registered.hook.owner(),
                registered.hook.stats().faults()
            );
            self.mirrored.set(0);
            return None;
        }
        slot.as_ref().map(|registered| registered.hook.clone())
    }

    /// Remember the summary of the car; a different car or a different summary is a new revision.
    fn note_car(&self, car: Option<String>, summary: Option<VehicleSummary>) {
        let mut spec = self.spec.borrow_mut();
        let Some(summary) = summary else {
            *spec = None;
            return;
        };
        let same = spec.as_ref().is_some_and(|known| known.car == car && known.summary == summary);
        if same {
            return;
        }
        *spec = Some(CarSpec { car, summary });
        self.revision.set(self.revision.get().wrapping_add(1).max(1));
    }
}

/// The summary in the units and layout of the ABI.
fn sheet_of(summary: &VehicleSummary, revision: u32) -> VehicleSpecSheet {
    let mut gear_ratios = [0.0; BB_SPEC_GEARS];
    let gears = summary.gear_ratios.len().min(BB_SPEC_GEARS);
    gear_ratios[..gears].copy_from_slice(&summary.gear_ratios[..gears]);
    VehicleSpecSheet {
        revision,
        mass_kg: summary.mass_kg,
        wheelbase_m: summary.wheelbase_m,
        track_m: summary.track_m,
        cg_height_m: summary.cg_height_m,
        front_weight_fraction: summary.front_weight_fraction,
        tyre_radius_m: summary.tyre_radius_m,
        tyre_width_m: summary.tyre_width_m,
        peak_friction: summary.peak_friction,
        sliding_friction: summary.sliding_friction,
        drive_layout: match summary.drive {
            DriveLayout::Front => BB_DRIVE_FRONT,
            DriveLayout::Rear => BB_DRIVE_REAR,
            DriveLayout::All => BB_DRIVE_ALL,
        },
        gear_count: summary.gear_ratios.len() as u32,
        max_power_w: summary.max_power_w,
        max_power_rpm: summary.max_power_rpm,
        redline_rpm: summary.redline_rpm,
        final_drive: summary.final_drive,
        gear_ratios,
        brake_torque_nm: summary.brake_torque_nm,
        drag_area_m2: summary.drag_area_m2,
        downforce_area_m2: summary.downforce_area_m2,
    }
}
