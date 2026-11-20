# Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` / `tests` modules in every crate | no | yes |
| Real-install tests | [`crates/nfsmw-data/tests/real_install/`](../crates/nfsmw-data/tests/real_install) (`#[ignore]`) | yes | no |
| Real-save tests | [`crates/nfsmw-career/tests/real_saves.rs`](../crates/nfsmw-career/tests/real_saves.rs) (runs only when `NFSMW_SAVE_DIR` points at the profile folder of the game under Documents, or at a folder of profile files; returns at once without it) | yes (saves) | no |
| GPU tests (renderer) | `#[ignore = "needs a GPU"]` in `blackbox-render`, scenes from [`blackbox-gfx-testkit`](../libs/blackbox-gfx-testkit); the passes' own in [`blackbox-gpu-passes`](../libs/blackbox-gpu-passes) | no | no (optional job) |
| Leak + size checks | `cargo xtask check` | no | yes, first job |
| Python tool tests | [`tests/`](../tests) | no | yes |

```sh
NFSMW_GAME_DIR="<your-install>" cargo test --release -p nfsmw-data -- --ignored
```

The real-install tests check:

- all 100 cars (15,781 solids) parse, with every index in range;
- the BMW M3 GTR matches [models.md](formats/models.md);
- the car and global texture packs decode;
- all 720 world sections parse (20,377 solids, 3,644 textures, 77,783 scenery instances), and tile scenery
  resolves;
- the visible-section tables parse (515 boundaries, 435 zones, 39 loading sections), zones never overlap,
  and only the panoramas `A91`, `A94`, `C99`, `O93` are in no zone's list.

The whole stream parses in under a second in release builds.

`nfsmw view-car … --screenshot out.png` and `nfsmw view-world … --wait-for-load --screenshot out.png`
render one frame off-screen. Use them to check rendering changes and backends without a window.

## GPU tests

The renderer's tests that need a GPU are `#[ignore = "needs a GPU"]`, run headless (no window) and need no game
data. They skip, not fail, when the machine has no adapter for the API.

```sh
cargo test -p blackbox-render --lib -- --include-ignored          # Vulkan
BLACKBOX_TEST_API=dx12 cargo test -p blackbox-render --lib -- --include-ignored   # or gl
BLACKBOX_GPU_FALLBACK=1 cargo test -p blackbox-render --lib -- --include-ignored  # software adapter (lavapipe, WARP)
```

They take turns on one lock, because creating Vulkan devices on parallel threads crashes some Mesa drivers. The
lock (`serial()`) lives in `blackbox-gpu-passes`' `test-support` feature, so the tests of both crates share it.
The passes crate tests its UI, effect, soft particle, filter and FSR 1 passes against a plain wgpu device, with no
renderer around them (`cargo test -p blackbox-gpu-passes -- --include-ignored`).

- **Structure tests** (`gpu/parity/structure_tests.rs`) check facts that hold on any adapter: depth order out to
  9 km, fog, alpha-test cut-outs, blend order, clip rectangles, and that the UI is never post-processed.
- **API tests** check the native `Capabilities` (FXAA; bilinear and FSR 1; every tone map; bloom; no ray
  tracing; `compressed_bc` from the adapter), `resolve` and `apply_graphics` on a real renderer.
- **Digest tests** guard the native image. Each testkit scene is rendered at 256x144 under five settings (direct
  path, offscreen, FXAA, bloom with ACES, FSR 1 at 67 %) and its 32x18 grid of mean colours is compared to a digest
  stored as a Rust constant (`gpu/parity/digests/`), within +-2 per channel. Any change to what the native
  renderer draws moves some cell, so an intended change updates the digests in the same commit.

Digests depend on the GPU and driver. These were recorded on **Intel Iris Xe (Vulkan, Mesa)**; the digest test
skips on any other adapter (its header comment names the GPU and driver). A different Mesa version can move a few
cells by rasterisation detail: regenerate and review the change.

To regenerate (the test prints the files; it never writes to the repository):

```sh
BLACKBOX_UPDATE_DIGESTS=1 cargo test -p blackbox-render --lib native_digests -- --include-ignored --nocapture > /tmp/digests.txt
python tools/split_digests.py /tmp/digests.txt
```

To add a GPU family, add a variant to `Family` in `gpu/test_support.rs`, a directory under `digests/`, and run
the generator on that GPU. Cross-backend comparisons use the testkit's metrics instead of digests (its README).

## Bevy renderer tests

`cargo test -p blackbox-bevy-render` runs the CPU tests. Its GPU tests (`-- --include-ignored --test-threads=1`, they take
the same `serial()` lock) draw the strict testkit scenes headless on the Bevy renderer and compare them with the
native one through `compare` against the plan's tolerances (max 4, mean 0.5, p99 2 of 255); they print the real
numbers (`--nocapture`) and skip without an adapter. `BLACKBOX_GPU_FALLBACK=1` runs them on lavapipe; where the
environment restricts Vulkan drivers (`VK_LOADER_DRIVERS_SELECT=*intel*`), also set it to `'*lvp*'`. Results and
how to compare the real city: [bevy-backend.md](bevy-backend.md) (`cargo xtask img-diff`).

## Repeatable comparison runs

Two per-run options make a run repeatable and comparable with the original game. They are on every command that opens
a window (`play`, `view-world` and the viewers), are never saved, and are off unless given
(`crates/nfsmw/src/parity/`).

| Option | Environment | Effect |
|---|---|---|
| `--seed <u64>` | `NFSMW_SEED` | Seeds what single player otherwise seeds from the clock: race opponent names and the player's grid slot, chase radio lines, police speech, the radio playlist, quick play picks, the marker cards and the first loading tip. It switches on none of the multiplayer session rules (`session on <seed>` does). The traffic streams keep their fixed seed |
| `--stock-rules` | `NFSMW_STOCK_RULES=1` | Forces every mechanics-changing setting to the original's behaviour, whatever `config.toml`, the mod-style ini files and the saved options say. The config file is not written |

The command line wins over the environment. Without `--seed` the choices above differ from run to run, as before.

`--stock-rules` forces, in one list (`crates/nfsmw/src/parity/stock.rs`):

- traffic lights off, the traffic budget 20 (unless the command line sets one) and the Reference traffic policy;
- `[pursuit_options]`: `enhanced` off, `bounty_fixes` off, the other values at their defaults;
- physics rate 60 Hz, game speed 1.0, animation speed 1.0, the rev limiter on, car scale 1.0, rain amount 100 %;
- every kind of game moment on (the main Game Moment Camera switch stays the player's);
- `NFSMWSpeedFixerSettings.ini`, `NFSMWExtraOptionsSettings.ini`, `NFSMWBartenderSettings.ini`,
  `NFSMWFairBustsSettings.ini` and `NFSMWHelicopterSettings.ini` treated as absent;
- no native mods (`mods = off`);
- the F5 testing overrides off.

The world applies the forced values where it takes its settings, so a change made in a menu during a stock run does
not reach the simulation. An explicit `--traffic <N>` or `--no-traffic` on the command line wins over the forced
budget, since the original can also run a race without traffic; the environment and `config.toml` do not. Not forced, because the original's value is not stated or the original has the same player
option: `cop_share`, `always_rain`, `transmission`, the Game Moment Camera switch, the control deadzones and
sensitivities, the wheel options and `eatrax_jukebox.toml` (music only).

### Run header

The pursuit trace's `session.json` records the run under `run`, next to the existing `settings` (which keeps its
layout); the UI trace's `header` record carries the same struct under `data.run`, without the `world` part
(`crates/nfsmw/src/parity/header.rs`).

| Key | Content |
|---|---|
| `seed`, `stock_rules` | the run's options; `seed` is `null` when the clock seeds the run |
| `ini_files` | each mod-style ini file: `name`, `present` (it exists next to the config file), `applied` (the run read it) |
| `extra_options` | the Extra Options pursuit keys in use: `heat_bounds`, `speeding_limit_mps`, `reckless_limit_mps` |
| `mods` | `mode` and the `loaded` mod ids; `null` when the run started no mods |
| `world` | `track`, `player_car`, `transmission`, `physics_rate_hz`, `game_speed`, `animation_speed`, `no_rev_limiter`, `car_scale`, `always_rain`, `rain_amount`, `traffic` (`cars`, `cop_share`, `manual`, `lights`, `policy`), `pursuit_options`, `game_moments`, `speedbreaker`, `session_rules` and the F5 `testing` overrides, each as the world runs it |

The header is taken when the recording starts. The audio trace records no header.

## Unattended race runs

One command runs a whole race with nobody at the controls and writes a record of it, to compare with the same race run
unattended in the original game. The options are on the commands that drive, `view-world --drive` and `play --drive`;
they are never saved, and without them nothing changes (`crates/nfsmw/src/parity/race_run.rs`,
`crates/nfsmw/src/cli/race_args.rs`, `crates/nfsmw/src/scenes/world/race/unattended.rs`).

| Option | Effect |
|---|---|
| `--race <EVENT>` | Starts in this race, skipping the menus: a career event id (`16.1.1`) or a race name, as the console's `race start` takes it. The race is queued as a quick race is and starts when the car stands, through the same start path. It needs `--drive` |
| `--race-opponents <N>`, `--race-laps <N>` | The opponents and the laps; left out, the event's own. `race start <race> opponents=N laps=N` in the console does the same |
| `--race-difficulty <LEVEL>` | The quick-race difficulty: `easy`, `medium` or `hard` (the levels 33, 66 and 100 the quick race writes) or a `DifficultyLevel` of 0 to 100. Left out, the event's own `DifficultyLevel`: 100, hard, in 250 of the 255 races ([gameplay-races.md](formats/gameplay-races.md) section 3.3). It sets the opponents' skills and the catch-up's class; a `--race` run is a quick race |
| `--autopilot` | In a race, the opponents' AI driver drives the player's car from the green light |
| `--autopilot-after <SECS>` | The driver takes the car this many seconds of race time after the green light; until then the car gets the real input, which is none with nobody at the controls |
| `--autopilot-skill <0-1>` | The driver's skill on the opponents' scale, default 1. The catch-up never changes it |
| `--race-trace <FILE>`, `NFSMW_RACE_TRACE=<FILE>` | Writes the race trace to this file, replacing it; the flag wins |
| `--exit-after-race` | Ends the process when a race is over, or is taken down without a result. A race asked for with `--race` that cannot start ends it with a failure |

The autopilot (`scenes/world/race/autopilot.rs`) is the driver a game moment lends the player's car to
(`scenes/world/ai/moment_driver.rs`): the opponents' think and PID controller, given what the race gives an opponent
each tick, which is the point past the next gate, the leg and the race's route without shortcuts. Its pedals go into the
car's own input step, and that step gives the car the control set-up the computer's cars use (AI steering, the simple
chassis and tires, an automatic gearbox) while the driver has it. The car is never placed or moved by hand. The driver
has no reverse-and-recover rule of its own beyond the controller's, and a game moment's camera takes the car for its
length as in any race.

### Race trace

JSON lines (`crates/nfsmw/src/parity/race_trace.rs`), written and flushed as they happen. Every line has `kind`, `tick`
(the physics tick it was raised in, counted from 1 since the world opened), `sim_time` (simulated seconds since the world
opened, the sum of the ticks' deltas) and `wall_ms` (monotonic milliseconds since the file was opened). Racer 0 is the
player; laps and checkpoints count from 1; `race_time`, `lap_time` and `total_time` are seconds since the green light.
A gate is tested once a tick, so a time is that of the tick the car crossed in.

| `kind` | Fields |
|---|---|
| `header` | `schema` (`nfsmw-race-trace-v1`), `created_unix_ms`, `run` (the [run header](#run-header), with the world part), `build`, `event`, `race`, `race_type`, `mode`, `difficulty`, `catch_up`, `traffic`, `cops`, `opponents` and `laps` as the race runs them, `checkpoints_per_lap`, `finish_line`, `rolling_start`, `racers` (`racer`, `name`, `car`, `human`, `skill`, `grid`), `autopilot` (`after_secs`, `skill`, or `null`). Written at the green light, just before `timer_start` (see below) |
| `timer_start` | `race_time` 0: the green light, or the start of a rolling race |
| `autopilot` | `race_time` at which the driver took the car |
| `checkpoint` | `racer`, `lap`, `checkpoint`, `race_time` |
| `lap` | `racer`, `lap`, `lap_time`, `race_time`; the last lap too |
| `racer_finished` | `racer`, `place`, `total_time` |
| `knocked_out` | `racer`, `race_time` |
| `sample` | `race_time`, `racers`: per racer on the road `racer`, `position`, `speed`, `controls` (`throttle`, `brake`, `steering`, `gear`, `rpm`) and, for a racer an AI driver drives, `driver` (`target_speed`, `base_skill`, `catch_up`, `skill`, `catch_up_force`, `stand_in`) |
| `race_finished` | `race_time`, `results` (`racer`, `place`, `finished`, `total_time` per racer) |

- The finish line is not a checkpoint: crossing it writes `lap`, and no `checkpoint`. A race with no finish line closes
  the lap at its last checkpoint.
- The race is over when the player has finished or is out, or the time ran out. The opponents still on the road are not
  timed after that: they are in `results` with `finished` false and no `lap` or `racer_finished` line follows.
- A restart of the race writes a new `header`.
- The `header` is the race's identity, read from the running race when the light turns green
  (`crates/nfsmw/src/parity/race_trace/header.rs`, `crates/nfsmw/src/scenes/world/race/trace_rows.rs`); nothing in it is
  copied from the command line. A race taken down before its green light writes no header.
  - `build`: `git_commit` (the commit the binary was built from, `null` without Git at compile time) and `git_dirty`
    (tracked files had changes when the build script last ran, `crates/nfsmw/build.rs`).
  - `mode`: `quick` or `career`, the rule set the race runs under (the skills, the catch-up's context, the result).
  - `difficulty`: the quick-race difficulty in effect, `level` (0 to 100) and `name` (`easy` below 34, `medium` to 66,
    `hard` above); `null` in a career race, which does not use it.
  - `catch_up`: whether the catch-up (rubber band) is on.
  - `traffic`: `level`, the event's traffic density in percent as the world applies it (0 in a drag race), `pattern`
    (`drag`, or `null`) and `cars`, the most traffic cars the world keeps.
  - `cops`: whether police may enter the race (false when the race bars them or the F5 police switch is off).
  - per racer `skill`: the base skill its AI driver uses (an opponent's from its car, the player's the autopilot's;
    `null` for a racer no AI drives), before the catch-up offsets it; and `grid`: where it stands at the green light,
    in the frame of `sample.position`.
- `sample` lines (`crates/nfsmw/src/parity/race_trace/sample.rs`) come every 0.1 s of race time from the green light
  until the race is over, on the first tick whose race time reaches the next multiple of 0.1 s (to half a tick; at 60 Hz
  every sixth tick, the first on the green light's tick, after its `timer_start`). None is written during the countdown.
  `position` is in the game's world coordinates, metres: x and y on the ground, z up, the axes of the data's `Position`
  (the physics space's `(x, y, z)` is `(-y, z, x)` of it). `speed` is the speed along the car in m/s, without its sign.
  `controls` are what each car stepped with in its last step after the input shaping, whoever drove it: `throttle`
  and `brake` 0 to 1, `steering` -1 to 1 (positive to the right), `gear` -1 reverse, 0 neutral, 1 and up forward, and
  `rpm` the engine speed after the step. The player's are those of the sample's tick; an opponent's are read after the
  world's step of that tick.
- `driver` (`crates/nfsmw/src/scenes/world/race/trace_rows.rs`) is what the AI driver asked for in that tick: the
  opponents' drivers, and the autopilot's while it has the player's car (absent before it takes the car). `target_speed`
  is the speed in m/s it is asked to reach (the think's, or the recovery speed while it reverses). `base_skill` is the
  character's skill, `catch_up` the offset the catch-up added this tick, and `skill` the sum in 0..1 that the driver uses
  ([ai-driver-speed-skill.md](specs/ai-driver-speed-skill.md) section 8). `catch_up_force` is the catch-up force budget,
  0 to 0.5, which scales the car's positive engine torque, drag and downforce; the player's car gets neither offset nor
  force (both 0). `stand_in` is true while the racer moves on its off-world stand-in rather than by physics; its
  `controls` are then those of its last physics step.
- The player's vehicle record is the car folder's own `pvehicle`: `--drive BMWM3GTR` races `bmwm3gtr`, and
  `--drive BMWM3GTRE46` races `bmwm3gtre46`, the record the prologue's M3 GTR uses (`CAR:PRESET` adds a preset ride's
  parts, as `BMWM3GTRE46:M3GTRCAREERSTART`). The two records differ in engine, gearing, tires, brakes, chassis and mass,
  and only `bmwm3gtre46` has induction. The header's `world.player_car` is the folder the run drove.

### Without a window and without sound

A run stays off the screen with the benchmark's hidden window: `--bench-seconds <N> --offscreen` keeps the window hidden,
unfocused and off the desktop (`crates/nfsmw/src/app/mod.rs`), and `--no-sound` opens no output device. The benchmark
ends the process after `N` seconds of measuring and prints its frame times, so `N` is the run's time limit: choose it
longer than the race. `--exit-after-race` ends the run first.

```sh
nfsmw view-world --drive BMWM3GTR --race 16.1.1 --race-opponents 3 --race-laps 2 \
  --autopilot --race-trace out/16.1.1.jsonl --exit-after-race \
  --seed 1 --stock-rules --no-sound --bench-seconds 900 --offscreen --max-fps 60
```

What this does not give:

- It needs a graphics adapter: the hidden window still renders. No command runs a single-player world on the null
  renderer (the dedicated server does, and takes no race or autopilot option).
- `--offscreen` is accepted only with `--bench-seconds`; there is no hidden run without the benchmark's time limit and
  its printed report.
- The run follows the wall clock: the ticks of a frame follow the frame time, so a run takes as long as the race and two
  runs do not put the same input on the same tick.
- `play --drive` takes the same options, but opens the front end, whose post-race screen freezes the world; use
  `view-world` for batch runs.
