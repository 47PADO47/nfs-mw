# Vehicle physics audit against the PC decompilation

Status: historical audit with current corrections below. Scope: the port's vehicle physics (`libs/blackbox-vehicle`,
`crates/nfsmw/src/scenes/world/drive/sim`, `libs/blackbox-damage`) compared with the PC v1.3 behaviour
recorded in the decompilation (`Speed/Indep/Src/Physics/Behaviors/{SuspensionRacer,Chassis,EngineRacer}.cpp`
of the Most-Wanted-Vehicles-Decomp project) and with the original binary `speed.exe`.

The decompilation has no licence and is EA code. Everything here is behaviour, numbers and units, written in
our own words. No code or identifiers from it are to be copied into the repository.

Evidence tags: [decomp] read from the decompilation (via a summary read, not a verbatim copy); [bytes] present
in `speed.exe` at the stated address; [capture] matched in a PC capture (see `vehicle-calibration.md`);
[port] read from our code; [unconfirmed] not checked against a second source.

**2026-10-25 update:** steering range is corrected from verified PC consumers.
Low-speed taper matches the original; the final lateral clamp already exists;
ordinary-impact punctures were removed. Historical capture replay used a different
tire profile from gameplay. See [current player-driving scope](player-driving-parity.md).
Unresolved launch/input items remain hypotheses.

## 1. Summary table

| Area | Original (PC) | Port (file:line) | Verdict | Impact |
|---|---|---|---|---|
| Lateral force families (7 slip families x 6 loads) | Same table, [decomp] [capture] | `tires/lateral.rs:5-13` | Match | None |
| Load scale of lateral lookup (x = load x 0.0008) | [decomp] [capture] | `tires/lateral.rs:25` | Match | None |
| Lateral output: GRIP_SCALE x boost x 2500 | [decomp] | `tires/lateral.rs:33` | Match | None |
| Slip angle to family (2 deg steps, 12 deg plateau) | [decomp] | `tires/lateral.rs:21-32` | Match | None |
| Lateral sign (opposes sliding) | [decomp] | `tires/loaded.rs:120-126` | Match | None |
| Longitudinal: torque/radius, friction limit, slip limit ratio squared | [decomp] | `tires/loaded.rs:108-163` | Match | None |
| Ellipse: driven axis x 1.5, undone when not limited | [decomp] | `tires/loaded.rs:139-160` | Match | None |
| Traction circle tuning (lateral 1+0.2t, longitudinal 1-0.2t) | [decomp] | `vehicle/assists.rs:73` | Match | None |
| Drag reduction of steered wheel (above 1 m/s) | [decomp] | `tires/loaded.rs:168-173` | Match | None |
| Low-speed taper (below 1 mph) | Sliding branch only [bytes] | `tires/loaded.rs`: sliding branch | Match | No correction needed |
| Final traction clamp 0.25 x mass x lateral speed / dT | Present [decomp] | `vehicle/wheels.rs`: applied force limit | Present | No correction needed |
| Pilot factor (rear only, 30 to 50 mph ramp, 0.85 to 1) | [decomp] | `tires/loaded.rs:22-29` | Match | None |
| Grip table and traction table (m/s, mph breakpoints 0..85) | [decomp] | `tires/scales.rs:5-27` | Match | None |
| Reverse: max slip 71, traction x2 | [decomp] [capture] | `tires/scales.rs:20-27` | Match | None |
| Forward max slip 0.5 + ramp(10..71 m/s) | [decomp] | `tires/scales.rs:24` | Match | None |
| Wheel order FL, FR, RL, RR; forward spin positive | [decomp] | `suspension/geometry.rs:10-30` | Match | None |
| Wheel order vs car-assembly TireOffsets | Not in decomp | Doc says index 2 is rear right | Unresolved (suspension spec Q1) | Could swap rear sides |
| Spring: stiffness x compression x (1 + progression x compression) | [decomp] | `suspension/spring.rs:40` | Match | None |
| Shock rates (compress / extend), lb s/in to N s/m | [decomp] | `suspension/spring.rs:41`, `suspension/spec.rs:56-58` | Match | None |
| Shock valving: inches to m, compared with rate in m/s | [decomp] | `suspension/spec.rs:63`, `spring.rs:37` | Match | None |
| Digression: exponent 1 - param, above valving speed | [decomp] | `suspension/spring.rs:37-39` | Match | None |
| Blow-off: damper zero above blowout x 9.81 x mass | [decomp] | `suspension/spring.rs:42-44` | Match | None |
| Anti-roll: left-right previous compression, opposite signs | [decomp] | `suspension/spring.rs:45-46` | Match | None |
| Ride height: inches to m plus tuning | [decomp] | `suspension/spec.rs:61` | Match | None |
| Wheel radius: full diameter / 2 with sidewall | Unclear [decomp summary] | `tires/spec.rs:28-35` | Unresolved (suspension spec Q4) | Ride and contact |
| Body dimension: half extents | Unclear [decomp summary] | `rigid_body/body.rs:37-48` | Unresolved (suspension spec Q4) | Ride and contact |
| Speed table axis (steering) 0..160 in m/s | m/s [capture] | `steering/tables.rs:7` | Match | None |
| Steering range table (pad) | [decomp] | `steering/tables.rs:10` | Match | None |
| Steering range table (wheel) | [decomp] | `steering/tables.rs:12` | Match | None |
| Steering speed coefficient table | [decomp] | `steering/tables.rs:14` | Match | None |
| Braking widening (1 + 1.45 x speed term x t) | [decomp] | `steering/human.rs:97-98` | Match | None |
| Range coefficient over average input | [bytes] consumer confirmed: [1,1,1.1,1.2,1.25,1.35] | `steering/tables.rs`: separate range table | Corrected (section 2) | Human steering range |
| Input-speed coefficient table | [decomp] | `steering/tables.rs:18` | Match | None |
| Rate coefficient table (input magnitude) | [decomp] [bytes] | `steering/tables.rs:20` | Match | None |
| Counter-steer floor from rear slip | [decomp] | `steering/human.rs:110-116` | Match | None |
| Post-collision limit (blend, speed term 0.7 x 170 mph) | [decomp] | `steering/human.rs:102-107`, `tables.rs:22` | Match (impulse threshold unconfirmed) | None |
| Pad input remap (feeds averages only; target uses raw input) | [bytes] [capture] selected PC curve | `steering/tables.rs`: PC history curve | Corrected | Partial-stick response |
| Pad dead zone (pedal snap) | Value not found in sources read | `input/mod.rs:56` = 0.05 | Unresolved (engine Q4) | Pedal snap |
| Torque curve samples over idle..max_rpm | [decomp] [capture] | `engine/spec.rs:31-36`, `drivetrain/powertrain.rs:117-133` | Match | None |
| Peak torque: raw samples through max_rpm, rpm clamped | [decomp] (provenance Q2) | `powertrain.rs:117-133` | Match | None |
| Rev limiter: red line; no width | No cut-off rpm found [decomp summary] | `engine/spec.rs:26-28` | Unresolved (no width known) | Redline behaviour |
| Engine braking curve over idle..max_rpm | [decomp] | `engine/spec.rs:45-54` | Match | None |
| Speed limiter in mph | [decomp] | `drivetrain/powertrain.rs:233-248` | Match | None |
| First-gear launch band and assistance | Band, staging, timer and peak-torque override [binary/numeric] | `launch.rs`, `drivetrain/torque_loop.rs` | Shared implementation; scene timing needs capture (section 6) | Race launch |
| Brake torque: BRAKES x ft lb to N m x 4 | [decomp] (engine/brakes Q8) | `brakes/mod.rs:7-25` | Consistent with spec; not re-verified | Stopping distance |
| Handbrake torque: x 10 | [decomp] (Q8) | `brakes/mod.rs:9,34-36` | Consistent with spec; not re-verified | Handbrake |
| Brake lock: BRAKE_LOCK x torque x 1.2 test | [decomp] | `tires/loaded.rs:95-105` | Match | None |
| Aero drag: speed x coefficient, gas factor 1 (full) to 2 (off) | [decomp] | `aero/mod.rs:66-72` | Match | None |
| Aero drag drop 0.1 x (1 - gas) when grounded | [decomp] | `aero/mod.rs:71-73` | Match | None |
| Downforce: upness, forwardness floor 0.4, tuning 1 + 0.25 t | [decomp] | `aero/mod.rs:76-91` | Match | None |
| Downforce airborne factor | 0.8 [decomp] | Shipped data `crates/nfsmw-data/src/car/physics/chassis.rs:35` = 0.8; library default 0.1 (`aero/mod.rs:29`) | Match in game data | None in game; library default differs |
| Downforce centre (AERO_CG percent of wheelbase) | [decomp] | `aero/mod.rs:88-90` | Match | None |
| GuessCompression | Not in decomp file | Not in port | Not applicable | None |
| NOS: mph speed thresholds, 1 + boost, recharge | [decomp] [capture] | `nos/mod.rs` | Match | None |
| Tire puncture from ordinary impacts | Collision puncture path not located | `world/events.rs`: body damage does not puncture tires | Port-specific rule removed | Regression covered |
| Physics tick: 60 Hz real time, variable delta | Variable delta at 60 Hz [decomp] (engine provenance Q5) | `libs/blackbox-vehicle/src/lib.rs:50` FIXED_STEP = 1/60 | Implemented (section 3) | Shared real-time cadence |

## 2. Steering range coefficient table (confirmed and corrected)

The maximum human steering range uses six linearly interpolated multipliers:
`1, 1, 1.1, 1.2, 1.25, 1.35` over absolute recent history 0..1. The rate uses
its separate `1, 1.05, 1.1, 1.2, 1.3, 1.4` table. The port previously used
rate values for both; only the range table changes.

Direct PC instruction/data inspection confirms the separate consumers and table
descriptors. Thirty-one selected-executable numeric vectors verify outputs.
See [specification](player-driving-parity.md) and
[provenance](../provenance/player-driving-parity.md).

At history 0.2 the multiplier changes from 1.05 to 1; at history 1 it changes
from 1.4 to 1.35. Both tables interpolate to 1.15 at history 0.5. The original
audit's example at 0.5 was incorrect. Existing caps and remembered maximum remain.

## 3. Real-time cadence and scaled delta (implemented correction)

The historical port used fixed simulation-time ticks. At full quarter-speed
Speedbreaker this reduced 60-Hz updates to 15 per real second. The original
instead retains its real-time cadence and scales the simulation delta of each
tick by the final time multiplier. The selected modded installation runs this
contract at approximately 144 Hz rather than the nominal 60-Hz reference.

The [shared clock](shared-physics-clock.md) now defaults to 60 real-time Hz,
with an explicit 144-Hz offline comparison option. Every body and shared rule
receives the tick delta; contacts remain on one timeline. Rendering is independent.
Network sessions retain their existing 60-Hz contract. Matching these clock
rules does not establish complete original driving or long-trajectory parity.

## 4. Low-speed taper scope (historical suspicion disproven)

The original loaded-tire routine divides only sliding forces by dynamic friction
below one mph, not the gripping branch. Current Rust agrees. Extending the division
to gripping forces would introduce a difference. No change is justified.

The original one-mph global is initialized at runtime to 0.4470300078392029 m/s;
its zero on-disk value must not be used as an offline numeric oracle.

## 5. Tire puncture from ordinary impacts (port rule removed)

The old port-specific severity-six corner rule no longer exists. Current event
handling retains body damage without puncturing tires. The existing regression
`severe_corner_impacts_damage_the_body_without_puncturing_tires` covers all four
corners. Spike-strip punctures remain separate.

## 6. First-gear launch band (consumer resolved)

Original instructions and numeric execution confirm the first-gear band, strict
RPM endpoints and staging strength. It feeds the vehicle's launch state, not
the converter or clutch ramps. Their use of peak-torque RPM is correct and
must remain. The missing behavior was the separate timed peak-torque override
and traction addition. See [player launch](player-launch.md) for the shared
implementation, caller configuration, component tests and remaining scene limits.

## 7. Open items that are not mismatches yet

- Pad dead zone (engine Q4, input). Port default 0.05 (`input/mod.rs:56`). The PC constant was not found.
  Byte search for 0.05 (`CDCC4C3D`) returns too many hits to be useful. Next lead in section 9.
- Pad remap selection is resolved. The PC human-steering consumer selects the
  milder curve, confirmed by guarded loaded points and 4,660 user-driven callbacks.
  The former medium curve is replaced; see [player driving parity](player-driving-parity.md).
- Wheel radius and body dimension (suspension Q4). The decomposition summary is unclear whether the wheel
  radius is a full diameter or a half. Port uses half of rim diameter plus two sidewalls.
- Wheel order against car-assembly (suspension Q1). The physics arms agree with the decomposition
  (index 2 = rear left). The car-assembly document says index 2 is rear right. Needs a runtime check.
- Brake scales (engine Q8). Port 4x brake torque and 10x handbrake (`brakes/mod.rs:7-9`). Consistent with the
  spec; not re-verified in bytes.
- Rev limiter width. None found in the decomposition summary. The port has none. Nothing to change.
- GuessCompression. Not present in the decompiled file read and not present in the port. Nothing to change.
- Speedbreaker, NOS-cheat and AI-only paths. Not audited here.

## 8. Method limits

The decompilation was read through summary fetches, which paraphrase; numbers and units are taken from those
summaries and cross-checked only where a second source or the binary exists. The binary is stripped (no
function names), so use sites of data tables cannot be tied to code by xref. A direct value check is still
needed for sections 2, 4, 5 and 6.

## 9. Next leads

1. Resolved on 2026-10-25; historical lead: find the code that reads the range table at `0x008f8ae4` and the rate table at `0x008f8b64` by searching
   instructions for an immediate address or a base-plus-offset pattern near steering-related code (search the
   instruction operands for `0x8f8a` and `0x8f8b` with memory operands in code regions).
2. For the pad dead zone, search for a 32-bit float 0.05 used next to a pedal snap (compare against the snap
   thresholds in `input/mod.rs:103-112`), restricting the search to functions that also reference the gear or
   handbrake fields.
3. Port rule removed; historical lead: for tire puncture, search for code that writes to a tire state field after a collision (xref the impact
   zone function from the collision callback), not for the event string.
4. Resolved: the launch-band consumer is vehicle staging, not the converter; see [player launch](player-launch.md).
5. Settle the wheel radius and body dimension by reading the call site where the body Y dimension is doubled.
