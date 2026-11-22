# The ac-handling mod

Part of the [modding catalogue](README.md); decision in [ADR 0016](../decisions/0016-handling-hooks.md), plan in
[plans/ac-handling.md](../plans/ac-handling.md), specs in [specs/ac-handling/](../specs/ac-handling/README.md), the host interface
in [api-handling.md](api-handling.md) ([suspension](api-handling-suspension.md), [controls](api-handling-controls.md)). The mod's own
folder is [`mods/ac-handling`](../../mods/ac-handling/README.md); its design notes and open questions are in its
[NOTES.md](../../mods/ac-handling/NOTES.md).

`ac-handling` (id `nfsmw.ac-handling`) is a handling mod that **targets the feel of Assetto Corsa** on the player's own car of
NFS: Most Wanted. It replaces the tyre forces with a model that has a slip curve with a peak and a fall-off, load sensitivity and
one grip budget for both directions, with the wheel integrated in substeps inside the call; and, as optional units, a suspension with
an unsprung hub, tyre temperature and wear, driver aids and a speed-sensitive steering. It is **off until you switch it on**.

> It is an independent project, not affiliated with or endorsed by Kunos Simulazioni or 505 Games. It contains **no game file and no
> foreign code**: it is written from our own specs, which use public vehicle-dynamics literature and design values. **Nothing in it was
> measured against Assetto Corsa**: the target numbers of [targets.md](../specs/ac-handling/targets.md) are design values and every
> Assetto Corsa reference in it is `unmeasured`. "The feel of" is the goal, not a claim; how it feels in the real game has not been
> tested by anyone yet (the owner's drive is gate A and B of the plan).

It is **offline only**: the host refuses every hook of `bb.handling` in an online session (the mod stays off and says so), and a
cop, a traffic car or a racer always keeps the built-in model: only the player's car is handled.

## Install

```sh
cargo install-mod ac-handling          # builds the mod in release and copies it into the game's mods folder
```

The mod is a member of the workspace but not of its default build. On Windows the repository's dev profile compiles with Cranelift, which
**cannot catch a panic inside a mod DLL**; the SDK's guard that turns a panic into an error status needs unwinding. Build the mod with LLVM
or in release (`cargo install-mod` builds in release):

```sh
cargo build -p ac-handling --config profile.dev.codegen-backend="llvm"      # or: cargo build -p ac-handling --release
```

The game needs `bb.handling` 1.2 and `bb.vehicles` 1.3 (the mod requires them); `bb.settings`, `bb.menus`, `bb.input`, `bb.commands`,
`bb.hud` and `bb.config` are optional and the mod works, with fewer switches, without any of them.

## Switching it on

Everything is applied at the next frame, never inside a handler, and the tyres start cold each time a switch changes the model.

| Switch | Where | What |
|---|---|---|
| **AC-style handling** (toggle) | Options, Gameplay, section "AC-style handling" | the master switch, **off** by default |
| Hotkey **F1** | in the game | flips the master switch (`bb.input`, default binding `key:F1`; not yet checked in the game window) |
| `ac-handling on` / `off` | the F12 console | the same, from the console |
| `config.toml` `enabled = true` | the mod's folder | the first value; the saved setting wins when it says on |

**Master off registers nothing**: no tyre, suspension or controls hook. A flip with the hotkey or the console is not reflected by the
settings row (`bb.settings` has no `set`); `ac-handling status` always says what is true.

## The units

The master switch puts the **tyre model** in. Four units sit on top of it and each toggles on its own:

| Unit | Hook it uses | What it does | Default |
|---|---|---|---|
| **suspension** | `bb.handling` suspension (1.1) | spring, two-rate damper, bump and droop stops, anti-roll bar, an unsprung hub integrated in substeps, camber from travel, roll and steering, roll-centre load transfer; its camber and tyre load reach the tyre of the same tick | on |
| **thermal** | inside the tyre hook | three surface stripes and a core per tyre, a pressure that follows the core, wear; a grip factor on the peak friction | on |
| **aids** | `bb.handling` controls (1.2) | traction control, ABS, stability control, brake bias | **off** |
| **steering** | the controls hook, with its steering flag | ratio and lock, speed-sensitive lock, rate limit and counter-steer help for a pad or keyboard; an optional force-feedback value | **off** |

The hooks are registered only while the master switch is on **and** a unit that needs them is: the suspension hook for the suspension
unit, the controls hook for the aids or the steering unit (one hook serves both; the steering flag is set only while the steering unit is on).
A hook the host refuses is logged once and not retried until a switch changes; the others stay.

Each unit's mode is **preset** (the car's preset decides: the defaults above), **on** or **off**. Set it in the settings (one list each:
"Preset default", "On", "Off"), with the console, or in `config.toml`:

| Console | Meaning |
|---|---|
| `ac-handling <suspension\|aids\|steering\|thermal> <on\|off\|preset>` | a unit's mode |
| `ac-handling tc <0-5>`, `abs <0-5>`, `esc <0-4>` | the level of traction control, ABS, stability control (0 is off) |
| `ac-handling bias <-20..20>` | brake bias in clicks of 0.005 of the front share |
| `ac-handling device <pad\|wheel>` | what steers: a pad or keyboard gets the speed-sensitive lock and the rate limit, a wheel is trusted |
| `ac-handling hud <on\|off>` | the tyre temperature line on the HUD |
| `ac-handling preset <auto\|economy\|road\|sport\|semislick\|slick>` | the tyre compound |
| `ac-handling ffb` | the force-feedback value now (see below) |
| `ac-handling status` | one line: on or off, the registered hooks, the preset, the grip, the units, the car |

### Sliders (Options, Mods page)

| Slider | Range | Default | What |
|---|---|---|---|
| grip scale | 0.5 to 1.5 | 1.0 | multiplies the grip of every tyre |
| traction control, ABS | 0 to 5 | 0 | 0 is off; a higher level intervenes earlier and harder |
| stability control | 0 to 4 | 0 | brakes the front wheel on the side the tail slides towards |
| brake bias | -20 to 20 | 0 | clicks; the aids unit only |
| steering speed sensitivity | 0 to 1 | 1 | 0 keeps the full lock at every speed (pad) |
| force feedback gain, curve | 0 to 3, 0.4 to 1.5 | 1, 1 | scale and shape of the force-feedback value |

A level slider that the player has not moved follows the preset's starting level (`[units]` in the preset, below); once moved, even to 0,
it is the player's. The settings also have the "tyre temperatures on the HUD" toggle and the "steering device" list.

## Presets and per-car tuning

The mapping from a car to its tyres is specified in [parameter-mapping.md](../specs/ac-handling/parameter-mapping.md). The car's grip level picks a
**compound class** (economy, road, sport, semislick, slick), its size and power a **car class** (compact, sport, super, heavy), then the preset
files and a per-car file refine it. The `preset` setting forces a compound for every car (it **re-bases** the grip level on the compound, so a
forced `slick` is never below a forced `sport`); a tuning file's `compound` comes next; the car's own grip last.

| File | What |
|---|---|
| `presets/compound-<name>.toml` | the tyre keys of a compound, the grip slope, and the optional `[units]` defaults |
| `presets/class-<name>.toml` | multipliers and additions per car class |
| `tuning/<id>.toml` | the override of one car |
| `config.toml` | first values of the settings: `enabled`, `preset`, `grip_gain`, `substeps`, `suspension`, `aids`, `steering`, `thermal`, `device`, `hud`, `warm_start`, `tc`, `abs`, `esc`, `brake_bias_clicks` |

The shipped presets are also built into the library, so an empty `presets/` folder works; a file that fails to parse or leaves a range is rejected
whole with one line in the log and its built-in copy is used. The player's settings never write these files.

### The car id

The host's car spec carries no name, so a car is named by its numbers: `spec-<kg>-<cm>-<kW>` with each rounded to the nearest integer (ties
away from zero): `spec-1450-245-300` is a 1450 kg car with a 2.45 m wheelbase and 300 kW. A spec with a number that is not finite, a mass or
wheelbase at or below 0 or a negative power is `spec-invalid` and gets no tuning file. Two trims that round to the same three numbers share an id
and a file; the automatic mapping still follows each car's own grip. The log prints the id once per car:

```
ac-handling: car "spec-1450-245-300": L=1.00 -> sport/sport, 3698 N front, 3414 N rear per wheel, no tuning/spec-1450-245-300.toml
```

The file lives in **`tuning/`**, not `cars/`: a mod's `cars/` folder is where the host reads car manifests.

### The schema of `tuning/<id>.toml`

`schema = 1` first; unknown keys are rejected at every level, every number must be finite and inside its range; a file that fails is rejected whole
with one line in the log and the automatic mapping stands.

```toml
schema = 1
compound = "sport"              # optional: forces the compound instead of the grip level
class = "super"                 # optional
axle_angle_bias_deg = 2.5       # optional: front minus rear peak slip angle
[tyre]                          # optional: replaces a key on both axles (see the spec for every key and range)
mu_y0 = 1.25
[tyre.front]                    # optional: one axle
peak_angle_deg = 9.5
[tyre.rear]
mu_y0 = 1.30
[units]                         # optional: which optional units start on for this car, and the aids' starting levels
suspension = true               # also: aids, steering, thermal (booleans)
aids = true
tc = 2                          # 0 to 5; also abs (0 to 5), esc (0 to 4), brake_bias_clicks (-20 to 20)
[suspension.front]              # optional: keys of suspension.md section 5 (k_w, k_arb, c_bs, h_rc, gamma0, ...)
k_w = 35000
[suspension.rear]
k_arb = 9000
[steering]                      # optional: keys of steering.md section 5 (theta_max_deg, delta_max_deg, s_sens, ...)
delta_max_deg = 35
```

`inertia_scale` (a `[tyre]` key, default 1.0) multiplies the host's wheel inertia. The host's drivetrain loop is only stable from about 0.5 up
(see the wheel-dynamics spec, contract point 3): a lower value is accepted and logs one warning.

## What is modelled, and what is not

**Modelled** (each part is a spec with its test vectors, all reproduced by the mod's tests):

- the tyre force: slip curve with peak and fall-off, load sensitivity, combined slip on one friction ellipse, camber, surface multipliers, aligning
  torque ([tyre-force.md](../specs/ac-handling/tyre-force.md));
- the wheel: spin, slip relaxation, brake and rolling resistance, low-speed behaviour, 600 substeps per second inside the call
  ([wheel-dynamics.md](../specs/ac-handling/wheel-dynamics.md));
- thermal state, pressure and wear ([tyre-thermal.md](../specs/ac-handling/tyre-thermal.md)); the host has no weather, so it runs at the 20 C and
  25 C the host reports, and a "warm start" (`warm_start = true`) begins the tyres at 70 C;
- the suspension of an axle ([suspension.md](../specs/ac-handling/suspension.md)), the aids ([aids.md](../specs/ac-handling/aids.md)) and the
  steering ([steering.md](../specs/ac-handling/steering.md)).

**Not modelled**: pitch-dependent camber, toe and bump steer, anti-dive and anti-squat, heave springs; wet or loose surfaces (the surface only scales
the grip); flat spots, blistering, punctures; an electronic differential, launch control, hill hold; a yaw-torque stability aid (the hook has no yaw
output, so stability control brakes one front wheel); a pressure model with leaks; sound, tyre smoke or skid marks that depend on the model beyond the
`traction`, `slip` and `slip_angle` the host's effects already read; the game's brake-balance slider and launch-stand rule while the aids unit is on
(a handled controls answer bypasses both, by the hook's design).

**Force feedback.** The game has no force-feedback output and the hook has none either. The steering unit computes the signal of the spec
(`-1..1`, positive pulls the wheel to the right) and drops it: the mod's `Control::ffb_signal()` accessor and the `ac-handling ffb` command read
it, nothing else uses it.

## Performance

A tick costs, per player car, four tyre calls, two suspension calls and one controls call. The tyre call integrates in `ceil(dt * 600)` substeps
(10 at 60 Hz, 5 at 144 Hz, 3 at 240 Hz, at most 24) with a handful of force evaluations each; the suspension call uses `ceil(dt / 2.5 ms)` hub
substeps (7 at 60 Hz). The model is allocation-free in the tyre path. The host's budget is 2 ms per call; no timing in the real game has been
measured, only that the loader tests drive a real vehicle through all three hooks without a fault.

## Known limits

- **Feel is untested in the real game.** The numbers pass their bands on the mod's own planar test car; the real vehicle has springs, roll and pitch
  the bands cannot see. The design bands of the scenarios are design values, not measurements of any other game, and a row can be wrong by more than
  its band.
- The default wheel inertia (1.0 of the host's constant) is what the host's drivetrain tolerates; it makes the stock brakes, not the tyres, set the
  braking distance of the reference car and costs launch time compared with a bare wheel (the inertia sweep in
  [scenarios.md](../specs/ac-handling/scenarios.md)).
- The suspension and thermal units have never run on a real NFS car in the game window; their defaults are on in every shipped preset and can be
  turned off in a second.
- One tuning file per three rounded numbers; the car's real name is not available.
- A hook refused by the host (an online session) leaves the mod off with a message and a log line.
- Presets exist for the five compounds and four classes; no per-car file ships.

## Tests

`cargo test -p ac-handling` needs no game and no install: the test vectors of every spec, the loaders (one test per class of TOML mistake), the
switches, the scenarios of the planar reference car with the inertia sweep, and the real loader against a fake host driving a real vehicle. The
Windows dev-build caveat above applies to the library the loader test builds; it builds its own copy with LLVM.
