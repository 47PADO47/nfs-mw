# nfs-mw

A Rust rewrite of **Need for Speed: Most Wanted** (2005, PC v1.3 / Black Edition) that reads all game
data at runtime from **your own install**. This repository contains no game files and no decompiled code.

- **Install:** finds and checks your copy of the game.
- **Data:** decodes the data containers (bChunk, JDLZ, HUFF) and reads cars, texture packs and the streamed
  city.
- **Viewers:** shows any car assembled from its stock parts, or lets you fly through Rockport while map tiles
  stream in zone by zone, as in the game.
  Both run on Vulkan, Direct3D 12 or OpenGL, on Windows and Linux.

You can drive a car through the city (`view-world --drive`), or start from the menus (`play`: the original boot
movies, title screen and main menu, then free roam, with the pause menu and the settings screens on Esc); the rest of
the game is on the [roadmap](docs/architecture.md#roadmap).

Cars, scenery contacts, cleanup, trailer constraints and installed pursuit-breaker
sequences share a 60 Hz physics clock. See [world interaction coverage](docs/world-interactions.md)
for checks against the installed game and [remaining parity work](docs/world-interactions-todo.md).
The player and chase camera interpolate together between physics ticks; nearby
collision coverage controls loading pauses while distant scenery streams.

## Quick start

You need Rust (stable) and an installed copy of the game.

1. Build the game (run this from the repository folder):

   ```sh
   cargo build --release -p nfsmw
   ```

2. Run the setup, which asks where the game is installed and which window mode you want, and saves the answers:

   ```sh
   ./target/release/nfsmw setup
   ```

3. Start the game:

   ```sh
   ./target/release/nfsmw
   ```

On Windows the executable is `target\release\nfsmw.exe`. `nfsmw --help` lists every command and option, and the
setup steps are explained in [docs/setup.md](docs/setup.md).

The install is found from `--game-dir`, `$NFSMW_GAME_DIR`, a `.env` file (copy `.env.example`), the per-user
config file written by `setup`, or the retail registry key ([details](docs/architecture.md#finding-the-install)).

| Command | What it does |
|---|---|
| `check-install` | Shows where the install was found, identifies `speed.exe`, checks required files |
| `keys` | Lists every key, button and stick binding (the console command `keys` shows the live ones) |
| `list-cars` | Lists car folders |
| `play` (no command does the same) | The game flow: boot movies, title screen, main menu (arrows or WASD, Enter, Esc; pad: D-pad, A, B), free roam, pause menu (Esc or Start) with the audio, video and gameplay settings. `--skip-boot`, `--drive` (straight to free roam) |
| `view-screen NAME` | One of the install's menu screens on its own (`list-screens` lists them, `dump-screen NAME` prints one): `--category audio\|video\|gameplay\|controls`, `--pause`, `--options` |
| `view-car [CAR]` | A car assembled from its stock parts (wheels, brakes, paint) on a floor: drag to rotate, scroll to zoom. Options: `--lod A..E`, `--preset NAME` (a `PresetRides` car such as `CE_GTRSTREET`), `--all-parts` (every solid, unassembled) |
| `asset-viewer [CAR]` | Every car in the install in a searchable list on the left; click one to show it on the right, its wheels turning and the front pair steering. Drag to rotate, scroll to zoom, Esc quits |
| `view-world` | Fly through the city: WASD, Space/C, Shift, mouse to look (Esc frees the cursor, click to capture it again), scroll for speed. Options: `--at X,Y`, `--height`, `--heading`, `--pitch`, `--fog-distance`, and `--drive [CAR]` to drive instead (W/S, A/D, Space handbrake, E/Q gears, Left Shift nitrous, R reset, F free camera; pad: triggers, left stick) |

LAN play (milestone 10, [details](docs/specs/multiplayer/game-wiring.md)):

| Switch | What it does |
|---|---|
| `--host` | Free roam with a LAN server on this PC and you in it; others join with `--connect`. `--port N`, `--name S`, `--server-config FILE` set the server up |
| `--connect ADDR` | Joins the LAN game at `ADDR` (`192.168.1.5`, `host:9760`; the default port is 9750) |
| `--dedicated` | A LAN server without a window, sound or GPU; Ctrl-C (or `--seconds N`) stops it |
| `--discover` | Lists the LAN servers that answer, then quits |

The main menu's LAN entry hosts and joins from the game too.

Options for both viewers:

- `--backend auto|vulkan|dx12|gl` picks the graphics API ([details](docs/architecture.md#graphics-backends)).
- `--reflections off|low|medium|high` controls local car reflections (default medium); Video options
  and the F12 `reflections` command switch it live ([behavior and limits](docs/specs/car-reflections.md)).
- `--max-fps 60` caps the frame rate (default `unlocked`; `--no-vsync` turns vsync off).
- `--window-mode windowed|borderless|exclusive` chooses the window mode; Alt+Enter toggles
  fullscreen. `--monitor primary|current|INDEX` and `--resolution native|WIDTHxHEIGHT` select
  the display and size ([window settings](docs/window-modes.md)).
- `--screenshot out.png` renders one frame and exits; add `--wait-for-load` in the world viewer.
- Driving draws tire smoke and skid marks from the vehicle's contact outputs. Use
  `--no-tire-smoke` or `--no-skid-marks` to disable them ([controls and testing](docs/tire-effects.md)).
- Esc quits (in the world viewer, the first Esc frees the mouse).

## In-game console

Press **F12** to open the developer console (Esc or F12 closes it). The console shows the log
(coloured by level) above a command line with history (Up/Down) and Tab completion. While the
console is open, game input is paused and the mouse is released. Type `help` for a list of commands.

The console and the developer panels draw in the active UI style's palette, corner radius, padding and text size,
so they change with the Options menu's UI style. The font stays egui's, because the game's fonts are not the TTF
files egui loads.

Pressing Ctrl+C in the terminal that runs the game closes it cleanly, as closing the window does. A second Ctrl+C
stops it at once.

## Settings

The per-user config file lives at:

- **Windows:** `%APPDATA%\nfsmw\config\config.toml`
- **Linux:** `$XDG_CONFIG_HOME/nfsmw/config.toml` (usually `~/.config/nfsmw/config.toml`)

Run `nfsmw check-install` to see the exact path on your system. The file uses [TOML](https://toml.io)
and lets you set the game directory, graphics backend, frame-rate cap, and other options without
passing CLI flags every time.

The Controls menu includes independent stick/trigger deadzones, sensitivity and camera inversion.
Keyboard, mouse and gamepad assignments can be changed and saved through the console or config file
([controller settings](docs/controller-settings.md)).
Gameplay also offers [HUD Layout](docs/hud-layout.md): PC widescreen placement,
the earlier centered layout, or the Xbox addon's 92% wide scale.
Xbox menus use bundled Kenney CC0 prompts that follow the live bindings and active input device.
A accepts, B goes back, Menu/Start pauses or resumes, and View/Back quits from the main menu.
Idle stick drift and console hotkeys do not switch the prompts. Losing the active controller pauses driving.

The UI style (Original, Custom or Remastered) in the Options menu sets the look of the menus and the HUD, and the
developer panels follow it. The world map has a Remastered overlay, with its own legend and the place and pool
blips restyled.

Video options include opt-in [experimental ambient occlusion](docs/ambient-occlusion.md), with quality,
intensity and a live split comparison. Start with `nfsmw play --ao medium`; it needs only base-game assets.

LAN play has its own keys (`net_name`, `net_port`, `net_snapshot_hz`, `net_interp_delay`, `net_show_stats`,
`net_identity_file`) and console commands (`net.host`, `net.connect`, `net.stats`, `net.sim`, ...); the main menu's LAN icon
opens its menu ([LAN front end](docs/specs/multiplayer/frontend.md)).

## Mods

The game loads native mods from one folder per mod under its config folder (`%APPDATA%\nfsmw\config\mods\<id>\` on
Windows, `~/.config/nfsmw/mods/<id>/` on Linux). A mod holds a `mod.toml`, its library and, optionally, files that are
layered over your install and cars that are added to the game. Mods run with your privileges, so install only ones you
trust. `--no-mods` (or `mods = "off"` in `config.toml`) loads none. How to write and use mods:
[docs/modding/README.md](docs/modding/README.md).

### The NFS: Most Wanted (2012) cars

`nfsmw-2012-content` adds the cars of **NFS: Most Wanted (2012)** to this game, converted on your PC from your own
2012 install (nothing from either game is in the repository). To use it:

1. Build it and copy it into the game's mods folder (`%APPDATA%\nfsmw\config\mods\nfsmw-2012-content\`, or
   `~/.config/nfsmw/mods/nfsmw-2012-content/` on Linux) with `cargo install-mod nfsmw-2012-content` (`cargo install-mods`
   does every mod of the workspace). Run it again after a change, with the game closed.
2. Start the game once; the mod writes a commented `config.toml` next to its `mod.toml`.
3. Set your 2012 install in it and start the game again:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'
   ```

The cars are converted in the background (a few minutes the first time, about 740 MB written into the mod's folder) and then show
up in the garage and the career, with their own paints, liveries, sounds and camera views. Without a `source` the
mod adds nothing. Settings, what is converted and the known limits:
[mods/nfsmw-2012-content/README.md](mods/nfsmw-2012-content/README.md).

## Repository

| Path | Contents |
|---|---|
| [`libs/`](libs) | **Engine-generic libraries** for EA Black Box games (codecs, bChunk, TPK, solids, streaming, scenery, AttribSys, car parts, install discovery, renderer), each with a README, kept free of MW-specific code so they can be reused |
| [`crates/`](crates) | The game: `nfsmw-data` (how MW's files fit together) and `nfsmw` (the launcher and viewers) |
| [`docs/`](docs/README.md) | File formats, prior art, architecture, licensing, behaviour specs |
| [`tools/`](tools) | Python research tools: `chunkdump.py` dumps the bChunk tree of any data file |
| [`xtask/`](xtask) | `cargo xtask check` (leak and file-size checks), `install-hooks` |

## Development

See [docs/development.md](docs/development.md) for the full setup guide (required tools,
first-time setup, CI, reverse-engineering toolchain).

The world-interaction branch's base, local mise/LLVM-MinGW build setup and measured
physics coverage are recorded in [world interactions](docs/world-interactions.md).
Cars, traffic and dynamic scenery share a [fixed physics clock](docs/physics-timing.md).
Full original-game interaction parity remains in progress.

## Credits and license

Built on a lot of community reverse-engineering work, listed with licenses in [NOTICE](NOTICE) and
[docs/research.md](docs/research.md). Rules for contributors: [CONTRIBUTING.md](CONTRIBUTING.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

*Need for Speed* and *Most Wanted* are trademarks of Electronic Arts Inc. This project is not affiliated
with or endorsed by Electronic Arts. You need your own copy of the game.
