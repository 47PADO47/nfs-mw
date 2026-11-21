# AC handling: plan

Part of [ADR 0016](../decisions/0016-handling-hooks.md); specs in [specs/ac-handling/](../specs/ac-handling/README.md).
An optional mod, `mods/ac-handling`, gives NFS:MW's own cars the feel of Assetto Corsa (a high-rate tyre model with a
real slip curve, load sensitivity, thermal state, proper suspension and driver aids), switched on and tuned by the
player. **Verdict: doable, in small stages, each leaving the game unchanged for anyone who does not turn it on.**

## Status (2026-11-20)

Built on one branch in one session, **tested with unit, loader and planar-scenario tests, never driven in the game**:

| Stage | State | Notes |
|---|---|---|
| 0 | done | nine specs, a provenance record, a NOTICE credit; reviewed once for copied expression (one passage rewritten). No AC install on the build machine, so every AC-derived target is `unmeasured` and the bands are design values. |
| 1 | done | `bb.handling` 1.0 tyre hook, `bb.vehicles` 1.3 `spec`. Deviations from the text below: **one call per wheel** (not a batch of four); the hook is a trait in `libs/blackbox-vehicle` that the facade adapts; the hook is accepted in both integration modes; no `PlayerCarChanged` event exists, the mod polls the spec `revision`; online refusal was added (it did not exist); `handling-probe` fixture has five variants but no `future-abi` one. |
| 3, 4 | done in the host | hooks 1.1 (per axle, not per wheel) and 1.2 (pedals after input shaping, per-wheel brake demand, steering flag). |
| 2 | done | `mods/ac-handling`: tyre model, wheel dynamics with substeps, parameter mapping, toggles, planar scenario tests against the design bands. |
| 3, 4, 5 | done | suspension, aids (traction control, ABS, stability, bias), steering and thermal units, each switchable on its own; the force-feedback value is an accessor only (the game has no known output for it). |
| 6 | partly | `docs/modding/ac-handling.md` and an example are written; presets cover every compound and car class of the mapping spec, not per-car data for the roster. Cops, traffic and racers stay on the built-in model. |

Verified: `cargo fmt`, `clippy -D warnings`, `cargo xtask check` and `cargo test --workspace` (14,080 passed, 0 failed, 1,418
ignored for lack of an install). Not verified: any of it in the running game, the hotkey and settings rows on screen, and
every Assetto Corsa reference value (`unmeasured`).

Findings that changed the specs: the wheel-inertia scale of 0.3 the first spec assumed makes the host drivetrain
limit-cycle (stable from about 0.5; the default is 1.0), the per-car folder is `tuning/` (the host's `cars/` is a
manifest folder), and the real hooks differ from the first sketch (see the ADR table). Open: the stability range was
seen only at 60 Hz, and the host's online gating treats only a multiplayer **client** as online (a listen-server host
was not checked).

## Why it is doable

- Tyre forces come from one call site (`wheel_forces`, `libs/blackbox-vehicle/src/vehicle/wheels.rs`) that already picks
  between four models from one input struct. A hook is a fifth branch.
- The stiff dynamics (wheel spin, slip relaxation, later the hub) can be integrated finely **inside the mod**, per wheel,
  so the shared 60 Hz tick, the pair order and the net contract do not change
  ([physics-timing.md](../physics-timing.md)).
- Toggles, menus, hotkeys, fault quarantine and the online refusal all exist ([ADR 0005](../decisions/0005-native-mod-plugins.md)).
- The licensing path is already defined (spec-first, [licensing.md](../licensing.md)).

What could still sink it, in order of likelihood: the feel needs the body integrated finer than 60 Hz (gate after stage 2);
NFS attributes give too few parameters, so the presets carry the feel (a tuning effort, not a design flaw); the parity
integration mode refuses hooks (stage 1 answers it).

## Keeping it optional

| Who | Needs | Does not need |
|---|---|---|
| A contributor working on anything else | nothing | the mod, its specs, AC, any other project |
| A contributor on the host hooks (stage 1) | the fixture mod `handling-probe` and a fake host | the real mod, AC |
| A contributor on the mod (stages 2 to 6) | the specs and their test vectors; `cargo test -p ac-handling` | an AC install, a rustyAC checkout, the NFS:MW install for unit tests |
| A maintainer refreshing reference numbers | their own AC install and the harness in `tools/` | to commit anything but a table of numbers |
| CI | builds the mod, runs its pure tests, runs the off-path regression | any install; install-dependent checks skip, as the existing ones do |

Rules that keep it so: no spec step output lives only in someone's head (the spec is the interface between sessions);
the mod crate depends on `blackbox-modding` (sdk) and `std`, no AC-specific crate; no file in the repo is derived from
rustyAC code or holds AC data (the leak check and review enforce it); NOTICE credits what the specs learned from.

## Stages

| Stage | Goal | Touches | Exit |
|---|---|---|---|
| 0 | specs and reference numbers | `docs/specs/ac-handling/`, `docs/provenance/`, `NOTICE`, `tools/` (harness) | specs written and reviewed; `targets.md` filled; no game code |
| 1 | the host seam | `libs/blackbox-modding` (`bb.handling` 1.0, `bb.vehicles` 1.3), `libs/blackbox-vehicle` (hook branch), `crates/nfsmw/src/modding/` | with no hook registered every existing trace is bit-identical; the fixture mod's trivial tyre drives; ABI snapshot test; online refusal |
| 2 | mod v0 | `mods/ac-handling` | tyre model with wheel-local substeps, master toggle, preset, grip scale, hotkey; 5 to 8 cars; scenario metrics inside the bands of `targets.md`. **Gate A** |
| 3 | suspension | `bb.handling` 1.1, mod | camber, roll centre, hub motion; scenario metrics re-checked. **Gate B** |
| 4 | aids and steering | `bb.handling` 1.2, mod | traction control, ABS, stability, speed-sensitive steering; levels as sliders |
| 5 | depth | mod | temperature, pressure, wear; HUD number; force-feedback output if the game has one |
| 6 | coverage and docs | mod, `docs/modding/` | presets for the whole roster; per-car overrides; modding page; example in `docs/modding/examples.md` |
| 7 | revisit online | ADR 0016 | only if the owner wants it |

Stages 2 to 6 cannot start before stage 0 has the spec they implement. Stages 0 and 1 are independent and can run in
parallel (different files, different sessions).

## Stage 0: specs and reference numbers

- **Spec files** (one concern each, under 500 lines, see the index): tyre force, wheel spin and relaxation, thermal and
  pressure, suspension and camber, aids, steering feel, parameter mapping. Each follows the template in the index:
  sources with licenses first, then inputs, outputs, state, equations, constants, **test vectors**, limits.
- **Sources.** Public vehicle-dynamics literature and observed AC behaviour first; rustyAC (GPL, a port of the
  original) only for facts observation cannot give. Reading is allowed, copying is not; describe in plain language and
  maths, never line by line.
- **Reference numbers.** A harness (maintainer-only, `tools/`, never run in CI) drives scripted inputs in the
  maintainer's own AC install and records scalars: skidpad lateral g per compound class, braking distance from
  100 km/h, understeer gradient, step-steer yaw overshoot and settling time, slide-and-recover time, launch traction. Only
  the **scalars and their tolerance bands** are committed in `targets.md`; recordings stay local.
- **Provenance.** `docs/provenance/ac-handling.md` per [provenance/README.md](../provenance/README.md).
- **Exit.** A reviewer who has never seen AC can implement stage 2 from the spec alone, and the spec's test vectors
  pin every formula.

## Stage 1: the host seam

- **Interface.** `bb.handling` 1.0 with the `tyre` hook as in the ADR; `bb.vehicles` 1.3 with a read-only `spec`.
  Table builders and trampolines generated like the others; `catch_unwind`; layout locked in `abi/iface/tests.rs`.
- **Dispatch.** In `wheel_forces`, before the existing `match`: if a hook is registered and the vehicle's role and
  integration mode allow it, call it with the four wheels batched; copy its results into the same `Tire` fields the
  existing models fill (`longitudinal_force`, `traction`, `slip`, `slip_angle`, `av`), loaded and airborne alike.
  `not handled` falls through to the existing branch untouched.
- **Fault policy.** A panic or an invalid (non-finite or out-of-range) result counts as a fault, uses the built-in model
  for that wheel this tick, and after 5 faults the mod is quarantined (ADR 0005). A time over budget warns once.
- **Online.** Registration refused in a networked session, with a one-line reason in the log and `mods` command.
- **Tests (the gate).**
  1. *Off-path identity:* the existing three-car frame-rate traces, the tyre capture tests and a drive-script sweep are
     bit-identical with the branch compiled in and no hook registered.
  2. *Fixture mod* `mods/handling-probe` (its own workspace, like `hello`): a linear tyre, loaded through the real
     loader against a fake host; a `not handled` fixture; a `panics` fixture that is quarantined; a `future-abi` fixture
     that is refused.
  3. *Mode question:* a test that states, for each vehicle mode, whether a hook applies, so the decision is recorded.
- **Risk.** A hook on the hot path of every wheel. Measure with `world_step_costs` at 16 cars with the player hooked;
  the budget is the mod callback budget, and a regression of more than a few percent in tick cost is a stop.

## Stage 2: mod v0

- **Crate.** `mods/ac-handling`, `cdylib` plus `rlib` like `ride6-bikes`, listed as a workspace member and left out of
  the default build as the other mods are; own `README.md`; `mod.toml` requiring `bb.handling` and `bb.vehicles`.
- **Model.** From the tyre spec: slip curve, load sensitivity, combined slip, relaxation, wheel-local substeps. The
  substep count is a constant of the model chosen by the spec (and a setting for testing), not a host concern.
- **Data.** `presets/*.toml` per compound class and car class; `tuning/<car>.toml` overrides; a loader with
  `deny_unknown_fields` and ranges, like the manifest loaders; an unknown car uses its class preset and logs it once.
- **Toggles.** `bb.settings` master toggle (default off) and preset list; `bb.menus` grip scale; hotkey and console
  command. Switching applies at a frame boundary and re-seeds the tyre state (cold, at the ambient temperature).
- **Scenario harness.** A test binary that runs `--drive-script`-style inputs on a headless world twice (built-in model
  and mod) and prints the scalars of `targets.md`; the test compares the mod's scalars to the bands, and the built-in
  model's to nothing (it is only shown for contrast).
- **Gate A (feel).** The owner drives it. If the body-level feel is flat or the car is unstable at 60 Hz with sound
  wheel-local numbers, open a separate plan for host body sub-steps (a refactor of the parity-tested step) before stage 3.

## Stage 3: suspension

- Hook 1.1 (`spring_force` replacement) feeding camber into the tyre hook; hub (unsprung) motion integrated wheel-local in
  the mod; roll centre and anti-roll from the spec. The engine's single sprung body stays.
- **Gate B.** If hub motion needs the body force at a finer rate, same answer as gate A.

## Stage 4: aids and steering

- Hook 1.2 filters pedals, brake bias and steering before `update_controls`. Levels are menu sliders (0 off to the
  spec's maximum). Speed-sensitive lock and a steering-rate limit come from the spec. A device with force feedback gets
  the tyre's aligning torque as an output only if the game exposes one (to check in stage 5; not assumed).

## Stage 5 and 6

- Thermal, pressure and wear state, with one HUD value through `bb.hud`; presets for every compound class and car class
  in the roster; `docs/modding/ac-handling.md` (the player and author page) and an example in
  [docs/modding/examples.md](../modding/examples.md).
- Cops, traffic and racers stay `not handled` unless the owner asks otherwise. Their balance depends on the built-in
  model, and a high-grip player against arcade cops is a design question, not an engine one.

## Risks

| Risk | Effect | Answer |
|---|---|---|
| Feel needs finer body integration | stages 3 and 4 stall | gate A and B; a separate plan for host body sub-steps |
| NFS attributes carry too few parameters | wrong feel on some cars | presets and overrides are the tuning surface; the mapping spec says which attributes are used |
| The hook is slow | tick cost up | batched four-wheel call, measure in stage 1, hard stop on regression |
| Balance of the game changes | cops and races feel unfair | off by default; cops built-in; documented in the mod's page |
| Spec leaks source | licensing | the firewall of the ADR; sources listed; review item on every spec and mod PR |
| Online use | desync or cheating | refused in v1 |

## Open items

- The real answer to which vehicle modes may take a hook (stage 1 test 3).
- Whether the game has a force-feedback output (stage 5).
- Where the Stage 0 harness lives and how a maintainer runs it (a short `tools/README` page with the scenario list).
