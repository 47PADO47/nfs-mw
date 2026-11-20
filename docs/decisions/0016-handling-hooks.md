# 0016 — Handling hooks: an optional, toggleable ac-handling mod

- **Status:** proposed (2026-11-20). The owner has fixed three points: the goal is the *feel and handling* of
  Assetto Corsa on NFS:MW's own cars (not AC cars); the work is a **mod the player switches on**, with its own
  toggles; and it must be **optional for contributors**: nobody needs an AC install or a checkout of another
  project to build, test or review it. Everything else here is a proposal.
- **Plan:** [plans/ac-handling.md](../plans/ac-handling.md). **Specs:** [specs/ac-handling/](../specs/ac-handling/README.md).
- **Builds on:** [ADR 0005](0005-native-mod-plugins.md) (mods, interfaces, quarantine, online policy),
  [ADR 0014](0014-vehicle-components.md) (named behaviours), [licensing.md](../licensing.md) (spec-first).

## Question

How does a mod replace the part of the car physics that decides how a car *feels* (tyres first, then suspension
and driver aids) without putting an AC-style model in `libs/`, without forking the vehicle step, and without
anyone who works on something else having to know it exists?

## Facts checked on 2026-11-20

| Fact | Where | Consequence |
|---|---|---|
| The mod API cannot change forces or steps. `bb.vehicles` reads state, teleports, repairs, spawns traffic, adds NOS and toggles the drift assist (a layer a mod turns on). | [modding/api.md](../modding/api.md) | A new interface is needed; `set_drift_assist` is the precedent for a toggled layer. |
| Toggles already exist: `bb.settings` (a toggle or list on the Gameplay page, saved by the game), `bb.menus` (sliders, lists, buttons), `bb.input` (hotkeys), `bb.commands`. | [modding/api-settings.md](../modding/api-settings.md) | The mod's switches need no new UI. |
| Every tyre force is computed at **one site**, `wheel_forces` in `libs/blackbox-vehicle/src/vehicle/wheels.rs`. It already dispatches between four models (`update_rolling`, `update_simple`, `update_coasting`, `update_loaded`) from one input struct, `LoadedInput` (patch velocity in the wheel frame, body speed, vertical load, `dt`, quarter mass, surface). | `wheels.rs`, `tires/state.rs` | A fifth branch is in the style of the code. `LoadedInput` has no camber. |
| The result is a lateral force (the return value) and `Tire::longitudinal_force`; the tyre also carries wheel spin `av`, `traction`, `slip`, `slip_angle` for effects. The wheel inertia is a constant (10 kg m^2) and the tyre has no mass. Airborne wheels run a separate `*_free` update. | `tires/state.rs`, `wheels.rs` | A hook must fill the same fields and be called for loaded and airborne wheels. |
| `Vehicle::step` is three phases the host orchestrates (`begin_body_step`, `finish_body_step`, `respond_step`); powertrain, steering and tuning run at the tick rate and the doc comment says the tuning assumes 60 Hz. | `vehicle/step.rs` | The body and powertrain stay at the tick rate; the stiff part (wheel spin, slip relaxation) must be integrated finer inside the hook. |
| The world has one shared clock: 60 Hz by default, an optional 144 Hz offline comparison mode, 60 Hz forced in network sessions; traces are bit-identical at 5 to 240 FPS render rates. | [physics-timing.md](../physics-timing.md), [specs/shared-physics-clock.md](../specs/shared-physics-clock.md) | A mod that sub-steps *inside* one wheel's update does not touch the tick order. A smaller tick is already known to be tolerated. |
| Mod callbacks run on the main thread with a 2 ms budget; 5 faults quarantine a mod; gameplay interfaces are off online unless every peer enables mods. | ADR 0005 sections 3, 4, 6 | Hooks must be cheap, fall back to the built-in model on a fault, and be refused online. |
| The only public AC re-implementation we know, rustyAC, is `GPL-3.0-or-later` and a 1:1 port of `acs.exe`. | its `LICENSING.md` | Nothing is copied or linked. It is a source for the spec step only, with credit in NOTICE. |

Not verified: whether the vehicles that use the parity (`Reference`) integration mode may take a hook, and whether
the engine tolerates a hook on every wheel of every vehicle at 144 Hz. Stage 1 of the plan answers both.

## Decision

### 1. Optional at three levels

1. **For players:** the mod is off until switched on. Off means the built-in model runs and the output is
   bit-identical to today (a regression test, stage 1).
2. **For the engine:** the new interface does nothing until a mod registers a hook. One branch per wheel when no hook
   is registered.
3. **For contributors:** the AC-style behaviour is written as **self-contained specs** (own words, equations, constant
   tables, test vectors). Building, testing and reviewing the mod needs only those specs. No AC install, no rustyAC
   checkout, no network. Only a maintainer who *refreshes* the reference numbers needs an AC install, and what they
   commit is numbers (facts), never game data.

### 2. A generic interface, `bb.handling` (1.0)

Generic means it names no game and no AC: a mod may register any of three optional hooks.

| Hook | Version | Replaces | Host sends | Mod returns |
|---|---|---|---|---|
| `tyre` | 1.0 | the four tyre models of `wheels.rs` | wheel index, vehicle role, tick `dt`, vertical load, patch velocity (wheel frame), body speed, wheel spin, drive and brake torque at the hub, wheel radius and inertia, surface grip tag and multipliers, ambient and road temperature | mean longitudinal and lateral force, self-aligning torque over the tick, the new wheel spin, `traction`, `slip`, `slip_angle`, optional temperature for the HUD |
| `suspension` | 1.1 (appended) | `spring_force` | travel, travel speed, side, load transfer inputs | force, camber (fed to the tyre hook) |
| `controls` | 1.2 (appended) | nothing; it filters | raw pedals, steering, speed, slip state | filtered pedals, brake bias, steering |

Rules that follow ADR 0005: size-prefixed `#[repr(C)]` tables, fixed-width fields, handles not pointers, batched
arrays of four wheels per call, `catch_unwind` in the SDK trampoline, a layout snapshot test.

- **`handled` / `not handled`.** A hook answers per call. `not handled` (for a cop, a traffic car or a vehicle kind the
  mod has no data for) runs the built-in model for that wheel. A faulted or quarantined mod is `not handled` for good.
- **The mod owns the wheel-local integration.** It integrates wheel spin, slip relaxation, thermal state and (stage 3)
  the hub in as many substeps as it wants and returns the tick's mean forces and the new spin. The host writes the spin
  back, so the drivetrain sees it. Body, powertrain, steering and contacts stay at the tick rate.
- **Per-car data without per-tick cost.** `bb.vehicles` 1.3 adds a read-only `spec` (mass, wheelbase, track, centre of
  gravity, tyre size, power, gearing). The mod reads it on `PlayerCarChanged` and builds its parameter set once.
- **Parity mode.** Hooks are ignored for a vehicle in the `Reference` integration mode unless stage 1 shows it is safe.

### 3. The mod

`mods/ac-handling` (id `nfsmw.ac-handling`). Pure Rust, `MIT OR Apache-2.0`, no game files, built like the other
mods (see the plan for placement). It is named for what it is, an Assetto Corsa handling mod: it targets the *feel* of
Assetto Corsa and says so in its name, README and `mod.toml`, with the usual notice that it is an independent project,
not affiliated with or endorsed by Kunos Simulazioni or 505 Games. Only the mod carries the name: the `bb.handling`
interface stays generic because engine code names no game and other mods may use the hooks.

- **Model:** tyre (slip curve with a peak and fall-off, load sensitivity, combined slip, relaxation, then thermal,
  pressure and wear), suspension (stage 3), aids and steering feel (stage 4). Pure functions with their own tests.
- **Data:** from the `spec` read, then a per-compound and per-class preset, then per-car overrides in TOML. NFS cars lack
  most of the parameters; presets and overrides carry the rest and are the main tuning surface.
- **Toggles:** `bb.settings` for the master switch and the preset; `bb.menus` for grip scale, aid levels and steering
  assist; a hotkey and a console command to flip it live. A change applies at a frame boundary and re-seeds the tyre state.

### 4. Offline only at first

The host refuses hook registration in an online session. A separately compiled mod cannot promise identical floating
point results on every peer, and the server never trusts client state anyway (ADR 0005 section 6). Revisit after the
mod is stable and the netcode has a way to compare digests.

### 5. The spec firewall

The spec step (reading AC behaviour, public vehicle-dynamics literature and rustyAC) and the build step are separate
sessions. The spec records its sources and their licenses at the top; the implementer works from the spec alone and
never opens the source. Provenance goes in `docs/provenance/ac-handling.md` and NOTICE credits every project learned
from, with URL and license. Reference numbers measured on a maintainer's own AC install are committed as tables of
facts (for example "peak lateral g of the reference tyre on the test pad"), never as recordings or game files.

## Alternatives considered

- **A `kind = "sim"` model in `libs/blackbox-vehicle`, like the bike.** Simplest to wire, and the bike is the
  precedent. Rejected by the owner: the model would be core code, always shipped, not a mod a player can switch off.
- **Replace the whole vehicle step from a mod.** Duplicates the drivetrain, contacts and the shared clock order; far
  larger ABI; no gain, since only the tyre, suspension and aids differ.
- **Sub-step the whole body in the host first.** The right answer if body-level feel at 60 Hz is not enough, and kept as
  the decision gate of stage 2. It is a bigger change to a parity-tested step, so it is not the first move.
- **Fork or depend on rustyAC.** GPL, and a port of a decompilation: excluded by [licensing.md](../licensing.md).
- **WASM or Lua for the model.** Rejected in ADR 0005 for mods in general.

## Consequences

- **Good.** AC-style behaviour without touching core physics for anyone who does not want it. The hooks are generic: a
  RIDE 6 tyre mod, a drift mod or a different sim model can use them. The host change is small and testable.
- **Cost.** An ABI surface to maintain (three hooks, one read). A hook on the hot path of every wheel when registered.
  The model's feel needs iterative tuning on real driving that no test can prove.
- **Risk accepted.** Body dynamics stay at the tick rate in v1; if that is not enough the host needs body sub-steps.
  NFS levels, cops and pursuit were tuned for arcade grip, so a high-grip sim tyre changes the game's balance (the
  mod leaves cops on the built-in model).

## Revisit triggers

- Body feel at 60 Hz is not enough: add host sub-steps (plan, gate after stage 2).
- Another mod wants a hook this design lacks: version the interface, do not special-case.
- Online play with mods becomes a goal: digest compare plus deterministic float rules first.

## Owner decisions this ADR asks for

1. Accept the three-hook shape, or start with the tyre hook only (proposal: tyre only, others appended later).
2. Where the mod sits in the workspace (proposal: a member like `ride6-bikes`, not in the default build).
3. Offline only for v1 (proposal: yes).
