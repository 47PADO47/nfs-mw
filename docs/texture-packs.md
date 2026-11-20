# Texture replacement and HD texture packs

A rewrite extension (the original has none): loose DDS files replace game textures as they load. Nothing is written to the
install, and the same files work on Windows and Linux. Made for the community HD packs, such as the NextGenGraphics
"RETEXTURE 4X UPSCALE" packs, but any folder of DDS files works.

Code: `crates/nfsmw-data/src/texture_packs/` (the index, the order and the replacement), `crates/nfsmw/src/modding/textures.rs`
(the sources of a run) and `crates/nfsmw/src/settings/hd_content/texture_packs.rs` (the settings).

## Which route to use

The game takes replacement content in two ways. Both can be used at once.

| | Loose DDS files (this page) | Whole files in a mod's `files/` ([modding](modding/README.md)) |
|---|---|---|
| Replaces | One texture at a time, by name or hash | Whole install files: texture packs, models, data |
| Input | DDS files as the packs ship them | Files in the install's own formats |
| Two sources of one file | Each texture goes to its highest source | The whole file goes to the highest layer |
| Use it for | Upscaled or repainted textures | Models, cars, data, and packs that ship as game files |

Use loose DDS files for textures. A whole-file mod replaces every texture of a file at once, so two such mods for one file
cannot be combined: per-texture files can.

## Sources

| Source | Where | Rank |
|---|---|---|
| A mod's `textures/` folder | `mods/<id>/textures/`, beside `mod.toml` | The mod's `[content] priority` |
| The HD texture pack folder | The `texture_packs` setting | Below every mod |

A mod with only a `mod.toml` (`[mod]`, `[api]`, `[games]` and `[content] priority`) and a `textures/` folder is a complete
texture mod. The mod manager on the Mods page enables and disables it like any other mod; its `[content] priority`
ranks it.

The settings' folder suits a pack installed elsewhere, such as a NextGenGraphics install:

| Where | Name |
|---|---|
| Config file | `texture_packs = 'D:\Mods\NextGenGraphics\TexturePacks'` (a top-level key, or the same key under `[hd_content]`) |
| Environment | `NFSMW_TEXTURE_PACKS` |
| Command line | `--texture-packs <DIR>` |

The usual order applies: command line over environment over config file. An empty value (`""`) names no folder, so it
switches a lower layer's folder off. Unset, the settings add no source.

- In TOML, a Windows path is easiest in single quotes (`'C:\...'`); in double quotes each `\` must be doubled. A key written
  below a `[table]` header belongs to that table: put `texture_packs` above the first header, or under `[hd_content]`.
- A relative path is taken from the working directory (the game folder when started from there), not the config folder.
- The path may be at most 255 bytes (the settings keep it inline). Nothing in the game writes the key.

The sources are read **once at start-up**, when the install opens with its mods: a new file, folder or setting needs a
restart. A mod that adds its `textures/` folder while the game runs is not read until the next start.

## Order

1. A texture takes the file of the highest source that names it. At equal priority the mod loaded later wins, as the
   `files/` layers do. The settings' folder is below every mod.
2. **A replacement never undoes a higher mod.** A texture read from a file inside a loaded mod's folder (a file its `files/`
   layer serves, or one of its map packs) is replaced only by a source at that mod's priority or higher. A texture read
   from the install can be replaced by any source.

So with a whole-file texture mod at priority 100 and an HD pack in the settings' folder, the textures of the files that mod
supplies stay the mod's, and the pack replaces the textures of the other files. To have a texture mod win, give it a
higher priority than the whole-file mod.

Priority decides; resolution does not. A higher source with a smaller file still wins.

## Folder layout

A source folder is searched with all its subfolders. A file is used by its name:

- `NAME.dds`: replaces the texture called `NAME` (any case), for example `ARC_BROWNBRICKWALL.dds`.
- `0x17C498C9.dds`: replaces the texture whose name hash is `0x17C498C9` (the hash the game stores; any case of the digits).
- `TexturePackInfo.json` (the NextGenGraphics layout): its `rootDirectory` names a folder beside the file, and each entry of
  `textureMappings` maps a `gameId` (a name or a `0x` hash, as above) to a `texturePath` under that root. A file may then be named
  differently from its texture. The file is read by a small scan rather than a JSON parser, since the packs in the wild have
  trailing commas.

A texture named by two files of one source keeps the first in path order. A NextGenGraphics install unpacks to
`NextGenGraphics/TexturePacks/<n>/TexturePackInfo.json` and `NextGenGraphics/TexturePacks/<n>/RESOURCES/0x*.dds`: point the
setting at `TexturePacks`, or copy `TexturePacks` into a mod as its `textures/` folder.

## Formats

DXT1, DXT3 and DXT5 (also as the DX10 header's BC1 to BC3) pass through byte for byte with the mip levels the file holds
(levels it declares but does not hold are dropped). Uncompressed 32-bit BGRA or RGBA and the BC4, BC5 and BC7 formats are
decoded to A8R8G8B8 with one level. Cube maps, volumes, arrays and 16 or 24-bit files are refused: that file is skipped and
the texture keeps its picture. Up to 16384 pixels a side.

The texture keeps its name, its alpha rules and its flags; it takes the file's size, format and mip chain.

## Size cap

`texture_max_size` caps the side of every replacement file, from every source: a larger file drops its largest stored mip
levels until it fits. Nothing is re-encoded, so a file without the smaller levels keeps its size. `0` (the default) is no
cap; otherwise 256 to 16384 pixels.

| Where | Name |
|---|---|
| Config file | `texture_max_size = 2048` (a top-level key, or under `[hd_content]`) |
| Environment | `NFSMW_TEXTURE_MAX_SIZE` |
| Command line | `--texture-max-size <PIXELS>` |

A 4x pack has sixteen times the pixels of the stock textures it replaces (see the cost below); a cap of 2048 keeps most of
the detail on an 8 GB card.

## Scope

Replaced: the world (the streamed sections, the shared sections and the global packs), cars (their own and shared texture
packs), the HUD and menu packs, the menu room (the front-end stage), and the NIS cutscene bundles.

Not replaced yet: vinyls, liveries, plate designs, exhaust and minimap textures, and fonts a mod adds through its own font
files. HD font glyphs keep the stock layout through the `[hd_content]`
fonts option ([hd-content-support](specs/hd-content-support.md)), which reads the stock size before a replacement comes in.

## Checking

- Start-up logs one line per source (its id, rank, file count and folder) and one for the whole (the textures that have a
  replacement and how many more than one source names), as warnings so release builds show them. A source folder that does not
  exist is reported.
- F8, Detail tab, "HD content": each source with its file count and how many textures it has replaced so far, how many
  replacements a higher mod's own copy held back, and the size cap.
- A file that cannot be read or decoded is logged at debug level (`RUST_LOG=nfsmw_data::texture_packs=debug`) and skipped.

## Converting a pack that ships as game files

Some packs ship whole game files rather than DDS files. Those go into a mod's `files/` folder under the install's paths.
Taking such a pack apart into DDS files needs a texture pack (TPK) extractor; the repo does not ship one. A DDS file per
texture, named by the texture's name or hash, then works as above.

## Limits

- Every load reads its replacement files from disk: the loader does not cache decoded files between loads, so the OS file
  cache serves repeated section loads.
- More memory: a 4x pack has sixteen times the pixels per replaced texture (see below); `texture_max_size` caps it.
- No check that a file shows the same picture as the texture it replaces; a wrong hash shows the wrong picture.

## Cost (measured)

RTX 3070 8 GB, Vulkan, Linux, release build, 3840x2160, highway at `--at 2418,3415`, day, the user's graphics settings (RT
reflections on), no traffic, 12 s benchmark (`--bench-seconds 12`), each case run twice after a discarded warm-up run. Pack:
RETEXTURE 4X UPSCALE 1.4.88 (21 packs, 1751 files: 1117 DXT1, 578 DXT3, 54 DXT5; 1.7 GB on disk), in the settings' folder.
World textures only (the scope at the time). 2026-11-16.

| | No pack | 4x pack |
|---|---|---|
| Frame time, median (ms) | 15.78, 15.77 | 15.85, 15.75 |
| Ready after (s) | 1.51, 1.52 | 3.64, 1.98 |
| Process memory, `rss` at the end (MiB) | 1718, 1588 | 1816, 1796 |
| Process memory, peak `rss` (MiB) | 1789, 1800 | 3157, 3109 |
| GPU memory of the process, peak (MiB, `nvidia-smi`) | 2510, 2510 | 4046, 4046 |

- Frame time: no measurable change (the difference is inside the run-to-run spread).
- Load: 0.5 to 2.1 s longer to the first ready frame here, with the files already in the OS file cache from earlier runs; a cold
  disk was not measured.
- Memory: about 1.5 GB more GPU memory at this spot, and about 1.3 GB more peak process memory while the files are read
  (most of it is given back: about 0.1 to 0.2 GB more at the end).
- Not measured: driving through the city (more sections stream in, each reading its files), cold-disk load, the size cap, and
  the 5070 Ti.
