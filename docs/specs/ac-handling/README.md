# AC handling: specs

Part of [ADR 0016](../../decisions/0016-handling-hooks.md) and [the plan](../../plans/ac-handling.md). These specs describe
an optional handling model, in our own words, with enough maths and numbers that the mod `mods/ac-handling` can be built
and tested **without** an Assetto Corsa install or a checkout of any other project. The status column of the table below
says which exist; this page is the contract they are written to.

## The firewall

The spec is the interface between two sessions.

1. The **spec author** may read any source that allows it ([licensing.md](../../licensing.md)): public vehicle-dynamics
   literature, observed behaviour of the reference game on their own install, and GPL or decompiled projects *for
   understanding only*. They write what the behaviour **is**: inputs, outputs, state, equations, constants, limits. They
   never paste code, translate line by line, or quote comments.
2. The **implementer** builds from the spec alone, with no restricted source open, and records the session in
   `docs/provenance/ac-handling.md`.
3. A **reviewer** checks the spec for copied expression and the code for anything the spec does not say.

If a spec is not enough to implement from, the answer is to fix the spec, not to open the source.

## Template

Every spec file starts with:

```
# <topic>
Sources: <name, URL, license, what was taken: facts or understanding> (one line each)
Written by: <session or person, date>   Implemented by: <session or person, date>
```

then these sections, in this order:

1. **Purpose and scope**, and what is out of scope.
2. **Inputs and outputs**, with units and sign conventions (the game's axes: x right, y up, z forward in physics space).
3. **State**, what persists between calls and its initial values.
4. **Equations**, in plain maths and, where useful, pseudocode that is not a transcription of anyone's code.
5. **Constants**, as a table with units, ranges and the reason for each value ("fitted to target T3", "from a published
   tyre", "a design value").
6. **Test vectors**, input to expected output, to a stated tolerance. Enough to pin every equation and every branch.
7. **Limits**, what the model does not do and where it is known to be wrong.

A spec stays under 500 lines; split by concern.

## Planned specs

| File | Covers | Stage | Status |
|---|---|---|---|
| `tyre-force.md` | slip curve with peak and fall-off, load sensitivity, combined slip, friction ellipse, surface scaling | 0, used by 2 | written, revised 2026-11-20 (camber input) |
| `wheel-dynamics.md` | wheel spin, slip ratio and angle relaxation, low-speed handling, the substep rule | 0, 2 | written, revised 2026-11-20 (inertia 1.0, vectors) |
| `tyre-thermal.md` | core and surface temperature, pressure law, wear and grip as a function of each | 0, 5 | written |
| `suspension.md` | spring, damper (bump and rebound), anti-roll, camber gain, roll centre, hub motion | 0, 3 | written, revised 2026-11-20 (built hook) |
| `aids.md` | traction control, ABS, stability control, brake bias; levels and their meaning | 0, 4 | written, revised 2026-11-20 (built hook) |
| `steering.md` | steering ratio, speed-sensitive lock, rate limit, self-aligning torque to a force-feedback signal | 0, 4 | written, revised 2026-11-20 (built hook) |
| `parameter-mapping.md` | which NFS attributes feed which parameters; presets per compound class and car class; per-car override keys | 0, 2 | written, revised 2026-11-20 (car id, forced compound, `tuning/`) |
| `targets.md` | the reference scalars and tolerance bands, with the scenario each comes from | 0 | written, revised 2026-11-20 (re-derived at inertia 1.0) |
| `scenarios.md` | the scripted inputs behind `targets.md`, runnable by the harness and by `--drive-script` | 0, 2 | written, revised 2026-11-20 (convention E, S5 ladder) |

## Conventions every file follows

| Quantity | Convention |
|---|---|
| Axes, wheels | x right, y up, z forward; wheel order FL FR RL RR (0 to 3); SI units; the wheel frame is the body frame turned by the steering angle |
| Slip ratio | positive when the wheel spins faster than the ground (driving), negative when it lags (braking); `sx` is the *theoretical* slip `kappa / (1 + kappa)` |
| Slip angle | positive when the patch slides to the right (+x); the maths uses **radians**, the host's `slip_angle` result is in **turns** (1.0 = 360 degrees): the one conversion is `atan(sy_r) / (2 pi)` in [wheel-dynamics.md](wheel-dynamics.md) |
| Forces | `fx` positive forward (+z), `fy` positive to the right (+x), so a force that resists a rightward slide is negative |
| Aligning torque `mz` | positive turns the wheel's heading towards +x (right), i.e. increases a rightward steering angle; the host's `aligning_torque_nm` has the same sign |
| Camber | **outward positive** in the suspension, thermal and steering specs (the top of the wheel leaning away from the car); the tyre force function takes it in the wheel frame (positive = top towards +x) and the mod converts: `+` on the right wheels, `-` on the left |
| Grip multipliers | the player's setting is `grip_gain`; the thermal factor is `grip_scale(T_eff, P, w)`; the tyre force function receives their product (and `tyre_condition`) as `grip` |
| Steering | positive to the right; the host's tyre query has no steering lock (the patch velocity is already in the wheel frame); its `camber_rad` (since 1.1) is outward positive and 0 until the suspension hook is on |
| Wheel inertia | the host's constant (10 kg m^2) times `inertia_scale`, default 1.0: the host's drivetrain is unstable below about 0.5 ([wheel-dynamics.md](wheel-dynamics.md)) |
| Car identity | the spec has no name: a car is `spec-<kg>-<cm>-<kW>` ([parameter-mapping.md](parameter-mapping.md) 2.1) |

## About `targets.md`

It holds **scalars only**: for each scenario a value, a tolerance band and the date it was measured, for example "peak
lateral g of the road-compound class on the 100 m skidpad". No recordings, no per-sample data, no game files, nothing the
leak check would reject ([licensing.md](../../licensing.md) rule 1 allows measured counts and values). A maintainer with
their own install of the reference game refreshes it with the harness in `tools/`; nobody else ever needs to. If a band is
unknown, the line says `unmeasured` and the matching test is skipped, not guessed.

## Where each thing lives

| Thing | Place |
|---|---|
| Why and how | [ADR 0016](../../decisions/0016-handling-hooks.md), [the plan](../../plans/ac-handling.md) |
| What the model is | this folder |
| The model | `mods/ac-handling/src/` (pure functions, unit tests from the test vectors) |
| Presets and per-car overrides | `mods/ac-handling/presets/`, `mods/ac-handling/tuning/` (TOML; not `cars/`, which the host reads as car manifests) |
| What was read and when | `docs/provenance/ac-handling.md`, `NOTICE` |
| The host interface | [docs/modding/api-handling.md](../../modding/api-handling.md) (`bb.handling`, and the `spec` of `bb.vehicles` 1.3) |
