# Vehicle physics audit against the PC decompilation

Status: audit and spec, not an implementation. Scope: the port's vehicle physics (`libs/blackbox-vehicle`,
`crates/nfsmw/src/scenes/world/drive/sim`, `libs/blackbox-damage`) compared with the PC v1.3 behaviour
recorded in the decompilation (`Speed/Indep/Src/Physics/Behaviors/{SuspensionRacer,Chassis,EngineRacer}.cpp`
of the Most-Wanted-Vehicles-Decomp project) and with the original binary `speed.exe`.

The decompilation has no licence and is EA code. Everything here is behaviour, numbers and units, written in
our own words. No code or identifiers from it are to be copied into the repository.

Evidence tags: [decomp] read from the decompilation (via a summary read, not a verbatim copy); [bytes] present
in `speed.exe` at the stated address; [capture] matched in a PC capture (see `vehicle-calibration.md`);
[port] read from our code; [unconfirmed] not checked against a second source.

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
| Low-speed taper (below 1 mph) | Divides lateral and longitudinal forces [decomp] | `tires/loaded.rs:109-112`: divides sliding terms only | Probable mismatch | Very low (below 1 mph) |
| Final traction clamp 0.25 x mass x lateral speed / dT | Present [decomp, summary read] | Not found in port | Probable mismatch [unconfirmed] | Unknown, sliding tires |
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
| Range coefficient over average input | [bytes] both tables present; [decomp] reads [1,1,1.1,1.2,1.25,1.35] | `steering/tables.rs:16`: [1,1.05,1.1,1.2,1.3,1.4] | Mismatch (section 2) | Steering range at medium stick |
| Input-speed coefficient table | [decomp] | `steering/tables.rs:18` | Match | None |
| Rate coefficient table (input magnitude) | [decomp] [bytes] | `steering/tables.rs:20` | Match | None |
| Counter-steer floor from rear slip | [decomp] | `steering/human.rs:110-116` | Match | None |
| Post-collision limit (blend, speed term 0.7 x 170 mph) | [decomp] | `steering/human.rs:102-107`, `tables.rs:22` | Match (impulse threshold unconfirmed) | None |
| Pad input remap (feeds averages only; target uses raw input) | [decomp] | `steering/human.rs:124-138` | Match (remap curve set unconfirmed) | Stick feel |
| Pad dead zone (pedal snap) | Value not found in sources read | `input/mod.rs:56` = 0.05 | Unresolved (engine Q4) | Pedal snap |
| Torque curve samples over idle..max_rpm | [decomp] [capture] | `engine/spec.rs:31-36`, `drivetrain/powertrain.rs:117-133` | Match | None |
| Peak torque: raw samples through max_rpm, rpm clamped | [decomp] (provenance Q2) | `powertrain.rs:117-133` | Match | None |
| Rev limiter: red line; no width | No cut-off rpm found [decomp summary] | `engine/spec.rs:26-28` | Unresolved (no width known) | Redline behaviour |
| Engine braking curve over idle..max_rpm | [decomp] | `engine/spec.rs:45-54` | Match | None |
| Speed limiter in mph | [decomp] | `drivetrain/powertrain.rs:233-248` | Match | None |
| First-gear launch window from peak rpm | Window 25% of (redline - idle), limit redline - 500, first gear only [decomp] | `powertrain.rs:37-38`, `torque_loop.rs:120-130` | Probable mismatch (section 6) | Launch feel |
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
| Tire puncture from ordinary impacts (severity 6) | Original punctures from spike strips [port comment]; collision path not located | `nfsmw-data/src/car/damage.rs:32-34`, `crates/nfsmw/src/scenes/world/events.rs:121-125` | Probable mismatch (section 5) | Tire blow-outs from hard hits |
| Physics tick: 60 Hz real time, variable delta | Variable delta at 60 Hz [decomp] (engine provenance Q5) | `libs/blackbox-vehicle/src/lib.rs:50` FIXED_STEP = 1/60 | Confirmed mismatch (section 3) | Speedbreaker timing |

## 2. Steering range coefficient table (probable mismatch)

Rule in plain words. The steering range is multiplied by a factor that grows with the recent average
absolute steering input. The factor is read from a six-point table over the average input from 0 to 1,
evenly spaced at 0, 0.2, 0.4, 0.6, 0.8, 1.0. The rate of the steering angle uses a different six-point table
over the same input magnitude, which the port already matches.

Constants.
- Range table, per the decomposition reading: 1.0, 1.0, 1.1, 1.2, 1.25, 1.35.
- Port table in `steering/tables.rs:16` (`RANGE_COEF`): 1.0, 1.05, 1.1, 1.2, 1.3, 1.4. This is a copy of the rate
  table (`INPUT_COEF`, `tables.rs:20`), which is correct for the rate.
- Both six-point tables are present as float data in `speed.exe`: the range values at 0x008f8ae4 and the rate
  values at 0x008f8b64 [bytes]. The two are distinct, so the port's range table is a copy of the wrong one.

Units: dimensionless multiplier. Domain: average of the absolute steering input over the averaging window
(0..1).

Expected effect. At medium stick deflection (average near 0.5) the range is 1.2 in the original and 1.25 in
the port, about 4 percent wider than the original. At full stick the original gives 1.35 and the port 1.4.
Effect is largest in sustained lock holds on low-speed corners. Small.

Implementation. Change only the `RANGE_COEF` constant in `steering/tables.rs` to the range values above.
Leave `INPUT_COEF` unchanged. Keep the comment naming the domain.

Test idea. Add a unit test that asserts `RANGE_COEF` differs from `INPUT_COEF` at index 4 (1.25 vs 1.3) and
equals the range table at all six points. In the calibration harness, hold full lock for 3 s at 10 m/s on the
pad device and compare the front wheel angle with the value at a steady average input near 1.0.

Open step before implementing. Confirm the use site of the 0x008f8ae4 table (which function reads it). It is
not yet proven that the steering code reads that table for the range. Evidence tag: [decomp] for the values
and [bytes] for presence; the use site is [unconfirmed].

## 3. Physics tick: variable 60 Hz delta versus fixed step (confirmed mismatch)

Rule in plain words. The original schedules the simulation at 60 Hz in real time and scales each tick's
simulation delta by the final time multiplier of the frame. The port advances the vehicle in a fixed
simulation step of one sixtieth of a second and does not scale per frame.

Constants. Original rate 60 Hz; port `FIXED_STEP = 1/60` s (`libs/blackbox-vehicle/src/lib.rs:50`).

Units: seconds per tick.

Expected effect. The two agree when the frame rate is exactly 60 Hz and the time multiplier is 1. They differ
during the Speedbreaker, when the original scales delta and the port does not, and wherever the original
scheduler delivers a variable delta. Long replays drift apart by the accumulated time error.

Evidence tag: [decomp] and provenance Q5 (resolved 2026-10-14, `docs/provenance/vehicle-engine-drivetrain.md`).

Implementation. Out of scope for a small patch. The fix is a design change in the caller: either accept the
fixed step as a documented difference, or feed a scaled delta through the existing simulation-time accumulator
described in `vehicle-engine-drivetrain.md` and `docs/specs/vehicle-rigid-body.md`. Pick one and record it.

Test idea. Run a replay at a fixed 60 Hz and at 40 Hz with the same inputs. The fixed-step port should give
identical results at 60 Hz and a documented, bounded drift at 40 Hz. A Speedbreaker replay should show the
expected time scaling once the delta path is wired.

## 4. Low-speed taper scope (probable mismatch, minor)

Rule in plain words. When the car moves slower than 1 mph and the dynamic friction is above 0.1, the original
divides the lateral and longitudinal tire forces by the dynamic friction. The port divides only the sliding
terms, not the total forces.

Constants. Threshold 1 mph (0.44703 m/s); dynamic friction threshold 0.1. Port: `tires/loaded.rs:109-112`.

Expected effect. Only below 1 mph. Negligible in play, but it affects a parked car starting to roll.

Evidence tag: [decomp] summary read. Not checked in bytes. [unconfirmed] whether the division covers the grip
force as well as the slide force.

Implementation. Only after the decomposition reading is confirmed. Then extend the division to the grip branch
under the same conditions.

Test idea. Apply a small lateral velocity at 0.5 mph and check that the lateral force scales with the
dynamic friction, both branches.

## 5. Tire puncture from ordinary impacts (probable mismatch)

Rule in plain words. In the port a hard hit that reaches the top impact level (6 of 0..6) on a corner zone
punctures the matching tire. The code comment says the original punctures tires only from spike strips.

Constants. `PUNCTURE_SEVERITY = 6` (`crates/nfsmw-data/src/car/damage.rs:34`). Impact level is
`force / strength x visual scale`, clamped to 0..6 (`events.rs:117`). Corner-to-wheel map in
`damage.rs:38`.

Units: level is an integer 0..6; force in newtons.

Expected effect. If the original does not puncture on impact, the port blows tires that should stay intact
after hard crashes, which changes handling and blow-out sounds. The effect is large when it happens.

Evidence. Not established. The decompilation files read (Chassis, SuspensionRacer, EngineRacer) contain no
puncture logic. Ghidra has the event strings `ETirePunctured` and `ETireBlown` at 0x008a7678 and 0x008a7664,
but no code xrefs (the event names are hashed). Tag: [unconfirmed].

Implementation. Hold until the collision-to-tire path is found. If the original has no impact puncture, remove
the rule (keep spike-strip punctures). If it does, record the severity and the zone-to-wheel map as [decomp].

Test idea. A synthetic hard hit on a front corner at level 6 should not puncture if the original does not
puncture on impact. Also keep the existing spike-strip puncture test unchanged.

## 6. First-gear launch window from peak rpm (probable mismatch)

Rule in plain words. The original builds a launch window around the peak-torque rpm that applies only in
first gear. The window width is 25 percent of the span from idle to red line. Its upper limit is 500 rpm
below the red line. The returned value is the smaller of peak plus window and the upper limit, minus the
window. Outside first gear the window is zero.

Constants. Width 0.25 x (red line - idle). Upper limit red line - 500. Only first gear.

Port. The port uses the peak rpm in two other places: the torque converter ramp (`torque_loop.rs:37-38`) and
the clutch slip ramp (`torque_loop.rs:127-130`). It has no window and no upper limit.

Units: rpm.

Expected effect. Launch torque shape and clutch slip near peak rpm in first gear. Medium, for automatic
launches.

Evidence tag: [decomp] summary read. [unconfirmed] that the window feeds the converter in the original.

Implementation. Not until the window function is read in bytes or verbatim-equivalent. Then add the window as
a first-gear-only function and feed it where the original uses it.

Test idea. At throttle 1 in first gear from rest, the rpm at which torque peaks should sit at the value given
by the window formula. Check that second gear is unaffected.

## 7. Open items that are not mismatches yet

- Pad dead zone (engine Q4, input). Port default 0.05 (`input/mod.rs:56`). The PC constant was not found.
  Byte search for 0.05 (`CDCC4C3D`) returns too many hits to be useful. Next lead in section 9.
- Pad remap set. Port uses the medium 21-point set (`steering/tables.rs:24`). The chassis reading describes a
  milder set. Which one the PC selects for pad is [unconfirmed].
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

1. Find the code that reads the range table at `0x008f8ae4` and the rate table at `0x008f8b64` by searching
   instructions for an immediate address or a base-plus-offset pattern near steering-related code (search the
   instruction operands for `0x8f8a` and `0x8f8b` with memory operands in code regions).
2. For the pad dead zone, search for a 32-bit float 0.05 used next to a pedal snap (compare against the snap
   thresholds in `input/mod.rs:103-112`), restricting the search to functions that also reference the gear or
   handbrake fields.
3. For tire puncture, search for code that writes to a tire state field after a collision (xref the impact
   zone function from the collision callback), not for the event string.
4. Read the launch-window routine for the first-gear window and tie it to the converter.
5. Settle the wheel radius and body dimension by reading the call site where the body Y dimension is doubled.
