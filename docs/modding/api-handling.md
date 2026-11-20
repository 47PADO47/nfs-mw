# Handling (`bb.handling`, and `bb.vehicles` 1.3 `spec`)

Part of the [modding catalogue](README.md); decision in [ADR 0016](../decisions/0016-handling-hooks.md). **W** marks a write
that is denied in an online session.

`bb.handling` lets a mod replace how a vehicle's tyres turn load, slip and torque into forces. It is generic: it names no
game and no model, and it assumes nothing about how the mod computes its answer. A drift mod that scales the lateral grip, an
arcade tyre with a constant friction coefficient, a thermal tyre model, a tyre mod for another game's bikes and a full
simulation model all use the same hook. The host calls the hook once per wheel per physics tick; the mod answers with that
wheel's forces and its new spin, or declines and the built-in tyre runs.

Status: **built** (the host seam). Version 1.0 has one hook, the tyre hook. The suspension and controls hooks are
reserved ([how a hook is added](#how-a-future-hook-is-added)). In NFS:MW it applies to the player's car only.

## Using it from a mod

```rust
use blackbox_modding::sdk::{Ctx, Error, Mod, ModResult, Role, TyreQuery, TyreResult, declare_mod};

struct Grippy;

impl Mod for Grippy {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let handling = ctx.handling().ok_or(Error::Unsupported)?;
        handling.register_tyre(|_ctx: &mut Ctx, q: &TyreQuery| -> Option<TyreResult> {
            if q.role != Some(Role::Player) {
                return None; // the built-in tyre runs for every other vehicle
            }
            Some(TyreResult { traction: 1.0, wheel_spin: q.wheel_spin, ..TyreResult::default() })
        })?;
        Ok(Grippy)
    }
}

declare_mod!(Grippy);
```

A complete working mod is [`mods/handling-probe`](../../mods/handling-probe/README.md), the fixture the host tests load. The
manifest says `[requires.interfaces] "bb.handling" = "1.0"`, and `"bb.vehicles" = "1.3"` under `[optional.interfaces]` for the
car's `spec`.

| SDK call | Meaning |
|---|---|
| `ctx.handling() -> Option<Handling>` | the interface, when the host offers it |
| `handling.register_tyre(hook) -> ModResult<HandlingHookId>` | register the tyre hook; `Error::Exists` when one is in place, `Error::Denied` in an online session |
| `handling.unregister_tyre(id)` | remove it; the built-in tyres run from the next tick |
| `ctx.vehicles()?.spec() -> ModResult<VehicleSpec>` | the car in physical numbers ([below](#the-car-in-physical-numbers-bbvehicles-13-spec)) |

The hook is `FnMut(&mut Ctx, &TyreQuery) -> Option<TyreResult>`. A panic inside it is caught by the SDK, returned as an error
status, and counted as a fault.

## Wire layout

`IFACE_HANDLING` is `bb.handling`, `IFACE_HANDLING_VERSION` is `(1, 0)`. Every table and struct starts with its size; the
layouts are locked by `abi/iface/handling_tests.rs`.

`BbHandling` is 32 bytes: `head` (`BbIface`) at 0, `ctx` at 8, `register_tyre` at 16, `unregister_tyre` at 24.

| Function | Signature | Answers |
|---|---|---|
| `register_tyre` | `(ctx, hook: *const BbTyreHook, out_id: *mut u64) -> BbStatus` | `BB_OK`; `BB_ERR_ARG` (null pointer, missing callback, a `size` that does not cover the callback); `BB_ERR_EXISTS` (a hook is in place; one per mod and one per game); `BB_ERR_DENIED` (online) |
| `unregister_tyre` | `(ctx, id: u64) -> BbStatus` | `BB_OK`; `BB_ERR_STALE` for a handle that is not the caller's hook |

`BbTyreHook` (registration, 56 bytes, align 8): `size` u32 at 0, `flags` u32 at 4 (`BB_TYRE_FLAG_ALL_ROLES` = 1 is reserved;
v1 ignores it), `user` pointer at 8, `callback` at 16, `reserved` `[u64; 4]` at 24. The host needs bytes up to the end of
`callback` (24); a smaller `size` is `BB_ERR_ARG`.

The callback: `unsafe extern "C" fn(user, query: *const BbTyreQuery, result: *mut BbTyreResult) -> BbStatus`. It returns
`BB_TYRE_HANDLED` (1) after writing the result, `BB_TYRE_NOT_HANDLED` (0) to decline, or a negative `BB_ERR_*` code, which
counts as a fault.

`BbTyreQuery` (120 bytes, align 4; SI units). The host writes it; a field a newer minor appends takes a reserved word, and a
reader reads `size` bytes and treats the rest as zero.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes the host filled in |
| 4 | `wheel_index` | u32 | 0 front left, 1 front right, 2 rear left, 3 rear right |
| 8 | `role` | u32 | the kind of vehicle, `BB_ROLE_*` (0 player, 1 traffic, 2 cop, 3 racer) |
| 12 | `loaded` | u32 | 1 when the wheel presses on the ground, 0 when it is airborne |
| 16 | `vehicle_revision` | u32 | the `revision` of `spec` for the vehicle this call is for |
| 20 | `surface_tag` | u32 | an opaque surface id from the game (0 when airborne) |
| 24 | `dt` | f32 | the tick length, s |
| 28 | `vertical_load_n` | f32 | vertical load on the tyre, N (0 when airborne) |
| 32 | `patch_lateral_velocity` | f32 | speed of the contact patch along the wheel's lateral axis, m/s, positive to the right (0 when airborne). Sampled at the start of this tick's wheel update, before this tick's forces move the body |
| 36 | `patch_forward_velocity` | f32 | speed of the contact patch along the wheel's heading, m/s, positive forward (0 when airborne). Sampled like the lateral one |
| 40 | `body_speed` | f32 | speed of the whole body, m/s |
| 44 | `wheel_spin` | f32 | rad/s, positive when rolling forward |
| 48 | `drive_torque_nm` | f32 | drive torque at the hub this tick, N m, **signed**: positive pushes the wheel forward, negative in reverse or under engine braking |
| 52 | `brake_torque_nm` | f32 | the **total** brake torque at the hub this tick, N m, handbrake contribution included: always at or above 0, and it opposes the rotation (the mod applies the sign against `wheel_spin`) |
| 56 | `wheel_radius_m` | f32 | rolling radius, m |
| 60 | `wheel_inertia_kgm2` | f32 | moment of inertia of the wheel, kg m^2: the host's constant for its own models (10 in NFS:MW, the same for every wheel). A mod may use its own |
| 64 | `surface_lateral` | f32 | surface multiplier of lateral force (1 on dry tarmac) |
| 68 | `surface_drive` | f32 | surface multiplier of longitudinal force (1 on dry tarmac) |
| 72 | `surface_rolling` | f32 | surface multiplier of rolling resistance (1 on dry tarmac) |
| 76 | `tyre_condition` | f32 | 1 for a sound tyre, below 1 for a damaged one (0.3 when blown) |
| 80 | `ambient_temperature_c` | f32 | degrees Celsius; the host has no weather model and reports 20 |
| 84 | `road_temperature_c` | f32 | degrees Celsius; the host has no thermal model and reports 25 |
| 88 | `corner_mass_kg` | f32 | the sprung mass the host attributes to this wheel, kg: a quarter of the vehicle mass. A model can bound its low-speed force with it: a force above `corner_mass_kg * speed / dt` reverses the motion within one tick |
| 92 | `reserved` | `[u32; 7]` | zero |

`BbTyreResult` (64 bytes, align 4). The host sets `size` and zeroes the rest before the call.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes of the layout the host reads |
| 4 | `longitudinal_force_n` | f32 | mean force over the tick, N, positive forward |
| 8 | `lateral_force_n` | f32 | mean force over the tick, N, positive to the right |
| 12 | `aligning_torque_nm` | f32 | mean self-aligning torque over the tick, N m, positive turns the wheel to the right |
| 16 | `wheel_spin` | f32 | the spin at the end of the tick, rad/s |
| 20 | `traction` | f32 | 1 while the tyre grips, below 1 while it slides, 0 to 1; feeds the effects |
| 24 | `slip` | f32 | wheel speed minus ground speed along the heading, m/s; feeds the effects |
| 28 | `slip_angle` | f32 | in turns (1.0 is 360 degrees), positive when the patch moves to the right; feeds the effects |
| 32 | `reserved` | `[u32; 8]` | zero |

The wheel frame: a forward axis (the wheel's heading, steering included) and a lateral axis that points to the right of it.
The force, velocity and torque signs follow from that: a positive longitudinal force pushes the car forward, a positive
lateral force pushes it to the right, a patch that moves to the right has a positive lateral velocity (so a force that resists
it is negative).

## What the host guarantees

- **Thread and scope.** The hook runs on the main thread, inside the physics step, and inside a mod callback scope: the host
  calls that are valid in a callback (for example `bb.vehicles` `spec` and `state`) may be made from it. Do not call a write
  that steps the world.
- **Order.** One call per wheel per tick, wheels in index order 0 to 3, the four calls of a tick one after the other. Nothing
  else of the mod runs in between.
- **`dt`.** The tick length the world is stepped with: 1/60 s by default, 1/144 s in the offline comparison mode, 1/240 s in
  a speed-break (see [physics-timing.md](../physics-timing.md)); it is the same for the four wheels of a tick. Never assume
  one value. The mod may integrate in as many substeps as it likes inside the call, and keeps whatever state it wants; the host
  sees only the mean forces and the new spin.
- **Loaded and airborne.** A loaded wheel (`loaded` 1) has a positive load and the patch velocities of its contact. An
  airborne wheel is called too (`loaded` 0): load, patch velocities and surface are zero, and only the torques the powertrain
  sends remain. Advance the spin and answer zero forces; the host takes only `wheel_spin` of an airborne answer.
- **`wheel_spin` is the host's.** The powertrain writes the spin between ticks (a locked clutch, a gear change). Read it from
  the query on every call, and do not keep your own copy.
- **Which vehicles.** In v1 the player's car only (`role` is always 0). Cops, traffic and racers run the built-in tyres. A
  hook that checks `role` keeps working when a later version offers other vehicles.
- **Which chassis.** A vehicle with the standard chassis takes the hook in both tyre integration modes. The special
  chassis (the rolling chassis, the coasting tyres and the simple police tyres) keep their own rules and are never asked.
- **What the host does with the answer.** It writes the spin back (the drivetrain sees it), adds the forces to the body at the
  contact patch, keeps `traction`, `slip` and `slip_angle` for the effects (smoke, skid marks, sound), and stores the aligning
  torque for readers that want it. It limits the lateral force so a tyre cannot reverse the patch's sideways motion within one
  tick, as it does for the built-in tyres.

## What the mod must return

- Finite numbers in the units above. A result is refused when a field is NaN or infinite, or out of range: forces up to
  1e6 N, the aligning torque up to 1e5 N m, the spin up to 5000 rad/s, `slip` up to 1000 m/s, `slip_angle` up to 1 turn, and
  `traction` from 0 to 1.
- `BB_TYRE_HANDLED` with a result, or `BB_TYRE_NOT_HANDLED` to leave the wheel to the built-in tyre. Declining for a wheel is
  free and is not a fault.

Nothing else is required: no state, no substep count, no curve.

## Faults, budget and removal

| Event | What the host does |
|---|---|
| The callback returns a negative status (the SDK returns `BB_ERR_PANIC` for a panic) | a fault; the built-in tyre runs for that wheel this tick |
| The answer is refused (not finite, or out of range) | a fault; the built-in tyre runs for that wheel this tick |
| The fifth fault | the hook is quarantined: it answers "not handled" for good, the vehicle drops the model, and the game removes the registration. The faults are also counted against the mod in the runtime's counter (`ModRuntime::record_fault`), so the mod's quarantine (no more events) follows |
| A mod quarantined by its events | its hook is dropped |
| A call over 2 ms | counted, and a warning with the mod id at most every 10 s. It is not a fault; a hook that blocks stalls the frame (ADR 0005 section 4) |
| `unregister_tyre`, or the mod unloads | the hook is retired at once (a car that still holds it never calls into a library that is gone), and the scene takes the model off the car at the next frame boundary, never between two wheels of a tick |
| An online session | see below |

## Online sessions

The hook is refused in an online session: `register_tyre` answers `BB_ERR_DENIED` and the game logs one line with the reason. A
hook registered before the session began is taken off the car while the session lasts and comes back when it ends. A separately
built mod cannot promise identical floating point results on every peer, and the server never trusts client state (ADR 0005
section 6, ADR 0016 section 4).

## The car in physical numbers (`bb.vehicles` 1.3 `spec`)

`spec(out)` writes `BbVehicleSpec` (168 bytes) to the mod's struct, as far as the mod's `size` word allows, or answers
`BB_ERR_NOT_READY` while no car is driven. `BbVehicles` is 72 bytes now: `spec` is at 64. Every value is the host's best estimate
from the car's data, in SI units; a mod may use or ignore any of them. Axle pairs are front then rear.

| Offset | Field | Meaning |
|---|---|---|
| 0 | `size` u32 | bytes the caller allows |
| 4 | `revision` u32 | changes whenever the car or its summary changes; starts at 1; the tyre query carries the same number |
| 8 | `mass_kg` | mass of the whole car, kg |
| 12 | `wheelbase_m` | distance between the axles, m |
| 16, 20 | `track_front_m`, `track_rear_m` | distance between the tyre centres of the axle, m |
| 24 | `cg_height_m` | height of the centre of gravity above the road at rest, m |
| 28 | `front_weight_fraction` | share of the weight on the front axle, 0 to 1 |
| 32, 36 | `tyre_radius_front_m`, `tyre_radius_rear_m` | rolling radius, m |
| 40, 44 | `tyre_width_front_m`, `tyre_width_rear_m` | section width, m |
| 48, 52 | `peak_friction_front`, `peak_friction_rear` | grip at the limit of adhesion, dimensionless, **in the host's own tyre scale** (see below) |
| 56, 60 | `sliding_friction_front`, `sliding_friction_rear` | grip while sliding, dimensionless, same scale |
| 64 | `drive_layout` u32 | `BB_DRIVE_FRONT` 0, `BB_DRIVE_REAR` 1, `BB_DRIVE_ALL` 2 |
| 68 | `gear_count` u32 | forward gears; the first `min(gear_count, 8)` of `gear_ratios` are set |
| 72 | `max_power_w` | highest engine power on the torque curve, W, before forced induction and nitrous |
| 76 | `max_power_rpm` | the engine speed of that power |
| 80 | `redline_rpm` | the rev limiter |
| 84 | `final_drive` | the final drive ratio |
| 88 | `gear_ratios` `[f32; 8]` | forward ratios from first gear, without the final drive; entries past `gear_count` are 0 |
| 120, 124 | `brake_torque_front_nm`, `brake_torque_rear_nm` | brake torque at one wheel, N m |
| 128 | `drag_area_m2` | `Cd * A`, m^2, such that `0.5 * 1.225 * area * speed^2` is the drag with the throttle open |
| 132 | `downforce_area_m2` | lift area of the downforce, m^2 |
| 136 | `reserved` `[u32; 8]` | zero |

These are the final names. The friction coefficients are per axle: `peak_friction_front`, `peak_friction_rear` (at the limit of
grip) and `sliding_friction_front`, `sliding_friction_rear` (while sliding), all dimensionless, taken from the car's data.
They are values of the host's own tyre scale, **not physical friction coefficients**: the host's tyre model is calibrated so
that a high-performance tyre reads about 2.075 (peak) and 1.85 (sliding). Treat them as relative figures: the ratio between
two cars or two axles is meaningful, the absolute number is not a coefficient of friction. Like every field they are **the
host's best estimate; a mod may ignore them** and use its own.

**Fields that are 0 in NFS:MW today.** `downforce_area_m2` is always 0: the engine's downforce grows linearly with the speed,
so no quadratic lift area describes it. `drag_area_m2` is 0 for a car whose data has no drag. The reserved words are 0. Every
other field is filled from the car's data. How they are derived: the track is the distance between the tyre centres (the
data gives the outer edges; the tyre width is taken off); the centre of gravity height is the data's roll centre height, which
is how the engine places the centre of gravity; the friction coefficients are the data's static and dynamic grip per axle; the
brake torque is the data's value converted to N m with the engine's brake scale; the drag area is `2 * k / 1.225` for the
engine's drag coefficient `k`.

**Knowing when the car changed.** The wire ABI has no `PlayerCarChanged` event today (the catalogue lists it as planned), so a
mod does not get one. It reads `spec().revision` on `FrameStart`, or compares `vehicle_revision` in the query: when the number
differs from the one the mod built its parameters for, read `spec` again. `spec` answers as of the start of the frame, so for
the first tick after a swap the revision in the query may be newer than the spec; keep the old parameters for that tick and
read again on the next.

## Which vehicles take the hook

| Vehicle | Takes the hook | Why |
|---|---|---|
| The player's car, standard chassis, regularized integration mode | yes | |
| The player's car, standard chassis, reference integration mode (every car the game builds from its data) | yes | the mode feeds the powertrain with the road speed while a tyre grips and with the wheel spin otherwise; the host fills `road_speed`, `load`, `traction` and the spin the same way for a model's answer, and the tests drive a car on a model in both modes (`vehicle/tests/tyre_model.rs`) |
| A cop, a traffic car, a racer | no (v1) | the game offers the hook to the player's car only; their balance depends on the built-in tyres |
| The second player of a co-op game | no | the hook is for the lead player |
| A vehicle with the rolling chassis, the coasting tyres or the simple police tyres | no | they keep their own rules |
| A cloned vehicle (a client's prediction) | no model | a clone runs the built-in tyres unless the model copies itself; the game's adapter does, but the hook is refused online anyway |

## Writing your own handling mod

Three shapes, none of them special:

- **A constant-friction arcade tyre.** Ignore the slip curves. Compute the force as `mu * load` against the direction the patch
  slides, ramped over the first half metre per second so it does not chatter, cap the combined force at `mu * load`, and
  integrate the spin implicitly with the torques you are given. The probe's `friction-circle` variant is exactly this, in 30 lines.
- **A drift mod that scales the lateral grip.** Do not replace the tyre at all: return `None` for every wheel of the vehicles
  you do not want, and for the player compute the built-in-like force yourself with a lateral stiffness scaled by a setting
  you read from `bb.settings` or `bb.config`. A mod that wants the built-in forces with one number changed cannot have them
  from this hook (the hook replaces the tyre; it does not wrap it), so its tyre is its own.
- **A thermal tyre mod.** Keep a temperature per wheel in the closure's state, advance it from `dt`, the sliding speed and
  `ambient_temperature_c`, scale a peak friction from `spec()` (`peak_friction_front` and `peak_friction_rear`) by it, and
  return `None` when `role` is not the player's. Reset the state when `vehicle_revision` changes.

Rules of thumb: keep the call under 2 ms for four wheels at the highest tick rate you expect (a sub-stepped integration should
bound its substeps); never allocate in the hook if you can avoid it; read `wheel_spin` from the query every time; and test the
mod without the game by calling your model on hand-made queries (the SDK's `TyreQuery` has `Default`).

## How a future hook is added

A hook is two function pointers appended after `unregister_tyre` in `BbHandling`, with a minor version bump (`1.1` for the
suspension hook, `1.2` for the controls hook, as the ADR proposes), and its own query, result and registration structs that
follow the struct rules: `size` first, fixed-width fields, reserved words at the end, appended fields taking reserved words.
Nothing already in the table moves. A mod built for 1.0 reads only the fields its `head.size` covers. The host's
`HandlingBackend` gains a method with a default that answers `BB_ERR_UNSUPPORTED`, so a game that does not implement the
hook keeps building. Neither hook is implemented in 1.0.

## Host notes (NFS:MW)

`crates/nfsmw/src/modding/backends/handling.rs` is the port. The tyre hook is put on the lead player's car by
`Scene::handling_feed`, once after the mods' callbacks and once before them each frame, so it reaches a new car within a
frame. The car's summary is `Vehicle::summary` in `blackbox-vehicle`. The first tests: `libs/blackbox-vehicle/src/vehicle/tests/tyre_model.rs`
(dispatch, modes, bit-identity with no model), `libs/blackbox-modding/src/host/table/handling_tests.rs` (the table and the
adapter), `libs/blackbox-modding/tests/handling_probe.rs` (the real loader and the fixture, the quarantine, the online refusal),
and `crates/nfsmw/src/modding/backends/handling_tests.rs` (the port).
