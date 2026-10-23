# Environment prop light provenance

Independent implementation from the written behavioral specification. No
reference code, decompiled output or game assets are included.

## Sources

- Installed PC world geometry and animation tables: measured marker roles,
  model identity and toll-booth texture references. Four files were audited:
  track index, world stream, global B, and in-game A. Private evidence retains
  file hashes, scan script and record inventory. No assets are published.
- Original PC 1.3 executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`:
  read-only static analysis of scenery submission, pool allocation/reset,
  pooled flare rendering, and general flare rendering. The selected program's
  identity was confirmed before queries. No original game process was run.
- A follow-up static pass resolved the PC profile loader/type table, ordinary
  and environment-map range constants, directional flag behavior and cat's-eye
  camera cone. An installed attribute audit checked all six profile collections
  and their eight fields. Private metadata and bounded tool responses retain
  the evidence; no decompiled code or installed assets are published.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0:
  `src/Speed/Indep/Src/World/Scenery.cpp` and
  `src/Speed/Indep/Src/Ecstasy/eLight.cpp` / `eLight.hpp`.
  Generated `AttribSys/Classes/light_flares_cg_hash.h` supplied profile names;
  `Camera/Camera.hpp` cross-checked camera position/direction field meanings.
  Used for conceptual names and cross-checking marker and blink behavior.
  Original code remains outside this repository. Existing NOTICE credits apply.
- This repository's `blackbox-solid`, `blackbox-tpk`, `resident`, `residency`,
  `sign_lamps`, `car_light_flares`, and `effects/world` were inspected for gaps.

## Limits

Original behavior evidence consists of installed-data measurements and static
analysis; no original-game runtime observation or matched pixel comparison was
performed. The independent implementation has synthetic, installed-data and
native Vulkan/DX12 rendering regressions. These validate remake behavior rather
than exact original-game timing, optical parity or performance.

Full streamed toll placement tests reproduced correct surface animation on this
revision, including both native material modes. No speculative toll rendering
change was made. Main-camera effect sharing still limits secondary views.

Behavior and proposed validation are in
[the specification](../specs/environment-prop-lights.md).
