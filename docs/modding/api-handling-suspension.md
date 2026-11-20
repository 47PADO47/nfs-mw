# Handling: the suspension hook (`bb.handling` 1.1)

Part of [handling](api-handling.md) and the [modding catalogue](README.md); decision in [ADR 0016](../decisions/0016-handling-hooks.md).
**W** marks a write that is denied in an online session.

The suspension hook lets a mod replace the vertical force of an axle: the spring, the damper, the stops and the anti-roll bar
that couple the two wheels of it. It is generic: it names no model and assumes nothing about how the mod gets its answer. A
linear spring with a viscous damper, a damper with a knee, an anti-roll bar, a sub-stepped wheel hub that carries its own mass,
and a model that only adds camber all use the same hook. The host calls it once per axle per physics tick; the mod answers with the
force each of the two wheels puts on the body, the vertical load of each tyre and the camber of each wheel, or declines and the
built-in spring runs for the axle.

Status: **built** (the host seam), since `bb.handling` 1.1. In NFS:MW it applies to the player's car only.

## Using it from a mod

```rust
use blackbox_modding::sdk::{Ctx, Error, Mod, ModResult, SuspensionQuery, SuspensionResult, SuspensionWheelResult, declare_mod};

struct Stiff;

impl Mod for Stiff {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let handling = ctx.handling().ok_or(Error::Unsupported)?;
        handling.register_suspension(|_ctx: &mut Ctx, q: &SuspensionQuery| -> Option<SuspensionResult> {
            let wheel = |w: &blackbox_modding::sdk::SuspensionWheelQuery| SuspensionWheelResult {
                body_force_n: w.static_load_n + 60_000.0 * (w.compression_m - q.rest_travel_m),
                tyre_load_n: w.static_load_n,
                camber_rad: -0.03,
            };
            Some(SuspensionResult { wheels: [wheel(&q.wheels[0]), wheel(&q.wheels[1])] })
        })?;
        Ok(Stiff)
    }
}

declare_mod!(Stiff);
```

The manifest says `[requires.interfaces] "bb.handling" = "1.1"`. A mod that can work without the hook asks for `1.0` and
treats `Error::Unsupported` from `register_suspension` as "this host is older".

| SDK call | Meaning |
|---|---|
| `handling.register_suspension(hook) -> ModResult<HandlingHookId>` | register the hook; `Error::Exists` when one is in place (one per mod and one per game), `Error::Denied` in an online session, `Error::Unsupported` on a host older than 1.1 |
| `handling.unregister_suspension(id)` | remove it; the built-in springs run from the next tick |

The hook is `FnMut(&mut Ctx, &SuspensionQuery) -> Option<SuspensionResult>`. A panic inside it is caught by the SDK, returned
as an error status and counted as a fault.

## Why one call per axle

The two wheels of an axle are coupled: an anti-roll bar is a spring on the difference of the two wheels' travel, and a model that
keeps a hub state per wheel must advance both in the same sub-step. A call per wheel would give the mod the left wheel's road
input before it knows the right wheel's, so the pair would run one wheel behind the other. The host therefore probes the ground
under all four wheels first, then asks the front axle and the rear axle, then runs the four wheels. One call per axle with both wheels
inside is the simplest layout that lets a model integrate the pair together, and it halves the number of calls compared with one
per wheel. A mod that wants per-wheel state keeps it itself; the host reports the wheels as `wheels[0]` (left) and `wheels[1]`
(right) of every call.

## Wire layout

`BbHandling` grew by 16 bytes in 1.1: `register_suspension` is at 32 and `unregister_suspension` at 40. The layouts are locked by
`abi/iface/handling_tests.rs`. The struct rules are those of [api-handling.md](api-handling.md#wire-layout): `size` first,
fixed-width fields, reserved words at the end, appended fields take reserved words.

| Function | Signature | Answers |
|---|---|---|
| `register_suspension` | `(ctx, hook: *const BbSuspensionHook, out_id: *mut u64) -> BbStatus` | `BB_OK`; `BB_ERR_ARG` (null pointer, missing callback, a `size` that does not cover the callback); `BB_ERR_EXISTS`; `BB_ERR_DENIED` (online); `BB_ERR_UNSUPPORTED` (the game does not implement it) |
| `unregister_suspension` | `(ctx, id: u64) -> BbStatus` | `BB_OK`; `BB_ERR_STALE` for a handle that is not the caller's hook |

`BbSuspensionHook` (registration, 56 bytes, align 8) has the layout of `BbTyreHook`: `size` at 0, `flags` at 4 (reserved), `user` at
8, `callback` at 16, `reserved` `[u64; 4]` at 24. The callback is `unsafe extern "C" fn(user, query: *const BbSuspensionQuery,
result: *mut BbSuspensionResult) -> BbStatus`. It returns `BB_SUSPENSION_HANDLED` (1) after writing the result,
`BB_SUSPENSION_NOT_HANDLED` (0) to decline, or a negative `BB_ERR_*` code, which counts as a fault.

`BbSuspensionQuery` (144 bytes, align 4; SI units). The host writes it; a reader reads `size` bytes and treats the rest as zero.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes the host filled in |
| 4 | `axle_index` | u32 | 0 front axle, 1 rear axle |
| 8 | `role` | u32 | the kind of vehicle, `BB_ROLE_*` (0 player) |
| 12 | `vehicle_revision` | u32 | the `revision` of `spec` for the vehicle this call is for; changes when the car changes |
| 16 | `dt` | f32 | the tick length, s; the same for both axles of a tick |
| 20 | `rest_travel_m` | f32 | the compression of a wheel of this axle standing at rest on flat ground, m: the sag at which the vehicle's own spring carries the wheel's static load (`static_load_n`). It depends on the car, not on the ride-height tuning, so it is constant for a `vehicle_revision`. A road input a model derives from the compression is `compression_m - rest_travel_m`, which is 0 for a car at rest |
| 24 | `max_travel_m` | f32 | the largest compression, m |
| 28 | `corner_mass_kg` | f32 | the sprung mass the host attributes to one wheel, kg: a quarter of the vehicle mass |
| 32 | `wheels[0]` | `BbSuspensionWheel` | the left wheel, 40 bytes |
| 72 | `wheels[1]` | `BbSuspensionWheel` | the right wheel, 40 bytes |
| 112 | `reserved` | `[u32; 8]` | zero |

`BbSuspensionWheel` (40 bytes, align 4; grows only by taking a reserved word).

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `loaded` | u32 | 1 when the wheel presses on the ground, 0 when it is airborne |
| 4 | `compression_m` | f32 | how far the wheel is pushed up from its full droop, m, from 0 to `max_travel_m`; positive when the wheel moves up towards the body; 0 when the wheel is not touching. This tick's value |
| 8 | `compression_speed_mps` | f32 | the rate of change of the compression over the last tick, m/s, positive when compressing |
| 12 | `steer_angle_rad` | f32 | the steering angle of this wheel, rad, positive to the right; 0 on an axle that does not steer |
| 16 | `body_accel_up_mps2` | f32 | the acceleration of the body point above the wheel along the body's up axis, m/s^2, kinematic: 0 at rest and about -9.81 in free fall. Taken over the last tick; 0 on the first tick after the hook is installed and after the vehicle is placed |
| 20 | `tyre_lateral_force_n` | f32 | the lateral force the tyre of this wheel put on the body in the last tick, N, positive to the right; 0 for an airborne wheel and on the first tick. It is what the tyre hook (or the built-in tyre) answered, after the host's own limit |
| 24 | `static_load_n` | f32 | the vertical load the host estimates for this wheel at rest, N: the weight the axle carries, shared by its two wheels |
| 28 | `reserved` | `[u32; 3]` | zero |

`BbSuspensionResult` (76 bytes, align 4). The host sets `size` and zeroes the rest before the call.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes of the layout the host reads |
| 4 | `wheels[0]` | `BbSuspensionWheelResult` | the left wheel, 20 bytes |
| 24 | `wheels[1]` | `BbSuspensionWheelResult` | the right wheel, 20 bytes |
| 44 | `reserved` | `[u32; 8]` | zero |

`BbSuspensionWheelResult` (20 bytes, align 4):

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `body_force_n` | f32 | the force on the body at the wheel along the body's up axis, N, positive up: the mean over the tick. It replaces the built-in spring force. It may be negative (a droop stop pulls the body down) |
| 4 | `tyre_load_n` | f32 | the vertical force between the road and the tyre, N, at or above 0: the mean over the tick. It becomes the vertical load of the wheel's tyre |
| 8 | `camber_rad` | f32 | the camber of the wheel, rad, **positive when the top of the wheel leans away from the centre line of the car**, on either side. It reaches the tyre hook as `camber_rad` |
| 12 | `reserved` | `[u32; 2]` | zero |

## What the host guarantees

- **Thread and scope.** The hook runs on the main thread, inside the physics step, and inside a mod callback scope: the host calls
  that are valid in a callback (for example `bb.vehicles` `spec` and `state`) may be made from it. Do not call a write that steps the
  world.
- **Call order.** In every tick the host first probes the ground under all four wheels, then calls the hook for the **front axle
  (`axle_index` 0), then the rear axle (1)**, then runs the four wheels in index order 0 to 3, asking the tyre hook for each. So
  both axles of a tick are answered before any tyre of that tick is asked, and the camber of a wheel reaches that wheel's tyre
  query in the same tick. Nothing else of the mod runs in between.
- **`dt`.** The tick length the world is stepped with: 1/60 s by default, 1/144 s in the offline comparison mode, 1/240 s in a
  speed-break (see [physics-timing.md](../physics-timing.md)); the same for both axles of a tick. Never assume one value. The mod may
  integrate in as many substeps as it likes inside the call and keeps whatever state it wants; the host sees only the mean forces.
- **`rest_travel_m` and `max_travel_m`** are in every query and are constants of the car: a mod that wants them once per car reads
  them when `vehicle_revision` changes. `rest_travel_m` is the compression a wheel reads when the car stands still on flat ground
  (it is **not** the design ride height of the vehicle's data, which can be larger than the whole travel); it is computed from the
  vehicle's own spring and the weight on the axle, so a car that sits on its springs reads `compression_m - rest_travel_m` of 0.
- **`compression_m`.** It is the host's own measure: the ride height plus the penetration of the contact point, clamped to
  `0..max_travel_m`. A wheel whose ground is within reach and pressed against it reads above 0; a hanging wheel reads 0 and
  `loaded` 0. The road input a model derives from it is `compression_m - rest_travel_m`.
- **Airborne wheels.** A wheel that does not press on the ground is in the query (`loaded` 0, compression 0), so a model with a
  hub state can let it hang. A force a model answers for an airborne wheel acts on the body (a droop stop pulls it down) as long as
  at least one wheel of the vehicle is loaded; when no wheel is loaded the host applies no wheel force to the body at all, as it
  does not for the built-in spring.
- **What the host does with the answer.** For each wheel: the body force replaces the built-in spring force and acts at the
  wheel's contact patch together with the tyre forces; the tyre load replaces the load the host derives from its own spring and
  becomes the vertical load of that wheel's tyre in the query of the tyre hook (or in the built-in tyre) without further scaling, so
  the host's slope correction of the load is **not** applied; the camber is given to the tyre hook of the same wheel the same
  tick (`TyreQuery::camber_rad`) and is 0 for a wheel the suspension hook did not answer. The built-in tyres ignore the camber. The
  host's own anti-roll bar and damper do not run on a handled axle.
- **Which vehicles.** In v1 the player's car only (`role` is always 0), with the standard chassis, in both tyre integration modes.
  The rolling chassis, the coasting tyres and the simple police tyres keep their own rules and are never asked (the same rule as
  for the tyre hook).

## What the mod must return

- Finite numbers in the units above. An answer is refused when a field is NaN or infinite, or out of range: `body_force_n` up to
  1e6 N in either direction, `tyre_load_n` from 0 to 1e6 N, `camber_rad` up to 0.6 rad in either direction.
- `BB_SUSPENSION_HANDLED` with a result for both wheels, or `BB_SUSPENSION_NOT_HANDLED` to leave the axle to the built-in
  spring. Declining is free and is not a fault. The host never mixes: it takes both wheels of an answer or neither.

Nothing else is required: no state, no substep count, no particular spring law.

## Faults, budget and removal

The policy is that of the tyre hook ([api-handling.md](api-handling.md#faults-budget-and-removal)), per hook: a negative status or
a refused answer is a fault and the built-in spring runs for that axle this tick; the fifth fault quarantines the hook, the vehicle
drops it and the game removes the registration; a call over 2 ms is counted and warned about (a call is per axle, so the budget
applies to each of the two calls of a tick). The faults are also counted against the mod in the runtime, so the mod's quarantine
follows. `unregister_suspension` or an unload retires the hook at once; the scene takes the model off the car at the next frame
boundary, never between two axles of a tick.

## Online sessions

The hook is refused in an online session, like the others: `register_suspension` answers `BB_ERR_DENIED`, and a hook registered
before the session began is taken off the car while it lasts.

## Writing your own suspension

- **A spring and a damper per wheel.** For each wheel, `body_force_n = static_load_n + k * (compression_m - rest_travel_m) + c *
  compression_speed_mps`, and `tyre_load_n = body_force_n` (no unsprung mass). It is the simplest model, and it is the built-in
  law without its progression and its anti-roll bar.
- **An anti-roll bar.** Add `k_arb * (c_left - c_right)` to the left wheel and subtract it from the right one; the pair is in one
  call, so the bar sees both compressions of the same tick.
- **A hub with mass.** Keep `x` and `v` per wheel, integrate them in substeps of `dt / n` with the road input `compression_m -
  rest_travel_m` interpolated over the tick, and return the mean link force as `body_force_n` and the mean tyre force as `tyre_load_n`.
  `body_accel_up_mps2` is the acceleration of the body the hub moves with; `static_load_n` reseeds the hub when `vehicle_revision`
  changes.
- **Camber only.** Return the host's own force. The hook has no pass-through: compute the force yourself.

Rules of thumb: keep the call under 1 ms (it runs twice a tick); never allocate in it; test the model without the game by calling
it on hand-made queries (the SDK's query structs have `Default`).

## Host notes (NFS:MW)

`crates/nfsmw/src/modding/backends/handling.rs` is the port and puts the model on the lead player's car with the others (see
[api-handling.md](api-handling.md#host-notes-nfsmw)). The vehicle side is `libs/blackbox-vehicle/src/suspension/model.rs` (the
`SuspensionModel` trait) and `vehicle/suspension_model.rs` (the seam); the tests are `vehicle/tests/suspension_model.rs` (dispatch,
bit-identity without a model, camber and load reaching the tyre), `host/table/handling_hooks_tests.rs` (the table and the adapter),
`sdk/ffi/handling_tests.rs` (the SDK round trip) and `crates/nfsmw/src/modding/backends/handling_hooks_tests.rs` (the port).
