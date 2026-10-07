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
| [`blackbox-hash`](../libs/blackbox-hash) | `bStringHash` | — |
| [`blackbox-chunk`](../libs/blackbox-chunk) | Zero-copy bChunk trees; chunk ids by domain | — |
| [`blackbox-tpk`](../libs/blackbox-tpk) | Texture packs (plain and compressed forms), CPU decoding | `layout/` per TPK version (5 = MW) |
| [`blackbox-solid`](../libs/blackbox-solid) | Solids: groups, every vertex buffer, indices | `layout/` per `SolidInfo` version (0x16 = MW) |
| [`blackbox-streaming`](../libs/blackbox-streaming) | The track streaming index | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-scenery`](../libs/blackbox-scenery) | Scenery infos and instances; visibility rule | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-render`](../libs/blackbox-render) | Backend-neutral renderer (wgpu inside) | — |
| [`blackbox-scene`](../libs/blackbox-scene) | Uploading solids and textures to the renderer; boxes; frustum culling | — |
| [`game-install`](../libs/game-install) | Finding, validating and reading an install, case-insensitively | driven by a `GameSpec` |

### Game crates

| Crate | Job |
|---|---|
| [`nfsmw-data`](../crates/nfsmw-data) | MW's `GameSpec`; car loading (part selection, texture sources); the world: streaming index, section parsing, a background section loader. Renderer-free. |
| [`nfsmw`](../crates/nfsmw) | The binary: CLI (`commands/`), a generic viewer (`viewer/`: window, input, orbit and fly cameras, screenshots) and the scenes (`scenes/car.rs`, `scenes/world/`). |
| [`xtask`](../xtask) | `cargo xtask check` (leak check + file-size check), `install-hooks` |

Rules that keep this structure working:

- **Format crates take `&[u8]` and never open files.** Only `game-install` touches the install.
- **The renderer has no game knowledge**, and its API has no wgpu types, so a second implementation
  (Direct3D 11) can sit behind it.
- **Files stay small:** no source or doc file over 500 lines (`cargo xtask size-check`). Split by domain
  into folders and modules, with one struct or concern per file.
- **Game rules will live in their own pure crates** (physics, AI), built spec-first ([licensing.md](licensing.md#spec-first)).

## Finding the install

[`game-install`](../libs/game-install/src/discover/mod.rs) looks in this order and uses the first hit; the
names come from MW's `GameSpec` ([`nfsmw-data/src/game.rs`](../crates/nfsmw-data/src/game.rs)):

1. `--game-dir <PATH>`;
2. `NFSMW_GAME_DIR`;
3. `NFSMW_GAME_DIR=<PATH>` in a `.env` file in the working directory or next to the executable;
4. `game_dir = "<PATH>"` in the per-user config file (`%APPDATA%\nfsmw\config\config.toml` /
   `~/.config/nfsmw/config.toml`);
5. on Windows, `HKLM` / `HKCU` `\SOFTWARE\EA GAMES\Need for Speed Most Wanted` → `Install Dir` (32-bit view).

The required files are checked, and `speed.exe` is hashed and identified (v1.3 = `80774c2e…1d253c`).
`GameDir` indexes the install once and resolves every path case-insensitively, so the same code works on
Linux. `nfsmw check-install` shows what was found.

## The streamed city (`view-world`)

```
TRACKS/L2RA.BUN ──► blackbox-streaming: 720 sections (605 map tiles, 115 shared sets)
TRACKS/STREAML2RA.BUN ─► loader threads (nfsmw-data::world::Streamer)
                          read a section's byte range → blackbox-solid / -tpk / -scenery
                        ─► render thread (scenes/world/residency.rs)
                          upload textures → meshes → place instances
```

- **Shared sets (V/X/Y/Z) load once, at startup**, all together, because their models and textures refer to
  each other (2,734 models, 2,111 textures).
- **Map tiles stream by distance.** Tiles within `--load-radius` (default 450 m) of the camera are requested
  nearest first, at most 8 in flight, on 2–6 worker threads. Up to 2 tiles are uploaded per frame. Tiles
  beyond 1.25 × radius + 50 m are released (GPU meshes and textures freed).
- **Resolution:** a tile's instances use the tile's own models first, then the shared sets. Measured on the
  whole stream, that resolves 77,722 of 77,776 tile instances; none needs another tile
  ([maps.md](formats/maps.md#scenery-placing-models-in-the-world)).
- **Map-wide tiles:** the 37 tiles with a radius over 1 km (the ocean planes, distant panoramas; 8 MB) are
  always resident.
- **Visibility:** instances hidden by their exclude flags (race barriers, animated props) are dropped when
  placed ([specs/scenery-visibility.md](specs/scenery-visibility.md)). Each frame the remaining instances are
  frustum-culled using their stored world boxes. Then the game's LOD rule picks slot 0 or slot 2, or nothing
  under 17 px, which is also the real draw distance ([specs/scenery-lod.md](specs/scenery-lod.md)).
  Finally the instances are sorted by mesh and drawn instanced.
- **Sky:** the `SKYDOME` and `SKYDOME_XENON` scenery models, textured from `GLOBAL/InGameA.bun`, drawn
  with the fog-free sky shading. Texture animations (water, signals) advance every frame. Depth is reverse-Z with an infinite far plane, so the 9.7 km dome is never clipped.
- **Shading:** world geometry is pre-lit (vertex colour × 2, no sun); blending follows each texture's
  `AlphaBlendType` ([textures.md](formats/textures.md#alpha)). Linear fog from 0.8× to 1.6× the load radius
  (default 700 m) hides the streaming edge.
- **Camera:** free-fly (WASD, Space/C, Shift, right-drag to look, scroll for speed). It starts above the
  centre of the city, or at `--at X,Y`, at `--height` metres above the ground; the ground is estimated from
  the scenery boxes until collision is loaded.

Known gaps, for later milestones:

- **Not drawn yet:** `SKY_SPECULAR`, water reflections, cars and traffic, and the world animations
  (cranes, the airliner).
- **Approximations:**
  - the LOD pixel scale assumes a 480-line reference screen;
  - no scenery overrides, so race barriers are never shown;
  - placeholder lighting instead of the game's `fx` effects and time of day.

## Graphics backends

Select with `--backend <auto|vulkan|dx12|dx11|gl>`:

| Backend | Windows | Linux | Implementation | Status |
|---|---|---|---|---|
| `auto` | Vulkan or D3D12, else GL | Vulkan, else GL | wgpu | working |
| `vulkan` | ✔ | ✔ | wgpu | working |
| `dx12` | ✔ | — | wgpu | working |
| `gl` | ✔ (WGL) | ✔ (EGL, X11/Wayland) | wgpu | working |
| `dx11` | ✔ | — | **not implemented** | planned |

**Why Direct3D 11 is separate.** wgpu removed its D3D11 backend in 2023, and v30 has none. The plan:

- a `blackbox-render-d3d11` library on the [`windows`](https://crates.io/crates/windows) crate (MIT OR
  Apache-2.0), implementing the same API;
- the same `scene.wgsl`, translated to HLSL SM 5.0 at build time with [naga](https://crates.io/crates/naga)
  and compiled with `D3DCompile`;
- `--backend dx11` dispatching to it on Windows.

This is best done once the renderer API has settled, so it is implemented once.

## Multi-platform

- **Targets:** Windows (x86_64-pc-windows-msvc) and Linux (x86_64-unknown-linux-gnu). CI builds and tests
  both ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- **No system libraries at build time.** winit and wgpu load X11, Wayland, Vulkan and EGL dynamically.
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
  resolves.

The whole stream parses in under a second in release builds.

`nfsmw view-car … --screenshot out.png` and `nfsmw view-world … --wait-for-load --screenshot out.png`
render one frame off-screen. Use them to check rendering changes and backends without a window.

## Roadmap

| Milestone | Content | Status |
|---|---|---|
| 1 | Workspace, guards, install discovery, bChunk/JDLZ/HUFF, solids, TPK, car viewer on Vulkan/DX12/GL | done |
| 2 | Generic `libs/` split; the streamed city: index, sections, scenery, background loading, instanced rendering, culling, fly camera | **done** |
| 3 | Sky dome, LODs, water, map-wide tiles; AttribSys reader; car assembly from the parts DB (wheels, kits, paint) | **in progress** (sky, LODs, water done) |
| 4 | Vehicle physics, spec-first (`docs/specs/vehicle-physics.md`); world collision (`CarpWCollisionPack`); drive a car | |
| 5 | Audio (EA-XA, EA-XAS engine loops, MicroTalk speech), VP6 movies, FEng menus | |
| 6 | AI racers, traffic, pursuit; career data | |
| — | Direct3D 11 backend | planned |
