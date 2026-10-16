# Telemetry channels

Every channel the server could publish, with its kind, sample rate, unit and the source it reads. Companion of
[features](features.md). Status words: **built** (the value exists in code now), **partial**, **not built**.

## 1. Conventions

**Kinds.**

- **S, snapshot**: the latest value, served by GET and included in every stream frame at its rate. Not buffered.
- **T, stream**: a value sampled at a fixed rate on the sim timeline. Subscribers choose a rate up to the channel's
  maximum. The rate is decimated from the 60 Hz sim tick, never raised above it.
- **E, event**: something that happened. Each event has `seq` (increasing, no gaps unless counted as dropped), the
  sim time `t` in seconds, the sim `tick`, a type name and a small payload.

**Rates.** The sim tick is 60 Hz (`FIXED_STEP = 1/60`, see [architecture](../../architecture.md) for the 60 Hz
timeline). A "60" below means the value is taken every sim tick. Slow channels are sampled at 10, 2 or 1 Hz. A
paused game (sim `dt` zero, see `day.rs`) publishes no new samples; the status page says `paused`.

**Units.** Stored and streamed in SI units: metres, metres per second, radians, seconds, newtons, rpm, psi. Derived
display units (km/h, mph, degrees) are for the consumer. A channel never mixes units.

**Wheel index.** The library order: 0 front left, 1 front right, 2 rear left, 3 rear right (`libs/blackbox-vehicle`
lib docs). A channel `wheel.<i>` uses this index. The model's wheel order differs in one place: the drive sim maps
model wheels to library wheels with `PHYSICS_WHEEL` in `crates/nfsmw/src/scenes/world/drive/sim/mod.rs`. Channels
always use the library index and say so.

**Coordinates.** Physics space: x right, y up, z forward, metres. The game world origin differs; the offset to the
world frame must be confirmed before a world position is published (open).

## 2. Vehicle (the driven car)

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `veh.speed` | T 60 | m/s, signed (negative in reverse) | `Vehicle::speed()` and `forward_speed()`; the HUD's `Telemetry::speed_mps` | built |
| `veh.rpm` | T 60 | rpm | `Vehicle::rpm()`; `Telemetry::rpm` | built |
| `veh.gear` | T 60, E on change | -1 reverse, 0 neutral, 1 and up | `Vehicle::gear()`; `Telemetry::gear` | built |
| `veh.shifting` | T 60 | bool | `Telemetry::shifting` | built |
| `veh.shift_up` | T 60 | bool (the shift light) | `Telemetry::shift_up` | built |
| `veh.rpm_limits` | S, on car change | rpm: `idle`, `red_line`, `max_rpm` | `Telemetry::idle`, `red_line`, `max_rpm` | built |
| `veh.nos` | T 10 | 0 to 1 tank | `Vehicle::nos_level()`; `Telemetry::nos` | built |
| `veh.nos_burning` | T 10, E on change | bool | `Vehicle::nos_burning()` | built |
| `veh.has_nos` | S | bool | `Vehicle::has_nos()` | built |
| `veh.boost` | T 10 | psi | `Vehicle::boost_psi()`; `Telemetry::boost_psi` | built |
| `veh.has_induction` | S | bool | `Telemetry::has_induction` | built |
| `veh.wheels_on_ground` | T 10 | count, 0 to 4 | `Vehicle::wheels_on_ground()` | built |
| `veh.slip_angle` | T 60 | rad (display in degrees) | `Vehicle::slip_angle()` | built |
| `veh.position` | T 60 | m, x y z | `Vehicle::position()` | built (world offset open) |
| `veh.orientation` | T 60 | unit quaternion x y z w | `Vehicle::orientation()` | built |
| `veh.linear_velocity` | T 60 | m/s, x y z | `Vehicle::linear_velocity()` | built |
| `veh.angular_velocity` | T 60 | rad/s, x y z | `Vehicle::angular_velocity()` | built |
| `veh.asleep` | S 1 | bool | `Vehicle::is_asleep()` | built |
| `veh.time` | S 1 | s, the vehicle's own clock | `Vehicle::time()` | built |
| `veh.controls` | T 60 | throttle, brake, clutch, steering, gear request (0 to 1 and integers) | `Vehicle::controls()` and `InputState`; fields to confirm in `libs/blackbox-vehicle/src/input/mod.rs` | built |

## 3. Wheels (`wheel.<i>`, i = 0 to 3)

All from `WheelState` (`libs/blackbox-vehicle/src/vehicle/state.rs`), read with `Vehicle::wheel(i)`.

| Channel | Kind, rate | Unit, range | Field | Status |
|---|---|---|---|---|
| `wheel.<i>.on_ground` | T 60 | bool | `on_ground` | built |
| `wheel.<i>.load` | T 60 | N, vertical tyre load | `load` | built |
| `wheel.<i>.compression` | T 60 | m, spring compression | `compression` | built |
| `wheel.<i>.slip` | T 60 | m/s, wheel speed minus ground speed (+ spinning) | `slip` | built |
| `wheel.<i>.lateral_speed` | T 60 | m/s, + right | `lateral_speed` | built |
| `wheel.<i>.slip_angle` | T 60 | rad | `slip_angle` | built |
| `wheel.<i>.lateral_force` | T 60 | N | `lateral_force` | built |
| `wheel.<i>.traction` | T 60 | 0 to 1, 1 is gripping | `traction` | built |
| `wheel.<i>.locked` | T 60, E on change | bool | `locked` | built |
| `wheel.<i>.steer_angle` | T 60 | rad, + right, 0 at the rear | `steer_angle` | built |
| `wheel.<i>.angular_velocity` | T 60 | rad/s, visual rate | `angular_velocity` | built |
| `wheel.<i>.simulated_angular_velocity` | T 60 | rad/s, raw physics rate | `simulated_angular_velocity` | built |
| `wheel.<i>.radius` | S on car change | m | `radius` | built |
| `wheel.<i>.skid`, `wheel.<i>.smoke` | T 60 | per-wheel skid and smoke amounts | the roadmap says `blackbox-vehicle` reports them; field names to confirm | built (names to confirm) |
| `wheel.<i>.temperature` | not built | no tyre temperature model exists | none | not built (F-15) |

Per-wheel values are published as an array in snapshots (`wheel[0..3]`) and as flat names in the CSV export.

## 4. Damage (`dmg.*`)

From `blackbox_damage::Damage` (`libs/blackbox-damage/src/lib.rs`). Zones are the car's impact zones; `MAX_LEVEL` is 3.

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `dmg.zone.<z>.panel` | S 2, E on change | 0 to 3 | `Damage::zone(z).panel` | built |
| `dmg.zone.<z>.paint` | S 2, E on change | 0 to 3 | `Zone::paint` | built |
| `dmg.zone.<z>.glass` | S 2, E on change | 0 to 3, only zones with glass | `Zone::glass` | built |
| `dmg.punctured.<w>` | S 2, E on change | bool | `Damage::is_punctured(w)` | built |
| `dmg.any` | S 2 | bool | `Damage::is_damaged()` | built |
| `dmg.zone_count` | S on car change | count | `Damage::zone_count()` | built |

## 5. Pursuit (`pur.*`)

From `libs/blackbox-pursuit` (pure rules) and the scene's state in `crates/nfsmw/src/scenes/world/ai/pursuit.rs`.

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `pur.active` | S 10, E on start and end | bool | scene `Pursuit` state | built |
| `pur.heat.value` | T 10 | float, base to max | `Heat::value()` | built |
| `pur.heat.level` | T 10, E on change | integer 1 to 10 | `Heat::level()`; the cop wave is picked from it | built |
| `pur.wanted` | T 2 | count of cops in the current wave | `Pursuit::wanted(settings)` | built |
| `pur.busted.timer` | T 10 | s | `Busted::timer()` | built |
| `pur.busted.meter` | T 10 | 0 to 1 (1 once busted) | `Busted::meter(tuning)` | built |
| `pur.busted.is_busted` | E | bool | `Busted::is_busted()` | built |
| `pur.cooldown.progress` | T 10 | 0 to 1 | `Cooldown::progress()` | built |
| `pur.cooldown.level` | T 10 | 0 to 1 (shown meter) | `Cooldown::level()` | built |
| `pur.cooldown.status` | E on change | enum from `CooldownStatus` | `Cooldown::status()` | built |
| `pur.cooldown.remaining` | S 2 | s | `Cooldown::remaining(tuning)` | built |
| `pur.infractions.count` | S 2, E on record | count | `InfractionLog::count()` | built |
| `pur.infractions.fines` | S 2, E on record | game fine units (not converted) | `InfractionLog::total_fines()` | built |
| `pur.end_reason` | E on end | enum from `EndReason` | `libs/blackbox-pursuit/src/end.rs` | built |
| `pur.bounty` | not built | the bounty is not fed to the pursuit | `crates/nfsmw/src/hud/elements.rs` says the bounty row has no feed | not built (F-14) |

## 6. Traffic, police and roadblocks (`ai.*`)

From `crates/nfsmw/src/scenes/world/ai/`. Counts are taken from the spawn census and the car lists the scene keeps;
the accessor names are to confirm in `manager.rs`.

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `ai.traffic.count` | T 2 | count of traffic cars | `traffic.rs`, `manager.rs` | built (accessor to confirm) |
| `ai.traffic.density` | T 2 | the density the spawn rate uses | `manager.rs` `density()` | built |
| `ai.cops.count` | T 2 | count of cop cars | `cop.rs`, `police.rs` | built (accessor to confirm) |
| `ai.roadblock.active` | E on change | bool | `roadblock.rs` | built |
| `ai.spikes.hit` | E | the player's car hit the spikes (to confirm in `spikes.rs`) | `spikes.rs` | built (event to confirm) |
| `ai.heli.state` | S 2 | helicopter flight, sight and height | `police/heli.rs`, `docs/specs/ai-helicopter.md` | built |
| `ai.racers.count` | not built | race AI racers | milestone 7 item (1) | not built (F-13) |

## 7. Races and checkpoints (`race.*`)

Not built. Milestone 7 items (1) race mode and its AI racers, (2) checkpoints (`MMICON_CHECKPOINT`). No channel is
reserved beyond the name prefix. Once they exist: `race.active`, `race.checkpoint.next` (index), `race.position`,
`race.lap` (events on each checkpoint and finish).

## 8. World clock (`clock.*`)

From `DayClock` in `crates/nfsmw/src/scenes/world/day.rs`. The curves are in `nfsmw_data::day_night`.

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `clock.hour` | T 1 | hours, float 0 to 24 | `DayClock::hour()` | built |
| `clock.seconds_per_hour` | S 1 | real seconds per game hour (10 to 3600 when set) | `DayClock` config | built |
| `clock.lights_on` | S 1, E on switch | bool | `DayClock::lights_on()` | built |
| `clock.sun_phase` | S 1 | the sun phase for the hour | `DayClock::sun_phase()` | built |
| `clock.running` | S 1 | bool, false while paused or loading (zero `sim_dt`) | the `sim_dt` the clock receives | built |

The default is 150 real seconds per game hour, starting at noon (the test in `day.rs`).

## 9. Audio (`audio.*`)

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `audio.radio.song` | S 1, E on change | song index, artist, title, album (strings) | `NowPlaying` in `crates/nfsmw/src/audio/radio/state.rs`; `now_playing()` in `glue.rs` | built |
| `audio.radio.elapsed` | T 2 | s | `NowPlaying::elapsed_secs` | built |
| `audio.radio.length` | S on change | s | `NowPlaying` length (the sum of the stored stream lengths) | built |
| `audio.radio.playing` | S 1 | bool | radio state (to confirm) | built (to confirm) |
| `audio.engine.frequency` | T 10, optional | Hz (P2, F-17) | `current_frequency()` in `crates/nfsmw/src/audio/engine.rs` | built |

Strings are published in snapshots and events only, never at the stream rate.

## 10. Camera (`cam.*`)

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `cam.mode` | S 1, E on change | enum `CameraMode` | `crates/nfsmw/src/scenes/world/camera_choice.rs`, `camera_cycle.rs` (`change_camera_mode`) | built |
| `cam.view` | S 1 | name of the original view (seven views) | [camera-views](../camera-views.md) | built (some field meanings unconfirmed) |

## 11. Frame metrics (`perf.*`)

From `Metrics` in `crates/nfsmw/src/devtools/metrics.rs` (the F12 console and the overlay read the same resource).
History holds 240 frames (`HISTORY`); the headline numbers average over 60 frames (`AVERAGE_OVER`).

| Channel | Kind, rate | Unit, range | Source | Status |
|---|---|---|---|---|
| `perf.fps` | T 2 | frames per second | `Metrics::fps()` | built |
| `perf.frame_ms` | T 2 | ms, average over 60 frames | `Metrics::frame_ms` (ring) | built |
| `perf.low_1pct_fps` | T 2 | fps | `Metrics::low_1pct_fps()` | built |
| `perf.worst_ms` | T 2 | ms, max of the history | `Metrics` max over `frame_ms` | built |
| `perf.history` | S on request | last 240 frame times, ms | `Metrics::frame_ms` | built |
| `perf.meshes`, `perf.textures` | S 1 | count | `Metrics::meshes`, `textures` | built |
| `perf.adapter` | S on start, off by default | string, the graphics adapter | `Metrics::adapter` | built (opt-in, see privacy) |
| `perf.telemetry_ms` | T 2 | ms, the server's own game-thread cost | new, F-73 | P1, to build |
| `perf.telemetry_dropped` | S 2 | count of events dropped | new | P1, to build |

The HUD readout string (`Metrics::hud`) is for the screen and is not exported.

## 12. Settings (`settings.*`)

From the layered settings in `crates/nfsmw/src/settings/` (command line, environment, config file, defaults). Only
the keys listed here are exported; paths, the config file location and any credential-like key are never exported.

| Channel | Kind, rate | Source | Status |
|---|---|---|---|
| `settings.show_metrics` | S on change | `show_metrics` (`off`, `basic`, `advanced`) | built |
| `settings.max_fps` | S on change | `MaxFps` in `settings/mod.rs` | built |
| `settings.backend` | S on start | render backend choice | built |
| `settings.volumes` | S on change | `master_volume`, `music_volume`, `sfx_volume`, `engine_volume`, `speech_volume` (0 to 100) | built |

## 13. Mode flags (`mode.*`)

| Channel | Kind, rate | Source | Status |
|---|---|---|---|
| `mode.scene` | S 1, E on change | the active scene name (world, front end, garage) | partial (accessor to confirm in `crates/nfsmw/src/scenes/world/mod.rs`) |
| `mode.drive` | S 1, E on change | the drive is active | partial (same accessor) |
| `mode.freecam` | S 1, E on change | free camera on | built (the `freecam` command toggles it: `commands.rs`) |
| `mode.garage` | S 1, E on change | garage open | partial (`garage` command exists) |
| `mode.paused` | S 1, E on change | pause menu open | partial (pause hook to confirm) |
| `mode.loading` | S 1, E on change | a streamed section or scene is loading | partial (`readiness.rs` to confirm) |

## 14. Events

Each event: `seq`, `t`, `tick`, `type`, payload. The types, and what raises them:

| Event type | Raised by | Payload |
|---|---|---|
| `gear.changed` | `veh.gear` change | from, to |
| `nos.engaged`, `nos.released` | `veh.nos_burning` change | level |
| `car.reset`, `car.repaired` | the `reset` and `repair` commands | source (console, server) |
| `damage.impact` | `Damage::impact` returns true | zone, severity, glazed |
| `damage.puncture` | `Damage::puncture` | wheel |
| `collision` | `notify_collision` | impulse (F-21) |
| `wall.contact` | `notify_wall_contact` | impulse, normal (F-21) |
| `pursuit.started`, `pursuit.ended` | scene `Pursuit` start and stop | heat level, end reason |
| `busted` | `Busted::is_busted` becomes true | timer |
| `evaded` | cool-down reaches EVADED (`Status`) | |
| `heat.changed` | `Heat::level` change | from, to |
| `infraction` | `InfractionLog::record` returns true | infraction, fine |
| `roadblock.spawned` | `roadblock.rs` | |
| `radio.song` | song change | index, title, artist |
| `clock.lights` | `DayClock` lights switch | on or off, hour |
| `camera.changed` | `change_camera_mode` | mode |
| `setting.changed` | a settings write through the console | key, new value (never a path) |
| `command.accepted`, `command.refused` | control writes (control.md) | command id, reason |
| `recording.started`, `recording.stopped` | recorder | file name only (no path) |
| `scene.changed` | `mode.scene` change | from, to |

## 15. Not in the first version

- The car sound's internal controllers (`CarSoundState`): internal state, large, no consumer yet.
- The HUD readout string and any drawn geometry.
- Save-file contents and the save slot names.
- Tyre temperature, races, checkpoints, career, bounty (see section 2 and features F-13 to F-15).
