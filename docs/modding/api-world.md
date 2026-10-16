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

`bb.menus`. The front end is hard-coded FEng screens (`frontend/factory.rs`, `screens.rs`, `options_rows.rs`).
The first delivery gives mods **rows and pages the host draws in the game's style**, not raw FEng access:

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
| Translate | `menus.text(key: &str) -> String` looks the key up in the mod's `content/strings.toml`, falls back to the key | G | `modding/strings.rs` (new) |
| Replace FEng package | `menus.override_package(name: &str)`: use a package from the overlay instead of the install's. **Reserved** for a later minor; today the VFS overlay can already replace `*.fng` files wholesale | G | `libs/blackbox-feng`, VFS overlay |

Mod page and row labels are plain UTF-8 drawn with the game's font (`libs/blackbox-text`); a mod can ship
its own font through `bb.content`.

## Game interaction

### Vehicles and the player

`bb.vehicles`. A vehicle handle can be the player's car, a traffic car, a cop or a racer.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Player vehicle | `vehicles.player() -> Option<VehicleId>` | G | `scenes/world/drive/` |
| State | `vehicles.state(VehicleId) -> Result<VehicleState { pos: Vec3, heading: f32, velocity: Vec3, speed_kph: f32, rpm: f32, gear: i8, nos: f32, on_ground: bool, damage: f32, role: Role, car_id }>` | G | `scenes/world/drive/`, `ai/car.rs`; HUD numbers from `hud/state.rs` |
| Teleport | `vehicles.teleport(VehicleId, pos: Vec3, heading: f32)` **W**; the player is put on the nearest road like `reset` | G | `scenes/world/commands.rs` (`tp`, `reset`) |
| Set velocity | `vehicles.set_velocity(VehicleId, v: Vec3)` **W** | G | `libs/blackbox-vehicle` state via the drive module |
| Add NOS | `vehicles.add_nos(VehicleId, amount: f32)`, `set_nos`, `set_infinite_nos(bool)` **W** | G | `scenes/world/drive/` |
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
