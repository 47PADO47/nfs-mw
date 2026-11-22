# ac-handling

An Assetto Corsa-style handling mod for the cars of NFS: Most Wanted. It replaces how the **player's tyres** turn load, slip and
torque into forces (a slip curve with a peak and a fall-off, load sensitivity, one grip budget for the two directions, a wheel
integrated in substeps) and targets the **feel** of Assetto Corsa. On top of the tyre model it has four optional units, each switched on
its own: **suspension** (a hub with its own mass, anti-roll bar, camber), **thermal** (tyre temperature, pressure and wear), **driver aids**
(traction control, ABS, stability control, brake bias) and **steering** (speed-sensitive lock, rate limit). It is off until you switch it
on, it is an offline mod, and every other vehicle (cops, traffic, racers) keeps the built-in model.

> **An independent project, not affiliated with or endorsed by Kunos Simulazioni or 505 Games.** The model is written from our
> own specs ([docs/specs/ac-handling/](../../docs/specs/ac-handling/README.md)): public vehicle-dynamics literature and our own
> design values. **No game file and no foreign code is in this mod**, and nothing in it needs an Assetto Corsa install. "Targets
> the feel" means the scalars of [targets.md](../../docs/specs/ac-handling/targets.md) are design values, **not measurements of the
> other game**; every Assetto Corsa reference there is still `unmeasured`, and how it feels in the real game has not been tested yet.

The full user and author page, with every toggle, slider, console subcommand and the tuning schema, is
[docs/modding/ac-handling.md](../../docs/modding/ac-handling.md). Decision: [ADR 0016](../../docs/decisions/0016-handling-hooks.md). Plan:
[plans/ac-handling.md](../../docs/plans/ac-handling.md). Host interface: [api-handling.md](../../docs/modding/api-handling.md).

## Using it

```sh
cargo install-mod ac-handling          # builds the mod (release) and copies it into the game's mods folder
```

Then switch it on: the **AC-style handling** toggle (Options, Gameplay), the hotkey **F1**, the console command `ac-handling on`, or
`enabled = true` in `config.toml`. The units have a list each in the same settings section (`Preset default`, `On`, `Off`) and a console
subcommand (`ac-handling suspension off`, `ac-handling aids on`, `ac-handling tc 3`, `ac-handling status`, ...). The sliders (grip scale,
traction control, ABS, stability control, brake bias, steering sensitivity, force-feedback gain and curve) are on the Mods page.

**Master off registers nothing.** With the master on, the tyre hook is in; the suspension hook only while the suspension unit is on; the
controls hook only while the aids or the steering unit is on. Defaults of the shipped presets: suspension and thermal on, aids and steering off.
When the game refuses a hook (an online session) the mod says so and that hook stays off.

## Tuning

| File | What |
|---|---|
| `presets/compound-<name>.toml` | one per compound class (`economy`, `road`, `sport`, `semislick`, `slick`): the tyre numbers, and the `[units]` defaults |
| `presets/class-<name>.toml` | one per car class (`compact`, `sport`, `super`, `heavy`): multipliers and additions on top of the compound |
| `tuning/<id>.toml` | an override for one car, named by its id `spec-<kg>-<cm>-<kW>` (the log prints it): compound, class, axle bias, tyre keys, `[units]`, `[suspension.front\|rear]`, `[steering]` |
| `config.toml` | the first values of the settings |

The shipped presets are also built into the library; a file that fails to parse or leaves a range is rejected whole with one line in the log.
The override folder is `tuning/`, not `cars/` (the host reads a mod's `cars/` as car manifests).

## Building and testing

```sh
cargo test -p ac-handling              # the test vectors of every spec, the scenarios, the real loader
cargo clippy -p ac-handling --all-targets -- -D warnings
```

No game and no install are needed. The tests are:

- `src/tests/`: every test vector of the specs (tyre force T1 to T14, wheel dynamics W1 to W10, parameter mapping with its forced-compound
  table and car ids, tyre-thermal T1 to T12, suspension V1 to V17, aids TC/ABS/BIAS/ESC, steering S1 to S10 and F1 to F6), the loaders (one test
  per class of TOML mistake), and the switches and units;
- `tests/scenarios/`: the planar reference car of the scenarios spec (convention E) runs the six scenarios, S2t, the 11-rung S5 ladder, the inertia
  sweep and the compound tables, against `targets.md`; a row whose reference is `unmeasured` is skipped with a printed reason;
- `tests/real_loader/`: builds the library with cargo, loads it with the real loader against a fake game, presses the hotkey, runs the console
  command, toggles every unit and drives a real vehicle with all three hooks.

**Windows dev builds.** The repository's dev profile compiles with Cranelift, which cannot catch a panic inside a mod DLL on Windows (the
SDK's guard needs unwinding). Build the mod with LLVM or in release:

```sh
cargo build -p ac-handling --config profile.dev.codegen-backend="llvm"      # or: cargo build -p ac-handling --release
```

`cargo install-mod ac-handling` builds in release. The loader test builds its own copy with LLVM, whatever the profile says.

## Where it stands

Stages 2 to 5 of [the plan](../../docs/plans/ac-handling.md) are in code: the tyre model, the suspension, the aids and steering, the thermal
state. The numbers pass their bands and reproduce every test vector; whether it **feels** right in the real game is the owner's drive (gates A
and B) and no test can say. Open questions and decisions are in [NOTES.md](NOTES.md).
