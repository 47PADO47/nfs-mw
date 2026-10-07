# blackbox-tpk

Reader for **TPK texture packs** (plain and per-texture-compressed forms) plus CPU decoding to RGBA8
(DXT1/3/5, A8R8G8B8, P8). Record layouts are chosen by the pack's version: version 5 (NFS: Most Wanted, PC)
is implemented; other games add a file under `src/layout/`.

Spec: `docs/formats/textures.md`.

License: MIT OR Apache-2.0.
