# Mod API catalogue: map, menus, game interaction

Part of the [modding catalogue](README.md). Conventions and the meaning of the columns are in
[api.md](api.md#how-to-read-an-entry). **W** marks a write that is denied in an online session.

## Map blips and the world map

`bb.map`. Today a `MapBlip` names a registered style (`hud/blip_style.rs`) and the scene fills the list
(`WorldScene::map_blips`, `scenes/world/hud.rs`). The style registry becomes open at run time; the mod
registers styles, then places blips by style name. A blip whose style is not registered is not drawn, as now.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register style | `map.register_style(StyleDef { name, icon: &str, colour: Option<Rgba>, visibility: Always or InZone, blink: Option<f32>, spin: Option<f32>, on_world_map: bool }) -> Result<StyleId>` | G | `hud/blip_style.rs`, `hud/blip_styles.rs`; registered in `Load` only |
| Style look callback | `map.set_look_fn(StyleId, f: impl FnMut(&mut Ctx, &LookInput) -> BlipLook)` for per-tick colour, alpha, spin (costs a call per blip per tick; prefer the declarative fields) | G | `hud/blip_style.rs` (`BlipLook`) |
| Icon source | `icon` names an object of a loaded HUD package, or a texture in the mod's `files/` overlay (a `.tpk` entry) | G | `hud/skin.rs`, `libs/blackbox-tpk` |
| Add blip | `map.add(style: StyleId, pos: Vec2) -> Result<BlipId>` | G | `hud/blips.rs`, `scenes/world/hud.rs` (extra blips next to `map_blips`) |
| Add blip on a vehicle | `map.attach(style, VehicleId) -> Result<BlipId>`: follows the vehicle, removed when it despawns | G | `hud/blips.rs`, `hud/cop_blip.rs` for the heading and flash |
| Move | `map.set_position(BlipId, pos: Vec2, heading: Option<Vec2>)` | G | `hud/blips.rs` |
| State | `map.set_active(BlipId, active: bool)` (the style reads it, the cop style flashes) | G | `hud/blip_style.rs` (`MapBlip::active`) |
| Remove | `map.remove(BlipId)` | G | `hud/blips.rs` |
| World-map marker | `map.add_marker(MarkerDef { style, pos, label: &str, on_select: Option<CallbackId> }) -> Result<MarkerId>`: a pin on the full-screen map with a name and an optional action | G | `frontend/world_map/` (`overlay.rs`, `pick.rs`) |
| Remove marker | `map.remove_marker(MarkerId)` | G | same |
| Waypoint | `map.set_waypoint(pos: Vec2)`, `map.clear_waypoint()`: the GPS route target | G | `frontend/world_map/gps.rs` |
| Query | `map.blips() -> Vec<BlipInfo>` (the mod's own) | G | `hud/blips.rs` |
| Minimap view | `map.view() -> MapView { centre, radius, rotation }` read only, for placing UI | G | `hud/minimap.rs` |

Coordinates are the map's own (render space metres), the same numbers as the `tp` command.

## Menus

**Delivered (`bb.menus` 1.0): rows on a Mods page.** A mod registers rows (`register`) and the game lists them on
the Options page under "Mods". The page's icon appears only while at least one mod has a row, on the main menu's
Options and on the pause menu's. A row is a toggle (on or off), a slider (from `min` to `max` in steps of `step`,
a step of 0 meaning a tenth of the range), a list (one of its texts), or a button (pressed, calls back with 0).
A player's change goes to the row, is saved, and calls the mod's callback with the new value. A mod reads and sets
a row's value with `get` and `set` (`set` saves but does not call back). Values are saved per mod in the config file
under `[mod_values."<id>"]`, one key per row name. The table is not `[mods.<id>]`: that is the mod's own configuration ([api-config.md](api-config.md)), which the mod writes, while a row value is saved state the player changed.
Rows the mod does not remove are removed when it unloads. The SDK wrapper is `ctx.menus()`: `toggle`, `slider`,
`list` and `button` take a Rust closure for the row's changes, and `get`, `set` and `unregister` take the row's id.
Not yet: pages of their own, read-only text rows, sub-pages, and disabling or hiding a row.

The rest of this section is the plan for the later deliveries. The front end is hard-coded FEng screens
(`frontend/factory.rs`, `screens.rs`, `options_rows.rs`). Those deliveries give mods **rows and pages the host draws
in the game's style**, not raw FEng access:

1. extra rows in the existing option categories (`Category::{Audio, Video, ...}` in `frontend/logic.rs`) and a
   new "Mods" category where a mod's rows live by default;
2. a **mod page**: a list of rows (toggle, choice, slider, button, read-only text, sub-page) shown over the
   pause or main menu; and
3. a **prompt**: message and confirm dialogs (`frontend/prompts.rs`, `confirm.rs`).

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Add option row | `menus.add_row(RowDef { category: Category or ModsPage, id, label: &str, kind: Toggle or Choice(&[&str]) or Slider { min, max, step } or Button, value }) -> Result<RowId>` | G | `frontend/options_rows.rs` (`rows`, `Row`), `options.rs`, `options_steps.rs` |
| Row callback | `menus.on_change(RowId, f: impl FnMut(&mut Ctx, RowValue))` | G | `frontend/options.rs`; also raises `MenuRowChanged` |
| Set row value | `menus.set_value(RowId, RowValue)` | G | same |
| Enable and show | `menus.set_enabled(RowId, bool)`, `menus.set_visible(RowId, bool)` | G | `frontend/disabled_rows_tests.rs` shows the dimming rule |
| Remove row | `menus.remove(RowId)` | G | `options_rows.rs` |
| Create page | `menus.page(title: &str) -> Result<MenuId>` | G | `frontend/widget_menu.rs`, `widgets.rs` |
| Page row | `menus.page_add(MenuId, RowDef)` | G | same |
| Open or close | `menus.open(MenuId)`, `menus.close(MenuId)`; a page opens over the current screen and Back returns | G | `frontend/flow.rs`, `plugin.rs` |
| Entry in a menu | `menus.add_entry(Where { screen: Pause or Main or Safehouse or Shop, label }, target: MenuId or CallbackId) -> Result<RowId>` adds an item to an existing list | G | `frontend/icon_menu.rs`, `icon_items.rs`, `screens.rs` |
| Message | `menus.message(title: &str, text: &str)` | G | `frontend/prompts.rs` |
| Confirm | `menus.confirm(title, text, f: impl FnOnce(&mut Ctx, bool))` | G | `frontend/confirm.rs` |
| Text input | `menus.ask_text(title, initial, f: impl FnOnce(&mut Ctx, Option<&str>))` | G | `frontend/prompts.rs` (needs a text field; new) |
| Translate | **Built** as `bb.i18n` 1.0: `i18n.t(id)` / `i18n.t_args(id, args)` format the mod's `lang/<locale>/*.ftl` messages; a row label or choice that is a message id is shown translated ([translations.md](translations.md)) | G | `modding/backends/i18n.rs`, `crate::i18n` |
| Replace FEng package | `menus.override_package(name: &str)`: use a package from the overlay instead of the install's. **Reserved** for a later minor; today the VFS overlay can already replace `*.fng` files wholesale | G | `libs/blackbox-feng`, VFS overlay |

Mod page and row labels are plain UTF-8 drawn with the game's font (`libs/blackbox-text`), or a message id of the mod's
catalogue ([translations.md](translations.md)); characters the font lacks are drawn in the language's fallback font.

## Game interaction

Status of the five interfaces (ids `bb.vehicles` (1.2), `bb.hud`, `bb.camera` (1.1), `bb.audio`, `bb.world`, all 1.0 otherwise). The rows below
say what each delivers; the rest of each table is still the plan. A call the game cannot serve answers
`BB_ERR_UNSUPPORTED`, and a call that needs a world or a car answers `BB_ERR_NOT_READY` while none is loaded.
The wire layouts are in [api.md](api.md#wire-numbers-abi-01).

### Vehicles and the player

`bb.vehicles`. A vehicle handle can be the player's car, a traffic car, a cop or a racer.

**Delivered (1.0):** the player's car, and a traffic spawn at a point (below). `state` reads a snapshot taken at the start of the frame (position, velocity,
heading, speed, rpm, gear, nitrous, damage), `teleport` queues a move to the road nearest the horizontal position (the
height is not used), and `repair` queues a repair. Both queued calls run after the mods, like the other world changes.
**Delivered (1.1): the car's dynamics and `add_nos`.** `state` also reads the body slip angle (`slip_angle`, radians, positive when
the car moves to the left of its heading, 0 below 1 m/s), the steering (`steering`, -1 to 1, positive to the left), the
strongest hit of the latest physics step against the world or an object (`impact`, 0 to 1, a closing speed of 20 m/s or more
is 1; the ground does not count, and the same hit is reported until the physics steps again), and the ground contact
(`on_ground`, at least one wheel down). `add_nos(amount)` adds a part of a full tank (0 to 1, finite, else `BB_ERR_ARG`) to the
player's nitrous, up to a full tank. It is queued like `repair`, so the tank has it after the mods ran; a car with no nitrous
system ignores it (the HUD's `nos` reads 0 for such a car); no car answers `BB_ERR_NOT_READY`. The state is size-prefixed
and `add_nos` is appended to the table, so a mod built for 1.0 reads the 1.0 fields and never sees the table grow; a mod that
wants the new calls asks for 1.1 (`"bb.vehicles" = "1.1"` in `[optional.interfaces]`). The host implements it in
`scenes/world/drive/mod_feed.rs` (hit, tank), `scenes/world/mod_hooks.rs` (the state) and `modding/backends/vehicles.rs`; the
the 2012 drift nitrous ([chameleon-drift.md](../formats/chameleon-drift.md)) is its first user.
**Delivered (1.2): the drift assist.** `set_drift_assist(enabled)` turns a physics layer of the player's car on or off. It is
queued like `repair` and stays as set: it follows a change of car only if the mod sends it again (the 2012 mod sends it on every
scene tick: `true` while the player drives one of its own cars, `false` in any other; it tells the car from `bb.engine` `car()`,
so `bb.vehicles` needed no identity field), and a car that is replaced starts with it off. The call answers `BB_ERR_NOT_READY` with no
car and `BB_ERR_UNSUPPORTED` on a host older than 1.2 (the SDK maps the missing table field to `Unsupported`). With the assist
off the physics is bit for bit what it was. With it on (`libs/blackbox-vehicle`, `Vehicle::set_drift_assist`):
**entry**, the handbrake pulled with the wheel turned (more than 0.15) at the car's drift speed (10 m/s) pushes the nose round
with a yaw torque in the steering direction for up to 0.45 s; **hold**, once the slip angle has reached the car's drift angle
(the slide counts as a drift until it stays under the exit angle for 0.25 s) a yaw torque, a damped spring on the slip angle,
holds the angle the steering asks for (full lock asks half of the car's largest drift angle, 20 degrees for 40; no steering
asks for none, so letting go straightens the car out), strongest with the throttle down (0.4 of it with the throttle up);
**grip**, the rear tyres keep 0.75 of their friction at the full drift angle. The strength is a yaw acceleration (6 rad/s^2
generic, or the car's `drift_yaw_torque_nm` over the body's inertia, kept between 3 and 10) times the body's inertia, so it holds any
car. The car's `[stats]` keys `drift_enter_speed_ms`, `drift_enter_angle_deg`, `drift_exit_angle_deg`, `drift_max_angle_deg` and
`drift_yaw_torque_nm` ([car-stats.md](car-stats.md#the-drift-keys)) tune it; a car without them (every 2005 car) has the generic
numbers. The model is the rewrite's own, built from what the 2012 data names ([chameleon-drift.md](../formats/chameleon-drift.md#the-sliding-physics)).
The host implements it in `scenes/world/drive/sim/mod_feed.rs` (`CarSim::set_drift_assist`), `scenes/world/mod_hooks.rs`
(`set_player_drift_assist_to`) and `modding/backends/vehicles.rs` (`WorldRequest::DriftAssist`).
**Delivered (1.0, `spawn_traffic`):** a traffic car at a point. The call answers a handle and queues the spawn; the scene
places it in the same frame, after the mods have run (`WorldRequest::SpawnTraffic`, `Scene::spawn_traffic_at`, and
`TrafficWorld::spawn_traffic_at`). The car is a random loaded traffic model (the traffic cars that mods add are among them, and `bb.traffic` places one by name: [api-traffic.md](api-traffic.md)) on the nearest lane that can take a car, with
the spacing of the manager from the other cars. A point within 150 m of the player is placed and logged, since a mod asked
for it (the manager's own cars stay out of view). The heading is in radians, as the car's `heading` reads. The position's height
is not used. A call with no world loaded answers `BB_ERR_NOT_READY`; a spawn the road or the traffic models refuse is
logged, not returned, and the handle does not yet name a car (no tracking, so no despawn by handle). No traffic model is
loaded until the manager asks for one, so an early spawn can be refused. The traffic density (`set_traffic`) is in `bb.world`, below.

**Open:** handles for other vehicles (traffic, cops, racers) and every call on them, `despawn`, `set_player_car`, the
AI goals, `near`, `set_velocity`, `set_nos`, `set_tuning`, `damage`, lights, horn and siren.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Player vehicle | `vehicles.player() -> Option<VehicleId>` | G | `scenes/world/drive/` |
| State | `vehicles.state(VehicleId) -> Result<VehicleState { pos: Vec3, heading: f32, velocity: Vec3, speed_kph: f32, rpm: f32, gear: i8, nos: f32, on_ground: bool, damage: f32, role: Role, car_id }>` | G | `scenes/world/drive/`, `ai/car.rs`; HUD numbers from `hud/state.rs` |
| Teleport | `vehicles.teleport(VehicleId, pos: Vec3, heading: f32)` **W**; the player is put on the nearest road like `reset` | G | `scenes/world/commands.rs` (`tp`, `reset`) |
| Set velocity | `vehicles.set_velocity(VehicleId, v: Vec3)` **W** | G | `libs/blackbox-vehicle` state via the drive module |
| Add NOS | `vehicles.add_nos(amount: f32)` (delivered in 1.1: the player's car only, `Vehicles::add_nos` in the SDK), `set_nos`, `set_infinite_nos(bool)` **W** | G | `scenes/world/drive/mod_feed.rs`, `libs/blackbox-vehicle` (`Vehicle::add_nitrous`) |
| Drift assist | `vehicles.set_drift_assist(on: bool)` (delivered in 1.2: the player's car, `Vehicles::set_drift_assist` in the SDK) **W** | G | `scenes/world/drive/mod_feed.rs`, `libs/blackbox-vehicle` (`Vehicle::set_drift_assist`, `vehicle/drift_assist.rs`) |
| Repair | `vehicles.repair(VehicleId)`, `vehicles.damage(VehicleId, zone: u8, severity: u8)` **W** | G | `commands.rs` (`repair`, `damage`), `libs/blackbox-damage` |
| Tune physics | `vehicles.set_tuning(VehicleId, Tuning { grip_scale, power_scale, mass_scale, top_speed_scale })` **W**, scaling the car's real values, reset by `None` | G | `libs/blackbox-vehicle`, `nfsmw-data/src/car/physics.rs` |
| Spawn | `vehicles.spawn(SpawnDesc { car: CarId, role: Player or Traffic or Cop or Racer, pos: Vec3, heading: f32, ai: Default or Idle or Chase(VehicleId), colour: Option<Rgba> }) -> Result<VehicleId>` **W** | G | `scenes/world/ai/spawn.rs`, `ai/manager.rs`, `ai/car.rs`; player role via `drive` |
| Despawn | `vehicles.despawn(VehicleId)` **W**; the player cannot be despawned | G | `ai/manager.rs` |
| Switch player car | `vehicles.set_player_car(CarId)` **W**, the `drive <car>` path (a car of the install or from `bb.content`) | G | `scenes/world/commands.rs` (`drive`), `car_loader.rs`, `garage/` |
| Nearby | `vehicles.near(pos: Vec3, radius: f32, out: &mut Vec<VehicleId>)` | G | `scenes/world/ai/population.rs` |
| Traffic density | `vehicles.set_traffic(count: u32, cop_share: f32)` **W**; the same fields as the traffic settings | G | `Scene::set_traffic`, `settings` `TrafficSettings` |
| Set AI goal | `vehicles.set_ai(VehicleId, AiGoal)` **W** (Idle, Follow(VehicleId), Chase(VehicleId), Flee, GoTo(Vec3)) | G | `scenes/world/ai/` (`pursuit.rs`, `stop.rs`) |
| Lights and horn | `vehicles.set_lights(VehicleId, headlights: bool)`; `vehicles.honk(VehicleId)` | G | `scenes/world/car_lights.rs`, `cop_lights.rs` |
| Cop lights | `vehicles.set_siren(VehicleId, on: bool)` | G | `ai/cop_lights.rs` |

### HUD

`bb.hud`.

**Delivered (1.0):** `message` shows one centred line for the given seconds (plain, warning or big) and replaces the
line on screen; `hide` takes it down. The line is drawn by the world's card (`hud/message.rs`), so it shows while a
world is up. **Open:** the interact prompt, meters, hiding HUD parts, screen text and the HUD size.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Message | `hud.message(text: &str, style: MessageStyle { Info, Warning, Big }, secs: f32)` | G | `hud/elements.rs`, `hud/state.rs` (`HudState`) |
| Interact prompt | `hud.prompt(text: &str) -> PromptId`, `hud.clear_prompt(PromptId)` (the "press X" text) | G | `hud/interact_prompt.rs` |
| Meter | `hud.meter(label: &str, style) -> Result<MeterId>`, `set_fraction(MeterId, f32)`, `remove(MeterId)` | G | `hud/` mod meter (new); look from `hud/speedbreaker.rs` |
| Show or hide HUD | `hud.set_visible(HudPart, bool)` (all, speedometer, minimap, pursuit bar) **W** | G | `hud/viewport.rs`, `settings/hud_layout.rs` |
| Read HUD state | `hud.speed_display() -> u32`, `hud.rpm_fraction() -> f32` | G | `hud/state.rs` (`display_speed`, `rpm_fraction`) |
| Screen text | `hud.text(TextDef { pos: Vec2, text, size, colour, secs }) -> TextId`, `remove` | G | `libs/blackbox-text`, `hud/` mod layer (new) |
| Screen size | `hud.size() -> Vec2` the HUD canvas in pixels | G | `hud/viewport.rs` |

### Audio

`bb.audio`. Sounds come from the mod's overlay in formats `libs/ea-audio` decodes, or from the install by
the sound's name.

**Delivered (1.0):** `play(bank, event, volume)` plays one sound on the sound-effects group, by bank and index. The game
has no lookup from an event name to a sound (the sound viewer also plays by bank and index), so the sound is named by its
index: `event` is the index as text (`"3"`), or `bank` is `"FE_MB.abk:3"` with `event` empty. A bank name with no dot is
that bank (`IG_GLOBAL` is `IG_GLOBAL.abk`); a bank name without a folder is looked up in the sound folders in the order
`ENGINE`, `FE`, `GLOBAL`, `IG_GLOBAL`, `NOS`, `SHIFTING`, `SKIDS`, `TURBO`. The call answers an id, queued for the audio
plugin (`audio/oneshot.rs`), which plays it in the frame it takes it. A name that is not an index, a bank that is not a `.abk`
file, or a volume that is not zero or more answers `BB_ERR_ARG`. A sound the install does not have is logged, not returned,
and there is no way to stop a one-shot yet. **Open:** the lookup by event name (the `.csi` tables), mod overlay sounds,
`stop`, music, bus volumes, the engine pitch and speech.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Play sound | `audio.play(SoundRef { Install(&str) or Mod(&str) }, volume: f32, pos: Option<Vec3>) -> Result<SoundId>` | G | `audio/mod.rs`, `audio/fx.rs`, `audio/common_effects.rs`, `libs/ea-audio` |
| Stop | `audio.stop(SoundId)` | G | `audio/mixer/` |
| Music track | `audio.play_music(track: &str)`, `audio.stop_music()`; tracks come from the music registry | G | `audio/radio/`, `nfsmw-data/src/music.rs` |
| Volume | `audio.set_bus_volume(Bus { Master, Sfx, Engine, Music, Speech }, f32)` **W** | G | `audio/volume.rs` |
| Engine sound | `audio.set_engine_pitch_scale(VehicleId, f32)` | G | `audio/engine.rs`, `audio/car/` |
| Speech line | `audio.speak(line: &str)` plays a registered speech line | G | `audio/speech/`, `nfsmw-data/src/speech.rs` |

### Camera

`bb.camera`. Camera modes are the settings `CameraMode` and the gameplay cycle (`Scene::next_camera_mode`).

**Delivered (1.0):** `get` reads the camera the player sees (position, forward, vertical field of view, and the view);
`set_mode` changes to one of the nine gameplay views (`BB_CAMERA_*`), queued. The free camera is not a gameplay view:
it is **open**, with `override_pose`, `release`, `shake`, the field-of-view scale and the photo path.

**Delivered (1.1): extra views.** A mod adds cameras of its own with `register_view`. The game lists them after its own
gameplay views: the camera hotkey (**C**) steps through the built-in views and then through the extra views that fit the
car, and the Camera row of the options menu (on the Player page and on the Gameplay page, which is also the pause menu's;
it lists only the extra views that fit the car being driven, and the developer tools' combo lists them all) shows them by
name. The choice is saved in the config as `camera_mode = "extra:<mod id>/<view id>"`; while the mod is not loaded, or the view does not fit the car (its
`available` answers no), the game draws the chase camera and keeps the setting. Pressing the hotkey onto an extra view shows
its name for two seconds.

While an extra view is the player's view the host calls its `update` once per rendered frame, on the main thread, after the
car has moved and before the frame's camera is read, with a `BbCameraInput` (the lead car's presented body: the position and
the forward and up axes of the car model, whose origin is on the ground between the axles, not the physics body's centre of
gravity; velocity, angular velocity, speed, steering, throttle, brake, ground contact, the strength of a hit this frame,
the car's id, and a reset flag for the first frame after the view was chosen or the car jumped). The mod answers a pose
(position, forward, up, vertical field of view, and a flag to hide the player's car for a camera inside it). Any other status
keeps the chase camera for the frame. All vectors are render space (x forward, y left, z up, metres), the same as the vehicle
state. The camera model, its smoothing and its per-car data belong to the mod: the 2012 content mod does it with
[`chameleon-camera`](../formats/chameleon-camera.md).

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register | `camera.register_view(id, name, impl CameraView) -> Result<CameraViewId>`: `id` is plain and unique in the mod, `name` is what the setting shows | G | `modding/camera_views.rs`, `modding/backends/camera.rs` |
| View trait | `CameraView::available(&mut self, car: &str) -> bool` (default yes), `CameraView::update(&mut self, &CameraInput) -> Option<CameraPose>` | G | `libs/blackbox-modding/src/sdk/camera.rs` |
| Unregister | `camera.unregister_view(CameraViewId)`; unloading the mod does it | G | same |
| Choose | `camera.select_view(CameraViewId)` **W**, queued like `set_mode` | G | `scenes/world/extra_view.rs` |

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Read | `camera.get() -> CameraState { pos: Vec3, forward: Vec3, fov: f32, mode: CameraKind }` | G | `viewer/camera.rs`, `scenes/world/camera_choice.rs` |
| Set mode | `camera.set_mode(CameraKind)` **W** (chase, bumper, hood, free, original) | G | `Scene::set_camera_mode`, `scenes/world/camera_cycle.rs`, `camera_original.rs` |
| Override | `camera.override_pose(pos: Vec3, forward: Vec3, fov: f32)` takes the camera until `release`; the free camera path | G | `scenes/world/focus.rs`, `viewer/camera.rs` |
| Release | `camera.release()` | G | same |
| Shake | `camera.shake(amplitude: f32, secs: f32)` | G | `scenes/world/camera_original/` |
| Field of view | `camera.set_fov_scale(f32)` | G | `viewer/camera.rs` |
| Photo path | `camera.screenshot(path: &str) -> Result<()>` | G | `app/screenshot.rs` |

### World state

`bb.world`.

**Delivered (1.0):** `track` (the track's name) and `section` (the district at the player) through a sink; `set_rain`
(keep it raining, or leave it to the weather, at an amount); and `set_traffic` (the car count and the cop share in
percent, keeping the lights and avoidance settings). The reads are the frame's copy of the scene. The two setters are
queued. **Open:** the time scale, pause, trigger zones, ground height, road position, props, fog and sun, and the
random seed.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Time scale | `world.set_time_scale(scale: f32)` **W**, 1.0 normal; the speed breaker uses the same lever | G | `scenes/world/simulation.rs`, `audio/speedbreaker.rs` |
| Pause | `world.set_paused(bool)` **W** | G | `Scene::paused` |
| Weather | `world.set_rain(always: bool, amount: f32)` **W** | G | `Scene::set_rain`, `weather/` |
| Trigger zone | `world.add_trigger(ZoneDef { centre: Vec3, radius: f32 }) -> Result<ZoneId>`, `remove_trigger(ZoneId)`; raises `ZoneEntered` and `ZoneExited` | G | `scenes/world/entry_zones.rs` (spheres) |
| Ground height | `world.ground_height(x: f32, y: f32) -> Option<f32>` | G | `scenes/world/ground.rs` |
| Road position | `world.nearest_road(pos: Vec3) -> Option<RoadPoint { pos, heading }>` | G | `scenes/world/road.rs`, `libs/blackbox-roads` |
| Prop and tile info | `world.props_near(pos, radius, out)` (the `props` command) | G | `scenes/world/props.rs` |
| Fog and sun | `world.set_fog(FogParams)`, `world.set_sun_scale(f32)` **W** | G | `nfsmw-data/src/fog.rs`, `scenes/world/sun.rs`, `lighting.rs` |
| Load area | `world.region() -> RegionInfo { name, bounds }` (name of the city district at a point) | G | `scenes/world/zone.rs` |
| Random | `world.random_seed() -> u64` a seed for deterministic mods (the same on every peer in a session) | G | `modding/runtime.rs` (new) |
