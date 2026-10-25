# blackbox-tpk

Reader and writer for **TPK texture packs** (plain and per-texture-compressed forms) plus CPU decoding to RGBA8
(DXT1/3/5, A8R8G8B8, P8). Record layouts are chosen by the pack's version: version 5 (NFS: Most Wanted, PC)
is implemented; other games add a file under `src/layout/`.

Writer: `write_tpk` builds a version 5 pack (the streamed car `TEXTURES.BIN` form or the plain form) from
encoded textures; the reader parses it back to the same textures.

Spec: `docs/formats/textures.md`.

License: MIT OR Apache-2.0.
