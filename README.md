# nfs-mw

A Rust rewrite of **Need for Speed: Most Wanted** (2005, PC v1.3 / Black Edition) that reads all game
data at runtime from **your own install**. This repository contains no game files and no decompiled code.

**Status: milestone 1.** It finds and checks the install, decodes the data containers (bChunk, JDLZ,
HUFF), reads car models and texture packs, and shows any car in a window on Vulkan, Direct3D 12 or OpenGL.
It runs on Windows and Linux. No driving yet; see the [roadmap](docs/architecture.md#roadmap).

## Quick start

You need Rust (stable) and an installed copy of the game.

```sh
cargo run --release -p nfsmw -- --game-dir "D:/Need For Speed Most Wanted Black Edition" check-install
cargo run --release -p nfsmw -- view-car BMWM3GTR
cargo run --release -p nfsmw -- view-car BMWM3GTR --backend dx12
```

The install is found from `--game-dir`, `$NFSMW_GAME_DIR`, a `.env` file (copy `.env.example`), the
per-user config file, or the retail registry key. Details:
[docs/architecture.md](docs/architecture.md#finding-the-install).

| Command | What it does |
|---|---|
| `check-install` | Shows where the install was found, identifies `speed.exe`, checks required files |
| `list-cars` | Lists car folders |
| `view-car [CAR]` | Orbit view of a car (drag to rotate, scroll to zoom, Esc to quit). Options: `--backend auto\|vulkan\|dx12\|dx11\|gl`, `--lod A..D`, `--all-parts`, `--screenshot out.png` |

`dx11` is accepted but not implemented yet: wgpu has no Direct3D 11 backend, so it needs its own
renderer ([plan](docs/architecture.md#graphics-backends)).

## Repository

| Path | Contents |
|---|---|
| [`crates/`](crates) | The Rust workspace ([architecture](docs/architecture.md)) |
| [`docs/`](docs/README.md) | File formats, prior art, architecture, licensing |
| [`tools/`](tools) | Python research tools: `chunkdump.py` dumps the bChunk tree of any data file |
| [`xtask/`](xtask) | `cargo xtask leak-check` / `install-hooks` |

## Credits and license

Built on a lot of community reverse-engineering work, listed with licenses in [NOTICE](NOTICE) and
[docs/research.md](docs/research.md). Rules for contributors: [CONTRIBUTING.md](CONTRIBUTING.md).

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

*Need for Speed* and *Most Wanted* are trademarks of Electronic Arts Inc. This project is not affiliated
with or endorsed by Electronic Arts. You need your own copy of the game.
