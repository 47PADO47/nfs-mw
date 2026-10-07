# Research: what's already known about NFS: Most Wanted (2005)

Summary of the prior-art search done before writing any tools. **Conclusion: we don't need Ghidra yet.**
A matching decompilation and several open-source format readers already exist; this project builds on
them. For the tag meanings, see [evidence tags](README.md#evidence-tags).

## Language and toolchain **[verified]**

MW is written in **C++**, using EA Black Box's in-house engine (shared with NFS Underground 2 and
Carbon).

Evidence from `speed.exe` (PE32, 6,029,312 bytes):

| Check | Result |
|---|---|
| PE optional header | Linker version **7.10**, i.e. Microsoft Visual C++ .NET 2003 |
| Imports | `d3d9.dll` (Direct3D 9) |
| C runtime | No `MSVCR*.dll` / `MSVCP*.dll` import: CRT statically linked |
| RTTI strings | Only CRT types (`.?AVexception@@`, `.?AVbad_cast@@`, …): game classes compiled without RTTI |

Engine-side languages, from the decomp's source tree:

- C++ for almost everything (`src/Speed/Indep/Src/**`);
- a little C for SDK and runtime libraries;
- an embedded **Lua** (`zLua`) for scripting;
- data-driven gameplay via **AttribSys** databases (see [formats/attributes.md](formats/attributes.md)).

## Existing community work

| Project | What it gives us | Language / license |
|---|---|---|
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) | Work-in-progress **1:1 matching decompilation**: GameCube (main target), PS2, Xbox 360, **PC v1.3**. Built from GameCube **DWARF debug symbols**, so names, structs and file paths are the originals. Very active (pushed 2026-10-02). Useful files: `symbols/bchunks.txt` (chunk IDs), `symbols/vlt.txt`, `symbols/hashes.txt`, `src/Speed/Indep/bWare/Inc/bChunk.hpp`, `src/Speed/Indep/Src/World/TrackStreamer.hpp`, `src/Speed/Indep/Src/EAGL4Anim/*`. | C++, CC0-1.0 |
| [NFSTools/NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) | Working readers for MW **geometry** (`MostWantedSolidReader.cs`), **scenery** (`MostWantedScenery.cs`), **TPK textures**, **light packs**. `AssetDumper` exports to FBX; `ChunkView` is a GUI chunk browser. **No animation support.** | C# |
| [SpeedReflect/Binary](https://github.com/SpeedReflect/Binary) + GlobalLib | Editor for global files (car presets, materials, strings, collections) | C#, GPL-3 / MIT |
| [berkayylmao/NFSPluginSDK](https://github.com/berkayylmao/NFSPluginSDK) | Reversed **in-memory** game structs for runtime (ASI) mods; MW about 60% | C++20 header-only, BSD-3 |
| NFS VltEd, Attribulator | Editors for `attributes.bin` / `gameplay.bin` | — |
| NFS-CarToolkit (nfsu360) | Builds add-on car `GEOMETRY.BIN` files (seen in this install) | — |

## Coverage by topic

| Topic | Before this project | After |
|---|---|---|
| Container format (bChunk) | known, scattered | spec in [formats/bchunk.md](formats/bchunk.md) + `tools/chunkdump.py` |
| Models | NFS-ModTools reads them | layout cross-checked: [formats/models.md](formats/models.md) |
| Maps | NFS-ModTools reads scenery | streaming index decoded and verified: [formats/maps.md](formats/maps.md) |
| Textures | NFS-ModTools reads them | overview: [formats/textures.md](formats/textures.md) |
| **Animation** | **no public parser** | identified as ELF32 MIPS relocatable objects with symbolic bone/skeleton/bank names: [formats/animation.md](formats/animation.md) |
| Gameplay data | VltEd / Attribulator | pointer: [formats/attributes.md](formats/attributes.md) |

## Engine notes from the decomp **[decomp]**

- **Chunk loading.** `bChunkLoader` keeps a table of ID → loader/unloader functions.
  `bChunkLoader::CallLoaders()` walks a chunk stream and dispatches each chunk to its loader (64-bucket
  hash: `(id + (id >> 6) + (id >> 12)) & 0x3F`).
- **In-place loading.** Data is used where it was loaded. Pointer fields inside on-disk structs are zero
  and get filled at runtime, as with `pMemory` and `pDiscBundle` in `TrackStreamingSection`
  ([verified] zero on disk).
- **Name hashing.** `bStringHash` (`h = h * 33 + c`, seed `0xFFFFFFFF`) links solids, textures and
  scenery. [verified]: it matches all 20,377 world solid names.
- **Animation loader.** `eagl4supportdlopen.cpp` / `eagl4supportsympool.cpp` act as a tiny dynamic
  linker for the EAGL ELF objects.

## When we would need Ghidra

Only for things neither the decomp nor the community tools cover yet, for example PC-specific code
(`src/Speed/PC` is sparse in the decomp) or confirming open questions such as `PermSize`. The decomp
already targets PC v1.3, so its symbol names can be applied to `speed.exe` instead of starting from
scratch. The local toolchain (Ghidra 12, Ghidra MCP) is described in
[TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md).
