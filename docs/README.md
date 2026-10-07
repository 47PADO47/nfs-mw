# nfs-mw docs

Documentation for the Need for Speed: Most Wanted (2005) reverse-engineering / modding work
in this repo: how the game stores its models, maps, textures and animations, verified against the
PC v1.3 "Black Edition" install at `D:/Need For Speed Most Wanted Black Edition`.

## Short answers

- **Language:** C++, built with Visual C++ .NET 2003 on Direct3D 9, using EA Black Box's in-house
  engine. → [research.md](research.md)
- **Container:** a tree of **bChunks** (`u32 id, u32 size, payload`; bit 31 = container), sometimes
  JDLZ-compressed. → [formats/bchunk.md](formats/bchunk.md)
- **Models:** "solids" in `GeometryPack` chunks, with 104-byte material records, 36-byte (static) or
  60-byte (skinned) vertices and u16 indices. → [formats/models.md](formats/models.md)
- **Maps:** one world (`L2RA`), made of a metadata file plus a 533 MB stream of 720 sections indexed
  by 92-byte `TrackStreamingSection` records. Models are placed by name hash.
  → [formats/maps.md](formats/maps.md)
- **Animation:** EAGL4 skeletons and animation banks stored as **ELF32 MIPS relocatable objects**,
  with bone names as symbols. There is no public parser yet. → [formats/animation.md](formats/animation.md)

## Contents

| Doc | What it covers | Status |
|---|---|---|
| [TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md) | The RE/modding toolchain (Ghidra, Ghidra MCP, ReAgent + differential gate, MSVC, uv), the project's own scripts, and the relevant Claude Code skills/agents/MCP servers. IDA status. | Current |
| [formats/bchunk.md](formats/bchunk.md) | The **bChunk** container used by most MW data files (`.BUN`/`.BIN`/`.LZC`): chunk header, container bit, padding, JDLZ/RAWW wrappers, the full JDLZ algorithm, and bare JDLZ blobs. Reference for [`tools/chunkdump.py`](../tools/chunkdump.py). | Complete |
| [formats/attributes.md](formats/attributes.md) | The **AttribSys `VPAK`** gameplay/tuning database (cars, physics, AI, pursuit) — a different container from bChunk. | **Stub** (layout not yet RE'd) |
| [research.md](research.md) | Prior art (decomp, community tools), language/compiler evidence, engine notes. | Current |
| [install-layout.md](install-layout.md) | What each folder of the install contains; full-scan numbers; mod changes spotted. | Current |
| [formats/models.md](formats/models.md) | Solids: chunk tree, header, shading groups, vertex formats, LOD naming, compressed add-on geometry. | Verified layouts |
| [formats/maps.md](formats/maps.md) | The L2RA world: metadata chunks, streaming index, section families, scenery placement. | Verified layouts |
| [formats/animation.md](formats/animation.md) | EAGL4 ELF objects, skeletons, banks, NIS cutscenes, codec sources in the decomp. | Container verified; codecs open |
| [formats/textures.md](formats/textures.md) | TPK texture pack chunk tree and links to models. | Overview |
| [tools/chunkdump.md](tools/chunkdump.md) | The chunk-tree dumper: options, output, recipes, limitations. | Current |

## Project tools

- [`tools/chunkdump.py`](../tools/chunkdump.py) — dump the bChunk tree of a data file (JDLZ/RAWW aware). See [formats/bchunk.md](formats/bchunk.md).
- [`tools/bchunk_names.py`](../tools/bchunk_names.py) — known bChunk ID → name map.
- [`tests/`](../tests) — unit tests (`python -m unittest discover tests`).

## Conventions

- File formats are documented under [`formats/`](formats), one file per container.
- Specs are grounded in the code that implements them; where the format isn't reverse-engineered
  yet, the doc says so explicitly (see the stub banner in [formats/attributes.md](formats/attributes.md))
  rather than guessing offsets.
- PC release = **little-endian** unless a doc states otherwise.

## Evidence tags

The format docs tag each claim with how it's known:

| Tag | Meaning |
|---|---|
| **[verified]** | Checked against the files in this install (with `tools/chunkdump.py` plus a short script); the numbers quoted come from those runs |
| **[decomp]** | From the [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) decompilation or its symbol files |
| **[community]** | From community tools, mainly [NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) |
| **[unconfirmed]** | Observation or hypothesis not yet proven |

## Open questions

| Question | Where to look |
|---|---|
| Meaning of `PermSize` in `TrackStreamingSection` (smaller than `Size` in sections that start with textures) | `World/TrackStreamer.cpp` in the decomp |
| Purpose of the non-spatial V / X / Y / Z section families | `World/TrackStreamer.cpp` |
| `NisScene` children `0x00037030` / `0x00037040` (152-byte records) | `Animation/AnimScene.hpp` |
| `AnimDirectory` children `0x00037060` / `0x00037070` | `Animation/AnimDirectory.hpp` |
| `TroughBoundary.bin` chunk `0x00034190` | not in the chunk name table |
| EAGL bank header and channel layouts; meaning of the `_q` / `_s` name suffixes | `EAGL4Anim/eagl4AnimBank.h`, `AnimMemoryMap.h`, `FnDelta*.cpp` |
| Exact MW layout of `TexturePackInfoTextures` / `Comps` records | NFS-ModTools `Common/Textures/` |

## TODO

- De-stub [formats/attributes.md](formats/attributes.md) (VPAK header/tables, value types, name hash) and add a `vpakdump.py` + tests.
- Write an EAGL ELF loader + decoders and export animations (plan in [formats/animation.md](formats/animation.md#suggested-next-steps)).
