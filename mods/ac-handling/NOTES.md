# ac-handling: implementation notes (stage 2)

Written by the stage-2 implementer session, 2026-11-20, from the specs alone (the firewall of
[ADR 0016](../../docs/decisions/0016-handling-hooks.md) section 5: no Assetto Corsa file, no other project's code, no restricted
source was opened). Sources used: AGENTS.md, CONTRIBUTING.md, ADR 0016, the plan, `docs/modding/api-handling.md` and `README.md`,
`docs/specs/ac-handling/{README,tyre-force,wheel-dynamics,parameter-mapping,scenarios,targets}.md`, and the example mods and the
SDK of `libs/blackbox-modding` for how a mod is built.

## Result

| What | State |
|---|---|
| Tyre force T1 to T14 and its properties | all reproduced to the spec's tolerance (`src/tests/tyre_force_vectors.rs`) |
| Wheel dynamics: the three one-substep tables, W1 to W9b, W10 (every cell of the convergence table, the three host rates) | all reproduced (`src/tests/wheel_vectors.rs`); two table-reading notes below |
| Parameter mapping: REF, COMPACT_FWD, SUPER, HEAVY_AWD, per-axle grip, override example, peak-angle override, loader mistakes | all reproduced (`src/tests/mapping_vectors.rs`, `src/tests/loader.rs`) |
| Scenarios S1 to S6 on the planar car, the ten scalars of `targets.md` 2.1 and the five rows of section 3 | every planar value reproduced to well inside 3 % (most to the last printed digit) and every `design` band holds; the 1/144 s checks hold (`tests/scenarios/`) |
| Rows skipped | the Assetto Corsa reference of every row is `unmeasured` in `targets.md`: skipped with a printed reason, never guessed |
| Real loader, fake host | switches, hotkey, command, preset, refused registration, car change, config, optional interfaces absent, unload, and a real vehicle driving on the hook (`tests/real_loader/`) |

## Spec questions

Where a spec was ambiguous or wrong the most defensible reading was taken, with the file and section here. None needed a restricted source.

1. **`wheel-dynamics.md` section 2 (contract point 3) and section 5 (`inertia_scale = 0.3`): the default breaks the host's drivetrain.**
   Driven on a real `blackbox-vehicle` through the real loader, a model with `inertia_scale` 0.3 (3 kg m^2) limit-cycles the host:
   the drive torque the host sends swings between about +1800 and -1500 N m tick by tick, the rear spin changes sign, and the car
   crawls at about 2 m/s under full throttle. The cycle repeats exactly for 0.35, 0.4 and 0.45; 0.5 and above drive normally. The
   cause is the loop between the torque the host's powertrain computes from the wheel spin the tyre returns and a wheel that is lighter
   than the host's own constant (10): the host's coupling is stable only at about half of it or more (the spec's open question:
   "whether the drivetrain's reflected inertia is on the host side"; the answer is yes). **Decision:** the shipped compound presets set
   `inertia_scale = 1.0` (the host's own value, the one its built-in tyres run with), and `Keys::default()` stays 0.3 as the spec says.
   The scenario harness pins 0.3 for every planar car (the planar car has no drivetrain compliance and the `targets.md` numbers were
   computed with 0.3); with 1.0 the planar car gives `brake_m` 40.9 (band 32 to 40), `slide_recover_s` 1.85 (0.3 to 1.5) and a slower launch. So
   the spec's bands and the game's stability disagree about this one number. Either the host takes the wheel+driveline stiff loop
   into its own integration (a host fix, see gate A in the plan), or the spec's bands are re-derived with 1.0. Pinned by
   `the_car_accelerates_smoothly_on_the_hook_with_the_hosts_drivetrain` and `the_shipped_presets_keep_the_hosts_wheel_inertia`.
   Untested on a real NFS car in the game: the test car is `VehicleSpec::example()`, one gear, one tyre.
2. **`parameter-mapping.md` section 2: "the car's name (lower case, as `bb.vehicles` reports it)".** `BbVehicleSpec` and `VehicleState`
   carry no name, and `VehicleSpawned` gives only a numeric car id. **Decision:** a car is named by its numbers,
   `spec-<mass kg>-<wheelbase cm>-<max power kW>` (`CarSpec::key_name`, for example `spec-1450-245-300`); that is the file name of its
   override and the name the log prints. A name field in the spec (a revision-bumped read) would replace it: `CarSpec::key_name` is the one place to change.
3. **`parameter-mapping.md` section 5.4: `cars/<car>.toml` collides with the content registry.** A mod's `cars/` folder is where the host
   reads **car manifests** (`docs/modding/README.md`, "Folder layout of a mod"). An override file placed there is seen by the host as a car
   manifest and is refused with an error in the log (and `cargo install-mod` keeps `cars/` untouched, which is good, but the host still scans it).
   **Decision:** the mod reads `<mod_dir>/cars/<car>.toml` as the spec and the task say; the folder is not shipped (empty), so nothing collides until a
   user adds a file. Suggested fix: rename the folder to `tuning/` (or `handling/`) in the spec and change one constant (`Control::car_file`).
4. **`parameter-mapping.md` sections 4.1 and 7: what a forced preset means.** The `preset` setting "forces the compound for every car",
   but `mu_y0 = preset.mu_y0 + grip_slope * (L - level_ref)` still uses the car's own grip level `L`. So `slick` on a car with `L = 1.0` gives
   `mu_y0 = 1.05`, less than `sport`'s 1.20 (the slick's shape, not its grip). **Decision:** the formula as written for every path. If a player's
   "slick" should also be grippier, the spec must say how the preset moves `L`.
5. **Precedence of the compound.** The `preset` setting, a car file's `compound` and the automatic choice: the spec does not rank them.
   **Decision:** the setting (when not `auto`) wins over the car file, which wins over the automatic choice.
6. **`parameter-mapping.md` section 4.4, step 9: ranges after the last step.** The loader validates every key of a file, but a class
   multiplier or the axle bias can push a value out of range (a mean peak angle of 14 degrees with a bias of 8 gives 18). **Decision:**
   every key is clamped to its range after step 8; the numbers of the test vectors are all inside, so none changes.
7. **`scenarios.md` appendix: two time and distance conventions.** The distance along the body axis advances with the velocity **at the
   end** of the tick (`dist += V_f(new) * dt`); with the start-of-tick velocity every distance is short by `V * dt` (0.46 m at 100 km/h) and
   `brake_m` is 35.31 instead of the spec's 34.85. And `T_LEAD` is the **end** time of the first tick at 100 km/h (4.98 s, 3.57 s; the
   start time gives 4.97 and 3.55), while `launch_t100_s` is the start time (4.97): the appendix says "start time" for both. Both are what reproduce the
   spec's numbers.
8. **`wheel-dynamics.md` section 6, the tick table.** `slip_angle` is printed to three decimals (0.004, 0.020, 0.024), so the stated
   tolerance of 1e-4 cannot hold against the table itself: the test allows 6e-4 (the table's own rounding). And the W7 airborne vectors
   need a positive `corner_mass_kg`, otherwise the not-handled rule of section 2 declines the call (the host always sends the mass).
9. **`wheel-dynamics.md` section 2: `dt > 0.1` is not handled.** The host sends `dt` as `f32`, so `0.1 s` arrives as 0.100000001: the check
   allows a relative 1e-6 over 0.1.
10. **`tyre-force.md` section 2: the thermal seam.** `tyre-thermal.md` was not among the sources, so the seam is a trait whose argument is a
    struct of three numbers (`GripInputs { t_eff, pressure, wear }`, units "as `tyre-thermal.md`"); stage 5 may add fields.
11. **The SDK delivers a payload-less `FRAME_START` as `Event::Unknown`** (what `ModRuntime::dispatch(kind, frame)` sends). The mod matches
    the frame start by `event.kind()`, so it works either way. Not a spec question; the host tests that dispatch bare events may want to know.
12. **Settings and the hotkey.** `bb.settings` has no `set`, so a flip by the hotkey or the console is not reflected in the settings row until the
    player touches it; `status` says what is true. The default hotkey `key:F1` is a choice (F2 to F12 are taken by the game's tools and other mods)
    and unverified in the game window; a refusal is logged and the other switches still work. Each switch-on registers a new hook closure that stays
    in the SDK's keep list until the mod unloads (a few hundred bytes per switch).

## Decisions

- **All model maths is `f64`**, narrowed once at the edge (`bridge.rs`). A result with a number the host would refuse (not finite, or past the
  ranges of `api-handling.md`) is never sent: the wheel answers "not handled" and its state is reset.
- **One concern per file**, none over 500 lines: `tyre_force.rs` (curve, load, camber, combined slip, aligning torque), `substep.rs` (slips,
  relaxation, the linearly implicit step, the brake clamp), `wheel.rs` (the tick, the trapezoid mean, the momentum limit, the airborne wheel),
  `keys.rs` (the one table of key, default and range that generates the parameter struct and the TOML table), `mapping/` and `presets/`.
- **Switches never register in a handler.** The settings handlers, the hotkey and the command only change `Control::requested`; the frame
  start (`AcHandling::frame`) applies it, so a switch always lands on a frame boundary and re-seeds the tyres.
- **The spec is read when the revision changes, not on a timer.** The hook compares `vehicle_revision` with the revision the parameters were
  built for; a mismatch keeps the old parameters for that tick and asks the next frame start to read `spec()` once. A host whose query
  runs ahead of the spec costs one spec read per frame until they agree.
- **`config.toml`** gives only first values. A saved settings choice other than the default wins over it; `enabled = true` in either place
  switches the model on.
- **Not handled** (nothing is guessed): not the player, no parameter set yet, `dt <= 0` or over 0.1 s, radius under 0.05 m, inertia not positive,
  `corner_mass_kg <= 0`, any non-finite input, a negative load, a wheel index past 3.

## Left for stages 3 to 5

- **The thermal seam (stage 5): `src/grip.rs`.** `GripScale::grip_scale(wheel, &GripInputs) -> f64` is called once per wheel per tick from
  `Control::tyre` (`src/control.rs`) and multiplies the `grip` input of `tyre_force` with the player's gain and the tyre condition; `reset(ambient_c)`
  is called when the model is switched on and when the car changes. `NoThermal` answers 1. A thermal model replaces the `Box<dyn GripScale>` that
  `Control::load_spec` puts in `Model`, keeps its state per wheel, and fills `t_eff`, `pressure` and `wear`; the HUD value goes through `bb.hud`
  from `AcHandling::frame`. The value is held for the whole tick (wheel-dynamics.md section 4.9); temperature moving inside the substeps would
  need `Frame` to carry the factor.
- **Suspension and camber (stage 3).** `tyre_force` already takes `gamma`; `substep::Frame::force_at` passes 0. Add a `camber` field to `WheelInput`
  (wheel-frame sign: plus on the right wheels, minus on the left, tyre-force.md section 2) and pass it there. The `bb.handling` 1.1 hook is registered
  next to the tyre hook in `AcHandling::register`/`unregister`, with its own state in `Model` next to `wheels`; the hub integrates wheel-local with
  its own substep rule. `CarSpec` already carries wheelbase, track, centre of gravity height and weight split.
- **Aids and steering (stage 4).** The `bb.handling` 1.2 controls hook is registered the same way; `CarSpec` carries the brake torques, gear ratios,
  final drive and power it needs; levels are new `bb.menus` sliders and `bb.settings` rows in `setup.rs`, and new keys in `settings.rs`.
- **Presets for the roster (stage 6).** Only the five compound and four class presets exist; per-car files are read but none ships. A roster
  needs the car name from the host (spec question 2) or the `spec-...` names.
- **The inertia question (spec question 1) is the first thing gate A will meet.**
