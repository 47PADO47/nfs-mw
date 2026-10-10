# Architecture

The Rust rewrite reads every asset from the user's own install at runtime and ships no game data. This
page covers how the workspace is split, how the install is found, how the city streams, the graphics
backends, multi-platform support and testing. The structure follows
[vladtrc/iw4L](https://github.com/vladtrc/iw4L) ([research.md § 8](research.md#8-reference-projects-for-the-rust-rewrite)),
scaled down.

## Two halves: `libs/` and `crates/`

| Folder | What goes there | Rule |
|---|---|---|
| [`libs/`](../libs) | Everything that also works for other EA Black Box games (Underground 2, Carbon, …) or other reimplementations: codecs, container and format readers, install discovery, the renderer | **No NFS:MW-specific code or defaults.** Game differences go into per-version / per-game layout tables, chosen from the data or passed in by the caller. Each lib has its own README and can move to a shared utilities repo unchanged. |
| [`crates/`](../crates) | NFS: Most Wanted itself: which files the game uses, how they fit together, the launcher and viewers | May depend on `libs/`; never the other way round. |

### Libraries

| Crate | Job | Version handling |
|---|---|---|
| [`ea-compress`](../libs/ea-compress) | JDLZ, HUFF, RAWW | self-describing headers |
| [`blackbox-attrib`](../libs/blackbox-attrib) | AttribSys gameplay databases (`VPAK`, vaults, classes, collections, inheritance) | `layout/` per AttribSys generation (legacy = MW), detected per vault |
| [`blackbox-carparts`](../libs/blackbox-carparts) | Car types, the parts database, slot types, presets, light materials | `layout::MOST_WANTED` (parts pack v6, 139 slots) passed by the caller |
| [`blackbox-hash`](../libs/blackbox-hash) | `bStringHash` | — |
| [`blackbox-chunk`](../libs/blackbox-chunk) | Zero-copy bChunk trees; chunk ids by domain | — |
| [`blackbox-tpk`](../libs/blackbox-tpk) | Texture packs (plain and compressed forms), CPU decoding | `layout/` per TPK version (5 = MW) |
| [`blackbox-solid`](../libs/blackbox-solid) | Solids: groups, every vertex buffer, indices, position markers | `layout/` per `SolidInfo` version (0x16 = MW) |
| [`blackbox-streaming`](../libs/blackbox-streaming) | The track streaming index | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-scenery`](../libs/blackbox-scenery) | Scenery infos and instances; visibility rule | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-collision`](../libs/blackbox-collision) | World collision packs, the collision grid, car and prop bounds, a ray-cast query | — (one layout so far) |
| [`blackbox-feng`](../libs/blackbox-feng) | FEng user-interface packages and fonts: reader, script and message runtime, a retained `UiTree`; no rendering | — (one format version) |
| [`blackbox-text`](../libs/blackbox-text) | Language string tables (`LANGUAGES/*.bin`) | — |
| [`ea-audio`](../libs/ea-audio) | EA audio: SCHl streams, ABK sound banks, MPF/MUS music and its node graph, `.big` speech, `.gin` engine loops; EA-XA, EA-XAS and MicroTalk decoders | self-describing headers |
| [`blackbox-ginsu`](../libs/blackbox-ginsu) | The Ginsu granular engine-sound synthesiser and the tables of a `.gin` file | — |
| [`blackbox-carsound`](../libs/blackbox-carsound) | Car sound controllers: telemetry in, engine mix out, on a fixed 60 Hz tick | — (tuning passed by the caller) |
| [`blackbox-minimap`](../libs/blackbox-minimap) | The minimap: the compressed map tiles, the world-to-map projection, the view around the player (tiles, scroll, turn) and the placement of blips | — (one format version) |
| [`blackbox-mixmap`](../libs/blackbox-mixmap) | The sound system's dynamic mixer: `MIXMAPS/*.mxb` parser and a deterministic evaluator (published values in, per-object volume, pitch and filter slots out) | — (one format version) |
| [`blackbox-aems`](../libs/blackbox-aems) | AEMS module banks inside `.abk` files: a reader and an interpreter of the event-sound graphs (which samples play, how loud and how high); voices and objects are a `Host` trait the caller implements | — (one format version) |
| [`blackbox-movie`](../libs/blackbox-movie) | EA VP6 movies (`.vp6`): demuxer and video decoder (the MIT `nihav-vp6`); no audio yet | — |
| [`blackbox-particles`](../libs/blackbox-particles) | Particle emitters: spawn and update rules (cone spray, drag, gravity, keyed size, angle and colour curves) as a deterministic simulation that yields sprites | — |
| [`blackbox-vehicle`](../libs/blackbox-vehicle) | Deterministic fixed-step vehicle physics: rigid body, engine and gearbox, suspension, tires, steering, aero; driven by plain parameter structs and a `Ground` ray-cast trait | — (parameters passed by the caller) |
| [`blackbox-gfx`](../libs/blackbox-gfx) | The renderer-neutral graphics interface: shared types, the `RenderBackend` trait, capabilities, graphics settings and the pure `resolve` | — |
| [`blackbox-gfx-testkit`](../libs/blackbox-gfx-testkit) | Test support for any renderer: procedural scenes, image metrics and digests (no game data, no GPU API) | — |
| [`blackbox-render`](../libs/blackbox-render) | Backend-neutral renderer (wgpu inside); implements the `blackbox-gfx` types and `RenderBackend`; headless mode | — |
| [`blackbox-scene`](../libs/blackbox-scene) | Uploading solids and textures to the renderer; boxes; frustum culling | — |
| [`game-install`](../libs/game-install) | Finding, validating and reading an install, case-insensitively | driven by a `GameSpec` |

### Game crates

| Crate | Job |
|---|---|
| [`nfsmw-data`](../crates/nfsmw-data) | MW's `GameSpec`; car assembly (stock and preset parts, wheel and brake placement, paint, texture swaps); the world: streaming index, section parsing, a background section loader. Renderer-free. |
| [`nfsmw`](../crates/nfsmw) | The binary: CLI (`commands/`), layered `settings/`, the Bevy `app/` (window, loop, render bridge, cursor, pacing, screenshots), the `input/` action layer, the `ui/` (the shared FEng host layer: packages, assets, presenter), the `hud/` (the in-game HUD), the `frontend/` (menus and the game flow), the `audio/` (sound output), the `viewer/` cameras and `Scene` trait, and the scenes (`scenes/car/`, `scenes/world/`). |
| [`xtask`](../xtask) | `cargo xtask check` (leak check + file-size check), `install-hooks` |

Rules that keep this structure working:

- **Format crates take `&[u8]` and never open files.** Only `game-install` touches the install.
- **The renderer has no game knowledge**, and its API has no wgpu types, so the implementation behind it can change (for example to Bevy's renderer) without touching callers.
- **`libs/` stays Bevy-free, with one exception** ([ADR 0004](decisions/0004-swappable-renderers.md)): Bevy-based renderer backend crates (for example `blackbox-bevy-render`) are allowed in `libs/` as optional leaf crates that nothing else depends on. Callers use the renderer-neutral `blackbox-gfx` interface only.
- **Files stay small:** no source or doc file over 500 lines (`cargo xtask size-check`). Split by domain into folders and modules, with one struct or concern per file.
- **Game rules live in their own pure crates** (physics in `blackbox-vehicle`, AI to come), built spec-first ([licensing.md](licensing.md#spec-first)). `blackbox-vehicle` takes no I/O and no collision dependency: the game fills its parameter structs from AttribSys and gives it a `Ground` over the collision world.

## Finding the install

[`game-install`](../libs/game-install/src/discover/mod.rs) looks in this order and uses the first hit; the
names come from MW's `GameSpec` ([`nfsmw-data/src/game.rs`](../crates/nfsmw-data/src/game.rs)):

1. `--game-dir <PATH>`;
2. `NFSMW_GAME_DIR`;
3. `NFSMW_GAME_DIR=<PATH>` in a `.env` file in the working directory or next to the executable;
4. `game_dir = "<PATH>"` in the per-user config file. Its location depends on the platform, and
   `nfsmw check-install` prints it:
   - Windows: `%APPDATA%\nfsmw\config\config.toml`, i.e. `C:\Users\<you>\AppData\Roaming\nfsmw\config\config.toml`;
   - Linux: `$XDG_CONFIG_HOME/nfsmw/config.toml`, usually `~/.config/nfsmw/config.toml`;
5. on Windows, `HKLM` / `HKCU` `\SOFTWARE\EA GAMES\Need for Speed Most Wanted` → `Install Dir` (32-bit view).

The required files are checked, and `speed.exe` is hashed and identified (v1.3 = `80774c2e…1d253c`).
`GameDir` indexes the install once and resolves every path case-insensitively, so the same code works on
Linux. `nfsmw check-install` shows what was found.

## The application shell

`nfsmw` is a Bevy app ([decision](decisions/0001-bevy.md)): `bevy_app` and `bevy_ecs` for the schedule and
resources, `bevy_winit` for the window and loop, `bevy_input` and `bevy_gilrs` for devices. Bevy's renderer
is not used: `blackbox-render` draws the frame, from one system, in `app/render.rs` (the render bridge).

```
bevy_winit window ─► PreUpdate: input/ resolves devices into actions (ActionState)
                  ─► Update:    create renderer ─ cursor ─ resize ─ scene.update ─ draw
                  ─► Last:      title (once a second) and the --max-fps limiter
```

- **Input layer** (`input/`): game code reads `ActionState` (`MoveForward`, `LookX`, `Boost`, `Cancel`…),
  never a key code. The `Bindings` resource maps keyboard, mouse and gamepad inputs to actions, with a stick
  independent response settings and per-second scaling for sticks; saved overrides and live console rebinding
  replace that resource ([controller settings](controller-settings.md)). Analog Bevy filtering is neutralized
  on connection so the action layer applies deadzones once; digital button hysteresis is retained. Default pad layout:
  left stick moves, right stick looks (and orbits), A/B go up/down, stick-click or right bumper boosts,
  D-pad up/down zooms, Start backs out. Keyboard and gamepad taps survive for one action frame.
  `InputPresentation` selects the active pad and prompt device ([menu policy](specs/controller-ui.md)).
- **Driving actions:** `Throttle` and `Brake` (0..1, so a pad's analog triggers are real pedals), `Steer` (-1..1),
  `Handbrake`, `ShiftUp`, `ShiftDown`, `Nos`, `ResetCar` and `ToggleCamera`. Keyboard: W/S or Up/Down pedals, A/D
  or Left/Right steer, Space handbrake, E/Q (or Right Shift/Ctrl) shift up/down, Left Shift nitrous, R reset, F camera. Pad: right
  and left trigger pedals, left stick steers, A handbrake, X nitrous, right/left bumper shift up/down, Back resets,
  Y toggles the camera. Like all bindings they are untested on a real controller. A wheel appears as a gamepad with
  device-specific codes, which the log prints on first use. `paddle_up` / `paddle_down`, `pedal:` / `pedal_inv:`
  full-range pedal axes, `gear_*` actions (H-shifter) and an optional `clutch` pedal cover it
  ([controls](controller-settings.md)); only synthetic tests ran, no real wheel was tried.
- **Cursor:** mouse-look scenes capture the cursor; the first Esc (or Start) releases it, the next quits.
- **Errors** from systems (no GPU, a failed present) are stored and returned from `main`; the app exits with an error code.

## Developer tools

- **UI layer.** `blackbox-render` draws a 2D layer over the scene: textured, clipped, premultiplied-alpha
  triangles (`UiLayer`, `UiTexturePatch`). It knows nothing about egui; the HUD and the front-end menus draw through the same layer.
- **egui host** (`gui/`): turns Bevy keyboard, mouse and wheel messages into egui events and egui's output into that layer. The panels only see an `egui::Context`.
- **Metrics** (`devtools/`): `Metrics` is plain data (240 frame times, GPU name, mesh and texture counts).
  `--show-metrics basic` draws fps and frame time; `advanced` adds the 1% low, worst frame, a frame-time graph
  with a 60 fps line, resource counts and the scene status.
- **Console** (F12; Esc or F12 closes it): the log (the last 1,000 lines, coloured by level, kept by a
  logger that wraps `env_logger`) above a command line with history (Up/Down) and Tab completion. The
  keyboard belongs to the console while it is open: game actions go quiet and the mouse is released
  (and recaptured on close). Typed lines are parsed into a `Command` (plain data, unit-tested) and run by
  one system, so the console never touches the renderer itself. Commands run in their own `FrameSet::Commands`,
  after `Prepare` and before the front end and the scene update, so a startup `--exec` setting is in place
  before a screenshot's scripted simulation starts.
  - Built in: `help`, `clear`, `quit`, `get [setting]`, `set <setting> <value>` (`fps`, `vsync`, `metrics`, `readout`, `transmission`),
    the shorthands `fps 60` / `vsync off` / `metrics advanced`, and `resolution <w> <h>`. Changes last for
    the run; the config file is not written.
  - The car viewer adds `car <folder>`, `garage` and `freecam` (orbit ↔ free camera). The world viewer adds
    `drive`, `reset`, `tp`, `goto`, `freecam`, `pos`, `props`, `debug collisions` and `garage` ([Driving](#driving-view-world---drive)).
    Scenes offer commands through `Scene::commands` and `Scene::command`; "spawn AI" arrives with milestone 7.
  - `--exec "<command>"` (repeatable) runs commands at startup, like Quake's `+exec`; `--open-console`
    (hidden) starts with the console open, which is how the screenshots in bug reports show it.
- **Scene readout:** `Scene::readout(level)` gives the scene's debug lines, which the overlay draws bottom left
  whatever the metrics level is. `--show-readout <off|minimal|full>` (env `NFSMW_SHOW_READOUT`, config
  `show_readout`, console `set readout full`) chooses how much. The original HUD already shows speed, rpm, gear and
  nitrous, so the default `minimal` is one line that adds to it (the car and where it is; "free camera" in the free
  camera); `full` is the old two-line readout (speed, rpm, gear, nitrous, a scripted run) for when the HUD is off.
- **Order of a frame:** `Prepare` (renderer, cursor, size) → `SceneUpdate` → `Ui` (egui pass) → `Draw`
  (the bridge uploads texture patches, sets the layer, renders). `--screenshot` runs a few frames first so
  the overlay is in the picture.

## Settings

Runtime options resolve in layers, highest first ([`crates/nfsmw/src/settings/`](../crates/nfsmw/src/settings)):

1. the command line (`--backend`, `--no-vsync`, `--max-fps`, `--show-metrics`, `--show-readout`);
2. environment variables (`NFSMW_BACKEND`, `NFSMW_VSYNC`, `NFSMW_MAX_FPS`, `NFSMW_SHOW_METRICS`, `NFSMW_SHOW_READOUT`);
3. the per-user config file (`backend`, `vsync`, `max_fps`, `show_metrics`, `show_readout`, `radio`; the same file as `game_dir`);
4. the defaults (`auto`, vsync on, unlocked, overlay off, readout minimal).

Gameplay keys: `hud`, `transmission` (`--transmission automatic|manual`, `NFSMW_TRANSMISSION`; automatic by default,
as in the original), the wheel's `paddle_up` / `paddle_down` codes and off-by-default `manual_clutch` / `h_shifter`
(config file, `NFSMW_MANUAL_CLUTCH` / `NFSMW_H_SHIFTER`, console `set`). Each key resolves on its own. A value that does not parse (in the environment or the file) is logged and
skipped, so the next layer applies; a broken config file never stops the game from starting. The
install directory has its own, longer lookup ([Finding the install](#finding-the-install)).

Window mode, monitor and resolution use the same layers (`window_mode`, `monitor`, `resolution`;
`NFSMW_WINDOW_MODE`, `NFSMW_MONITOR`, `NFSMW_RESOLUTION`; `--window-mode`, `--monitor`,
`--resolution`). Alt+Enter and console commands switch during a run. Bevy owns monitor selection,
DPI and mode changes; the render bridge follows the surface size. See [window-modes.md](window-modes.md)
for restoration, exclusive-mode fallback and the fixed hidden screenshot window.

## The streamed city (`view-world`)

```
TRACKS/L2RA.BUN ──► blackbox-streaming: 720 sections (605 map tiles, 115 shared sets),
                    435 zones with their visible lists
TRACKS/STREAML2RA.BUN ─► loader threads (nfsmw-data::world::Streamer)
                          read a section's byte range → blackbox-solid / -tpk / -scenery
                        ─► render thread (scenes/world/residency.rs)
                          upload textures → meshes → place instances
```

- **Shared sets (V/X/Y/Z) load once, at startup**, all together, because their models and textures refer to
  each other (2,734 models, 2,111 textures).
- **Map tiles stream by zone, as in the game** ([specs/visible-sections.md](specs/visible-sections.md)).
  The map is split into 435 2D zones (one per drivable section). The camera's zone names the tiles to
  load (its loading section's union of visible lists, or its own list) and the tiles to draw (its own
  visible list); nothing else is drawn, even when loaded. Tiles are requested drawn ones first, nearest
  first, at most 8 in flight, on 2–6 worker threads; up to 2 are uploaded per frame. When the zone
  changes, the previous zone's tiles stay drawn until the new zone's tiles are resident; then the tiles
  it no longer needs are released (GPU meshes and textures freed).
- **Free camera:** the 66 zones outside the region's drivable list are places a car never reaches, with
  almost empty lists; over them, and off the map, the camera keeps its last zone. It starts in the
  region zone nearest to the start point. The status line shows the zone (`zone D14`).
- **Resolution:** a tile's instances use the tile's own models first, then the shared sets. Measured on the
  whole stream, that resolves 77,722 of 77,776 tile instances; none needs another tile
  ([maps.md](formats/maps.md#scenery-placing-models-in-the-world)).
- **Panoramas** (tiles 90–99: islands, mountains, low-detail city cards, the oceans `R88`/`R89`) are
  only drawn from the zones that list them. They sit in the middle of the map: drawn from anywhere else
  (as the old distance-based loading did) they stand across roads and over the real buildings
  ([maps.md](formats/maps.md#section-families-verified)).
- **Visibility:** instances hidden by their exclude flags (race barriers, animated props) are dropped when
  placed ([specs/scenery-visibility.md](specs/scenery-visibility.md)). Each frame the remaining instances are
  frustum-culled using their stored world boxes. Then the game's LOD rule picks slot 0 or slot 2, or nothing
  under 17 px, which is also the real draw distance ([specs/scenery-lod.md](specs/scenery-lod.md)).
  Finally the instances are sorted by mesh and drawn instanced.
- **Sky:** the `SKYDOME` and `SKYDOME_XENON` scenery models, textured from `GLOBAL/InGameA.bun`, drawn
  with the fog-free sky shading. Texture animations (water, signals) advance every frame. Depth is reverse-Z with an infinite far plane, so the 9.7 km dome is never clipped.
- **Shading:** world geometry is pre-lit (vertex colour × 2, no sun); blending follows each texture's
  `AlphaBlendType` ([textures.md](formats/textures.md#alpha)). Linear fog from half of `--fog-distance` to
  all of it (default 3 km); placeholder until the game's fog is known. Cars use the renderer's glossy shading (`Shading::Glossy`): three directional lights (`set_lighting_rig`), a sun highlight and a reflection of an environment cube map (a generated sky unless `set_environment_faces` replaces it), with the constants of each part's light material ([specs/car-assembly.md §8](specs/car-assembly.md#8-the-car-shader)).
- **Camera:** free-fly (WASD, Space/C, Shift, right-drag to look, scroll for speed). It starts above the
  centre of the city, or at `--at X,Y`, at `--height` metres above the ground; the ground is estimated from
  the scenery boxes until collision is loaded.

- **Collision:** each map tile carries a collision pack (`CarpWCollisionPack`,
  [formats/collision.md](formats/collision.md)); the residency puts it into a `CollisionWorld` when the tile is
  resident and removes it when the tile is released, and the track file's collision grid says which instances a
  query can touch. Packs are keyed by their own section number, which is not the tile's (tile `A41` carries pack
  101), so they are kept per tile and removed with it. Queries use physics space, so `scenes/world/space.rs`
  swaps axes (`(x, y, z)` render to `(-y, z, x)` physics). A ray cast costs about 1.3 µs.

Known gaps, for later milestones:

- **Not drawn yet:** `SKY_SPECULAR`, water reflections, cars and traffic, and the world animations
  (cranes, the airliner).
- **High above the streets** the city has holes and buildings without ground: the zone lists only hold
  what can be seen from the road, as in the game.
- **Approximations:**
  - the LOD pixel scale assumes a 480-line reference screen;
  - no scenery overrides, so race barriers are never shown (and the garage-door group the game enables
    at load time stays hidden);
  - no zone prediction from the car's velocity, no visible-section overlays (`FlyBy`), and every V/X/Y
    set stays loaded instead of streaming with the zones;
  - placeholder lighting instead of the game's `fx` effects and time of day.

## Driving (`view-world --drive`)

`nfsmw view-world --drive [CAR] [--at X,Y]` puts a car (default `BMWM3GTR`; a folder or a unique
prefix) on the street nearest to the start and follows it with a chase camera. Keys and pad are listed
under [the application shell](#the-application-shell); `F` (or `freecam`) swaps to the free camera, which
parks the car. The original FEng HUD ([The HUD](#the-hud)) shows speed, rpm, gear and the gauges; the readout in the
corner is a debug aid with levels ([Developer tools](#developer-tools)). It lives in [`scenes/world/drive/`](../crates/nfsmw/src/scenes/world/drive):

```
input (actions or --drive-script) ─► DriveInput ─► 60 Hz fixed step (accumulator, interpolated for drawing)
   CarSim = blackbox-vehicle Vehicle ◄─ WorldGround (ray casts on the resident collision packs, surface grips)
   │  after each step: walls.rs (barriers, steep faces, props) pushes the body out and reacts through the rigid body
   ▼
CarPose (render axes) ─► CarRig (assembled car, wheels posed: steer, spin, suspension travel) ─► ChaseCamera
```

- **Axes.** Physics space is x right, y up, z forward (left-handed); the world and the car models are x forward,
  y left, z up. `sim.rs` builds the pose from the body's axes column by column, puts the model origin at the
  body box's centre minus the collision pivot, and maps the library's wheel order (rear left, rear right) to the
  model's (rear right, rear left).
- **Data.** `nfsmw-data::car::physics` reads everything `pvehicle` links to (engine, transmission, chassis,
  tires, brakes, induction, nitrous, rigid body, aerodynamics), the car's collision bounds (its root box is
  the body size) and the `simsurface` grips into a `VehicleSpec`. Stock classes are used, except nitrous: the
  first level that has a tank, so `N` does something.
- **Placing.** The car waits until the tiles around it are resident, then `road.rs` searches the collision
  surfaces for asphalt or concrete in rings (streets of 40 m or more before car parks), takes the longest run
  as the heading and centres across it. `reset` and `tp X Y` do the same near the car or the given point, `reset`
  facing the same way.
- **Last good road.** Twice a second, with three wheels down on a road, the position is remembered. `reset`
  with no street near, a fall (no wheel down and nothing within 30 m below for 0.75 s, or 80 m under that
  road) and a non-finite state all bring the car back there. Leaving the free camera more than 25 m from the car
  brings the car to the street nearest the camera.
- **Walls and props.** Barriers and steep faces of the collision packs (guard rails, concrete barriers,
  fences: 45,815 barriers) are met by eight probes on the body's outline; props are tested with a box overlap.
  A barrier is a wall from its front side only (the drivable side) unless it is flagged two-sided; the 8,345
  barriers of scenery groups (the road blocks and gates of races, off in free roam) are left out of the probes, the
  tyre rays and the chase camera, and faces flagged "not ground" are skipped by the tyre rays.
  Rigid contacts react with `WALL_FRICTION`, `WALL_ELASTICITY` and `WORLD_MOMENT_SCALE`. A light prop (see
  [collision.md](formats/collision.md#props)) costs the car `m_car / (m_car + m_prop)` of its speed, disappears
  and comes back after six seconds.
- **Chase camera.** It keeps a smoothed offset from the car (no lag at speed), backs off and widens its view
  up to 60 m/s (7 m, 68 degrees) and no further, and an obstacle shortens its reach quickly but lets it grow
  back slowly, never below 2.8 m from the car.
- **Gears.** The `transmission` setting (the Gameplay options row, `--transmission`, console `set transmission
  manual`) says who changes gear ([spec](specs/vehicle-manual-shifting.md)). Automatic shifts by itself and the shift
  keys are sport shifts. In manual nothing shifts by itself, down from first is neutral, the limiter holds the revs at
  the red line in every gear, a downshift into an over-rev is taken (the wheels are pulled down to the new gear's
  red-line speed, a hard engine braking), the buttons do nothing in reverse and braking to a stop still engages
  reverse. A shift or reset press that arrives in a frame with no 60 Hz physics step waits for the next step. The
  setting reaches the car through `Scene::set_transmission`.
- **Contact markers.** `debug collisions [on|off]` (no argument: toggle) draws what the car touches, as a cube at
  the point and a stick along the normal: red for a barrier, orange for a steep face, magenta for a rigid prop,
  yellow for a light prop, green for where each tyre's ray met the road. Wall and prop contacts stay for half a
  second (they are recorded whether or not they are drawn); the scene status line counts them. A wall the car
  cannot see (the playtest report of walls on leaves, curbs and sidewalks) shows up as a marker with nothing
  visible behind it.
- **Console:** `drive [car]`, `reset`, `tp <x> <y>`, `goto <x> <y> [height]`, `freecam`, `pos`, `props [radius]`,
  `debug collisions` and `garage` (every car for now; later the player's own). `--drive-script "3:throttle=1;1:steer=0.5,throttle=0.6;0.1:reset"`
  (hidden option) drives with a script: keys `throttle`, `brake`, `steer`, `handbrake`, `nos`, `up`, `down`,
  `reset`. With `--screenshot` the script runs in batches, waits for the map around the car to load and the
  picture is taken when it ends; the run logs one line per second (speed, rpm, gear, position, wheels down).

The driven car's fixed steps also feed its existing per-wheel smoke/skid intensities to
`scenes/world/effects/`. Short collision projections anchor procedural marks to resident ground;
camera-facing smoke and grounded strips use `blackbox-render::EffectLayer`, depth-tested after
scene geometry and before the UI. CPU histories and reused GPU buffers have hard budgets.
See [tire-effects.md](tire-effects.md) for controls and [the spec](specs/tire-effects.md) for the design.

Optional sparks and wind trails use stock attribute links in `nfsmw-data::vehicle_effects` and bounded presentation in `scenes/world/vehicle_effects/`; see [vehicle-effects.md](vehicle-effects.md).

Known gaps: the car shader, car-versus-car and traffic, damage, the
original's wall steering, a controller that has been tried on real hardware, steering wheel support, the invisible
walls a playtest found on leaves, curbs and sidewalks (not identified: no prop is called a leaf and the data holds
almost no low steep geometry; `debug collisions` is the tool to find one), and calibration of the handling against
the original.

## The HUD

The in-game HUD is the original package `HUD_SingleRace.fng` run by `blackbox-feng`
([decision](decisions/0002-ui-presentation.md), [runtime spec](specs/feng-runtime.md), section 8 for the rules of each
element). `hud/` in the binary has a plain `HudState` resource (speed, rpm, the engine's `MAX_RPM` and red line, gear,
shifting, shift light, nitrous, boost) and the binding that copies the state into named FEng objects. The shared `ui/`
loads the packages by name (`Catalog`) and the fonts, textures and strings once (`UiAssets`), and has exactly one
presenter (`ui/present/blackbox.rs`) that draws the tree through the UI layer, under egui; it draws a multi image
through its mask by composing the picture with the rotated mask into one texture slot. A scene supplies its state
with `Scene::hud_state`; nothing else knows FEng.

- `--hud` shows it in any viewer (idle numbers; `--hud-demo SPEED,RPM,MAX_RPM,GEAR[,NOS_PERCENT[,BOOST_PSI]]` for
  reference shots), and it is on while driving unless the `hud` setting is off (`hud = false` in the config file,
  `NFSMW_HUD=off`, `--no-hud`, console `set hud off`). The free camera never shows it. `--screenshot` captures it.
- **Shown, from game state:** the speedometer (whole units, cut off; km/h); the tachometer (the face and the end of the
  needle's sweep from `MAX_RPM`, the red zone mask turned by the original's table, the gear digit that dims while a gear
  change is in progress, the shift light lit by the gearbox's shift-up wish); the nitrous gauge (bar mask and icon
  scripts) for a car with a nitrous system; the turbo dial for a car with forced induction.
- **Minimap:** the original's, from `TRACKS/L2RA/MINI_MAP.BIN` (64 tiles), the track table's calibration and the
  game's per-frame scroll and turn around the car ([spec](specs/hud-minimap.md), [data](formats/minimap.md)); the
  retail game has no speed zoom. `minimap = fixed|rotating|off` (config file, `NFSMW_MINIMAP`, console `set minimap`;
  fixed, as the original's free roam). Blips of cops and racers wait for milestone 7 (`blackbox_minimap::place_blip`).
- **Hidden:** the elements that need the race and pursuit state of milestone 7 (radar detector, pursuit, heat, busted,
  cost-to-state, milestone and race boards, countdown, infractions, speed breaker meter, the wrong-way sign, online
  fields); the engine temperature gauge (drag HUD only). The runtime still runs their scripts.
- **HUD layout:** Gameplay's [HUD Layout](hud-layout.md) chooses PC wide placement, centered 4:3 or the Xbox
  addon's 92% wide scale. A HUD-only viewport shifts the map and all gauge descendants together after their
  rotations; resizing and live preset changes leave package state untouched ([spec](specs/hud-viewport.md)).
- **Gaps:** the custom tachometer
  skins other than 00 are not loaded; the original also zeroes the shift light for a frame after a gear change and
  when the wheels lack traction (not in the decompilation, not done); the mask blend of multi images is inferred from
  the gauge textures; the HUD has not been compared with a capture of the original.

## The front end

The menus are the install's own FEng screens run by the same runtime and drawn by the same presenter
([decision](decisions/0002-ui-presentation.md), [input spec](specs/feng-input.md),
[menus spec](specs/frontend-menus.md)). `frontend/` has three layers:

- **Screens.** `Screens` keeps the stack of loaded packages and one `Runtime`; the pad mask goes in, the messages
  a package sends to the game come out and are handed to the screen's `ScreenLogic`, which is what the original's
  screen classes are: `IconMenu` (the main menu, the option categories, the pause menu: an icon scroller with the
  original geometry), `WidgetMenu` (the option rows: titles, data strings, sliders), `Splash`. A screen with no
  logic of its own runs as its package says. `options.rs` maps rows to settings.
- **Flow.** `Frontend` (a Bevy resource, driven in `FrameSet::Frontend`) decides what the window shows: the boot
  movies (`ealogo`, `psa` through the movie scene), the title screen, the main menu over an empty backdrop,
  free roam (the world scene, wrapped in `Pausable` so Escape or Start freezes it under the pause menu) and the
  way back. Career and Quick Race both start free roam: there is no career yet.
- **Input.** The `Menu*` actions of the input layer (arrows or WASD or the D-pad, Enter or Space or A, Escape or B,
  Start or P, Q on the main menu) become a pad mask for the runtime, which sends the engine's messages and moves the
  focus by geometry. `--ui-script` replaces input and clock for screenshots and tests.

Settings: the option rows change `Settings` (the resource the console edits, so volumes, vsync, the frame cap, the
overlay and the HUD apply at once) and record the change; the config file layer is written when a screen is left
(`settings/write.rs` keeps `game_dir` and unknown keys).

`nfsmw play` runs the whole flow (`--skip-boot`, `--drive`); `nfsmw view-screen NAME` shows one screen
(`--category`, `--pause`, `--options`, `--ui-script`, `--screenshot`); `list-screens`, `dump-screen NAME` and
`strings TEXT|0xHASH` read the install's screens and language table.

- Gaps: the screens besides these (career, quick race, customisation, online, dialogs, the keyboard) are not run; no
  confirmation dialogs (changes apply at once and stay); no mouse; no 3D backdrop behind the menus; the
  widescreen title screen is not used; the attract movie and its timeout are not played; the sounds the
  screens send are ignored; the original's player, controller and credits categories are not shown.

## Sound

`audio/` in the binary owns the output device (`kira` 0.12, cpal) and three mixer groups, effects, music and engine,
under a master volume; the volumes are settings (`master_volume`, `music_volume`, `sfx_volume`, `engine_volume`,
`speech_volume`, 0 to 100) and `--no-sound` turns the device off. Without a device the game runs silent. Decoding
is `ea-audio`, engine synthesis is `blackbox-ginsu`, and the mix controller is `blackbox-carsound`; `nfsmw-data`'s
`sound/` reads a car's sound set from the database.

- **The engine.** A scene reports the car it drives with `Scene::car_sound` (a `CarSoundState`: car type, the
  controllers' telemetry, the collision events since the last frame and the scrape going on). An ECS system feeds
  it to the car's `EngineMixer`; the result (Ginsu frequency and loop volumes) is handed to the engine voice, a
  custom `kira` sound that runs the accelerate and decelerate synthesisers on the audio thread and resamples them
  to the device rate.
- **The effects.** The same state feeds an `EffectsMixer` (`blackbox-carsound`), which returns commands: one-shots
  (shift clunks and sweeteners, brake mash, turbo blow-offs, purge), loops that keep a voice (reverse whine, turbo
  spool, nitrous, a skid loop per axle, road noise per side, wind, scrape) and landings. `audio/refs.rs` resolves
  each to a bank sound of the car's sound set (`nfsmw-data`'s `sound/`); `audio/fx.rs` plays them. A hit plays its
  collision stitch: the `audioimpact` collection the car's `pvehicle` links for the kind of hit and the surface,
  the level by the hit's size, and the pieces from `InGameB.bun` one after the other. The wheels' surfaces come
  from the vehicle library (`SurfaceGrip::tag`) and `simsurface`'s `Aud_Skid_Type` and `Aud_Roadnoise_LOOP`.
- **The mixer.** `audio/mixer/` publishes the car's state to the original's dynamic mixer (`MAPOUTPUT.mxb`, read by
  `blackbox-mixmap`: the physics, engine and hybrid-motor controllers, the 3D positions, the shift, nitrous and skid
  objects) and reads back, per frame, the levels of each sound: the engine's Ginsu and sample layers and pitch, the
  clunks, sweeteners, turbo, nitrous, skids, road noise per surface, wind and landings (`Levels`). They multiply the
  generated volumes (`scale_command`); `MAKEUP` (1.5) stands for the unknown full scale of the original's volumes.
  `engineaudio.Master_Vol` is a mixer input of the engine controller. The wind and road noise are placed by the
  chase-camera distance, which is a guess.
- **The sample layer.** `audio/aems/` runs two modules of the AEMS banks with `blackbox-aems` on a 60 Hz tick: the
  engine's `CAR` module (eight looped samples that cross-fade over the Ginsu frequency and the torque, and the
  redline sample) and the sweetener bank's `CAR_Sputter` (the pops when the throttle lifts, whose output volume
  feeds the spark-chatter input of the map). Their voices are bank sounds played through `kira` in the engine
  group (`KiraSampler`); the tests mix the same voices in software and write them to a WAV. Parameters:
  `audio/aems/params.rs`, from `docs/specs/engine-sound-aems.md`. The shift sweeteners are the module's bank
  sounds 1 and 2 played directly (what the `CAR_SWTN` module does).
- **Console.** `sound [bank [index]]` lists or plays a bank sound; `engine <car> [percent] | off` holds an engine
  at a share of its RPM range; `speech` (below). `RUST_LOG=nfsmw::audio=debug` logs the loops and hits that start.
- **Movies.** `nfsmw play-movie <name> [--start SECONDS]` and `list-movies`: `movie/` demuxes the file with
  `blackbox-movie`, decodes the sound up front with `ea-audio` (handed to the audio system through
  `Scene::take_clip`), decodes the video against a clock and shows the frames through `Scene::fullscreen`, which a
  small plugin turns into a letterboxed UI quad (`FrameSet::Hud`). The scene ends when picture and sound do. The
  boot flow plays `ealogo` and `psa` before the title screen; the video follows the frame clock, not the audio clock.
- **The radio.** `audio/radio/`: licensed songs from `MW_Music.mus`, played through the music group.
  `ea-audio::mus::graph` reads the PathFinder graph of `MW_Music.mpf` (nodes, routers, events; spec
  [music-graph.md](specs/music-graph.md)); a song's start event gives its first node, and following the graph to its
  end gives the chain of streams (39 to 164 per song) that `ChainReader` decodes as one gapless run. A decoder
  thread reads the chain from the file with positioned reads and a custom `kira` sound plays the blocks (linear
  resampling to the device, a fade-out when it is stopped), so no song is held in memory. `nfsmw-data`'s `music`
  reads the 26 songs (artist, title, event, `DefPlay`); `Playlist` picks the next one by the original's rules
  (front-end and in-game lists, ordered or shuffled without replacement). The radio plays in game: it starts with
  free roam, goes on under the pause menu (`Scene::paused`; the car falls silent) and stops when the scene is left. It starts the next song when one ends. `music_volume` and `master_volume` are read from the settings
  every frame, so the pause menu's audio rows change the song on the air at once; a music volume of zero silences
  the song without dropping it, and a new song only starts when the music is audible. `--no-sound` turns the radio
  off. Console: `radio` (status), `radio next|prev|pause|resume|toggle`, `radio on|off`, `radio list`, `radio play <n>`,
  `radio shuffle|ordered`; the keys `radio_toggle`, `radio_next`, `radio_previous` work while driving (spec section 7). `Audio::now_playing()` feeds the card of `hud/radio.rs` (artist, title, album, time), shown at the
  HUD's left edge for 6 s after a song starts, pauses or resumes. Unheard; how the original ends a song is inferred.
- **The pursuit music.** `audio/interactive/`: the four pursuit sets of the graph, steered by a control value (a
  `MusicInput` resource, hooks for milestone 7, or the `music` command). A director and conductor play them on a child
  track (`graph::Cursor` walks the bars), cross-fade sets, keep songs off for the chase and 40 s after. Map events are
  not run; unheard ([spec](specs/interactive-music.md)).
- **Speech.** `audio/speech/`: a `Dispatcher` and the `copspeech.big` takes on their own track. Only the 28 events
  with banks can be said; nothing asks yet (console: `speech say|play <event>`); [spec](specs/speech.md).
- **Not done:** speech sentences and triggers, the ambience music, the jukebox, the mixer's reverb, low-pass and
  azimuth outputs, Doppler, and a level check (`MAKEUP`, chase-camera distances: guesses).

## Graphics backends

Select with `--backend <auto|vulkan|dx12|gl>`:

| Backend | Windows | Linux | Implementation | Status |
|---|---|---|---|---|
| `auto` | Vulkan or D3D12, else GL | Vulkan, else GL | wgpu | working |
| `vulkan` | ✔ | ✔ | wgpu | working |
| `dx12` | ✔ | — | wgpu | working |
| `gl` | ✔ (WGL) | ✔ (EGL, X11/Wayland) | wgpu | working |

Direct3D 11 is not offered: wgpu removed its D3D11 backend in 2023. Direct3D 12 and Vulkan cover the same hardware on Windows 10 and later, and OpenGL covers older GPUs.

## Render pipeline

A frame in `blackbox-render` takes the cheapest path that gives the requested look. With no post effect, no upscaler and render scale 1.0 (the default) the scene draws straight into the surface next to a reverse-Z depth buffer, with no offscreen image and no copy. Otherwise it draws into an offscreen colour image plus the depth buffer at the internal render size (surface size × render scale, `set_render_scale`, 0.25 to 2.0), `Rgba16Float` only when bloom or tone mapping needs HDR and the surface's own format otherwise, and an ordered chain of fullscreen post-process passes reads that image (and the depth buffer, as a sampled texture) and ping-pongs through scratch images; the last pass writes the surface. The chain always ends with `resolve` (clamp, alpha 1; an exact copy at scale 1.0, bilinear otherwise); the optional bloom, tone mapping and FXAA passes sit at the front (`Renderer::set_post_effects`, all off by default, [post-processing.md](post-processing.md)) and upscalers insert before `resolve` (FSR 1 and the bilinear fallback: [upscaling.md](upscaling.md)). The UI layer draws last at surface resolution, so it is never post-processed or upscaled, and `--screenshot` captures the final image. Pipelines and the glossy car shader's resources are built on first use; the cost of each setting is in [low-end.md](low-end.md).

## Multi-platform

- **Targets:** Windows (x86_64-pc-windows-msvc) and Linux (x86_64-unknown-linux-gnu). CI builds and tests
  both ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- **One system package at build time, on Linux: `libudev-dev` (and `pkg-config`)**, which gamepad support (gilrs) links against.
  winit (inside `bevy_winit`) and wgpu load X11, Wayland, Vulkan and EGL dynamically, so those need nothing to build.
- **Linux installs:** point `--game-dir`, `.env` or the config file at the game folder (for example in a
  Wine prefix). There is no registry lookup on Linux.
- **Endianness:** every reader decodes explicitly with `from_le_bytes`.

## Testing

Unit tests, real-install tests and the guard rails are described in [testing.md](testing.md).

## Roadmap

| Milestone | Content | Status |
|---|---|---|
| 1 | Workspace, guards, install discovery, bChunk/JDLZ/HUFF, solids, TPK, car viewer on Vulkan/DX12/GL | done |
| 2 | Generic `libs/` split; the streamed city: index, sections, scenery, background loading, instanced rendering, culling, fly camera | done |
| 3 | Sky dome, LODs, water, panoramas; zone-based streaming (visible sections); AttribSys reader; car assembly from the parts DB (stock parts, wheels, brakes, paint). Playtest fixes: misplaced and floating scenery, mouse look without holding a button, `--max-fps`, clearer config-file path | done |
| 4 | Engine foundation: decide on Bevy (ECS, events, UI) in an ADR and migrate the viewers if adopted; layered settings (command line > environment > per-user config file > defaults, with a settings menu in 6); input layer with controller support; developer console (F12: log view, commands such as change car, toggle free camera, change settings); performance overlay (`--show-metrics off\|basic\|advanced`). Decided: Bevy as the shell with our renderer ([ADR 0001](decisions/0001-bevy.md)), full Bevy renderer revisited in 8 | done |
 | 5 | Vehicle physics, spec-first (`docs/specs/vehicle-*.md`); world collision (`CarpWCollisionPack`); drive a car with the original HUD: read the FEng HUD packages (`HUD_*.fng` in `InGameB.bun`) and draw them with the UI layer; steering wheel controller support (wheel axes, pedals, shifters) on the input layer from 4 |  in progress: `blackbox-vehicle`, the collision reader, input actions, `view-world --drive` (placing, chase camera, one-sided walls, props, reset and fall recovery, scripted runs), manual shifting (Q/E, pad bumpers and wheel paddle buttons, with a transmission setting and an options row), the original HUD (speedometer, tachometer with its red zone and shift light, gear, nitrous bar, turbo dial, minimap; the rest of the package waits for the race and pursuit state of milestone 7), the `--show-readout` levels and the `debug collisions` command are in; steering wheel support (pedal axes, paddles, direct gears for an H-shifter and an optional clutch pedal are in; hardware testing on a real wheel is open); controller testing on real hardware and calibration against the original are done ([Driving](#driving-view-world---drive), [The HUD](#the-hud)) |
 | 6 | Audio (EA-XA, EA-XAS engine loops, MicroTalk speech), VP6 movies, FEng menus (the same FEng runtime as the HUD), in-game settings menu | in progress: the codecs, banks, music and movie decoders, Ginsu synthesis, the car sound data, the engine and effects mixers, the dynamic mixer maps, the sample (AEMS) layer of the engine and the sputters, the output device, the driven car's engine and effects, a movie player and the radio (licensed songs, gapless, play lists, pause and previous/next keys, a HUD card; not yet heard by a human) are in, and so are the front end (boot movies, title screen, main menu, option screens for audio, video and gameplay, the pause menu, free roam) and the settings written to the config file; speech (the queue and the 28 phrases with banks of their own; no sentences, no triggers) and the pursuit music (four sets steered by game state, the hooks waiting for milestone 7; not yet heard) are in, and the ambience music is open; and so is the playtest report that the engine sounds muted at the rev limiter was fixed by [PR 5](https://github.com/47PADO47/nfs-mw/pull/5); `RUST_LOG=nfsmw::audio=debug` logs the limiter ([Sound](#sound), [The front end](#the-front-end)) |
| 7 | AI racers, traffic, pursuit, races; career data; console commands to spawn AI | |
| 8 | Graphics: the car shader and lighting rig, tire smoke and skid marks (`blackbox-vehicle` already reports per-wheel `skid` and `smoke`; this draws them), exhaust flames (nitrous, gear-change blow-off and a lift-off backfire on the sputter pops; [guide](exhaust-flames.md), [spec](specs/exhaust-flames.md)), post-processing, upscaling (FSR; DLSS where the backend allows it), ReShade compatibility. Bevy Solari left milestone 8 for a later spike ([ADR 0003](decisions/0003-renderer-after-milestone-8.md)) | in progress: the offscreen HDR render pipeline with a render scale, the glossy car shader with a three-light rig and a procedural environment, optional bloom, tone mapping and FXAA, FSR 1 upscaling with a texture LOD bias, the settings and Video option rows for them, and the ReShade depth contract are in ([post-processing](post-processing.md), [upscaling](upscaling.md), [ReShade](reshade.md)); tire smoke and skid marks are in ([tire effects](tire-effects.md)); the DLSS seams (jitter, motion vectors) and the DLSS feature itself are open; swappable renderers (a Bevy backend with TAA, ray tracing, FSR 3 and DLSS behind a `blackbox-gfx` interface) are planned in [ADR 0004](decisions/0004-swappable-renderers.md) and the [plan](plans/gfx-renderers/README.md) |
| 9 | Discord Rich Presence | |
| 10 | Lan multiplayer | |
| 11 | Online multiplayer | |
| 12 | scripting API for mods | |
| 13 | Websocket/server/something for telemetry/api info on player career etc | |
| 14 | Drift mode: a handling variant with more controlled sliding at lower speeds (its own tire and assist tuning in `blackbox-vehicle`, the original has burnout and drift assists that are not modelled yet) | |

Why this order:

- **Bevy before gameplay (4).** Physics, AI, races and menus are all game objects, systems and events. Choosing the
  engine structure afterwards would mean writing them twice. The `libs/` readers stay engine-agnostic either
  way: only `blackbox-render` and the viewers would be bridged into Bevy. Bevy renders through wgpu, so the
  Vulkan / Direct3D 12 / OpenGL choice stays.
- **Settings, input, console and metrics with the foundation (4).** Every later milestone adds options,
  controls and debug commands, so they need a home first. The settings *menu* needs the front-end UI, so it
  arrives with the menus (6).
- **Graphics extras late (8).** DLSS needs NVIDIA's SDK (its own license, not MIT/Apache, so it would be an
  optional feature the user enables) and a backend that exposes it. Upscalers and ReShade want a finished
  renderer: ReShade reads the depth buffer, which is reverse-Z here, so ReShade's reversed-depth setting has to
  be documented.
- **Multiplayer and scripting (9)** need stable game state to sync and expose. **Discord Rich Presence (10)**
  is the last item on purpose.
