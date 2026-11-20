//! The hooks of a mod as the vehicle sees them: an adapter from the mod's C callback to the vehicle's model trait, for each of the
//! three hooks of `bb.handling` (tyre, suspension, controls).
//!
//! [`ModHook`] is the part they share: the call into the mod, the callback scope (so the mod may call the host's read calls from
//! inside), the measurement against the callback budget, and the fault count with the same [`FaultCounter`] the event dispatcher
//! uses. Five faults quarantine the hook: it answers "not handled" for good and reports itself retired, so the vehicle drops it. A
//! hook that is retired by its owner (unregistered, or the mod unloaded) behaves the same way at once, so a vehicle that still
//! holds the adapter never calls into a library that is gone. The tyre adapter is at the end of this file; the other two are in
//! `handling_suspension_model.rs` and `handling_controls_model.rs`.

use core::ffi::c_void;
use core::mem::size_of;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use blackbox_vehicle::tires::{TyreModel, TyreQuery, TyreReject, TyreResult};

use crate::abi::{
    BB_CONTROLS_HANDLED, BB_CONTROLS_NOT_HANDLED, BB_SUSPENSION_HANDLED, BB_SUSPENSION_NOT_HANDLED, BB_TYRE_HANDLED,
    BB_TYRE_NOT_HANDLED, BbControlsQuery, BbControlsResult, BbStatus, BbSuspensionQuery, BbSuspensionResult,
    BbTyreQuery, BbTyreResult,
};
use crate::host::fault::{FaultCounter, QUARANTINE_AFTER};
use crate::host::gate;

/// A call that takes longer than this is over budget (ADR 0005 section 4; the setting `mods.callback_budget_ms`).
pub const CALLBACK_BUDGET: Duration = Duration::from_millis(2);
/// The warning for a slow hook is logged at most this often.
const WARN_EVERY: Duration = Duration::from_secs(10);

// The three hooks answer with the same two status values; the shared call below compares against these.
const HANDLED: BbStatus = 1;
const NOT_HANDLED: BbStatus = 0;
const _: () = assert!(
    BB_TYRE_HANDLED == HANDLED
        && BB_SUSPENSION_HANDLED == HANDLED
        && BB_CONTROLS_HANDLED == HANDLED
        && BB_TYRE_NOT_HANDLED == NOT_HANDLED
        && BB_SUSPENSION_NOT_HANDLED == NOT_HANDLED
        && BB_CONTROLS_NOT_HANDLED == NOT_HANDLED
);

/// A wire query of one of the hooks: it names the hook in the log.
pub trait WireQuery {
    /// The hook this query belongs to, as the log says it: "tyre", "suspension" or "controls".
    const KIND: &'static str;
}

impl WireQuery for BbTyreQuery {
    const KIND: &'static str = "tyre";
}

impl WireQuery for BbSuspensionQuery {
    const KIND: &'static str = "suspension";
}

impl WireQuery for BbControlsQuery {
    const KIND: &'static str = "controls";
}

/// What the host knows about one registered hook: its owner, its faults, its calls and its time. Shared by every adapter
/// made from the hook and by the game's backend, which reads it to show the state and to retire the hook.
pub struct HookStats {
    owner: String,
    kind: &'static str,
    faults: RefCell<FaultCounter>,
    calls: Cell<u64>,
    over_budget: Cell<u32>,
    last_warning: Cell<Option<Instant>>,
    active: Cell<bool>,
}

impl HookStats {
    fn new(owner: &str, kind: &'static str) -> Self {
        HookStats {
            owner: owner.to_owned(),
            kind,
            faults: RefCell::new(FaultCounter::default()),
            calls: Cell::new(0),
            over_budget: Cell::new(0),
            last_warning: Cell::new(None),
            active: Cell::new(true),
        }
    }

    /// The id of the mod that registered the hook.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Which hook this is: "tyre", "suspension" or "controls".
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    /// Faults counted so far.
    pub fn faults(&self) -> u32 {
        self.faults.borrow().faults()
    }

    /// Whether the hook reached the fault limit.
    pub fn quarantined(&self) -> bool {
        self.faults.borrow().is_quarantined()
    }

    /// Whether the hook may still be called: not retired by its owner and not quarantined.
    pub fn is_live(&self) -> bool {
        self.active.get() && !self.quarantined()
    }

    /// Calls made into the mod.
    pub fn calls(&self) -> u64 {
        self.calls.get()
    }

    /// Calls that took longer than [`CALLBACK_BUDGET`].
    pub fn over_budget(&self) -> u32 {
        self.over_budget.get()
    }

    /// Count one fault. The fault that reaches the limit is logged as a quarantine.
    pub(crate) fn fault(&self, what: &str) {
        let quarantined = self.faults.borrow_mut().record();
        let kind = self.kind;
        log::warn!("mod {}: {kind} hook fault ({what}), {} of {}", self.owner, self.faults(), QUARANTINE_AFTER);
        if quarantined {
            log::error!(
                "mod {}: {kind} hook quarantined after {QUARANTINE_AFTER} faults; the built-in {kind} code runs",
                self.owner
            );
        }
    }

    fn note_time(&self, took: Duration) {
        if took <= CALLBACK_BUDGET {
            return;
        }
        self.over_budget.set(self.over_budget.get().saturating_add(1));
        let due = self.last_warning.get().is_none_or(|last| last.elapsed() >= WARN_EVERY);
        if !due {
            return;
        }
        self.last_warning.set(Some(Instant::now()));
        let ms = took.as_secs_f64() * 1000.0;
        log::warn!(
            "mod {}: {} hook took {ms:.2} ms, over the {} ms budget",
            self.owner,
            self.kind,
            CALLBACK_BUDGET.as_millis()
        );
    }
}

/// The wire callback of a hook: user pointer, query, result.
pub(super) type WireFn<Q, R> = unsafe extern "C" fn(user: *mut c_void, query: *const Q, result: *mut R) -> BbStatus;

/// The call into the mod: the wire query in, the wire result out, and the status the callback returns.
type Call<Q, R> = dyn Fn(&Q, &mut R) -> BbStatus;

/// What one call into a hook came to.
pub(crate) enum Outcome {
    /// The callback answered: the result is valid.
    Handled,
    /// The hook declined, or is retired: the host runs its own code. Not a fault.
    NotHandled,
    /// The callback failed; the fault is counted.
    Fault,
}

/// A hook a mod registered: the call into the mod and the shared [`HookStats`]. Cheap to clone.
pub struct ModHook<Q: 'static, R: 'static> {
    stats: Rc<HookStats>,
    call: Rc<Call<Q, R>>,
}

/// The tyre hook of a mod.
pub type TyreHook = ModHook<BbTyreQuery, BbTyreResult>;
/// The suspension hook of a mod.
pub type SuspensionHook = ModHook<BbSuspensionQuery, BbSuspensionResult>;
/// The controls hook of a mod.
pub type ControlsHook = ModHook<BbControlsQuery, BbControlsResult>;

impl<Q: 'static, R: 'static> Clone for ModHook<Q, R> {
    fn clone(&self) -> Self {
        ModHook { stats: self.stats.clone(), call: self.call.clone() }
    }
}

impl<Q: WireQuery + 'static, R: 'static> ModHook<Q, R> {
    /// A hook for the mod `owner`. The registration trampoline makes one from what the mod registered; a game's tests make one
    /// from a callback of their own.
    ///
    /// # Safety
    /// `callback` must be safe to call with `user` for as long as the hook is not retired.
    pub unsafe fn new(owner: &str, callback: WireFn<Q, R>, user: *mut c_void) -> Self {
        // SAFETY: the caller keeps `callback` and `user` valid while the hook is live (see `# Safety`), and the adapter never calls
        // a retired hook.
        let call = move |query: &Q, result: &mut R| unsafe { callback(user, query, result) };
        ModHook { stats: Rc::new(HookStats::new(owner, Q::KIND)), call: Rc::new(call) }
    }

    /// A hook for the mod `owner` made from a Rust function instead of a C callback: for a game's own tests and for an
    /// implementation that lives inside the host. It follows the same contract as the callback: it returns the handled status
    /// after writing the result, the not-handled status, or a negative status.
    pub fn from_fn(owner: &str, call: impl Fn(&Q, &mut R) -> BbStatus + 'static) -> Self {
        ModHook { stats: Rc::new(HookStats::new(owner, Q::KIND)), call: Rc::new(call) }
    }

    /// The hook's faults, calls and time.
    pub fn stats(&self) -> &Rc<HookStats> {
        &self.stats
    }

    /// The id of the mod that registered the hook.
    pub fn owner(&self) -> &str {
        self.stats.owner()
    }

    /// Stop the hook for good: every adapter made from it answers "not handled" and reports itself retired from now on. The
    /// game's backend calls this when it drops the hook (unregistered, or the mod unloaded).
    pub fn retire(&self) {
        self.stats.active.set(false);
    }

    /// Whether the hook must not be called again (retired, or quarantined).
    pub(crate) fn is_retired(&self) -> bool {
        !self.stats.is_live()
    }

    /// Call into the mod with `query`; the callback writes `answer`. A retired hook is not called. The call runs inside the
    /// callback scope and is timed, and a failure status is counted as a fault.
    pub(crate) fn invoke(&self, query: &Q, answer: &mut R) -> Outcome {
        let stats = &self.stats;
        if !stats.is_live() {
            return Outcome::NotHandled;
        }
        let started = Instant::now();
        let status: BbStatus = {
            let _scope = gate::enter();
            // `stats.is_live()` was checked above: the hook is not retired, so the mod's library is still loaded.
            (self.call)(query, answer)
        };
        stats.calls.set(stats.calls.get().saturating_add(1));
        stats.note_time(started.elapsed());
        match status {
            HANDLED => Outcome::Handled,
            NOT_HANDLED => Outcome::NotHandled,
            error if error < 0 => {
                stats.fault(&format!("the callback returned status {error}"));
                Outcome::Fault
            }
            other => {
                stats.fault(&format!("the callback returned the unknown value {other}"));
                Outcome::Fault
            }
        }
    }

    /// The vehicle refused the last answer: count a fault.
    pub(crate) fn refused(&self, why: TyreReject) {
        let what = match why {
            TyreReject::NotFinite => "a result that is not finite",
            TyreReject::OutOfRange => "a result out of range",
        };
        self.stats.fault(what);
    }
}

impl TyreHook {
    /// An adapter for one vehicle. `revision` is read at each call and echoed in the query as `vehicle_revision`, so the game
    /// can change it while the adapter lives (the car's data changed).
    pub fn model(&self, revision: Rc<Cell<u32>>) -> Box<dyn TyreModel> {
        Box::new(ModTyreModel { hook: self.clone(), revision })
    }
}

/// The adapter the vehicle holds.
struct ModTyreModel {
    hook: TyreHook,
    revision: Rc<Cell<u32>>,
}

fn wire_query(query: &TyreQuery, revision: u32) -> BbTyreQuery {
    BbTyreQuery {
        size: size_of::<BbTyreQuery>() as u32,
        wheel_index: query.wheel_index,
        role: query.role,
        loaded: u32::from(query.loaded),
        vehicle_revision: revision,
        surface_tag: query.surface_tag,
        dt: query.dt,
        vertical_load_n: query.vertical_load_n,
        patch_lateral_velocity: query.patch_lateral_velocity,
        patch_forward_velocity: query.patch_forward_velocity,
        body_speed: query.body_speed,
        wheel_spin: query.wheel_spin,
        drive_torque_nm: query.drive_torque_nm,
        brake_torque_nm: query.brake_torque_nm,
        wheel_radius_m: query.wheel_radius_m,
        wheel_inertia_kgm2: query.wheel_inertia_kgm2,
        corner_mass_kg: query.corner_mass_kg,
        surface_lateral: query.surface_lateral,
        surface_drive: query.surface_drive,
        surface_rolling: query.surface_rolling,
        tyre_condition: query.tyre_condition,
        ambient_temperature_c: query.ambient_temperature_c,
        road_temperature_c: query.road_temperature_c,
        camber_rad: query.camber_rad,
        reserved: [0; 6],
    }
}

fn vehicle_result(wire: &BbTyreResult) -> TyreResult {
    TyreResult {
        longitudinal_force_n: wire.longitudinal_force_n,
        lateral_force_n: wire.lateral_force_n,
        aligning_torque_nm: wire.aligning_torque_nm,
        wheel_spin: wire.wheel_spin,
        traction: wire.traction,
        slip: wire.slip,
        slip_angle: wire.slip_angle,
    }
}

impl TyreModel for ModTyreModel {
    fn wheel(&mut self, query: &TyreQuery) -> Option<TyreResult> {
        let wire = wire_query(query, self.revision.get());
        let mut answer = BbTyreResult { size: size_of::<BbTyreResult>() as u32, ..BbTyreResult::default() };
        match self.hook.invoke(&wire, &mut answer) {
            Outcome::Handled => Some(vehicle_result(&answer)),
            Outcome::NotHandled | Outcome::Fault => None,
        }
    }

    fn rejected(&mut self, why: TyreReject) {
        self.hook.refused(why);
    }

    fn retired(&self) -> bool {
        self.hook.is_retired()
    }

    fn duplicate(&self) -> Option<Box<dyn TyreModel>> {
        Some(self.hook.model(self.revision.clone()))
    }
}
