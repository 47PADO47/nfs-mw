# HD texture packs

A rewrite extension (the original has none): loose DDS files from a folder replace the world's textures when they load. Made
for the community HD packs, such as the NextGenGraphics "RETEXTURE 4X UPSCALE" packs, but any folder of DDS files works.
Nothing is written to the install.

Code: `crates/nfsmw-data/src/texture_packs.rs` (the index and the replacement) and
`crates/nfsmw/src/settings/hd_content/texture_packs.rs` (the setting).

## Setting

| Where | Name |
|---|---|
| Config file | `texture_packs = 'D:\Mods\NextGenGraphics\TexturePacks'` (a top-level key, or the same key under `[hd_content]`) |
| Environment | `NFSMW_TEXTURE_PACKS` |
| Command line | `--texture-packs <DIR>` |

The usual order applies: command line over environment over config file. An empty value (`""`) names no folder, so it switches a
lower layer's folder off. Unset, nothing is replaced.

- The folder is read **once at start-up**: a different folder needs a restart. Nothing in the game writes the key.
- In TOML, a Windows path is easiest in single quotes (`'C:\...'`); in double quotes each `\` must be doubled. A key written
  below a `[table]` header belongs to that table: put `texture_packs` above the first header, or under `[hd_content]`.
- A relative path is taken from the working directory (the game folder when started from there), not the config folder.
- The path may be at most 255 bytes (the settings keep it inline).
- Start-up logs one line with the folder and the number of files, as a warning so that release builds (which drop info lines)
  show it: `texture packs: 1751 replacement texture file(s) under ...`. A path that is not a folder is reported too.
- F8, Detail tab, "HD content": the folder in force, the number of files, and how many world textures have been replaced so far.

## Folder layout

The folder is searched with all its subfolders. A file is used by its name:

- `NAME.dds`: replaces the texture called `NAME` (any case), for example `ARC_BROWNBRICKWALL.dds`.
- `0x17C498C9.dds`: replaces the texture whose name hash is `0x17C498C9` (the hash the game stores; any case of the digits).
- `TexturePackInfo.json` (the NextGenGraphics layout): its `rootDirectory` names a folder beside the file, and each entry of
  `textureMappings` maps a `gameId` (a name or a `0x` hash, as above) to a `texturePath` under that root. A file may then be named
  differently from its texture. The file is read by a small scan rather than a JSON parser, since the packs in the wild have
  trailing commas.

A texture named by two files keeps the first in path order. A NextGenGraphics install unpacks to
`NextGenGraphics/TexturePacks/<n>/TexturePackInfo.json` and `NextGenGraphics/TexturePacks/<n>/RESOURCES/0x*.dds`: point the
setting at `TexturePacks`.

## Formats

DXT1, DXT3 and DXT5 (also as the DX10 header's BC1 to BC3) and uncompressed A8R8G8B8, with the mip levels the file holds (levels
it declares but does not hold are dropped). Cube maps, volumes and other formats are refused: that file is skipped and the stock
texture stays. Up to 16384 pixels a side.

The texture keeps its name, its alpha rules and its flags; it takes the file's size, format and mip chain.

## Scope

Only the **world's** textures are replaced: those of the streamed sections and of the global packs the world loads with them.
Not replaced: cars (bodies, wheels, vinyls, plates), the HUD, the menus and the front end, fonts, movies and the NIS cutscenes'
own packs. A pack's car or UI files are simply never asked for.

## Limits

- Every section load reads its replacement files from disk; the loader does not cache decoded files between loads.
- More memory: a 4x pack has sixteen times the pixels per replaced texture (see below).
- No check that a file shows the same picture as the texture it replaces; a wrong hash shows the wrong picture.

## Cost (measured)

RTX 3070 8 GB, Vulkan, Linux, release build, 3840x2160, highway at `--at 2418,3415`, day, the user's graphics settings (RT
reflections on), no traffic, 12 s benchmark (`--bench-seconds 12`), each case run twice after a discarded warm-up run. Pack:
RETEXTURE 4X UPSCALE 1.4.88 (21 packs, 1751 files: 1117 DXT1, 578 DXT3, 54 DXT5; 1.7 GB on disk). 2026-11-16.

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
- Not measured: driving through the city (more sections stream in, each reading its files), cold-disk load, and the 5070 Ti.
