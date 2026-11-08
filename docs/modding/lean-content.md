# Lean content: streaming, lazy files and what the 2012 import writes

Part of the [modding catalogue](README.md). The rule behind this page: **content of another game is read from that game's
files at the time it is needed**; where that is impossible, a mod converts, and the conversion must be fast and small on disk.
This page is the measured story of the [2012 content mod](../../mods/nfsmw-2012-content/README.md) and the generic mechanism
that came out of it, **lazy files** (`bb.content` 1.1). Plan and inventory: [2012-import-inventory.md](../plans/2012-import-inventory.md).

**Status of the numbers.** The "before" sizes were measured on the folder an earlier conversion left
(`%APPDATA%\nfsmw\config\mods\nfsmw-2012-content`, 78 cars: 67 cars and 11 traffic cars) with scratch Python readers of the
chunk tree and by content hash. The "after" sizes are **computed from those measurements, not yet measured on a new run**: the
code was written without a compiler. The commands that measure them are at the end.

## What was on disk, and what is left

| Part | Before | After | How |
|---|---|---|---|
| `GEOMETRY.BIN` (78) | 510 MB | 278 MB | the second level of detail `_B` is a copy of `_A` and is no longer written ([below](#the-second-level-of-detail)) |
| `SOUND/MODENGINE` | 173 MB | 50 MB | each distinct recording once ([below](#recordings-once)) |
| `TEXTURES.BIN` (78) | 82 MB | 82 MB | nothing shared to share: 6.7 MB of the 78 MB of pictures are equal across cars (8 %), see [textures](#textures-and-the-driver) |
| the rest (manifests, bounds, cameras, plates, variants, shared sounds) | 17 MB | 17 MB | |
| **Total** | **750 MB** | **about 430 MB** | |
| **With `stream_cars`** (stubs and a recipe in place of the first three; traffic cars and ignitions stay files) | | **about 40 MB** | [lazy files](#lazy-files) |

## The second level of detail

The game asks for a car's solids at the levels `A` to `D`. The 2012 cars have their finest level at both `A` and `B`, so every
`_B` solid was a byte copy of its `_A` with another name: bodies 57 MB, wheels 10.5, drivers 14, driver poses 110, hinged and
other parts 40, **232 MB (46 %)**. `nfsmw_data::car::mods::levels::alias_second_level` now adds, when a mod car's `GEOMETRY.BIN`
is read, a copy named `<stem>_B` of every `<stem>_A` that has no `_B` (never for the game's own cars, and a `_B` the file has
is kept). Everything that looks a solid up by name (parts, hinged parts, driver poses) finds it. In memory nothing changes.
The rule is documented for every mod in [car-customization.md](car-customization.md#the-files-of-a-car).

## Recordings once

Of 1,338 engine, shift, turbo, backfire and pop files of 67 cars, 1,070 were byte-for-byte the same as another car's (the cars
share engine families). `SoundStore` writes each recording once as `SOUND/MODENGINE/SHARED/<hash><length>.<ext>` and the
manifests name that path: 123 MB saved (the ignition of each car stays its own file: the mod plays it by path).

## Textures and the driver

- **Textures.** Hashing the picture of every texture of the 78 packs found 71 MB distinct of 78 MB: the cars' pictures are cut
  per car and mostly belong to one car. Sharing would save 6.7 MB and need a shared texture pack in the host; not done.
- **Driver.** The plain driver is a translation of one of four characters, but the 8 posed figures reach for **that car's**
  wheel grips, so no two cars' poses are equal (hashing found no identical vertex buffer). The driver therefore cannot be
  shared without losing hands on the wheel; the setting `driver_poses = false` drops the poses (110 MB over 67 cars), and lazy
  files make them free on disk.

## Lazy files

A *lazy file* is a file of a mod's `files/` layer that is a small **stub** and is made when the game reads it.

```
BBLAZY1
nfsmw-2012-content          the owner: the id the maker is registered under
car/POR911CARR12/GEOMETRY.BIN   the key: opaque to the host, understood by the maker
```

| Piece | Where | What it does |
|---|---|---|
| Stub format, `LazySource`, `GameDir::add_lazy_source` | `libs/game-install/src/vfs/lazy.rs` | `GameDir::read` recognises a stub (at most 1,024 bytes, first line `BBLAZY1`) and asks the source added for its owner for the whole file. The stub is a real file, so existence, listing, provider and `geometry_in_mod` work with no change. `open_file`, `read_range` and `size` see the stub; a lazy file is for whole reads |
| `bb.content` 1.1 `provide_lazy_files(read, user)` | `libs/blackbox-modding/src/abi/iface/content.rs` | the mod gives one function, called with the key and a sink that receives the bytes (no allocation crosses the boundary). **Called on any thread**, several at once, never inside a mod callback: it may not call the host |
| Host trampoline | `libs/blackbox-modding/src/host/table/content.rs` | wraps the function in `LazyFiles` (`Send + Sync`) behind a read lock; unloading takes the write lock, so the mod's function is never called after the mod is gone |
| SDK | `libs/blackbox-modding/src/sdk/content.rs` | `Content::provide_lazy_files(impl LazyFiles)`, `supports_lazy_files()`; a panic in the maker is an error status |
| Game | `crates/nfsmw/src/modding/backends/content.rs`, `bridge.rs` | the maker joins the install (every clone shares it); a mod loads before the install is known, so it waits in the bridge until `set_install` |

**Versioning (ADR 0005 section 7).** `bb.content` goes from 1.0 to 1.1 by appending one field to the table; a mod built for 1.0
loads on a 1.1 host, a mod that asks for 1.1 is not given the table by an older host, and `ContentBackend::provide_lazy_files`
defaults to `BB_ERR_UNSUPPORTED`. The layout is in the ABI snapshot test (`abi/iface/tests.rs`).

### The 2012 mod's use of it (`stream_cars`)

- The conversion still converts every car once (the manifest needs its stats, paints and hinged parts) and writes **stubs** in
  place of `GEOMETRY.BIN`, `TEXTURES.BIN` and each recording, and `lazy-cars.toml` in the mod folder: the 2012 install's path,
  the settings the files depend on, and for each car its bundle number, its liveries' numbers and its engine speeds.
- At load the mod reads the recipe and gives the host a `Maker`. When the game opens a stub the maker converts that car again
  with the same steps (`prepare.rs`), keeps the last three cars in memory, and records how long it took (`Maker::timings`).
- The recipe travels with the content it describes (`job::promote` replaces both), so a changed setting converts again at the
  next start and the stubs of the content that is live keep working until then.
- **Toggles.** `stream_cars` in `config.toml` or `[mods.nfsmw-2012-content]` (default `true`), and the gameplay option **Stream 2012
  cars** (on until switched off; offered when a 2012 install is set and the host makes lazy files). A host without 1.1, or the
  setting off, writes every file, as before.
- **Left as files:** the 11 traffic cars (about 25 MB before the second-level change), the cars' ignitions, the shared sounds,
  plates, variants.

### Costs

- **Start of the game:** the mod reads a TOML file of about 8 KB and registers the maker; the 2012 install is opened (about 30 MB
  of shared bundles read) at the first lazy read, not at start.
- **First use of a car:** the car's share of the full conversion: the README measured about 1 s for all cars on six cores, so a
  few tens to a few hundred milliseconds one car at a time **[not measured]**; its recordings (decoding the bundle's samples) cost
  more, and are read when the car's engine starts. Both are kept for the next reads (three cars).
- **Memory:** three cars' files, a few MB each.
- **Risk:** a stub read with the mod not loaded is an error that names the mod and the file.

## Measuring it

```sh
# sizes: run the game once with the 2012 install set (stream_cars on, then off), and compare the mod folder
du -sh "%APPDATA%/nfsmw/config/mods/nfsmw-2012-content"            # on Windows: the folder's properties
# times and byte equality, both ways, for two cars (prints sizes and the milliseconds of every file):
MW2012_GAME_DIR='D:\need For Speed - Most Wanted' cargo test -p nfsmw-2012-content --release --test lazy -- --ignored --nocapture
# the cost of converting everything, as before:
MW2012_GAME_DIR='D:\need For Speed - Most Wanted' cargo test -p nfsmw-2012-content --release --test import every_car -- --ignored --nocapture
# a car, offscreen and muted, opened through the lazy files (the first run of the game converts; the second only makes the car):
nfsmw view-car POR911CARR12 --muted --screenshot out.png
```
