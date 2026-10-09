# nfs-mw

A Rust rewrite of **Need for Speed: Most Wanted** (2005, PC v1.3 / Black Edition) that reads all game
data at runtime from **your own install**. This repository contains no game files and no decompiled code.

**Status: milestone 3 done; milestone 4 (engine foundation) next.**

- **Install:** finds and checks your copy of the game.
- **Data:** decodes the data containers (bChunk, JDLZ, HUFF) and reads cars, texture packs and the streamed
  city.
- **Viewers:** shows any car assembled from its stock parts, or lets you fly through Rockport while map tiles
  stream in zone by zone, as in the game.
  Both run on Vulkan, Direct3D 12 or OpenGL, on Windows and Linux.

You can drive a car through the city (`view-world --drive`), or start from the menus (`play`: the original boot
movies, title screen and main menu, then free roam, with the pause menu and the settings screens on Esc); the rest of
the game is on the [roadmap](docs/architecture.md#roadmap).

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
| `view-screen NAME` | One of the install's menu screens on its own (`list-screens` lists them, `dump-screen NAME` prints one): `--category audio\|video\|gameplay`, `--pause`, `--options` |
| `view-car [CAR]` | A car assembled from its stock parts (wheels, brakes, paint) on a floor: drag to rotate, scroll to zoom. Options: `--lod A..E`, `--preset NAME` (a `PresetRides` car such as `CE_GTRSTREET`), `--all-parts` (every solid, unassembled) |
| `view-world` | Fly through the city: WASD, Space/C, Shift, mouse to look (Esc frees the cursor, click to capture it again), scroll for speed. Options: `--at X,Y`, `--height`, `--heading`, `--pitch`, `--fog-distance`, and `--drive [CAR]` to drive instead (W/S, A/D, Space handbrake, E/Q gears, Left Shift nitrous, R reset, F free camera; pad: triggers, left stick) |

Options for both viewers:

- `--backend auto|vulkan|dx12|gl` picks the graphics API ([details](docs/architecture.md#graphics-backends)).
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

## Settings

The per-user config file lives at:

- **Windows:** `%APPDATA%\nfsmw\config\config.toml`
- **Linux:** `$XDG_CONFIG_HOME/nfsmw/config.toml` (usually `~/.config/nfsmw/config.toml`)

Run `nfsmw check-install` to see the exact path on your system. The file uses [TOML](https://toml.io)
and lets you set the game directory, graphics backend, frame-rate cap, and other options without
passing CLI flags every time.

The Gameplay menu includes independent stick/trigger deadzones, sensitivity and camera inversion.
Keyboard, mouse and gamepad assignments can be changed and saved through the console or config file
([controller settings](docs/controller-settings.md)).

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

## Credits and license

Built on a lot of community reverse-engineering work, listed with licenses in [NOTICE](NOTICE) and
[docs/research.md](docs/research.md). Rules for contributors: [CONTRIBUTING.md](CONTRIBUTING.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

*Need for Speed* and *Most Wanted* are trademarks of Electronic Arts Inc. This project is not affiliated
with or endorsed by Electronic Arts. You need your own copy of the game.
