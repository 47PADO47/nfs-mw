# nfs-mw docs

Documentation for the Rust rewrite of Need for Speed: Most Wanted (2005, PC v1.3 / Black Edition): how
the game stores its data, what prior work exists, and how the rewrite is built. Format claims are checked
against the install at `D:/Need For Speed Most Wanted Black Edition`.

## Short answers

- **Language:** C++, Visual C++ .NET 2003, Direct3D 9, EA Black Box's in-house engine.
  → [research.md](research.md)
- **Prior art:** a CC0 matching decomp (PC 1.3 at 14%, GameCube at 61%), format readers for most
  data, a working MW world viewer (noclip), and no existing engine rewrite. → [research.md](research.md)
- **Container:** a tree of **bChunks** (`u32 id, u32 size, payload`; bit 31 = container), often
  compressed with **JDLZ** (LZ77) or **HUFF** (Huffman + run-length).
  → [formats/bchunk.md](formats/bchunk.md), [formats/huff.md](formats/huff.md)
- **Models:** "solids" in `GeometryPack` chunks, with 104-byte material records, 36- or 60-byte vertices
  and u16 indices. → [formats/models.md](formats/models.md)
- **Textures:** TPK packs, either plain or one compressed blob per texture; DXT1/3/5, P8, ARGB.
  → [formats/textures.md](formats/textures.md)
- **Maps:** one world (`L2RA`): a metadata file plus a 533 MB stream of 720 sections. Models are placed by
  name hash. → [formats/maps.md](formats/maps.md), [formats/world.md](formats/world.md)
- **Gameplay data:** AttribSys `VPAK` vaults (57 classes, Jenkins lookup2 hashes).
  → [formats/attributes.md](formats/attributes.md)
- **Animation:** EAGL4 skeletons and banks stored as ELF32 MIPS relocatable objects.
  → [formats/animation.md](formats/animation.md)
- **Audio / video:** EA-XA, EA-XAS and MicroTalk audio in EA containers; VP6 movies.
  → [formats/audio.md](formats/audio.md), [formats/video.md](formats/video.md)

## Contents

### The rewrite

| Doc | What it covers |
|---|---|
| [architecture.md](architecture.md) | `libs/` (engine-generic) vs `crates/` (MW), finding the install, the streamed city, graphics backends (incl. the D3D11 plan), Windows/Linux, testing, roadmap |
| [rust-stack.md](rust-stack.md) | Crates used and planned, with versions and licenses; rejected options |
| [licensing.md](licensing.md) | Project license, what each source license allows, unlicensed repos, the spec-first process |
| [specs/](specs/README.md) | Behaviour specs written before implementing it from decompiled sources: [scenery visibility](specs/scenery-visibility.md) |
| [provenance/](provenance/README.md) | Records for modules whose behaviour came from restricted sources |

### Research

| Doc | What it covers |
|---|---|
| [research.md](research.md) | Prior art: decomps, sibling-engine RE, rewrites and viewers, hook projects, format tools, reference repos |
| [install-layout.md](install-layout.md) | What each folder of the install contains; full-scan numbers; mod changes; saves and registry |
| [TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md) | The RE toolchain on the development machine (Ghidra, Ghidra MCP, ReAgent, MSVC) |
| [tools/chunkdump.md](tools/chunkdump.md) | The Python chunk-tree dumper |

### File formats

| Doc | What it covers | Status |
|---|---|---|
| [formats/bchunk.md](formats/bchunk.md) | bChunk container, padding, JDLZ/RAWW/HUFF wrappers, bare JDLZ blobs | Complete |
| [formats/huff.md](formats/huff.md) | EA HUFF codec (`0x30FB`): bit stream, numbers, canonical code, run-length clue | Complete; all 10,813 blobs decode |
| [formats/models.md](formats/models.md) | Solids: chunk tree, header, shading groups, vertex buffers per effect run, indices, LODs, compressed add-on cars | Verified; Rust reader |
| [formats/textures.md](formats/textures.md) | TPK: both pack forms, streaming entries, `TextureInfo`, platform record, pixel formats, alpha / blend modes | Verified; Rust reader |
| [formats/maps.md](formats/maps.md) | The L2RA world: metadata chunks, streaming index, section families, scenery placement and rotation encoding | Verified; Rust readers |
| [formats/world.md](formats/world.md) | World grid and road network, collision packs, bounds, triggers, emitters, sky, minimap | Partial (decomp only for several) |
| [formats/attributes.md](formats/attributes.md) | AttribSys `VPAK` packs, vaults, exports, hash, the 57 classes | Layout verified |
| [formats/cardata.md](formats/cardata.md) | Car types, parts database, presets, vinyls | Partial |
| [formats/animation.md](formats/animation.md) | EAGL4 ELF objects, skeletons, banks, NIS cutscenes | Container verified; codecs open |
| [formats/audio.md](formats/audio.md) | Sound banks, GIN engine loops, MPF/MUS music, speech, reverb, mix maps | Codecs known; some indexes open |
| [formats/video.md](formats/video.md) | EA VP6 container and decoder options | Complete |
| [formats/frontend.md](formats/frontend.md) | FEng UI packages, compressed packages, fonts | Packages known; font glyphs open |
| [formats/text.md](formats/text.md) | Language string blocks, labels, subtitles, memory-card locale files | Mostly verified |
| [formats/shaders.md](formats/shaders.md) | The 31 D3DX effects embedded in `speed.exe` | Located; not decompiled |
| [formats/saves.md](formats/saves.md) | Save files: header, MD5, payload order | Partial |

## Project tools

- **Rust:** `nfsmw check-install | list-cars | view-car | view-world` (see the [root README](../README.md)).
  The engine-generic libraries are in [`libs/`](../libs), each with its own README.
- [`tools/chunkdump.py`](../tools/chunkdump.py) dumps the bChunk tree of a data file (JDLZ/RAWW aware).
- [`tools/bchunk_names.py`](../tools/bchunk_names.py) maps 330 known chunk IDs to names.
- [`tests/`](../tests) has the Python tests (`python -m unittest discover tests`).

## Conventions

- One file per format under [`formats/`](formats). Where a format isn't reverse-engineered yet, the doc
  says so instead of guessing offsets.
- PC data is **little-endian** unless a doc says otherwise.
- No game data and no decompiled code in docs: layouts, offsets, IDs and measured numbers only
  ([licensing.md](licensing.md)).

## Evidence tags

| Tag | Meaning |
|---|---|
| **[verified]** | Checked against the files in this install (with the Rust readers, `tools/chunkdump.py` or a short script); the numbers quoted come from those runs |
| **[decomp]** | From the [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) decompilation or its symbol files |
| **[community]** | From community tools such as NFS-ModTools, VaultLib, vgmstream |
| **[unconfirmed]** | Observation or hypothesis not yet proven |

## Open questions

| Question | Where to look |
|---|---|
| Meaning of `PermSize` in `TrackStreamingSection` | `World/TrackStreamer.cpp` in the decomp |
| Purpose of the non-spatial V / X / Y / Z section families | `World/TrackStreamer.cpp` |
| `TroughBoundary.bin` chunk `0x00034190` (not in the decomp's chunk list) | — |
| EAGL bank header and channel layouts | `EAGL4Anim/*` in the decomp |
| Where a compressed palettized texture keeps its palette | no example in car packs; check world/frontend streams |
| Speech/event index formats (`.idx`, `.evt`, `.csi`), reverb and mix presets | [formats/audio.md](formats/audio.md) |
| FEngFont glyph format | [formats/frontend.md](formats/frontend.md) |
| Save header bytes 0x08–0x33 | [formats/saves.md](formats/saves.md) |
| How the `CarShader` paint, reflections and lighting work | the `fx_2_0` effects in `speed.exe` ([formats/shaders.md](formats/shaders.md)) |
| How water surfaces (river, sea) are drawn: no water geometry is placed by the scenery we read | world effects / `SKY_*`, `RFL_*` solids, the decomp's `World/` |
| What the subtractive / overbright blend types look like in the game | the effects in `speed.exe` |
