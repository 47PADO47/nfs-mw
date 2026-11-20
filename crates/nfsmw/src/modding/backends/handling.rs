//! `bb.handling` and the car's physical summary (`bb.vehicles` 1.3 `spec`): the player's car takes the tyre, suspension and
//! controls hooks of a mod.
//!
//! One hook of each kind is kept for the whole game (the player's car has one slot per kind). It is applied to the lead player's
//! car only: cops, traffic and racers keep the built-in code, and so does the second player of a co-op game. The scene puts the
//! models on the car at a frame boundary ([`Bridge::handling_frame`], called after the mods' callbacks and before the physics
//! run), so a register, an unregister, a quarantine, a mod unloading or a new car is reconciled within a frame and never between
//! two wheels of one tick.
//!
//! A multiplayer client refuses the hooks: a separately built mod cannot promise identical float results on every peer, and the
//! server never trusts client state (ADR 0005 section 6, ADR 0016 section 4). The refusal is `BB_ERR_DENIED` at registration, and a
//! hook registered before the session began is taken off the car for as long as the session lasts.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use blackbox_modding::abi::{
    BB_DRIVE_ALL, BB_DRIVE_FRONT, BB_DRIVE_REAR, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_STALE, BB_SPEC_GEARS,
};
use blackbox_modding::host::{
    BackendResult, ControlsHook, HandlingBackend, ModHook, SuspensionHook, TyreHook, VehicleSpecSheet, WireQuery,
};
use blackbox_vehicle::{DriveLayout, VehicleSummary};

use super::bridge::Bridge;
use crate::viewer::{HandlingFeed, HandlingMakers, Scene};

/// The hook of one kind a mod registered, and how many of its faults the runtime has already counted against the mod.
struct Kept<Q: 'static, R: 'static> {
    hook: RefCell<Option<(u64, ModHook<Q, R>)>>,
    mirrored: Cell<u32>,
}

impl<Q: 'static, R: 'static> Default for Kept<Q, R> {
    fn default() -> Self {
        Kept { hook: RefCell::new(None), mirrored: Cell::new(0) }
    }
}

impl<Q: WireQuery + 'static, R: 'static> Kept<Q, R> {
    /// Keep `hook` for `owner` under `handle`, unless the session is online or a hook is in place.
    fn put(
        &self,
        owner: &str,
        hook: ModHook<Q, R>,
        networked: bool,
        handle: impl FnOnce() -> u64,
    ) -> BackendResult<u64> {
        let kind = Q::KIND;
        if networked {
            log::warn!("mod {owner}: bb.handling refused in an online session; the built-in {kind} code runs");
            return Err(BB_ERR_DENIED);
        }
        if let Some((_, existing)) = self.hook.borrow().as_ref() {
            log::warn!("mod {owner}: bb.handling refused, the {kind} hook of mod {} is in place", existing.owner());
            return Err(BB_ERR_EXISTS);
        }
        let handle = handle();
        *self.hook.borrow_mut() = Some((handle, hook));
        log::info!("mod {owner}: {kind} hook registered");
        Ok(handle)
    }

    /// Drop the hook `owner` registered under `handle`, and retire it.
    fn remove(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let removed = self.hook.borrow_mut().take_if(|(known, hook)| *known == handle && hook.owner() == owner);
        let Some((_, hook)) = removed else {
            return Err(BB_ERR_STALE);
        };
        hook.retire();
        self.mirrored.set(0);
        log::info!("mod {owner}: {} hook removed", Q::KIND);
        Ok(())
    }

    /// The hook, unless it has been quarantined or retired, in which case it is dropped.
    fn live(&self) -> Option<ModHook<Q, R>> {
        let mut slot = self.hook.borrow_mut();
        let dead = slot.as_ref().is_some_and(|(_, hook)| !hook.stats().is_live());
        if dead {
            let (_, hook) = slot.take()?;
            log::warn!("mod {}: {} hook dropped after {} faults", hook.owner(), Q::KIND, hook.stats().faults());
            self.mirrored.set(0);
            return None;
        }
        slot.as_ref().map(|(_, hook)| hook.clone())
    }

    /// The faults of the hook since the last call, with the id of its mod.
    fn take_faults(&self) -> Option<(String, u32)> {
        let slot = self.hook.borrow();
        let (_, hook) = slot.as_ref()?;
        let faults = hook.stats().faults();
        let new = faults.saturating_sub(self.mirrored.replace(faults));
        (new > 0).then(|| (hook.owner().to_owned(), new))
    }

    /// Drop and retire the hook when it is `owner`'s (the mod is quarantined).
    fn drop_of(&self, owner: &str) {
        let Some((_, hook)) = self.hook.borrow_mut().take_if(|(_, hook)| hook.owner() == owner) else {
            return;
        };
        hook.retire();
        self.mirrored.set(0);
        log::warn!("mod {owner}: {} hook dropped, the mod is quarantined", Q::KIND);
    }

    /// The owner of the hook in place, if any.
    fn owner(&self) -> Option<String> {
        self.hook.borrow().as_ref().map(|(_, hook)| hook.owner().to_owned())
    }
}

/// The car's summary as the mods read it, with the number that says when it changed.
struct CarSpec {
    car: Option<String>,
    summary: VehicleSummary,
}

/// What `bb.handling` and the spec keep in the bridge.
#[derive(Default)]
pub(super) struct HandlingState {
    tyre: Kept<blackbox_modding::abi::BbTyreQuery, blackbox_modding::abi::BbTyreResult>,
    suspension: Kept<blackbox_modding::abi::BbSuspensionQuery, blackbox_modding::abi::BbSuspensionResult>,
    controls: Kept<blackbox_modding::abi::BbControlsQuery, blackbox_modding::abi::BbControlsResult>,
    /// The scene is part of a networked session, as of the last frame.
    networked: Cell<bool>,
    /// The summary of the player's car, or `None` while no car is driven.
    spec: RefCell<Option<CarSpec>>,
    /// Changes whenever the car or its summary changes; shared with the adapters, which echo it in each query.
    revision: Rc<Cell<u32>>,
    /// The refusal of a hook in an online session is logged once per session.
    refusal_logged: Cell<bool>,
}

/// The `bb.handling` port of the game: one per mod table, sharing the bridge.
pub struct HandlingPort(pub(super) Rc<Bridge>);

impl HandlingBackend for HandlingPort {
    fn register_tyre(&self, owner: &str, hook: TyreHook) -> BackendResult<u64> {
        let state = &self.0.handling;
        state.tyre.put(owner, hook, state.networked.get(), || self.0.next_handle())
    }

    fn unregister_tyre(&self, owner: &str, handle: u64) -> BackendResult<()> {
        self.0.handling.tyre.remove(owner, handle)
    }

    fn register_suspension(&self, owner: &str, hook: SuspensionHook) -> BackendResult<u64> {
        let state = &self.0.handling;
        state.suspension.put(owner, hook, state.networked.get(), || self.0.next_handle())
    }

    fn unregister_suspension(&self, owner: &str, handle: u64) -> BackendResult<()> {
        self.0.handling.suspension.remove(owner, handle)
    }

    fn register_controls(&self, owner: &str, hook: ControlsHook) -> BackendResult<u64> {
        let state = &self.0.handling;
        state.controls.put(owner, hook, state.networked.get(), || self.0.next_handle())
    }

    fn unregister_controls(&self, owner: &str, handle: u64) -> BackendResult<()> {
        self.0.handling.controls.remove(owner, handle)
    }
}

impl Bridge {
    /// Once per mod frame: put the hooks of the mods on the player's car (or take them off), and read what the scene reports for
    /// `bb.handling` and `bb.vehicles` `spec`. Called after [`Bridge::sync`] and after [`Bridge::apply`].
    pub(super) fn handling_frame(&self, scene: &mut dyn Scene) {
        let state = &self.handling;
        let online = state.networked.get();
        let revision = state.revision.clone();
        let tyre = state.tyre.live().filter(|_| !online).map(|hook| {
            let revision = revision.clone();
            move || hook.model(revision.clone())
        });
        let suspension = state.suspension.live().filter(|_| !online).map(|hook| {
            let revision = revision.clone();
            move || hook.model(revision.clone())
        });
        let controls = state.controls.live().filter(|_| !online).map(|hook| {
            let revision = revision.clone();
            move || hook.model(revision.clone())
        });
        let makers = HandlingMakers::of(tyre.as_ref(), suspension.as_ref(), controls.as_ref());
        let mut feed = HandlingFeed::default();
        scene.handling_feed(&makers, &mut feed);
        state.networked.set(feed.networked);
        let any = tyre.is_some() || suspension.is_some() || controls.is_some();
        if feed.networked && any {
            scene.handling_feed(&HandlingMakers::default(), &mut feed);
            if !state.refusal_logged.replace(true) {
                log::warn!("bb.handling: online session, the mod's hooks are off while it lasts");
            }
        }
        state.refusal_logged.set(state.refusal_logged.get() && feed.networked);
        let car = self.engine.borrow().as_ref().map(|readout| readout.car.clone());
        state.note_car(car, feed.summary);
    }

    /// The faults the hooks have had since the last call, with the id of their mods, for the runtime's counter. One entry per hook
    /// that has new faults.
    pub(crate) fn take_hook_faults(&self) -> Vec<(String, u32)> {
        let state = &self.handling;
        [state.tyre.take_faults(), state.suspension.take_faults(), state.controls.take_faults()]
            .into_iter()
            .flatten()
            .collect()
    }

    /// The mod `owner` is quarantined: its hooks are dropped and retired (the car loses the models at the next frame boundary).
    pub(crate) fn drop_hook_of(&self, owner: &str) {
        let state = &self.handling;
        state.tyre.drop_of(owner);
        state.suspension.drop_of(owner);
        state.controls.drop_of(owner);
    }

    /// The owners of the hooks in place (one entry per hook).
    pub(crate) fn hook_owners(&self) -> Vec<String> {
        let state = &self.handling;
        [state.tyre.owner(), state.suspension.owner(), state.controls.owner()].into_iter().flatten().collect()
    }

    /// The physical summary of the player's car as the mods read it.
    pub(super) fn vehicle_sheet(&self) -> Option<VehicleSpecSheet> {
        let spec = self.handling.spec.borrow();
        Some(sheet_of(&spec.as_ref()?.summary, self.handling.revision.get()))
    }
}

impl HandlingState {
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
