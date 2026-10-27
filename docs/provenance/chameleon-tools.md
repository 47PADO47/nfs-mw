# Provenance: the community Chameleon tools and the findings log

Covers the work that added string tables, texture export and the fuller bundle reader:
[`libs/chameleon-text`](../../libs/chameleon-text), [`Texture::to_dds`](../../libs/chameleon-gfx/src/dds.rs) in
`libs/chameleon-gfx`, and the new reading in [`libs/chameleon-bundle`](../../libs/chameleon-bundle) (header streams,
ids, the full type registry, debug names). Specs: [formats/chameleon-bundle.md](../formats/chameleon-bundle.md),
[formats/chameleon-language.md](../formats/chameleon-language.md), [formats/chameleon-gfx.md](../formats/chameleon-gfx.md),
[formats/chameleon-games.md](../formats/chameleon-games.md). Earlier Chameleon work: [chameleon.md](chameleon.md).
Process: [licensing.md](../licensing.md).

The aim was data extraction and a base for later ports, not a copy of what the tools do: the libs read the data
straight from a bundle's blocks (no `.dat` unpack step, no JSON), and none of the tools' code is in the repository.

## Sources

| Source | License | Commit read | What was used |
|---|---|---|---|
| [YzENT/Chameleon-String-Editor](https://github.com/YzENT/Chameleon-String-Editor) | MIT | `d9bbe84d73ef` (2025-01-17) | The layout of the `LocalisedText` block (32 and 64 bit), the list of the 14 language files. Facts only; the script was read in full (246 lines) |
| [DGIorio/bundle_packer_unpacker](https://github.com/DGIorio/bundle_packer_unpacker) | MIT | `54b3eb242b6e` (2026-01-18) | The `bnd2` header beyond 0x28, flag bit names, the id split, import-type bit, per-type import locators, the other games' headers, platform numbers and type tables. Read: the unpack and pack paths for MW and HP, the header, flag, platform, id and type tables, the debug-table conversion (about 2,000 of 5,235 lines). Facts only |
| [YzENT/Chameleon-dat-to-dds](https://github.com/YzENT/Chameleon-dat-to-dds) | **none** (the repository has no license file) | `13c5f35b4db9` (2024-05-18) | The texture header offsets of Hot Pursuit and Hot Pursuit Remastered, the tool's file naming. Facts only; the script (164 lines) was read in full, none of it was copied |
| The user's own NFS: Most Wanted (2012) install (v1.4.0.0, `D:\Need for Speed - Most Wanted`) | n/a (never copied into the repo) | | Every number tagged **[verified]** in the four docs |

No code was copied from any of the three: the string editor and the bundle tool are MIT and could have been ported
with their notice, but their designs (a tkinter dialog, directory unpacking, JSON files) do not fit a library that
takes `&[u8]`, so the Rust was written from the specs. The dat-to-dds tool has no license, so under the project's rule
only its facts are used, with credit in `NOTICE`. No decompiled code and no game data is in the repository; the install
was read, never changed.

Not done in this work, so that nothing here is attributed to it: **Ghidra, IDA and Cheat Engine were not used**, and
`NFS13.exe` was not read again (the type-name table already in [chameleon.md](chameleon.md) is from the earlier work).
The game was not started. The packer of the bundle tool was not ported (see "Not implemented").

## Method and tools

Everything measured was measured with **Python 3.14 scratch scripts** that read the install (nothing was written
into it), cross-checked by the Rust readers and the ignored install tests (`CHAMELEON_GAME_DIR`), plus the GitHub CLI
(`gh api`) to read the three repositories. The scripts are not in the repository (scratch work); each is described so the
result can be reproduced, and the Rust test that now repeats the check is named.

| Script | What it does | Reproduced by |
|---|---|---|
| `probe.py` | Reads a `bnd2` header and table, inflates blocks (struct + zlib). Basis of the others | `chameleon-bundle` install tests |
| `scan.py` | Over all 2,021 bundles: flag combinations, version/platform, default resource, stream names, id split, alignment codes, import offset bit, on-disk size nibbles, entry flags | `every_bundle_header_and_table_follows_the_spec` |
| `ids.py` | Top byte of every id, entry tail bytes 0x42 to 0x47, stream index against named streams | same test |
| `dbg.py` | Parses the debug XML of a `MAPS` bundle; tries name hashes (below) | `reads_the_debug_name_table`, the debug assertion of the test above |
| `imp.py` | Import table position (tail rule) and each per-type locator rule, over all resources with imports | the table in chameleon-bundle.md; the tail rule is not repeated by a Rust test |
| `kinds.py` | Counts of every resource type id, listing of unnamed or absent types | `ResourceKind::ALL` test and the table in chameleon-bundle.md |
| `lang.py`, `lang2.py` | Reads the string table of languages 1, 3, 11 and 14, checks sort order, duplicates, terminators, contiguity, padding; statistics of the English file | `every_language_round_trips_byte_for_byte` |
| `tex.py` | Graphics block length against computed pixel bytes for 89 textures | `the_shared_car_textures_export` (53 car textures) |

## Findings

In the order they were found. Each is in the format docs with its tag; this is the log with what was checked.

1. **The header has a second part.** All 2,021 files have the resource table at 0x70, and 0x28 to 0x70 holds a default
   resource id (u64), a default stream index (i32: -1 in 1,880 files) and four 15-byte stream names (the 60 bytes end
   exactly at 0x70). The old reader skipped them. 720 files name streams, 1,301 do not.
2. **Bundle flags have names** (community) and the install agrees: 0x1 in 1,936 files, 0x2 in the 85 plain `MAPS`
   bundles (the only ones with debug data), 0x4 in 3, 0x8 in 720 (exactly the files with named streams), 0x20 in 56
   (exactly the files with a non-zero default id), 0x10 in none. Sets seen: 0x1, 0x9, 0x29, 0x2, 0x21, 0x5.
3. **The resource entry tail was misread.** 0x42 is one flags byte (0 in 353,289 of 353,289 entries), **0x43 is the
   stream index** (0 to 3, always below the number of named streams), 0x44 is padding (0). The old reader had a u16
   of flags at 0x42 and a u32 stream index at 0x44, so the stream index showed up as `flags = index << 8` and the real
   field was never read. No other crate used these fields; `Entry.flags` and `stream_index` are now `u8`.
4. **The id is four fields.** key (u32), sub-type (u16), index (u8), class (u8): the community tool's decomposition,
   confirmed. Class 0 in 100,659 entries, class 1 in 252,630, nothing else. A class 1 key is a plain number: the
   `MAPS` debug name `GameChanger (ID=1658001,type=0,index=0)` belongs to id `0100000000194C91`, and 0x194C91 =
   1,658,001. 1,119 ids carry 0x53 in the sub-type (shaders), 865 carry 0x8.
5. **Alignment codes are all 4** (16 bytes) in all 353,289 entries' non-empty blocks except the graphics block of one
   texture (0). In plain bundles the on-disk size has the code in its top nibble (4 for block 0, 7 for blocks 1 to 3),
   which the reader already masked. The community tool's per-type table of codes for MW is all 4, consistent.
6. **Import bit 31.** Set in 1,079,705 of 2,263,381 imports, clear in the rest, second u32 always 0. Two crates
   (`chameleon-vehicle`, `chameleon-sound-graph`) already read this bit as the marker of type imports in Genesys
   objects; the community name is "resource handle". A first version of the new reader stripped the bit from
   `Import::offset`; the vehicle stats and lamps install tests failed, so `offset` keeps the stored value and
   `slot()` / `kind()` split it.
7. **The import table is the last thing in block 0** for all 252,671 resources that have imports (`offset + 16 * count`
   equals the block size). That makes the community packer's per-type walk unnecessary for reading. Its per-type header
   rules were tested one by one: Material, Shader, CharacterSpec, Font, the three instance lists, CameraTakeList and
   TrafficLaneData hold in 100 % of the resources; the BearEffect rule holds in 17 of 188 and is wrong.
8. **The type registry.** The 61 names of `ResourceKind` cover every type id in the install (no unnamed id). Counts per
   type were taken; five registered types never occur. The community tool's names differ from the executable's for eight ids
   (`Language` for `LocalisedText`, `TrafficData` for `TrafficLaneData`, ...). The old `ResourceKind` had 22.
9. **The string table.** The layout of the string editor holds exactly for the 32-bit PC files, and
   the writer reproduces all 14 blocks byte for byte (including the zero padding to 16). New facts: ids are numbers in
   13,554 to 2,366,054; 54 entries share an id with another (nine ids, identical texts); private-use characters
   U+F000 to U+F0FF are inline colour or icon codes; `{Namespace.Name...}` markup is plain text; all 14 files have
   the same 12,432 ids. The editor's hex-string approach (re-encoding UTF-16 through hex) is not needed.
10. **Texture graphics blocks are padded to 128 bytes** (82 textures probed: 74 padded, 8 exact, none off by anything
    else). The earlier gfx doc said a 1024 x 1024 BC1 with 11 mips is 699,136 bytes "as stored"; the pixels are
    699,064 and 699,136 is the padded block. The reader (`Texture::mip`, `slice_size`) already used the computed size,
    so no code was wrong; the doc was.
11. **DDS export.** The pixel data is already in DDS order, so the export is a header plus a copy. The header is built from
    Microsoft's published DDS layout, not from the tool's byte string, with classic FourCCs for BC1 to BC3 and the DX10
    extension for the rest, which the tool cannot write. The 53 car textures export to files of `128 + pixels` bytes
    with the fields the reader reports.
12. **Hash of names: not found.** `CRC32` (exact, lower, upper case) and `FNV-1`/`FNV-1a` were tried against the only debug
    row with a name in the install; it is a class 1 id (a number) so the test proves nothing, and the key hash of
    class 0 ids stays unknown. The community tool does not know it either (it keeps ids as numbers).
13. **Other games and platforms** are from the tool's source only and untested (see chameleon-games.md): headers of HP,
    HPR and BP, platform numbers, flag differences, type id differences, per-game alignment codes, the packer's way of
    laying a bundle out.

## Corrections to earlier docs and code

| Where | Was | Now |
|---|---|---|
| chameleon-bundle.md, entry table | u16 flags at 0x42, u32 stream index at 0x44 | u8 flags, **u8 stream index at 0x43**, u32 padding |
| chameleon-bundle.md, header | "bytes between 0x28 and the table may hold a table of block names ... the reader ignores them" | default resource, stream index and stream names, all read |
| chameleon-bundle.md, id | "top byte 0x01 for most, 0x00 for a few; the low 32 bits are a name hash" | four fields, class 1 ids are numbers |
| chameleon-bundle.md, flags | "bits 2, 3 and 5 also occur and are unknown" | named, with the files that have them |
| chameleon-gfx.md, texture | 699,136 bytes "as stored" | pixels 699,064, block padded to 128 |
| `Entry` (code) | `flags: u16`, `stream_index: u32` | `flags: u8`, `stream_index: u8` |
| `ResourceKind` (code) | 22 variants | 61 variants from one table, with `ALL` and `name()` |

## Not implemented

- **Packing bundles.** The facts a writer needs are in the docs (header, tail rule, locators, alignment codes, block
  order), but a writer would need the Genesys and graphics types decoded enough to rebuild imports for edited
  resources, and the game was never run on a repacked bundle. The string table is the one writer here: it is a
  whole resource that needs no imports.
- **Other platforms and games.** No install to check them against.
- **The 64-bit string table** (Hot Pursuit Remastered).
- **Volume and cube textures** in `to_dds`.

## Reproducing

```sh
CHAMELEON_GAME_DIR="D:/Need for Speed - Most Wanted" cargo test -p chameleon-bundle -p chameleon-text -p chameleon-gfx -- --ignored
```

runs the install tests (every bundle's header and table, every language table's byte-exact round trip, the 53 car
texture exports). `-p chameleon-vehicle -p chameleon-sound-graph` with `--include-ignored` (and `MW2012_GAME_DIR` for
the effects crate) checks that the bundle changes did not move anything else.
