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
| [`blackbox-vehicle`](../libs/blackbox-vehicle) | Deterministic fixed-step vehicle physics: rigid body, engine and gearbox, suspension, tires, steering, aero; driven by plain parameter structs and a `Ground` ray-cast trait | — (parameters passed by the caller) |
| [`blackbox-render`](../libs/blackbox-render) | Backend-neutral renderer (wgpu inside) | — |
| [`blackbox-scene`](../libs/blackbox-scene) | Uploading solids and textures to the renderer; boxes; frustum culling | — |
| [`game-install`](../libs/game-install) | Finding, validating and reading an install, case-insensitively | driven by a `GameSpec` |

### Game crates

| Crate | Job |
|---|---|
| [`nfsmw-data`](../crates/nfsmw-data) | MW's `GameSpec`; car assembly (stock and preset parts, wheel and brake placement, paint, texture swaps); the world: streaming index, section parsing, a background section loader. Renderer-free. |
| [`nfsmw`](../crates/nfsmw) | The binary: CLI (`commands/`), layered `settings/`, the Bevy `app/` (window, loop, render bridge, cursor, pacing, screenshots), the `input/` action layer, the `viewer/` cameras and `Scene` trait, and the scenes (`scenes/car/`, `scenes/world/`). |
| [`xtask`](../xtask) | `cargo xtask check` (leak check + file-size check), `install-hooks` |

Rules that keep this structure working:

- **Format crates take `&[u8]` and never open files.** Only `game-install` touches the install.
- **The renderer has no game knowledge**, and its API has no wgpu types, so the implementation behind it
  can change (for example to Bevy's renderer) without touching callers.
- **Files stay small:** no source or doc file over 500 lines (`cargo xtask size-check`). Split by domain
  into folders and modules, with one struct or concern per file.
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
  dead zone and per-second scaling for sticks; rebinding will replace that resource. Default pad layout:
  left stick moves, right stick looks (and orbits), A/B go up/down, stick-click or right bumper boosts,
  D-pad up/down zooms, Start backs out.
- **Cursor:** mouse-look scenes capture the cursor; the first Esc (or Start) releases it, the next quits.
- **Errors** from systems (no GPU, a failed present) are stored and returned from `main`; the app exits with
  an error code.

## Developer tools

- **UI layer.** `blackbox-render` draws a 2D layer over the scene: textured, clipped, premultiplied-alpha
  triangles (`UiLayer`, `UiTexturePatch`). It knows nothing about egui; the front-end menus of milestone 6
  will use the same layer.
- **egui host** (`gui/`): turns Bevy keyboard, mouse and wheel messages into egui events and egui's output
  into that layer. The panels only see an `egui::Context`.
- **Metrics** (`devtools/`): `Metrics` is plain data (240 frame times, GPU name, mesh and texture counts).
  `--show-metrics basic` draws fps and frame time; `advanced` adds the 1% low, worst frame, a frame-time graph
  with a 60 fps line, resource counts and the scene status.
- **Console** (F12; Esc or F12 closes it): the log (the last 1,000 lines, coloured by level, kept by a
  logger that wraps `env_logger`) above a command line with history (Up/Down) and Tab completion. The
  keyboard belongs to the console while it is open: game actions go quiet and the mouse is released
  (and recaptured on close). Typed lines are parsed into a `Command` (plain data, unit-tested) and run by
  one system, so the console never touches the renderer itself.
  - Built in: `help`, `clear`, `quit`, `get [setting]`, `set <setting> <value>` (`fps`, `vsync`, `metrics`),
    the shorthands `fps 60` / `vsync off` / `metrics advanced`, and `resolution <w> <h>`. Changes last for
    the run; the config file is not written.
  - The car viewer adds `car <folder>`, `cars` and `freecam` (orbit ↔ free camera). Scenes offer commands
    through `Scene::commands` and `Scene::command`; "spawn AI" arrives with milestone 7.
  - `--exec "<command>"` (repeatable) runs commands at startup, like Quake's `+exec`; `--open-console`
    (hidden) starts with the console open, which is how the screenshots in bug reports show it.
- **Order of a frame:** `Prepare` (renderer, cursor, size) → `SceneUpdate` → `Ui` (egui pass) → `Draw`
  (the bridge uploads texture patches, sets the layer, renders). `--screenshot` runs a few frames first so
  the overlay is in the picture.

## Settings

Runtime options resolve in layers, highest first ([`crates/nfsmw/src/settings/`](../crates/nfsmw/src/settings)):

1. the command line (`--backend`, `--no-vsync`, `--max-fps`, `--show-metrics`);
2. environment variables (`NFSMW_BACKEND`, `NFSMW_VSYNC`, `NFSMW_MAX_FPS`, `NFSMW_SHOW_METRICS`);
3. the per-user config file (`backend`, `vsync`, `max_fps`, `show_metrics`; the same file as `game_dir`);
4. the defaults (`auto`, vsync on, unlocked, overlay off).

Each key resolves on its own. A value that does not parse (in the environment or the file) is logged and
skipped, so the next layer applies; a broken config file never stops the game from starting. The
install directory has its own, longer lookup ([Finding the install](#finding-the-install)).

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
  all of it (default 3 km); placeholder until the game's fog is known.
- **Camera:** free-fly (WASD, Space/C, Shift, right-drag to look, scroll for speed). It starts above the
  centre of the city, or at `--at X,Y`, at `--height` metres above the ground; the ground is estimated from
  the scenery boxes until collision is loaded.

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

## Graphics backends

Select with `--backend <auto|vulkan|dx12|gl>`:

| Backend | Windows | Linux | Implementation | Status |
|---|---|---|---|---|
| `auto` | Vulkan or D3D12, else GL | Vulkan, else GL | wgpu | working |
| `vulkan` | ✔ | ✔ | wgpu | working |
| `dx12` | ✔ | — | wgpu | working |
| `gl` | ✔ (WGL) | ✔ (EGL, X11/Wayland) | wgpu | working |

Direct3D 11 is not offered: wgpu removed its D3D11 backend in 2023. Direct3D 12 and Vulkan cover the same
hardware on Windows 10 and later, and OpenGL covers older GPUs.

## Multi-platform

- **Targets:** Windows (x86_64-pc-windows-msvc) and Linux (x86_64-unknown-linux-gnu). CI builds and tests
  both ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- **One system package at build time, on Linux: `libudev-dev` (and `pkg-config`)**, which gamepad support (gilrs) links against.
  winit (inside `bevy_winit`) and wgpu load X11, Wayland, Vulkan and EGL dynamically, so those need nothing to build.
- **Linux installs:** point `--game-dir`, `.env` or the config file at the game folder (for example in a
  Wine prefix). There is no registry lookup on Linux.
- **Endianness:** every reader decodes explicitly with `from_le_bytes`.

## Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` / `tests` modules in every crate | no | yes |
| Real-install tests | [`crates/nfsmw-data/tests/real_install/`](../crates/nfsmw-data/tests/real_install) (`#[ignore]`) | yes | no |
| Leak + size checks | `cargo xtask check` | no | yes, first job |
| Python tool tests | [`tests/`](../tests) | no | yes |

```sh
NFSMW_GAME_DIR="D:/Need For Speed Most Wanted Black Edition" cargo test --release -p nfsmw-data -- --ignored
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

## Roadmap

| Milestone | Content | Status |
|---|---|---|
| 1 | Workspace, guards, install discovery, bChunk/JDLZ/HUFF, solids, TPK, car viewer on Vulkan/DX12/GL | done |
| 2 | Generic `libs/` split; the streamed city: index, sections, scenery, background loading, instanced rendering, culling, fly camera | done |
| 3 | Sky dome, LODs, water, panoramas; zone-based streaming (visible sections); AttribSys reader; car assembly from the parts DB (stock parts, wheels, brakes, paint). Playtest fixes: misplaced and floating scenery, mouse look without holding a button, `--max-fps`, clearer config-file path | done |
| 4 | Engine foundation: decide on Bevy (ECS, events, UI) in an ADR and migrate the viewers if adopted; layered settings (command line > environment > per-user config file > defaults, with a settings menu in 6); input layer with controller support; developer console (F12: log view, commands such as change car, toggle free camera, change settings); performance overlay (`--show-metrics off\|basic\|advanced`). Decided: Bevy as the shell with our renderer ([ADR 0001](decisions/0001-bevy.md)), full Bevy renderer revisited in 8 | done |
| 5 | Vehicle physics, spec-first (`docs/specs/vehicle-*.md`); world collision (`CarpWCollisionPack`); drive a car with the original HUD: read the FEng HUD packages (`HUD_*.fng` in `InGameB.bun`) and draw them with the UI layer; steering wheel controller support (wheel axes, pedals, shifters) on the input layer from 4 | |
| 6 | Audio (EA-XA, EA-XAS engine loops, MicroTalk speech), VP6 movies, FEng menus (the same FEng runtime as the HUD), in-game settings menu | |
| 7 | AI racers, traffic, pursuit, races; career data; console commands to spawn AI | |
| 8 | Graphics: the car shader and lighting rig, post-processing, upscaling (FSR; DLSS where the backend allows it), ReShade compatibility, Bevy Solari | |
| 9 | LAN multiplayer; scripting API for mods | |
| 10 | Discord Rich Presence | |

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
