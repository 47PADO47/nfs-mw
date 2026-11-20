# Driving the remake unattended and reading what it did

Reference for parity work: every way to run the remake without a person at the controls, and every record it can
write. Each entry names the file that defines it. Read from the code on 2026-11-17; paths are relative to the
repository root, and `world/` means `crates/nfsmw/src/scenes/world/`.

The repository holds no scenario file format, no trace comparator and no seed flag. See [Known gaps](#known-gaps).

## Traces the game writes

Three recorders exist. Each is off unless asked for, writes on its own thread through a bounded queue, and drops
records when the queue is full; a drop is counted, never waited for.

| | Pursuit trace | UI trace | Audio trace |
|---|---|---|---|
| Variable | `NFSMW_PURSUIT_TRACE` | `NFSMW_UI_TRACE` | `NFSMW_AUDIO_TRACE` |
| Value | a folder (created if missing) | an absolute path of a new file | a file path |
| Read in | `world/pursuit_trace/mod.rs` | `crates/nfsmw/src/frontend/screens/trace.rs` | `crates/nfsmw/src/audio/mod.rs` |
| Read when | the first world update | the screen host is built | the audio system is built |
| Other ways to start | F5 "Record pursuit trace"; console `extra trace on` and `off` | none | console `audio-trace <path>` and `audio-trace off` |
| Output | `<folder>/<UTC stamp>-remake/` with `samples.jsonl`, `events.jsonl`, `session.json` | the one file | the one file |
| Format | JSON lines (serde); `session.json` is one pretty-printed object | JSON lines (serde) | JSON lines, written by hand in `audio/trace.rs` |
| Schema tag | `"schema": 1` in `session.json` | `"schema": "nfsmw-ui-trace-v1"` in the `header` record | the text `schema=1` inside the `detail` of the `session_start` record |
| Existing file | a new folder per recording | refused: the file must not exist | overwritten |
| Queue | 256 records | 256 records | 8192 lines |
| Size limit | 512 MiB per session | none | 1,000,000 records |

### Pursuit trace

Defined in `world/pursuit_trace/` (`mod.rs`, `records.rs`, `session.rs`, `writer.rs`, `diff.rs`).

- Without the variable, F5 and the console write under `pursuit-traces` in the app's log folder.
- `NFSMW_PURSUIT_TRACE=<folder> nfsmw view-world --drive` starts recording when the world opens.
- `samples.jsonl`: one `sample` record at most every 50 ms (20 Hz, `SAMPLE_HZ` in `session.rs`). The period is
  measured in **wall time**, so the number of physics ticks between two samples varies. A late frame does not make a
  burst. Samples continue while the game is paused; `remake.paused` marks them.
- `events.jsonl`: events found by comparing consecutive 20 Hz snapshots (`diff.rs`), radio lines, and the
  `reference_jump` event. An event carries the stamp of the sample that found it, not of the tick it happened on.
  Unspawn reasons are inferred.
- A sample holds `heat`, `player` (position, velocity, forward, speed) and `cops` (per cop: id, role, position,
  velocity, speed, goal, controls). Fields only the remake has are under `remake`. Vectors are physics space (x right,
  y up, z forward), metres and metres per second.

| Time field | Meaning |
|---|---|
| `time_ms` | Wall time: monotonic milliseconds since the recording started (`Instant`) |
| `wall_time_ms` | Wall time: Unix time in milliseconds |
| `sim_time` | Simulated seconds since the recording started: the sum of each frame's simulated time, so it stops while paused and runs slow under the speedbreaker |

`session.json` is written at the start, again after a reference jump, and at the end with the counters.

| Key | Content |
|---|---|
| `schema`, `source`, `created_utc` | `1`, `"remake"`, ISO 8601 start time |
| `executable` | path of the running binary |
| `remake` | crate version, `git_revision`, `git_dirty`, `debug_build`. The revision is `"unknown"` unless `NFSMW_BUILD_REVISION` was set when the binary was compiled |
| `options` | `game_dir`, `sample_hz`, `max_mb`, `fixture` |
| `settings` | see below |
| `reference` | the bounty spot, pose and heat of an F5 reference jump, once one has landed |
| `schema_notes` | units, id rules and which original fields are null |
| `capture` | records and bytes saved, records dropped by the queue and by the size limit, errors |
| `stop_reason` | why the recording ended |

`settings` records exactly these (`trace_settings` in `mod.rs`): `physics_rate_hz`, `game_speed`,
`time_scale_allowed`, `traffic_cars`, `cop_share`, `traffic_manual`, `traffic_lights`, `traffic_policy`,
`pursuit_options`, `track`, and the F5 `testing` overrides `traffic_off`, `police_off`, `lock_heat`, `heat`, `paused`,
`career_bin`. Nothing else is recorded: see [Known gaps](#known-gaps).

### UI trace

Defined in `crates/nfsmw/src/frontend/screens/trace.rs` and `trace/writer.rs`. Full record list:
[ui-parity-tracing.md](ui-parity-tracing.md).

- The path must be absolute, its folder must exist and be outside the source checkout, and the file must be new.
  A bad path logs an error and leaves tracing off.
- `NFSMW_UI_TRACE=<absolute file> nfsmw play --skip-boot`.
- Record kinds: `header`, `input`, `device`, `stack`, `game_message`, `sound_request`, `package_request`,
  `state_sample`, `end`.
- `state_sample` is taken at most once per 33 ms of **wall time** and omitted when unchanged. `input` is written on
  change and as a one-second wall-time heartbeat. The other kinds are written as they happen.

| Time field | Meaning |
|---|---|
| `elapsed_ns` | Wall time: monotonic nanoseconds since the writer was created |
| `unix_ns` (header only) | Wall time: Unix nanoseconds, as a decimal string |
| `frame` | Count of front-end screen updates since the trace started |
| `data.ui_seconds` | Accumulated frame time: the sum of the front end's steps. A step is the frame delta capped at 0.1 s, or exactly 1/60 s per frame under `--ui-script` |
| `data.frame_dt` | The step of the last update |

The header records `schema`, `unix_ns`, `pid`, `sample_period_ms`, `queue_capacity` and four fixed notes. It records
no settings, no build revision and no install.

### Audio trace

Defined in `crates/nfsmw/src/audio/trace.rs`; started in `audio/mod.rs` and `audio/commands.rs`. Background:
[specs/car-audio-observability.md](specs/car-audio-observability.md).

- `NFSMW_AUDIO_TRACE=<file> nfsmw view-world --drive`. A run with `--no-sound`, and a `--screenshot` run without
  `--radio`, hand the scene no audio (`crates/nfsmw/src/cli/view_args.rs`); expect no audio trace from them.
- Event driven: there is no sample rate. Every line has the same keys: `sequence`, `wall_ms`, `logical_ms`,
  `instance`, `event`, `trigger`, `group`, `bank`, `class`, `params`, `player`, `inputs`, `success`, `detail`.
- The first line is `session_start`. The last line, written when the trace stops, is
  `{"event":"session_end","records":N,"dropped":N,"write_failed":bool}` and has none of the other keys.

| Time field | Meaning |
|---|---|
| `wall_ms` | Wall time: monotonic milliseconds since the trace started |
| `logical_ms` | Accumulated frame time: the sum of the app's frame deltas since the audio system was built (`audio/plugin.rs`), in microsecond steps. It does not start at zero when the trace starts later |

The trace records no settings, no build revision and no install.

### Logs of a scripted drive

`world/drive/trace.rs` writes two log lines; neither is a file format.

| Log | Content | Available |
|---|---|---|
| `log::info!`, every 60 physics steps of a `--drive-script` run | script time in simulated seconds, speed, rpm, gear, position, wheels on ground, inputs | debug builds only: `crates/nfsmw/Cargo.toml` compiles `info` out of release builds |
| `log::trace!` on target `nfsmw::movement`, every frame | frame time, ticks, simulated time, car and camera positions | never: `trace` is compiled out of every build by the same `log` features |

## Test inputs and outputs named by environment variables

These are read only by tests. The "trace" and "replay" variables below name **input** files recorded outside the
repository; the tests ship no data and write no trace.

| Variable | Read in | Direction | File expected or written |
|---|---|---|---|
| `NFSMW_RACER_TRACE` | `world/ai/racer/pace_tests.rs` | input | one JSON object: `event` (track title), optional `catch_up`, `finish_time`, `grid`, `physics_rate_hz`, and `samples` of `[race seconds, position, forward, speed]`; times must increase |
| `NFSMW_RACER_CATCHUP_TRACE` | `world/race/catch_up.rs` | input | a JSON array of rows `[id, time, percent, error, integral, derivative, forced, normalized, skill]`, more than 1000 rows |
| `NFSMW_PURSUIT_TARGET_TRACE` | `world/ai/pursuit/tests/installed_city_catchup.rs` | input | one JSON object, at most 16 MiB: `heat` must be 5, `frames` of `time` (seconds from 0, 30 to 60 s long, gaps at most 0.2 s), `position`, `velocity`, `forward` (x, z), `speed` |
| `NFSMW_JUMP_REPLAY` | `libs/blackbox-vehicle/src/vehicle/tests/jump_replay.rs` | input | text, one row per line of whitespace-separated numbers after a label; `#` starts a comment |
| `NFSMW_CREEP_REPLAY` | `libs/blackbox-vehicle/src/vehicle/tests/settle_replay.rs` | input | text, same row convention; outputs must match exactly |
| `NFSMW_CAPTURE` (with `NFSMW_FROM`, `NFSMW_TO`, `NFSMW_CAR`) | `crates/nfsmw-data/tests/real_install/compare.rs` | input | a CSV capture of the original; the test prints both sides and asserts nothing about them |
| `NFSMW_RACER_DIAGNOSTICS` | `world/ai/racer/installed_tests.rs` | switch | prints the grid to stderr |
| `NFSMW_SCENARIO_REPORT_DIR` | `world/ai/trailer/tests/installed_spawn/scenarios/controlled.rs` | output | a report per truck scenario |
| `NFSMW_BENCH_SERIES` | `crates/nfsmw/src/app/bench.rs` (`--bench-seconds`) | output | CSV `ms_since_start,frame_ms` |
| `NFSMW_BENCH_TRACE` | `world/step_bench_tests.rs` | switch | starts a `blackbox-trace` profiler capture |
| `NFSMW_AUDIO_CAPTURE_DIR`, `NFSMW_AUDIO_EVIDENCE_DIR`, `NFSMW_FXEDIT_EVIDENCE_DIR`, `NFSMW_ENGINE_WAV`, `NFSMW_AEMS_WAV` | tests under `crates/nfsmw/src/audio/` | output | sound evidence from audio tests |
| `NFSMW_UI_CAPTURE_DIR`, `NFSMW_SURFACE_CAPTURE_DIR`, `NFSMW_EFFECTS_CAPTURE_DIR`, `NFSMW_MIRROR_CAPTURE_DIR`, `NFSMW_SHOT_DIR`, `NFSMW_TEST_OUTPUT_DIR`, `NFSMW_LAMP_OUT`, `NFSMW_PERF_OUT` | tests under `crates/nfsmw/src/` | output | pictures and reports from rendering tests |

Run one with, for example,
`NFSMW_GAME_DIR=<install> NFSMW_RACER_TRACE=<file> cargo test -p nfsmw installed_stock_racer_pace -- --ignored --nocapture`.

## Scripted input

Two script languages exist. They are not interchangeable and neither follows a road.

| | `--drive-script` | `--ui-script` |
|---|---|---|
| Commands | `view-world` only, with `--drive` | `play` and `view-screen` |
| Flag declared in | `crates/nfsmw/src/cli/command.rs` (hidden on `view-world` and `play`) | same file |
| Parser | `world/drive/script.rs` | `crates/nfsmw/src/frontend/script.rs` |
| Drives | the lead car's pedals, steering and buttons | the menu pad, keys, pointer, typed text and console commands |
| Time base | simulated time: advanced by each physics tick's simulated delta | frames: every frame is one step of exactly 1/60 s of front-end time |
| Reference | [modding/drive-script.md](modding/drive-script.md) | the module comment of `frontend/script.rs` |

### `--drive-script`

`seconds:key=value,key=value` segments separated by `;`. A segment holds its inputs for that many seconds (0 to
3600). A key without a value is 1. After the last segment the driver lets go of everything.

| Key | Value |
|---|---|
| `throttle`, `brake` | 0 to 1 |
| `steer`, `weight`, `look`, `lookup` | -1 to 1 |
| `handbrake`, `nos`, `speedbreaker` | held when above 0.5 |
| `up`, `down`, `reset`, `pop` | once, on the first tick of the segment |
| `fall` | `low`, `high` or `generic`, once (bikes only) |

- Script time advances by the simulated delta of each tick, so the speedbreaker and `game_speed` stretch a segment in
  wall time, and at `physics_rate_hz = 144` a tick is 1/144 s.
- In an interactive run the ticks follow the frame time (see [Timestep](#timestep-and-randomness)).
- In a `--screenshot` run the world is not ready until the script has finished: the whole script is run in batches of
  six ticks per update before the first capture (`world/simulation.rs`, `world/readiness.rs`), and the car then holds
  its last pose.
- It cannot follow a road, read the car's position, branch, loop or wait for an event. Steering is open loop.
- It drives the lead car only, and only in `view-world`. `play` has no drive script.

### `--ui-script`

Steps separated by `;` or `,`.

| Step | Effect |
|---|---|
| `wait SECONDS` | no input for that long (rounded to frames) |
| `up`, `down`, `left`, `right`, `accept`, `back`, `start`, `button1`, `button2`, `button3`, `ltrigger`, `rtrigger` | pressed for 4 frames, released for 4 |
| `hold NAME SECONDS` | that button held |
| `key NAME` | one key press: a letter, `Space`, `Enter`, `NumpadEnter` or `Escape` |
| `pointer X Y` | the pointer position in FEng units, from then on |
| `type TEXT` | typed text (one word) |
| `console COMMAND` | runs a console command, for example `console story race 16.2.1` |
| `device xbox`, `device keyboard` | the prompt device shown |

- While a script has steps left it replaces the real pad and the front end's clock. The world scene's own update still
  takes the real frame delta (`crates/nfsmw/src/app/render/mod.rs`), so driving that a UI script starts is not on the
  script's clock.
- It has no step that drives a car. `console` is the only way it reaches the world.

### Console commands at start

`--exec "<command>"` (repeatable, every viewer and `play`; `crates/nfsmw/src/cli/view_args.rs`) runs a console command
once the window is up, before a screenshot run's scripted simulation. Commands useful here: `extra trace on`,
`extra testing <key> <value>`, `audio-trace <path>`, `session on <seed>`, `session` (prints the rules and the state
hash).

## Running without a window or a GPU

| What | Window | GPU | Command or entry point |
|---|---|---|---|
| The toy world (`ToyWorld`): kinematic cars on a plane, for the server's tests and bots | no | no | `cargo test -p nfsmw-sim`. `crates/nfsmw-sim` holds only the `WorldSim` seam and this toy, which its own comment says is not a physics model of the game |
| The real world on `NullBackend`, inside tests of the `nfsmw` binary crate | no | no | `NFSMW_GAME_DIR=<install> cargo test -p nfsmw a_free_roam_drive_gives_the_same_state_hash -- --ignored` (`world/determinism_tests.rs`); two hosts in `world/net/tests/installed.rs` |
| The real world on `NullBackend`, as a LAN server | no | no | `nfsmw --dedicated --server-config <file> [--seconds N]` (`crates/nfsmw/src/net/dedicated.rs`, feature `multiplayer`). It takes no drive script; its cars are driven by connected clients. Whether the trace variables take effect there was not checked |
| One or more frames to PNG | hidden | yes | `nfsmw view-world --drive --drive-script "<script>" --screenshot out.png` and the other viewers with `--screenshot` (`crates/nfsmw/src/app/screenshot.rs`) |
| A timed run with the window hidden | hidden | yes | `nfsmw view-world --bench-seconds N --offscreen` |
| Interactive play with scripted input | shown | yes | `nfsmw view-world --drive --drive-script "<script>"`, `nfsmw play --skip-boot --ui-script "<script>"` |

- `NullBackend` (`crates/nfsmw/src/app/null_backend.rs`) is a renderer that draws nothing. `WorldSimHost::open`
  (`world/net/host.rs`) builds a `WorldScene` on it under session rules, with one tick per update.
- No command runs a single-player world with a drive script on `NullBackend`. Outside tests, the only headless entry
  is the dedicated server.
- A `--screenshot` run creates a hidden window and needs a graphics adapter. Capture times (`--screenshot-delay`,
  `--screenshot-interval`) are compared against the wall clock in `Plan` (`app/screenshot.rs`).
- GPU tests of the renderer can use a software adapter: see [testing.md](testing.md#gpu-tests).

## Timestep and randomness

### The step

| Fact | Where |
|---|---|
| The physics step is 1/60 s (`STEP`); `physics_rate_hz = 144` selects 1/144 s | `world/drive/clock.rs` |
| In play, `FixedClock::advance_real` adds the frame's real time to an accumulator and returns one period per whole step in it. The number of ticks in a frame follows the frame time; the remainder is kept for interpolation | `world/drive/clock.rs` |
| The frame time given to the world is the app's frame delta capped at 0.1 s | `crates/nfsmw/src/app/render/mod.rs` |
| Each tick's simulated delta is its period scaled by the speedbreaker dilation, `game_speed`, cinematic moments and the photo shot | `world/simulation.rs` |
| Police, sight, roadblocks and a running race step inside the tick loop | `world/simulation.rs` |
| While the players' surroundings are not resident, no tick runs | `world/simulation.rs` |
| A server's world runs exactly one tick per update (`set_one_per_update`), always at 60 Hz | `world/drive/clock.rs`, `world/net/host.rs` |
| Under session rules the rate is forced to 60 Hz and the scale to 1.0 | `world/simulation.rs`, `world/session_rules.rs` |

The car integration is frame-rate independent ([physics-timing.md](physics-timing.md)). Which tick an input lands on,
and what has streamed in by then, still depend on frame timing in play.

### Random numbers and seeds

| Source | Seed | Where |
|---|---|---|
| Traffic, population, police, roadblock and air streams | fixed `WORLD_SEED` in single player; the session seed under session rules | `world/ai/streams.rs`, `world/session_rules.rs` |
| Race opponent names | the clock (`SessionRules::default` has `seed: None`, so `name_seed` reads `SystemTime`); the session seed under session rules | `world/session_rules.rs` |
| Chase radio line choice | the clock (`Rng::from_clock`) | `crates/nfsmw/src/audio/mod.rs`, `audio/chase/radio/mod.rs` |
| Police speech files and events | the clock | `crates/nfsmw/src/audio/speech/glue.rs`, `audio/speech/random.rs` |
| Radio playlist order | the clock | `crates/nfsmw/src/audio/radio/mod.rs`, `audio/radio/playlist.rs` |
| Quick play picks, map markers, loading tips | the clock | `crates/nfsmw/src/frontend/quick_play.rs`, `frontend/career_screens/markers.rs`, `frontend/loading_tips.rs` |
| A server's session seed | `ServeOptions::seed`, else the clock | `crates/nfsmw/src/net/options.rs` |

`session on <seed>` in the console (`world/session_rules.rs`) sets a seed, but it sets every session rule with it: no
speedbreaker or time scaling, each player's own chase camera as the view for gameplay rules, blocking model loads,
lockstep streaming and no race screens. It is not a seed for single-player rules.

### Wall time that decides gameplay

| What | Rule | Where |
|---|---|---|
| A strict placement (a race start) waiting for ground | gives up after 60 s of wall time and becomes an ordinary road search | `world/drive/placement/strict.rs` |
| A car waiting for its surroundings to load | placed on the nearest road after 10 s of wall time; 600 updates under lockstep streaming | `world/drive_input.rs` |
| Traffic model availability | depends on a loader thread unless `blocking_models` is set | `world/session_rules.rs`, [specs/multiplayer/determinism.md](specs/multiplayer/determinism.md) |

## Settings that change mechanics relative to the original

No single switch selects stock behaviour. Each row must be set or checked on its own. "Original" is the value the
code or the named doc states; "not stated" means neither says.

Settings resolve as command line over environment over `config.toml` over `NFSMWExtraOptionsSettings.ini` over
defaults (`crates/nfsmw/src/settings/load.rs`). The ini files sit next to `config.toml`; `nfsmw check-install` prints
that path. `BLACKBOX_PROFILE_DIR` moves the folder.

| Setting | Defined in (`crates/nfsmw/src/settings/`) | Default | Original |
|---|---|---|---|
| `traffic_lights` (`NFSMW_TRAFFIC_LIGHTS`) | `defaults.rs` | `true` | `false`: the original has no traffic lights |
| `traffic` (`NFSMW_TRAFFIC`), the active vehicle budget | `defaults.rs` | 20 | 20, described as the installed PC reference budget |
| `cop_share` (`NFSMW_COP_SHARE`), percent of that budget that are patrol cops | `defaults.rs` | 5 | not stated |
| `[traffic_policy]`: spawn range, retire distance, off-screen seconds, density, `keep_on_minimap` | `traffic_policy.rs` | Reference preset: 300 to 400 m, 650 m, 4 s, 100 %, `keep_on_minimap = false` | `keep_on_minimap = false` is stated as the original rule; the other numbers are named "Reference" ([specs/traffic-presets.md](specs/traffic-presets.md)) |
| `[pursuit_options] bounty_fixes` | `pursuit_options.rs` | `true` | `false` restores the original's bounty bugs |
| `[pursuit_options] enhanced`, `max_active_cops`, `heat_cap`, `wave_percent` | `pursuit_options.rs` | `false`, 16, 0, 100 | `enhanced = false` keeps the reference limits; the other three apply only when it is on |
| `physics_rate_hz` (`NFSMW_PHYSICS_RATE_HZ`, `--physics-rate-hz`) | `physics_rate.rs` | 60 | 60, labelled "60 Hz (original)"; 144 is a comparison mode |
| `game_speed` (ini `[Gameplay] GameSpeed`) | `defaults.rs`, `extra_options/mod.rs` | 1.0 | 1.0 |
| `animation_speed` (ini `WorldAnimationSpeed`) | `defaults.rs` | 1.0 | 1.0 |
| `no_rev_limiter` (ini `NoRevLimiter`) | `defaults.rs` | `false` | `false` |
| `always_rain`, `rain_amount` (ini `[Weather]`) | `defaults.rs` | `false`, 100 | 100 is the original amount; `always_rain` not stated |
| `transmission` (`NFSMW_TRANSMISSION`) | `transmission.rs` | automatic | automatic is the original's starting value; it is a player setting there too |
| `[game_moments]` camera and its four kinds | `game_moments.rs` | all on | the original has the one switch, on by default; the kinds are the remake's |
| `NFSMWSpeedFixerSettings.ini` (speedbreaker speeds, drain, recharge, dilation, mass, grip) | `speedbreaker.rs` | file absent: vanilla values | file absent |
| `NFSMWExtraOptionsSettings.ini` `[Pursuit] HeatLevelOverride`, `MinimumHeatLevel`, `MaximumHeatLevel`, `SpeedingLimit`, `RecklessDrivingLimit`; `[Gameplay] SBRechargeTime`, `SBRechargeSpeedLimit`, `SBMassMultiplier` | `extra_options/mod.rs`, `extra_options/pursuit.rs`, `extra_options/breaker.rs` | file absent: unset | unset. With `SpeedingLimit` unset the remake uses its own threshold, and no Reckless Driving is recorded |
| `NFSMWBartenderSettings.ini` (cop cap, per-heat cop count, heat time, bust speed, escape heat share, fines, busted timer) | `bartender/mod.rs`, `bartender/vanilla.rs` | file absent: vanilla values | file absent. `MaxCops = 8` is the remake's cap; the stock limit is cited from mod notes, not verified here |
| `NFSMWFairBustsSettings.ini` (reset penalty of the busted timer) | `fairbusts.rs` | file absent: disabled | disabled: 90 m reach, no speed gate, four times the fill |
| `NFSMWHelicopterSettings.ini` (fuel, respawn, sight, speeds per heat and race level) | `helicopter.rs` | file absent: vanilla values | file absent. The despawn distance of 600 m is not checked against the install |
| `eatrax_jukebox.toml` (which songs play, order) | `eatrax.rs` | file absent: each song's own default | file absent. It changes music only |
| `mods` (`NFSMW_MODS`): native mods can change cars, traffic and rules | `mods.rs` | `all` | `off` loads none |
| Steering, trigger and camera deadzones and sensitivity; wheel options | `controls.rs`, `wheel.rs` | see file | not stated |
| F5 testing overrides: `traffic_off`, `police_off`, `heat`, `lock_heat`, `career_bin`, `unlimited_nos`, `unlimited_breaker`, pause; weather and time | `crates/nfsmw/src/extra_options/view/testing.rs`, [extra-options.md](extra-options.md) | all off; session only, never saved as active | all off |
| Session rules (`session on`, multiplayer) | `world/session_rules.rs` | single-player rules | single-player rules |

`turn_signals`, `cop_health_bars`, `car_scale`, `show_hud_on_look_back` and the graphics settings are documented in the
code as visual. This page did not verify that none of them reaches the simulation.

## Front-end screen dump

`NFSMW_FE_DUMP=<file>` writes what the front end hands the presenter for each screen as JSON lines, once the screen has
settled (`crates/nfsmw/src/parity/fe_dump.rs`, rows in `crates/nfsmw/src/frontend/screens/dump_rows.rs`). It works on any
command that shows FEng screens (`play`, `view-screen`). `NFSMW_FE_DUMP_SCREENS=SMS_Message,InGamePhotoMaster` limits it
to some screens. A screen has settled when it has been drawn for 1.5 s without a break and none of its objects plays a
play-once script that has not ended; a screen still animating after 2.5 s gets its first snapshot then anyway, with
`settled: false`. A screen gets up to three snapshots 2 s apart. Each line (`schema`
`fe-screen-dump-v1`) names the screen, the screens drawn in the same frame and the build, and holds one row per package
object: `name_hash`, `guid`, `type` (the package file's type number), `parent`, `flags`, `hidden`, `colour` (RGBA),
`pos`, `pivot`, `rot`, `size`, `uv`, `texture`, `label`, `text`, `script` (`id`, `time` in runtime ticks, `length`,
`mode`), `visible`, `drawn` (in the presented tree's draw order), `in_tree` and `world` (the tree's screen point), and for a
node in the tree `world_colour`, `z` and a coloured image's `corners`. `frame_drawn` gives, per package of the frame, the
number of objects it drew (0 for a package walked but drawing nothing, as the original's HUD under an in-game screen);
the comparer counts a package in the frame only when it drew something. A screen covered by another screen (the inbox
under a message) starts a new set of snapshots. Nodes
the host added to the tree (footer hints) follow with `host_added: true`. An object the presented tree does not hold has
`in_tree: false`. Nothing drawn outside the package trees (egui panels, the 3D backdrop) is in the dump.

```sh
NFSMW_FE_DUMP=out/fe_dump.jsonl nfsmw --game-dir "<install>" play --new-career CAREERRUN --career-start first-car-lot \
  --skip-cinematics --autopilot --exit-after-first-visit \
  --ui-script "loop; wait 6; accept; wait 1; left; wait 0.5; accept; wait 6; right; wait 0.5; right; wait 6; back" \
  --seed 1 --stock-rules --silent-audio --backend vulkan --bench-seconds 900 --offscreen --max-fps 60
```

The scripted pad must leave a screen up for longer than the settle time: the shorter loop of "Unattended career run" ([testing.md](testing.md))
closes the drive's message after less than a second.

## Known gaps

- **No seed flag.** No command-line option or environment variable seeds a single-player run. Opponent names, chase
  radio lines, police speech and the radio playlist are seeded from the clock. `session on <seed>` changes the rules
  as well as the seed.
- **No comparator and no scenario format.** The repository has no tool that compares a remake trace with an
  original-game trace, and no scenario data file. The tests in the table above compare against private input files
  with their own formats.
- **Three time bases.** The pursuit trace stamps wall time and simulated seconds; the UI trace stamps wall time,
  a front-end update count and accumulated front-end time; the audio trace stamps wall time and accumulated app frame
  time. No trace records a physics tick number, and no two share a zero point.
- **Sampling follows wall time.** The pursuit trace samples every 50 ms of wall time and the UI trace every 33 ms,
  so two runs of the same script do not sample the same ticks.
- **Headers omit the mod-style ini settings.** `session.json` does not record the Bartender, FairBusts, Helicopter
  or SpeedFixer files, the Extra Options pursuit and speedbreaker keys, the session seed and rules other than
  `time_scale_allowed`, the loaded mods, the car, or the transmission. The UI and audio traces record no settings
  at all.
- **The build revision is usually unknown.** `git_revision` is filled only when `NFSMW_BUILD_REVISION` is set at
  compile time.
- **No headless single-player run.** A scripted single-player drive needs a window (hidden or shown) and a GPU.
- **Scripts are open loop.** `--drive-script` cannot follow a road or react to the world, and it exists only on
  `view-world`.
- **Play is not deterministic.** Ticks per frame, streaming and model loading follow this machine's timing unless
  session rules are on. The determinism test under session rules has itself failed on a first run after a build
  ([specs/multiplayer/determinism.md](specs/multiplayer/determinism.md), step 5).
- **Screenshot timing.** [modding/drive-script.md](modding/drive-script.md) describes `--screenshot-delay` as
  simulated time; `Plan` in `app/screenshot.rs` counts wall time after the script has been batched. This page did not
  resolve the difference.
- **Scripted-drive log lines are missing from release builds**, because `info` logging is compiled out there.
