# Mod API catalogue: core

Part of the [modding catalogue](README.md); decision in [ADR 0005](../decisions/0005-native-mod-plugins.md).
This page: conventions, lifecycle, events, commands, hotkeys and abilities, time of day. The other areas are in
[api-world.md](api-world.md), [api-content.md](api-content.md) and [api-nfsmw.md](api-nfsmw.md).

## How to read an entry

Each table row is `entry | signature sketch | scope | implemented in`.

- **Signature** is the safe Rust SDK form (`blackbox_modding::sdk`), shortened: `Result<T>` stands for
  `Result<T, Error>`, `&mut Ctx` is left out of method signatures, and `Id` types are `Copy` handles. The C
  table behind each is `extern "C"` with `ctx` first and a `BbStatus` return.
- **Scope**: `G` is generic (lives in `libs/blackbox-modding`, same meaning in every Black Box game that offers
  the interface). `MW` is NFS:MW only (`crates/nfsmw-modding`).
- **Implemented in**: the game module behind the facade (`crates/nfsmw/src/...` unless the path starts with
  `libs/` or `crates/nfsmw-data`). `new` means a module that does not exist yet. All of them are reached
  through the facade in `crates/nfsmw/src/modding/`, never directly by the mod.

## Conventions

| Topic | Rule |
|---|---|
| Thread | Main thread, inside a mod callback only (`BB_ERR_NOT_IN_CALLBACK`, `BB_ERR_WRONG_THREAD` otherwise) |
| Strings | UTF-8 `BbStr { ptr, len: u64 }`, valid for the call; names at most 128 bytes, messages 1024, paths 512 |
| Units | Metres, seconds, radians, kilometres per hour only where a name says `kph`. Render space: x forward, y left, z up |
| Vectors | `Vec3 { x, y, z }`, `Vec2 { x, y }`, plain `f32` `#[repr(C)]` structs |
| Colour | `Rgba { r, g, b, a }`, `u8` each |
| Handles | `VehicleId`, `BlipId`, `ZoneId`, `AbilityId`, `ActionId`, `MenuId`, `RowId`, `TimerId`, `SoundId`: 64-bit, generation-checked. A stale handle returns `Stale` |
| Names | Registry keys are `modid:name`; the SDK adds the `modid:` prefix when a call takes a local name |
| Errors | `Error::{Unsupported, Arg, Stale, Phase, NotReady, NotInCallback, WrongThread, Denied, NotFound, Exists, Removed, Panic, Host}` map to negative `BbStatus` values. `Host` carries a message read with `ctx.last_error()` |
| Phases | `Load` (inside `Mod::load`), `Run`, `Unload`. Some calls are valid only in `Load` (content registration); they return `Phase` otherwise |
| Multiplayer | Calls marked `W` (write) return `Denied` in an online session unless mods are enabled for it (ADR section 6) |
| Deprecation | Marked here with **deprecated**; kept two minor releases (ADR section 7) |

## Lifecycle

The `Mod` trait is the entry point. Methods are called by the generated trampolines.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Load | `fn load(ctx: &mut Ctx) -> ModResult<Self>` | G | `modding/runtime.rs` (new): after the manifest is accepted; registries are open |
| Event | `fn event(&mut self, ctx: &mut Ctx, e: &Event)` | G | `modding/events.rs` (new) |
| Unload | `fn unload(&mut self, ctx: &mut Ctx)` | G | `modding/runtime.rs` (new): at exit, or on a dev reload; the host then removes every registration |
| Log | `ctx.log(level: Level, msg: &str)` | G | host table `log` -> `log` crate, target `mod:<id>` |
| Game info | `ctx.game() -> GameInfo { id, version, abi }` | G | `modding/runtime.rs` (new) |
| Mod folder | `ctx.mod_dir() -> &Path` (read only; copied path) | G | `modding/runtime.rs` (new) |
| Frame number | `ctx.frame() -> u64` | G | `app/host.rs` (`Host::frames`) |
| Interface probe | `ctx.has(id: &str, min: (u16, u16)) -> bool` | G | `query_interface` in `modding/runtime.rs` (new) |
| Extension | `ctx.ext::<E: Extension>() -> Option<E>` | G/MW | `query_interface`; `E` is e.g. `nfsmw_modding::sdk::Pursuit` |
| Timer once | `ctx.timers().after(secs: f32, f: impl FnMut(&mut Ctx)) -> Result<TimerId>` | G | `modding/timers.rs` (new), driven in `ModsPre` |
| Timer repeat | `ctx.timers().every(secs: f32, f: ...) -> Result<TimerId>` | G | same |
| Cancel timer | `ctx.timers().cancel(TimerId)` | G | same |
| Mailbox | `Mailbox::<T>::new() -> (Sender<T>, Receiver<T>)`, a mod's worker thread sends, the callback drains | G | SDK only; no host call |

## Events

`bb.events`. A mod subscribes by kind; unsubscribed kinds are never delivered (the host skips the call).
Payload structs start with a `size` field so they can grow. Unknown kinds must be ignored. **Cancelable**
events return `EventResult::Cancel` to stop the action; the first mod to cancel wins, later mods still see the
event with `cancelled = true`. Events are queued by the scene during its update and delivered in `ModsPost`
(frame events in `ModsPre`), in load order.

| Subscribe call | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Subscribe one kind | `ctx.events().subscribe(kind: EventKind)` | G | `modding/events.rs` (new) |
| Subscribe all | `ctx.events().subscribe_all()` | G | same |
| Unsubscribe | `ctx.events().unsubscribe(kind: EventKind)` | G | same |
| Post a mod event | `ctx.events().post(topic: &str, payload: &[u8])` delivers `ModMessage` to every mod that subscribed; payload at most 4 KiB, copied | G | same |

| Event (kind) | Payload | Cancelable | Scope | Raised by |
|---|---|---|---|---|
| `ModLoaded` | `mod_id`, `version` (every other mod is told) | no | G | `modding/runtime.rs` (new) |
| `ModUnloading` | `mod_id` | no | G | same |
| `ModMessage` | `from`, `topic`, `payload` | no | G | same |
| `GameReady` | install indexed, registries sealed | no | G | `app/mod.rs` after start-up |
| `Shutdown` | | no | G | `app/mod.rs` |
| `FrameStart` | `dt_real`, `frame` | no | G | `ModsPre` set |
| `SceneTick` | `dt_sim`, `paused`, `time_scale` | no | G | after `Scene::update`, `ModsPost` |
| `SceneChanged` | `kind: Frontend, World, Viewer, Movie` | no | G | `app/host.rs` `Host::replace_scene` |
| `WorldLoaded` | `world_id`, `streaming_ready` | no | G | `scenes/world/readiness.rs` |
| `WorldUnloading` | `world_id` | no | G | `scenes/world/mod.rs` |
| `PauseChanged` | `paused` | no | G | `Scene::paused` edge in `app/host.rs` |
| `PlayerCarChanged` | `vehicle`, `car_id` | no | G | `scenes/world/drive/`, `car_loader.rs` |
| `VehicleSpawnRequested` | `role`, `car_id`, `position` | **yes** | G | `scenes/world/ai/spawn.rs`, `manager.rs` |
| `VehicleSpawned` | `vehicle`, `role`, `car_id` | no | G | same |
| `VehicleDespawned` | `vehicle`, `role` | no | G | same |
| `VehicleCollision` | `vehicle`, `other: Vehicle, Prop, Wall, None`, `other_id`, `impulse`, `point` | no | G | `scenes/world/events/damage.rs` (`DamageHit`) |
| `VehicleDamaged` | `vehicle`, `zone`, `severity` | no | G | same |
| `NosChanged` | `vehicle`, `active`, `amount` | no | G | `scenes/world/drive/` |
| `ActionPressed`, `ActionReleased` | `action: ActionId` (a mod action or a vanilla name) | no | G | `input/systems.rs`, `ModsPre` |
| `AbilityActivated`, `AbilityReady` | `ability` | no | G | `modding/abilities.rs` (new) |
| `HourChanged` | `hour` (whole hours) | no | G | `scenes/world/day.rs` |
| `LightsChanged` | `on` (headlights switch hour) | no | G | `scenes/world/day.rs` |
| `ZoneEntered`, `ZoneExited` | `zone`, `vehicle` | no | G | `scenes/world/entry_zones.rs` and mod triggers |
| `InteriorEntered`, `InteriorExited` | `interior` | no | G | `scenes/world/entry.rs` |
| `MenuOpened`, `MenuClosed` | `menu` (a screen id or a mod page) | no | G | `frontend/flow.rs` |
| `MenuRowChanged` | `row`, `value` | no | G | `frontend/options.rs` |
| `MenuButton` | `row` (a mod button was activated) | no | G | `frontend/` mod page |
| `SettingChanged` | `key`, `value` | no | G | `settings/mod.rs`, console `set` |
| `CommandRun` | `line` (after the command ran), `ok` | no | G | `devtools/console/exec.rs` |
| `ScreenshotTaken` | `path` | no | G | `app/screenshot.rs` |

NFS:MW events (pursuit, career, races) are in [api-nfsmw.md](api-nfsmw.md#events).

## Commands

`bb.commands`. Mod commands are consulted after the built-in list (`devtools/console/parse.rs` `BUILT_IN`) and
before the scene fallback (`Scene::command`), so a mod cannot shadow a built-in name. They show in `help` and in
tab completion under the mod's id.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register | `commands.register(name: &str, help: &str, f: impl FnMut(&mut Ctx, &Args) -> CmdResult) -> Result<CommandId>` | G | `modding/commands.rs` (new), hooks in `devtools/console/exec.rs` |
| Register with usage | `commands.register_with(CommandDef { name, usage, help, complete: Option<CompleteFn> }, f)` | G | same |
| Unregister | `commands.unregister(CommandId)` | G | same |
| Reply | `ctx.reply(text: &str)` appends a line to the command's console output; an `Err(msg)` result prints in the error colour | G | `devtools/console/view.rs` |
| Run a line | `commands.run(line: &str) -> Result<String>` executes any console line (built-in, scene or mod) and returns its output. **W** for lines that change state | G | `devtools/console/exec.rs` |
| List | `commands.list() -> Vec<CommandInfo { name, help, owner }>` | G | `devtools/console/parse.rs` + `Scene::commands()` |
| Get setting | `settings.get(key: &str) -> Result<String>` (same keys as `get`) | G | `devtools/console/settings_cmd.rs`, `settings_keys.rs` |
| Set setting | `settings.set(key: &str, value: &str) -> Result<()>` **W**; same validation as `set` | G | same |
| Scene command | `commands.run("drive <car>")` reaches `scenes/world/commands.rs` (`drive`, `tp`, `traffic`, `repair`...) | G | `scenes/world/commands.rs` through `Scene::command` |

`Args` is the already-split list of words (`args.len()`, `args.str(i)`, `args.f32(i)`, `args.rest(i)`); parse
errors come back as `Error::Arg` with a usage line. A command runs on the main thread and must return quickly.

## Hotkeys and abilities

`bb.input` gives mod-defined actions and reads of the vanilla ones. `bb.abilities` builds on it. The closed
`input::Action` enum gets one new variant, `Action::Mod(u16)`, indexed by the host's table of mod actions; the
bindings file, the remap screen (a "Mods" group) and `keys` list them like any other action.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register action | `input.register(ActionDef { name, label, kind: Button or Axis, default: &[&str] }) -> Result<ActionId>` | G | `input/action.rs`, `bindings.rs`, `remap.rs` |
| Unregister | `input.unregister(ActionId)` | G | same |
| Is down | `input.down(ActionId) -> bool` | G | `input/state.rs` (`ActionState`) |
| Axis value | `input.axis(ActionId) -> f32` | G | same |
| Pressed this frame | `input.pressed(ActionId) -> bool` (edge); the event `ActionPressed` carries the same | G | same |
| Vanilla action | `input.vanilla(name: &str) -> Result<ActionValue>`: read only, names as in the `keys` list (`throttle`, `brake`, `steer`, `nos`...) | G | `input/describe.rs`, `state.rs` |
| Rebind | `input.bind(ActionId, binding: &str)` **W**; the text is what the `bind` console command takes | G | `input/commands.rs` |
| Binding text | `input.describe(ActionId) -> String` ("F5", "Gamepad X") for prompts | G | `input/describe.rs` |
| Block | `input.consume(InputRef)` with `InputRef` a mod `ActionId` or a vanilla name: hides the press from the game for this frame (a mod that reuses a key) | G | `input/systems.rs` |
| Ability register | `abilities.register(AbilityDef { name, label, action: ActionId, cooldown: f32, duration: f32, meter: Option<MeterStyle> }) -> Result<AbilityId>` | G | `modding/abilities.rs` (new) |
| Activate | `abilities.activate(AbilityId) -> Result<()>` (fails with `NotReady` while cooling down) | G | same |
| State | `abilities.state(AbilityId) -> AbilityState { Ready, Active { left }, Cooling { left } }` | G | same |
| Set cooldown | `abilities.set_cooldown(AbilityId, secs: f32)` | G | same |
| Meter | `abilities.set_meter(AbilityId, fraction: f32)`; draws the HUD bar the style names (re-uses the speedbreaker meter look) | G | `hud/speedbreaker.rs` for the look, `hud/` mod meter (new) |

An ability is a named action with a cooldown and optional HUD meter; the mod receives `AbilityActivated` and
does the effect itself (apply NOS, slow the world with `world.set_time_scale`, spawn a cop). The host keeps
the cooldown so it survives a quarantine of the mod.

## Time of day

`bb.time`. The clock is the world's day-night clock (`Environment { hour, seconds_per_hour }` on the scene,
the curves in `nfsmw-data/src/day_night.rs`). The sky model does not render a full night yet, so a mod that sets
the hour changes lights, sun and shadow, not every sky detail.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Read | `time.get() -> TimeOfDay { hour: f32, seconds_per_hour: f32, frozen: bool, lights_on: bool }` | G | `Scene::environment()` in `viewer/mod.rs`; `scenes/world/day.rs` |
| Set hour | `time.set_hour(hour: f32)` **W**, wraps into 0..24 | G | `Scene::set_environment`, `day.rs` |
| Advance | `time.advance_hours(hours: f32)` **W** | G | `day.rs` (`DayClock::advance`) |
| Day speed | `time.set_day_length(seconds_per_hour: f32)` **W**; `0` freezes | G | `Scene::set_environment` |
| Freeze | `time.freeze(on: bool)` **W**; the hour holds while the world runs | G | `day.rs` (`DayClock::update` `frozen`) |
| Lights hours | `time.set_lights(on_hour: f32, off_hour: f32)` **W** | G | `nfsmw-data/src/day_night.rs` (`DayNightConfig`), `day.rs` |
| Sun phase | `time.sun_phase() -> f32` (0 night, 1 day, between at dawn and dusk) | G | `nfsmw-data/src/time_of_day.rs` (`sun_phase_limit`) |
| Sim clock | `time.sim_seconds() -> f64` seconds since the world started | G | `day.rs` (`DayClock::seconds`) |
| Wall clock | `time.local_hour() -> f32` the player's real local hour, for "match real time" mods | G | `modding/runtime.rs` (new) |

Related: `world.set_time_scale` (slow motion) is in [api-world.md](api-world.md#world-state).
