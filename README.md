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

There's no driving yet; see the [roadmap](docs/architecture.md#roadmap).

## Quick start

You need Rust (stable) and an installed copy of the game.

```sh
cargo run --release -p nfsmw -- --game-dir "D:/Need For Speed Most Wanted Black Edition" check-install
cargo run --release -p nfsmw -- view-world
cargo run --release -p nfsmw -- view-car BMWM3GTR --backend dx12
```

The install is found from `--game-dir`, `$NFSMW_GAME_DIR`, a `.env` file (copy `.env.example`), the per-user
config file, or the retail registry key ([details](docs/architecture.md#finding-the-install)).

| Command | What it does |
|---|---|
| `check-install` | Shows where the install was found, identifies `speed.exe`, checks required files |
| `list-cars` | Lists car folders |
| `view-car [CAR]` | A car assembled from its stock parts (wheels, brakes, paint) on a floor: drag to rotate, scroll to zoom. Options: `--lod A..E`, `--preset NAME` (a `PresetRides` car such as `CE_GTRSTREET`), `--all-parts` (every solid, unassembled) |
| `view-world` | Fly through the city: WASD, Space/C, Shift, mouse to look (Esc frees the cursor, click to capture it again), scroll for speed. Options: `--at X,Y`, `--height`, `--heading`, `--pitch`, `--fog-distance` |

Options for both viewers:

- `--backend auto|vulkan|dx12|gl` picks the graphics API ([details](docs/architecture.md#graphics-backends)).
- `--max-fps 60` caps the frame rate (default `unlocked`; `--no-vsync` turns vsync off).
- `--screenshot out.png` renders one frame and exits; add `--wait-for-load` in the world viewer.
- Esc quits (in the world viewer, the first Esc frees the mouse).

## Repository

| Path | Contents |
|---|---|
| [`libs/`](libs) | **Engine-generic libraries** for EA Black Box games (codecs, bChunk, TPK, solids, streaming, scenery, AttribSys, car parts, install discovery, renderer), each with a README, kept free of MW-specific code so they can be reused |
| [`crates/`](crates) | The game: `nfsmw-data` (how MW's files fit together) and `nfsmw` (the launcher and viewers) |
| [`docs/`](docs/README.md) | File formats, prior art, architecture, licensing, behaviour specs |
| [`tools/`](tools) | Python research tools: `chunkdump.py` dumps the bChunk tree of any data file |
| [`xtask/`](xtask) | `cargo xtask check` (leak and file-size checks), `install-hooks` |

## Credits and license

Built on a lot of community reverse-engineering work, listed with licenses in [NOTICE](NOTICE) and
[docs/research.md](docs/research.md). Rules for contributors: [CONTRIBUTING.md](CONTRIBUTING.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

*Need for Speed* and *Most Wanted* are trademarks of Electronic Arts Inc. This project is not affiliated
with or endorsed by Electronic Arts. You need your own copy of the game.
