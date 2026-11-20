# ac-handling

An Assetto Corsa-style handling mod for the cars of NFS: Most Wanted. It replaces how the **player's tyres** turn load, slip
and torque into forces (a slip curve with a peak and a fall-off, load sensitivity, one grip budget for the two directions, a
wheel integrated in substeps) and targets the **feel** of Assetto Corsa. It is off until you switch it on, it is an offline
mod, and every other vehicle (cops, traffic, racers) keeps the built-in tyres.

> **An independent project, not affiliated with or endorsed by Kunos Simulazioni or 505 Games.** The model is written from our
> own specs ([docs/specs/ac-handling/](../../docs/specs/ac-handling/README.md)): public vehicle-dynamics literature and our own
> design values. **No game file and no foreign code is in this mod**, and nothing in it needs an Assetto Corsa install. "Targets
> the feel" means the scalars of [targets.md](../../docs/specs/ac-handling/targets.md) are design values, not measurements of
> the other game; every Assetto Corsa reference there is still `unmeasured`.

Decision: [ADR 0016](../../docs/decisions/0016-handling-hooks.md). Plan: [plans/ac-handling.md](../../docs/plans/ac-handling.md)
(this is stage 2, "mod v0"). The host interface it uses: [api-handling.md](../../docs/modding/api-handling.md).

## Using it

```sh
cargo install-mod ac-handling          # builds the mod (release) and copies it into the game's mods folder
```

Then start the game and switch it on in any of four ways (they all do the same thing, at the next frame):

| Switch | Where | Notes |
|---|---|---|
| **AC-style handling** (toggle) | Options, Gameplay, section "AC-style handling" | the master switch, **off** by default; the game saves it |
| **AC-style tyre preset** (list) | same section | `Automatic` (the car's grip picks the compound), `Economy`, `Road`, `Sport`, `Semi-slick`, `Slick`: forces that compound for every car |
| **AC-style handling: grip scale** (slider) | Options, Mods page | 0.5 to 1.5, default 1.0; multiplies the grip of every tyre |
| Hotkey **F1** | in the game | flips the master switch (the default binding; the hotkey is registered through `bb.input`) |
| Console command `ac-handling` (F12) | the console | `ac-handling on`, `off`, `status`, `preset <auto\|economy\|road\|sport\|semislick\|slick>` |

Switching on registers the tyre hook; switching off unregisters it and the built-in tyres run again. Either way the change lands
on a frame boundary, and the tyres start cold (fresh slip state) every time. When the game refuses the hook (an online session:
the host does not allow hooks online) the mod says so on the screen and in the log, and stays off. If you flip the model with the
hotkey or the console, the settings row keeps showing what you last set there; the console command `status` always tells the truth.

The model handles **only the player's car** (the vehicle role 0). Cops, traffic and racers keep the built-in tyres: their balance
was tuned for arcade grip, and a high-grip player against arcade cops is a design question, not an engine one.

## What it does to a car

When the player's car changes (the spec revision of `bb.vehicles` changes), the mod reads the car's numbers once and builds the
tyre parameters of each axle: the car's grip level picks a **compound class** (economy to slick), its size picks a **car class**
(compact, sport, super, heavy), and a per-car file can override anything ([Tuning](#tuning)). The log says what it chose, once per car:

```
ac-handling: car "spec-1450-245-300": L=1.00 -> sport/sport, 3698 N front, 3414 N rear per wheel, no cars/spec-1450-245-300.toml
```

The host does not report a car's name through `bb.vehicles`, so the mod names a car by its numbers: `spec-<mass kg>-<wheelbase cm>-<power kW>`.
That name is the file name of the car's override (`cars/<name>.toml`).

## Tuning

| File | What |
|---|---|
| `presets/compound-<name>.toml` | one per compound class (`economy`, `road`, `sport`, `semislick`, `slick`): the tyre numbers of the compound |
| `presets/class-<name>.toml` | one per car class (`compact`, `sport`, `super`, `heavy`): multipliers and additions on top of the compound |
| `cars/<name>.toml` | an override for one car: forces a compound or class, sets the axle bias, replaces any key for both axles (`[tyre]`) or one (`[tyre.front]`, `[tyre.rear]`) |
| `config.toml` | the first values of the settings: `enabled`, `preset`, `grip_gain`, `substeps` (0 is automatic) |

The shipped presets are also built into the library, so the mod works with an empty `presets/` folder; a file in the folder that
fails to parse or leaves a range is rejected whole with one line in the log, and the built-in copy of that file is used. The
schema, every key, its range and the order the layers combine in are in
[parameter-mapping.md](../../docs/specs/ac-handling/parameter-mapping.md) sections 4 and 5. Every number is a design value: the
presets and the car files are the main tuning surface. Do not look for `cars/` in the content sense of the modding API: see
[NOTES.md](NOTES.md) (spec question 3).

## Building and testing

```sh
cargo test -p ac-handling              # the pure model, the specs' test vectors, the scenarios, the real loader
cargo clippy -p ac-handling --all-targets -- -D warnings
```

No game and no install are needed. The tests are:

- `src/tests/`: every test vector of the specs (tyre force T1 to T14, wheel dynamics W1 to W10 with the substep convergence,
  parameter mapping with its four cars and spot checks), the loaders (one test per class of TOML mistake) and the switches;
- `tests/scenarios/`: the planar reference car of the scenarios spec runs the six scenarios and checks the scalars against the
  bands of `targets.md`; a row whose reference is `unmeasured` is skipped with a printed reason (`-- --nocapture` shows the report);
- `tests/real_loader/`: builds the library with cargo, loads it with the real loader against a fake game, presses the hotkey,
  runs the console command, and drives a real vehicle with the hook.

**Windows dev builds.** The repository's dev profile compiles with Cranelift, which cannot catch a panic inside a mod DLL on
Windows (the SDK's guard that turns a panic into an error status needs unwinding). Build the mod with LLVM or in release:

```sh
cargo build -p ac-handling --config profile.dev.codegen-backend="llvm"      # or: cargo build -p ac-handling --release
```

`cargo install-mod ac-handling` builds in release. The loader test builds its own copy with LLVM, whatever the profile says.

## Where it stands

Stage 2 ([the plan](../../docs/plans/ac-handling.md)): the tyre model, the master toggle, the preset, the grip scale, the hotkey
and the command. Suspension, driver aids, steering feel and the thermal model are later stages; the seams for them are described in
[NOTES.md](NOTES.md). Gate A is the owner's drive: the numbers pass their bands, but whether it *feels* right is not something a
test can say.
