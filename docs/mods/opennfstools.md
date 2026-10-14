# OpenNFSTools

A set of C# modding tools for Need for Speed games on the EAGL engine. The main tool, VLTEdit, edits VLT database files for Need for Speed: Most Wanted (2005). This is a research note for nfs-mw. It records what the repository contains, what can be verified, and what a clean-room Rust equivalent would need. No code is to be copied.

Verification status: this page was built from GitHub web pages and raw file contents read through WebFetch. Nothing was cloned, built or run. The README fetch returned only an excerpt, so statements that depend on the full README are marked as such.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/MWisBest/OpenNFSTools |
| Author | Original VLTEdit author: Arushan (has stopped working on it). Current maintainer: MWisBest. [README] |
| Type | Tool suite (C# desktop utilities and libraries). Not an .asi plugin. [source] |
| Licence | None stated. No LICENSE file was found at the repo root or in the LibNFS folder. Treat as all rights reserved until the maintainer says otherwise. |
| Language | C# (.NET, Visual Studio solution `OpenNFSTools.sln`). [source] |
| Target game | Need for Speed: Most Wanted (2005, PC). [README] |
| Latest visible activity | Not shown on the captured pages. The repo lists 162 commits but no commit date was visible. |
| Stars and forks | Not captured. |

## Summary

OpenNFSTools is a multi-project C# solution. Its folders are LibNFS (shared library), VLTEdit and VLTEdit-Legacy (VLT database editor), VPak (archive tool), TextureCompiler (texture definition compiler), TestingArea (scratch project), and res (resources). The README states that VLTEdit is the program used for making mods for NFS:MW. According to the README, the code was decompiled with ILSpy, obfuscated with Dotfuscator by the original author, and only partly deobfuscated by the current maintainer. Cleanup is still in progress. [README]

## What it does

Each feature is listed with its evidence. Evidence is the file or README section that supports it.

1. **VLT file reading and writing.** `VLTEdit/VLTFile.cs` reads VLT files. Each VLT file pairs with a companion raw `.bin` file, and the reader resolves pointers between them. The bin file name comes from the Dependency chunk. [source: VLTEdit/VLTFile.cs]
2. **Chunk-based parsing.** The VLT format is parsed as tagged chunks with an 8-byte header (ID and length). Chunk types are Dependency, StringsRaw, Strings, Data, Expression, Pointers and Null. [source: VLTEdit/VLTFile.cs]
3. **Typed database, class and collection records.** The `VLTTypes/` folder and `VLTDataBlock.CreateOfType` create typed records for database load, class load and collection load expressions. [source: VLTEdit/VLTFile.cs, VLTEdit/VLTTypes/ (listing only, contents not read)]
4. **Hash-to-name resolution.** `VLTEdit/HashResolver.cs` exists and is referenced from `VLTFile.cs` to map hashes to names. The file itself was not read. [source: file listing; call site in VLTFile.cs]
5. **Pointer resolution for raw data.** `VLTPointers.ResolveRawPointers` writes resolved offsets into the bin stream at the source positions. [source: VLTEdit/VLTFile.cs]
6. **Layout switch for CARBON.** A `CARBON` preprocessor symbol selects a different structure layout. [source: VLTEdit/VLTFile.cs]
7. **Windows form UI.** `MainForm.cs` and `MainForm.resx` provide the editor window. Only the file names were confirmed. [source: file listing]
8. **Archive tool.** `VPak/VPak.cs` is listed as a separate project. Its contents were not read, so its function is unverified. [inferred from folder name only]
9. **Texture compilation.** `TextureCompiler/` contains `DefinitionParser.cs`, `Program.cs` and a set of short-named classes (for example `a9.cs`, `bf.cs`, `u.cs`). Their function was not read. [inferred from file names only]
10. **Shared library.** `LibNFS/` has Common, Compression, Crypto and Misc folders. Compression and Crypto suggest that it handles packed or encrypted game data, but the contents were not read. [inferred from folder names only]

## How it does it

Only the VLT reader was inspected in source. The rest of this section is limited to what the file listings show.

- **No ASI loader, DLL injection, or game process hooks were found.** The tools work offline on files. The VLT reader takes a `Stream`, `BinaryReader` and `BinaryWriter`, with no game process access. [source: VLTEdit/VLTFile.cs]
- **No hard-coded game memory addresses.** The reader takes all offsets from the file contents. The only magic numbers are format constants: chunk header size 0x8, alignment 0x10, pointer block size 0xC, expected ClassLoadData length 0x20 under CARBON, and flag masks 0x20 and 0x40. [source: VLTEdit/VLTFile.cs]
- **Expression type hashes** are fixed 32-bit values that tag each expression block: DatabaseLoadData 0xCBBC628F, ClassLoadData 0x5E970CBC, and CollectionLoadData 0x8E112EB7. [source: VLTEdit/VLTFile.cs]
- **Pointer block format.** Each pointer is a 12-byte record: source offset (int32), type (int16), identifier (int16) and destination offset (int32). Block types are Done=0, RuntimeLink=1, Switch=2 and Load=3. Switch identifiers 0 and 1 select VLT or raw mode. [source: VLTEdit/VLTFile.cs]
- **Placeholder value.** The writer uses 0xEFFECADD as a placeholder that is later replaced when pointers are resolved. [source: VLTEdit/VLTFile.cs]
- **Strings** are ASCII and null-terminated. [source: VLTEdit/VLTFile.cs]
- **Config files.** Each project has an `app.config` (standard .NET) and a `.csproj`. No application settings keys were read. [source: file listing]
- **Known build problems in VLTFile.cs.** Without `CARBON` defined, the code refers to `_countTypes` and `_zero`, which are never declared, so the non-CARBON build will not compile. `Open` assumes the Dependency and Expression chunks exist. `Open(string)` and `Open(Stream, Stream)` do not dispose their streams. `WriteChunk` pads to 16-byte alignment, but nothing calls it and there is no public save method, so the tool reads VLT but does not write it back. [source: VLTEdit/VLTFile.cs]

## Configuration

No user-facing configuration keys were documented. The README, as captured, has no config section. The only configuration files named in the listings are the standard .NET `app.config` files in VLTEdit, VPak and TextureCompiler. Their contents were not read.

## How we could implement it in nfs-mw

The licence rule applies. With no licence stated, no code, identifiers, structure layouts copied as code, or comments may be taken from this repo. Work from observed file behaviour and from facts that can be verified against the game install. Spec first.

### Behaviours to specify

- Parse the VLT chunk container: header, IDs, lengths and alignment.
- Parse the Dependency chunk: the list of hashes and names, and the companion bin file reference.
- Parse expression blocks (database, class and collection load) and their indexes into the bin file.
- Resolve pointers of types RuntimeLink, Switch and Load into the bin stream.
- Round-trip: read a VLT and bin pair, and write it back byte-identical when nothing has changed. This is the first test to write.
- Hash-to-name lookup for the names used by the game, if a name table exists in the install.

### Data and formats needed

- A reference VLT and bin pair from the NFS:MW install. Only files the user owns may be used, and they must not be committed to the repo. The `.gitignore` is a whitelist, and the project rules forbid game data in the repo.
- A written format spec in `docs/formats/` before code. The existing `docs/formats/` folder and `docs/README.md` index are the right places. Evidence tags should follow the existing convention.
- The hash function. The repo uses fixed hashes, but the hash algorithm must be derived from data, not copied. Check `docs/formats/` first for any existing reading.

### Natural home

- Generic chunk container and pointer-resolution code that does not name NFS:MW: `libs/`. The format crate takes `&[u8]` and does not open files. Only `game-install` touches the install.
- VLT field layouts specific to NFS:MW (the CARBON switch is a game-era choice): `crates/`. The layouts go in per-version layout tables chosen from the data or passed in by the caller, as the project rules require. No NFS:MW names, paths or defaults in `libs/`.
- Editing UI, if one is wanted later: `crates/` or a tool crate, depending on `libs/`, never the reverse.

### Risks

- **Licence risk.** Porting structure definitions or logic from this repo would be copying. Use only facts verified against the game files.
- **Decompiled and partly obfuscated source.** The reader's correctness is unknown. Its own test coverage was not found. Verify every field against the install.
- **Unfinished write path.** Writing is not implemented, so there is no working reference for saving VLT files.
- **Format variance.** The `CARBON` switch shows that layouts differ between game builds. Keep per-build tables and test each target build.
- **Pointer semantics.** Placeholder values and runtime links need careful spec work before any writer is built.

### Not applicable to a clean-room rewrite

- ASI loader and DLL injection are not needed. The tools work offline on files, and nfs-mw does not need this repo's approach to loading.
- Nothing in the repo implies game memory patching, so no hook or patch work is to be taken from it.

## Open questions and what could not be verified

- **Licence.** No licence is stated. The LICENSE fetch at `raw.githubusercontent.com/MWisBest/OpenNFSTools/master/LICENSE` returned HTTP 404, and no LICENSE file appears at the root or in LibNFS. Whether a licence exists on another branch was not checked. Ask the maintainer before any reuse.
- **Full README.** The fetch returned only an excerpt. Install steps, feature list, supported game versions beyond the title, and any config keys could not be read. Re-fetch the full README or read it in a browser.
- **Activity.** The last commit date and message were not visible. Activity level is unknown.
- **Contents of most files.** Only `VLTEdit/VLTFile.cs` was read. `HashResolver.cs`, `MainForm.cs`, `VLTDatabase.cs`, `VPak/VPak.cs`, the TextureCompiler classes, and LibNFS contents are unverified. Their roles above are inferred from names.
- **Language breakdown.** Not captured.
- **Whether the tools work.** Not tested. The repo was not cloned or run, as the task rules require.
- **Relationship to other NFS:MW tools in this repo.** The `docs/mods/` folder has related entries (for example `nfsmw-save-editor.md` and `nfsmw-unlimiter.md`). Overlap was not checked.

## Sources

- https://github.com/MWisBest/OpenNFSTools (repo root listing, description, commit count)
- https://raw.githubusercontent.com/MWisBest/OpenNFSTools/master/README.md (README excerpt only)
- https://raw.githubusercontent.com/MWisBest/OpenNFSTools/master/LICENSE (returned HTTP 404)
- https://github.com/MWisBest/OpenNFSTools/tree/master/LibNFS (listing)
- https://github.com/MWisBest/OpenNFSTools/tree/master/VLTEdit (listing)
- https://raw.githubusercontent.com/MWisBest/OpenNFSTools/master/VLTEdit/VLTFile.cs (read through summary)
- https://github.com/MWisBest/OpenNFSTools/tree/master (root folder listing)
- https://github.com/MWisBest/OpenNFSTools/tree/master/VPak (listing)
- https://github.com/MWisBest/OpenNFSTools/tree/master/TextureCompiler (listing)
