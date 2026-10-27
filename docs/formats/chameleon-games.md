# Bundles of other Criterion games and platforms

The 2012 PC game's bundles are described in [chameleon-bundle.md](chameleon-bundle.md). The community tool
[DGIorio/bundle_packer_unpacker](https://github.com/DGIorio/bundle_packer_unpacker) (MIT) unpacks and packs the
bundles of four Criterion games, and what it does for the others tells how the format varies. Nothing on this page
could be checked here: the only install available is the 2012 PC game, so everything is **[community]** (read from the
tool's source, see [provenance/chameleon-tools.md](../provenance/chameleon-tools.md)) and none of it is read by
`libs/chameleon-bundle`, which refuses anything but version 5 on PC. The page exists so a port to another platform or
game knows what to expect.

The tool's `<game>` argument: `BP` (Burnout Paradise), `HP` (Need for Speed: Hot Pursuit 2010), `HPR` (Hot Pursuit
Remastered, 2020), `MW` (Most Wanted 2012). Other tools' names for the game of each column are in the tool's README.

## Platform numbers

The u16 (MW) or u32 (others) at offset 0x06 or 0x08 of a `bnd2` file. Big-endian platforms store every integer big endian.

| Value | MW 2012 | HP 2010 / HPR | BP (`bnd2`) |
|---|---|---|---|
| 1 | PC (also read for PS4) | PC (also PS4 and Switch) | PC |
| 2 | PS3, big endian | Xbox 360, big endian | Xbox 360, big endian |
| 3 | Xbox 360, big endian | PS3, big endian | PS3, big endian |
| 4 | PS Vita, little endian | | |
| 5 | Wii U, big endian | | |

The tool decides the byte order from this field by trying the value in both orders (a PC file reads 1 little endian,
a console file 2 or 3 big endian).

## Headers

| Game | Magic | Version, platform | Header | Entry size | Data blocks |
|---|---|---|---|---|---|
| MW 2012 | `bnd2` | u16, u16 | 0x70: the 0x28 fixed fields, default resource id (u64), default stream (i32), four 15-byte stream names | 0x48 | 4 |
| HP 2010, HPR | `bnd2` | u32, u32 | 0x2C: magic, version, platform, debug offset, count, entries offset, 4 data offsets, flags; the "notes" between 0x2C and the table are kept | 0x50 | 4 |
| BP | `bnd2` | u32, u32 | 0x30: as HP but 3 data offsets and 8 bytes of padding; the debug data sit between the header and the table | 0x40 | 3 |
| BP, other builds | `bndl` (the tool says "B5") | u32 | magic, version, count, then 5 or 6 (PS3) pairs of data offset and alignment; the platform sits at 0x58 (1 PC, 2 Xbox 360) or 0x64 (3 PS3) | 0x70 (5 blocks) or 0x84 (PS3, 6 blocks) | 5 or 6 |

Entry layouts, as the tool reads them (little endian shown; in a big-endian file every field, the 8-byte id included, is
read as a big-endian integer):

- **MW (0x48)**: see the main doc.
- **HP and HPR (0x50)**: `u64 id`, then a second `u64` the tool calls the import hash (it has the same id layout: key,
  sub-type, index, class), `u32[4]` sizes with alignment, `u32[4]` sizes on disk, `u32[4]` offsets, `u32` import offset,
  `u32` type id, `u16` import count, `u8` flags, `u8` stream index; 4 bytes that are not read.
- **BP bnd2 (0x40)**: `u64 id`, `u64` import hash, then the same fields with 3 blocks instead of 4.
- **BP bndl (0x70/0x84)**: a different design: the ids are in a separate array of 8-byte records, the sizes in a
  "compression information" table (0x28 bytes per resource for 5 blocks), and each entry carries a memory address, an
  import offset, a type id and 5 or 6 (size, alignment) pairs for the disk and the offsets. The import table is
  elsewhere (`u32 count`, 4 bytes, 16 bytes per import).

How the data blocks are stored differs by game **[community]**:

- A bundle is compressed when flag bit 0 is set. For HP the tool also reads the flags value 0x8 exactly (debug data
  and nothing else) as uncompressed, and every other value without bit 0 as compressed.
- MW: when a block's on-disk size is not smaller than the file, the field holds the uncompressed size with the
  alignment code, and the tool subtracts the code to find the real size (the same thing the main doc says about plain
  bundles).
- zlib streams are inflated with the expected size as a limit on little-endian platforms and without a limit on
  big-endian ones (the tool falls back to no limit if the first attempt fails).

HP/HPR flag bits differ from MW's:

| Bit | HP / HPR | MW |
|---|---|---|
| 0x01 | zlib | zlib |
| 0x02 | main-memory optimisation | debug data |
| 0x04 | graphics-memory optimisation | non-asynchronous fixup required |
| 0x08 | debug data | multi-stream |
| 0x10 | non-asynchronous fixup required | delta bundle |
| 0x20 | multi-stream | default resource |
| 0x40 | delta bundle | |

## Resource type ids

The tables of the tool for the games share most ids with the 2012 table in the main doc. The differences:

| | HP 2010 / HPR | MW 2012 |
|---|---|---|
| Genesys types and objects | 0x12 `GenesysDefinition`, 0x13 `GenesysInstance` | 0x14 `GenesysType`, 0x15 `GenesysObject` |
| Camera and cutscene | 0x210 `ControlMesh`, 0x211 `Cutscene`, 0x212 `CutsceneList` | the same ids exist only in early PS3 betas |
| Characters | 0x20A `CharacterList` | not used |
| Compound and prop types | not present | 0x215 to 0x219 |
| Navigation, text, fonts | 0x30 `Font`, 0x70 `TextFile` | the same, plus 0x68 `NavigationMesh` |

Console builds add program types (0xC0 `CgsVertexProgramState`, 0xC1 `CgsProgramBuffer`) and the beta type 0xDE
`DeltaDeleted`. The alignment code per type and block differs by game and platform: HP on PC uses 2 (4 bytes) for the
vertex programs and textures' main block and 4 for renderables, HPR uses 3 (8 bytes) for textures, descriptors and
sampler states, and 7 (128 bytes) for waves, world paint maps and groundcover; the 2012 PC files use 4 (16 bytes)
everywhere, which is what the main doc measured.

## Writing bundles

The tool's packer rebuilds a bundle from the unpacked `.dat` files and a JSON table (ids, types, stream names, flags,
debug names). Its main job is to rebuild the **import count and offset** of each resource, which it does per type by
reading the resource's own header (the table in the main doc lists the rules that were checked on the 2012 install)
or by walking back from the end of the block. It writes the blocks one after another with no padding between them,
compresses with zlib level 9 when the bundle is compressed, computes each block's data offset in the order
block 0, 1, 2, 3, and puts the debug XML after the last block. The 2012 files are laid out the same way, so
a bundle written like this would be accepted by the reader here; whether the game loads it was not tested.

## Texture and string headers of the other games

The texture header offsets of HP and HPR are in [chameleon-gfx.md](chameleon-gfx.md) (the dat-to-dds section), the
64-bit string table layout in [chameleon-language.md](chameleon-language.md).
