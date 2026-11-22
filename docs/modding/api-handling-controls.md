# Handling: the controls hook (`bb.handling` 1.2)

Part of [handling](api-handling.md) and the [modding catalogue](README.md); decision in [ADR 0016](../decisions/0016-handling-hooks.md).
**W** marks a write that is denied in an online session.

The controls hook is a filter between the driver and the vehicle. The host shapes the driver's input as usual, asks the hook once per
tick, and uses its answer in place of the throttle, in place of the brake split and, if the mod asks for it, in place of its own
steering shaping. It is generic: it names no aid and no model. Traction control, ABS, a stability aid that brakes single wheels, a
brake-bias map, a steering-feel model with a speed-sensitive lock, a throttle map and a plain pass-through all use the same hook.
The mod sees the driver's request and the slip and acceleration state of the last tick, and answers with the throttle, a brake demand
per wheel and (optionally) a steering target; or it declines and the controls stay as shaped.

Status: **built** (the host seam), since `bb.handling` 1.2. In NFS:MW it applies to the player's car only.

## Using it from a mod

```rust
use blackbox_modding::sdk::{ControlsQuery, ControlsResult, Ctx, Error, Mod, ModResult, declare_mod};

struct Gate;

impl Mod for Gate {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let handling = ctx.handling().ok_or(Error::Unsupported)?;
        handling.register_controls(|_ctx: &mut Ctx, q: &ControlsQuery| -> Option<ControlsResult> {
            // Cut the throttle while a driven wheel spins, and pass the brake to every wheel unchanged.
            let spinning = q.slip_ratio[2].max(q.slip_ratio[3]) > 0.2 && q.forward_speed.abs() > 1.0;
            Some(ControlsResult {
                throttle: if spinning { q.throttle * 0.5 } else { q.throttle },
                brake: [q.brake; 4],
                steer_target_rad: None,
            })
        })?;
        Ok(Gate)
    }
}

declare_mod!(Gate);
```

The manifest says `[requires.interfaces] "bb.handling" = "1.2"`. A mod that can work without the hook asks for `1.0` and treats
`Error::Unsupported` from `register_controls` as "this host is older".

| SDK call | Meaning |
|---|---|
| `handling.register_controls(hook) -> ModResult<HandlingHookId>` | register the hook; `Error::Exists` when one is in place (one per mod and one per game), `Error::Denied` in an online session, `Error::Unsupported` on a host older than 1.2 |
| `handling.unregister_controls(id)` | remove it; the controls stay as the host shapes them from the next tick |

The hook is `FnMut(&mut Ctx, &ControlsQuery) -> Option<ControlsResult>`. A panic inside it is caught by the SDK, returned as an error
status and counted as a fault.

## Wire layout

`BbHandling` grew by 16 more bytes in 1.2 (64 bytes in all): `register_controls` is at 48 and `unregister_controls` at 56. The layouts
are locked by `abi/iface/handling_tests.rs`; the struct rules are those of [api-handling.md](api-handling.md#wire-layout).

| Function | Signature | Answers |
|---|---|---|
| `register_controls` | `(ctx, hook: *const BbControlsHook, out_id: *mut u64) -> BbStatus` | `BB_OK`; `BB_ERR_ARG` (null pointer, missing callback, a `size` that does not cover the callback); `BB_ERR_EXISTS`; `BB_ERR_DENIED` (online); `BB_ERR_UNSUPPORTED` (the game does not implement it) |
| `unregister_controls` | `(ctx, id: u64) -> BbStatus` | `BB_OK`; `BB_ERR_STALE` for a handle that is not the caller's hook |

`BbControlsHook` (registration, 56 bytes, align 8): `size` at 0, `flags` at 4 (reserved), `user` at 8, `callback` at 16, `reserved`
`[u64; 4]` at 24. The callback is `unsafe extern "C" fn(user, query: *const BbControlsQuery, result: *mut BbControlsResult) ->
BbStatus`. It returns `BB_CONTROLS_HANDLED` (1) after writing the result, `BB_CONTROLS_NOT_HANDLED` (0) to decline, or a negative
`BB_ERR_*` code, which counts as a fault.

`BbControlsQuery` (112 bytes, align 4; SI units unless a field says otherwise). Positive steering, yaw rate and lateral acceleration
are to the right. Wheel arrays are in the order front left, front right, rear left, rear right.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes the host filled in |
| 4 | `role` | u32 | the kind of vehicle, `BB_ROLE_*` (0 player) |
| 8 | `vehicle_revision` | u32 | the `revision` of `spec` for the vehicle this call is for |
| 12 | `dt` | f32 | the tick length, s |
| 16 | `throttle` | f32 | the throttle request, 0 to 1 |
| 20 | `brake` | f32 | the brake request, 0 to 1 |
| 24 | `handbrake` | f32 | the handbrake request, 0 to 1 |
| 28 | `steer` | f32 | the steering request, -1 to 1, positive to the right |
| 32 | `forward_speed` | f32 | the speed along the vehicle's heading, m/s, negative while moving backwards |
| 36 | `yaw_rate` | f32 | rotation about the body's up axis, rad/s, positive to the right |
| 40 | `lateral_accel` | f32 | acceleration along the body's lateral axis over the last tick, m/s^2, positive to the right |
| 44 | `longitudinal_accel` | f32 | acceleration along the vehicle's heading over the last tick, m/s^2, positive forward; 0 on the first tick after the hook is installed |
| 48 | `wheel_loaded_mask` | u32 | bit `i` is set when wheel `i` pressed on the ground in the last tick |
| 52 | `slip_ratio` | `[f32; 4]` | per wheel, the last tick: (wheel surface speed - ground speed) / ground speed, measured along the direction of travel of the wheel. Above 0 the wheel spins faster than the ground moves under it, below 0 it drags, -1 is a locked wheel. The ground speed in the denominator is floored at 0.5 m/s and the ratio is limited to +-10, so it stays finite at a standstill. 0 for an airborne wheel |
| 68 | `slip_angle` | `[f32; 4]` | per wheel, the last tick, **in turns** (1.0 is 360 degrees, the game's unit; multiply by 2 pi for radians), positive when the contact patch moves to the right of the wheel's heading. 0 for an airborne wheel |
| 84 | `reserved` | `[u32; 7]` | zero |

`BbControlsResult` (64 bytes, align 4). The host sets `size` and zeroes the rest before the call.

| Offset | Field | Type | Meaning |
|---|---|---|---|
| 0 | `size` | u32 | bytes of the layout the host reads |
| 4 | `flags` | u32 | `BB_CONTROLS_FLAG_STEER` (1): `steer_target_rad` is valid. Without it the host keeps its own steering and ignores the field |
| 8 | `throttle` | f32 | the throttle the engine is given, 0 to 1 |
| 12 | `brake` | `[f32; 4]` | the brake demand of each wheel, 0 to 1.5, bias folded in; 1 is the full brake torque of that wheel |
| 28 | `steer_target_rad` | f32 | the angle of the inside front wheel, rad, positive to the right |
| 32 | `reserved` | `[u32; 8]` | zero |

## What the host guarantees

- **Thread, scope and order.** The hook runs on the main thread, inside the physics step and a mod callback scope, once per tick,
  after the host has shaped the driver's input and before anything reads the controls: so before the engine, the brakes and the
  steering of that tick are set. It is not called on a tick the vehicle does not respond (a frozen body), nor for a vehicle whose
  engine is blown or that is disabled (the host's own rule is full brake, no throttle).
- **What the pedals are.** The query carries the driver's request **after** the host's input shaping, not the raw device values:
  the dead zone is applied (by default a pedal under 5 % reads 0 and over 95 % reads 1), the pedals are already swapped while the vehicle
  reverses (the brake pedal is the throttle of reverse), and the idle hold counts as a brake request. A mod that filters these
  values therefore filters what the engine and brakes would otherwise get, in every gear, without knowing the host's pedal rules.
  `steer` is the raw steering clamped to -1 to 1.
- **Last tick's state.** `slip_ratio`, `slip_angle`, `wheel_loaded_mask` and the two accelerations are results of the previous tick:
  this tick's wheels have not run yet. An aid built on them reacts one tick late.
- **`dt`.** The tick length of the world: 1/60 s by default, 1/144 s in the offline comparison mode, 1/240 s in a speed-break. The
  hook is called once per tick whatever it is, so a mod that wants a fixed check period counts time with `dt`.
- **What the host does with the answer.**
  - The throttle replaces the throttle the engine is given.
  - The brake is a **demand per wheel** and replaces the host's brake split for the tick. The host's brake-balance slider and its
    launch-stand rule (gas and brake together at a crawl) are not applied on top of a handled answer: the mod's bias is the only bias.
    A blown tyre still brakes fully, and the handbrake request still acts on the rear wheels as before. The brake lamp (the
    vehicle's `controls().brake`) shows the strongest demand, limited to 1.
  - The steering target, when `BB_CONTROLS_FLAG_STEER` is set, is the angle of the inside front wheel: the host **bypasses its own
    steering shaping** (the speed-dependent range, the rate limit, the counter-steer window and the tuning slider) and splits the
    angle between the two front wheels with its own Ackermann geometry. It keeps the 45 degree mechanical limit and the
    post-collision limit (after a hard hit the target is scaled down for up to a second, as the shaped steering is). Without the
    flag, the host's steering runs as usual and the field is ignored. A scripted manoeuvre of the game that sets the steering angle
    itself still wins.
- **No torque output.** The hook has no yaw-torque or body-torque output. A stability aid that works by braking one wheel is
  expressed as a brake demand on that wheel; an aid that applies a yaw moment directly cannot be built on this hook.
- **Which vehicles.** In v1 the player's car only (`role` is always 0), with the standard chassis, in both tyre integration modes.
  The rolling chassis, the coasting tyres and the simple police tyres keep their own rules and are never asked.

## What the mod must return

- Finite numbers in the units above. An answer is refused when a field is NaN or infinite, or out of range: `throttle` from 0 to 1,
  each `brake` from 0 to 1.5, `steer_target_rad` up to 1.5 rad in either direction (the host limits it further to 45 degrees).
- `BB_CONTROLS_HANDLED` with a result, or `BB_CONTROLS_NOT_HANDLED` to leave the controls as shaped. Declining is free and is not a
  fault.

A mod that wants to change only the throttle answers the query's own brake on all four wheels (`brake: [q.brake; 4]`, which the host
turns into exactly its own split with the balance slider at 0) and leaves the steering flag clear.

## Faults, budget and removal

The policy is that of the tyre hook ([api-handling.md](api-handling.md#faults-budget-and-removal)), per hook: a negative status or
a refused answer is a fault and the controls stay as the host shaped them this tick; the fifth fault quarantines the hook, the
vehicle drops it and the game removes the registration; a call over 2 ms is counted and warned about. The faults are also counted
against the mod in the runtime, so the mod's quarantine follows. `unregister_controls` or an unload retires the hook at once; the
scene takes the model off the car at the next frame boundary, never inside a tick.

## Online sessions

The hook is refused in an online session, like the others: `register_controls` answers `BB_ERR_DENIED`, and a hook registered
before the session began is taken off the car while it lasts.

## Writing your own controls filter

- **A slip-based throttle or brake limiter.** Read `slip_ratio` of the driven or of every wheel, hold a decision for a period you
  choose (count `dt`), and scale the throttle or the wheel's brake demand while it lasts. Ignore a wheel whose bit in
  `wheel_loaded_mask` is clear, and ignore very low speeds, where a slip ratio means little.
- **A brake bias.** Return `brake: [q.brake * f, q.brake * f, q.brake * r, q.brake * r]` with the front and rear factors you like.
  There is no host slider underneath it.
- **A brake-based stability aid.** Compare the slip angles of the front and the rear axle (`slip_angle` is in turns), and when the
  rear slides more than the front, add a brake demand to the front wheel on the side the tail slides towards, even with the pedal
  released.
- **A steering model.** Map `steer` and `forward_speed` to a lock and rate of your own and return the angle as `steer_target_rad`
  with the flag set. You keep your own state between calls (the previous angle, for a rate limit); the host keeps none of it.

Rules of thumb: keep the call well under 1 ms; never allocate in it; test the filter without the game by calling it on hand-made
queries (the SDK's `ControlsQuery` has `Default`).

## Host notes (NFS:MW)

`crates/nfsmw/src/modding/backends/handling.rs` is the port and puts the model on the lead player's car with the others (see
[api-handling.md](api-handling.md#host-notes-nfsmw)). The vehicle side is `libs/blackbox-vehicle/src/input/model.rs` (the
`ControlsModel` trait) and `vehicle/controls_model.rs` (the seam); the tests are `vehicle/tests/controls_model.rs` (dispatch,
bit-identity without a model and with a pass-through, the throttle, the per-wheel brake, the steering target),
`host/table/handling_hooks_tests.rs` (the table and the adapter), `sdk/ffi/handling_tests.rs` (the SDK round trip) and
`crates/nfsmw/src/modding/backends/handling_hooks_tests.rs` (the port).
