# Architecture

The Rust rewrite reads every asset from the user's own install at runtime and ships no game data. This
page describes the crate layout, how the install is found, the graphics backends, multi-platform
support and testing. The structure follows [vladtrc/iw4L](https://github.com/vladtrc/iw4L) (see
[research.md § 8](research.md#8-reference-projects-for-the-rust-rewrite)), scaled down.

## Crates

```
                       nfsmw  (launcher binary: CLI, window, camera)
                 ┌───────┼──────────────┬──────────────┐
           nfsmw-install  nfsmw-render   nfsmw-geometry  nfsmw-texture
           (only crate     (wgpu; no      └──────┬──────────┘
            that reads     game knowledge)     nfsmw-bchunk   nfsmw-compress   nfsmw-hash
            game files)
```

| Crate | Job | Reads files? |
|---|---|---|
| [`nfsmw-hash`](../crates/nfsmw-hash) | `bStringHash` | no |
| [`nfsmw-compress`](../crates/nfsmw-compress) | JDLZ, HUFF, RAWW wrappers ([bchunk.md](formats/bchunk.md), [huff.md](formats/huff.md)) | no |
| [`nfsmw-bchunk`](../crates/nfsmw-bchunk) | Zero-copy bChunk tree over a byte slice; alignment; bare JDLZ blobs | no |
| [`nfsmw-geometry`](../crates/nfsmw-geometry) | Solids: header, shading groups, vertices, indices ([models.md](formats/models.md)) | no |
| [`nfsmw-texture`](../crates/nfsmw-texture) | TPK packs, both forms; CPU decode to RGBA8 ([textures.md](formats/textures.md)) | no |
| [`nfsmw-install`](../crates/nfsmw-install) | Finding, validating and indexing the install; case-insensitive reads | **yes, the only one** |
| [`nfsmw-render`](../crates/nfsmw-render) | Renderer behind a backend-neutral API (meshes, textures, draw ranges) | no |
| [`nfsmw`](../crates/nfsmw) | The binary: `check-install`, `list-cars`, `view-car` | through `nfsmw-install` |
| [`xtask`](../xtask) | `cargo xtask leak-check`, `cargo xtask install-hooks` | repo files only |

Rules that keep this structure working:

- **Format crates take `&[u8]` and never open files.** They can be tested on synthetic bytes and fuzzed.
- **Only `nfsmw-install` touches the install.** It is the single place that handles paths, case and
  platform differences.
- **`nfsmw-render` has no game knowledge, and its API has no wgpu types.** A second implementation (for
  example Direct3D 11) can slot in behind the same API.
- **Game rules will live in their own pure crates** (physics, AI, pursuit). They will have no rendering or
  windowing dependencies and will be built spec-first ([licensing.md](licensing.md#spec-first)).

## Finding the install

[`nfsmw-install`](../crates/nfsmw-install/src/discover.rs) looks in this order and uses the first hit:

1. `--game-dir <PATH>` on the command line;
2. the `NFSMW_GAME_DIR` environment variable;
3. `NFSMW_GAME_DIR=<PATH>` in a `.env` file in the working directory or next to the executable (copy
   [`.env.example`](../.env.example); `.env` is never committed);
4. `game_dir = "<PATH>"` in the per-user config file: `%APPDATA%\nfsmw\config\config.toml` on Windows,
   `~/.config/nfsmw/config.toml` on Linux;
5. on Windows, `HKLM` (then `HKCU`) `\SOFTWARE\EA GAMES\Need for Speed Most Wanted`, value `Install Dir`,
   read from the 32-bit registry view. This is the retail installer's key. Repacks often don't write it.

**Validation.** The required files are listed in `REQUIRED_FILES`. `speed.exe` is hashed and compared
with known builds (PC v1.3 = `80774c2e…1d253c`). The exe is not needed yet: later it will be needed for the
shaders embedded in it.

**Case-insensitive access.** The game was written for Windows and refers to files in any case, and the
shipped names mix cases (`GlobalB.lzc`, `GLOBALA.BUN`). `GameDir` indexes the install once
(about 2,000 files) and resolves every lookup through a lower-cased key, so the same code works on Linux's
case-sensitive filesystems.

Run `nfsmw check-install` to see what was found and why.

## Graphics backends

Select with `--backend <auto|vulkan|dx12|dx11|gl>`:

| Backend | Windows | Linux | Implementation | Status |
|---|---|---|---|---|
| `auto` | Vulkan or D3D12, else GL | Vulkan, else GL | wgpu | working |
| `vulkan` | ✔ | ✔ | wgpu | working |
| `dx12` | ✔ | — | wgpu | working |
| `gl` | ✔ (WGL) | ✔ (EGL, X11/Wayland) | wgpu | working |
| `dx11` | ✔ | — | **not implemented** | planned |

All four working combinations render the same image of the BMW M3 GTR on the development machine
(RTX 4070 SUPER).

**Why Direct3D 11 is separate.** wgpu removed its D3D11 backend in 2023 and v30 (the version used here)
has none. Supporting D3D11 therefore needs its own implementation of the `nfsmw-render` API. The plan:

- a `nfsmw-render-d3d11` crate on the [`windows`](https://crates.io/crates/windows) crate (MIT OR
  Apache-2.0), implementing the same `create_texture` / `create_mesh` / `render` API;
- shaders kept in one place: the WGSL is translated to HLSL (SM 5.0) at build time with
  [naga](https://crates.io/crates/naga), which wgpu already uses, and compiled with `D3DCompile`;
- the `dx11` option dispatching to it on Windows, and erroring elsewhere as it does today.

This is worth doing once the renderer API has settled (after world rendering), so it is implemented
once, against the final API.

## Multi-platform

- **Targets:** Windows (x86_64-pc-windows-msvc) and Linux (x86_64-unknown-linux-gnu). CI builds and tests
  both ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- **No system libraries needed at build time.** winit and wgpu load X11, Wayland, Vulkan and EGL
  dynamically. Linux needs a Vulkan or GL driver at runtime.
- **Linux installs:** point `--game-dir`, `.env` or the config file at the game folder, for example inside
  a Wine prefix (`~/.wine/drive_c/Program Files (x86)/EA GAMES/Need for Speed Most Wanted`). There is no
  registry lookup on Linux.
- **Endianness:** PC data is little-endian; every reader decodes explicitly with `from_le_bytes`, so
  big-endian hosts would also work.

## Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` in every crate | no | yes |
| Real-install tests | [`crates/nfsmw/tests/real_install.rs`](../crates/nfsmw/tests/real_install.rs) (`#[ignore]`) | yes | no |
| Leak check | `cargo xtask leak-check` | no | yes, first job |
| Python tool tests | [`tests/`](../tests) | no | yes |

Run the real-install tests with:

```sh
NFSMW_GAME_DIR="D:/Need For Speed Most Wanted Black Edition" cargo test -p nfsmw -- --ignored
```

They currently check that all 100 cars (15,781 solids) parse with every index in range, that the BMW
M3 GTR matches the numbers in [models.md](formats/models.md), and that the texture packs in `CARS/`,
`GLOBAL/` and `FRONTEND/` decode.

`nfsmw view-car <CAR> --screenshot out.png` renders one frame off-screen. Use it to check rendering
changes and backends without a visible window.

## Roadmap

| Milestone | Content | Status |
|---|---|---|
| 1 | Workspace, guards, install discovery, bChunk/JDLZ/HUFF, solids, TPK, car viewer on Vulkan/DX12/GL | **done** |
| 2 | World: L2RA streaming sections, scenery placement, a free-fly camera through the city | next |
| 3 | AttribSys reader; car assembly from the parts DB (wheels, kits, paint); the game's lighting model | |
| 4 | Vehicle physics, spec-first (`docs/specs/vehicle-physics.md`); drive a car on the world collision | |
| 5 | Audio (EA-XA, EA-XAS engine loops, MicroTalk speech), VP6 movies, FEng menus | |
| 6 | AI racers, traffic, pursuit; career data | |
| — | Direct3D 11 backend (see above) | planned |
