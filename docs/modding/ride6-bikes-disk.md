# RIDE 6 bikes: disk, time, duplicated textures and streaming

Part of the [RIDE 6 bikes mod](ride6-bikes.md) and [its batch](ride6-bikes-batch.md). The rule of this project is that content of an
external game is streamed from the original files when that is possible and otherwise converted by something small, fast and
small on disk. This page measures what the conversion costs, says what is duplicated, assesses streaming, and says what was cut.
**[measured]** is a number read from the user's files on 2026-11-07, **[estimate]** one computed from them, **[unverified]** a
number that was not measured again after the last importer change (no cargo run was allowed while this was written).

## What the conversion costs

| | Size | Time |
|---|---|---|
| exporter output (`C:\r6work\bikes`, intermediate, not part of the mod) | 34 GB for 293 bikes: 125 MB a bike (RC 390: glb 47 MB, of which the chassis' level 0 is 24, textures 78: normal maps 40, packed pictures 36, base colours 4.5) **[measured]** | first run 10.3 s a bike on average, 3,000 s of work, about 17 min with 3 jobs; second run (JSON cache warm) 1.7 s a bike, 539 s **[measured]** |
| the installed mod (`%APPDATA%\nfsmw\config\mods\ride6-bikes`) | 3.8 GB `files` + 7.7 MB `cars`: `CARS` 3.4 GB (288 bikes, 11.8 MB a bike on average: `GEOMETRY.BIN` 1.9 GB, 6.6 MB; `TEXTURES.BIN` 1.5 GB, 5.2 MB), sound 320 MB, rider 13 MB **[measured]** | import 5 to 7 s a bike with one thread **[unverified]**, so 288 bikes take under 30 minutes with 4 threads |
| after the changes of this branch | geometry about +10 percent (the nearest body is level 1, about 54,000 vertices, instead of level 2, 34,000), textures +2.1 MB for each extra livery of the 209 bikes that have them (a livery is one 2048 px DXT1 picture without mip levels; 387 extra pictures at most, 0.8 GB), so about 4.8 GB **[estimate]** | the same **[unverified]** |

The exporter now leaves out the normal maps unless asked (`export_all.py --normals`): the importer reads them only for
`--normal-maps`, which is off. That takes the intermediate folder from 125 to about 85 MB a bike.

## What is duplicated

A texture is stored in the `TEXTURES.BIN` of the car that uses it, under the name `<CAR>_<picture>`; the compressed blobs of two
cars that use the same picture are **not** equal either (the blob ends with the texture's info record, which holds the
car-specific name hash): 0 of the 288 packs share a blob **[measured]**. By source picture the repetition is small, because every
bike has a chassis of its own and a livery and engine picture of its own (293 different chassis numbers for 293 bikes):

| Picture | Used by | Share of all texture bytes |
|---|---|---|
| tyre pictures (`fronttyre001_d` in 279 bikes, `backtyre001_d` in 164, `backtyre011_d` in 108, ...) | many | about 8 percent **[estimate]** |
| rim pictures shared by a wheel family (`rim709_001_d` in 24 bikes, ...), `mirrorview_d` (154 bikes), `gauge_emrm2019` (25) | some | about 2 percent |
| everything else (livery, engine, cockpit, gear, discs) | one bike | 90 percent |

The estimate counts the base colour of every material of every part at its size cap with DXT1 and mip levels and comes out at
5.1 GB where the packs hold 1.6 GB (it counts materials the meshes do not draw), so only its **proportions** are meant: sharing
every picture that two bikes use would save about **10 percent of the texture bytes, 150 MB of the 3.8 GB**. Within one bike a
picture is already stored once (the two livery materials, the front and back rim, the parts that share a texture), and the
rider's meshes and textures are one shared set (`files/RIDER/`). Liveries with the same picture are one texture.

**Why it is not shared across bikes.** A car's textures are looked up in its own pack, in the packs of the shared part folders that
supplied one of its solids (`WHEELS`, `BRAKES`, `ROOF`, `SPOILER*`, `PLATES`), and in the install's global packs
(`crates/nfsmw-data/src/car/textures.rs`, `car/mod.rs`); a mod's own shared pack would need a host change (a texture pack that
every car of the mod searches, with a name rule like the `MODSHARED` folder of the 2012 mod's sounds,
[`shared.rs`](../../mods/nfsmw-2012-content/src/shared.rs)) and a two-pass import (find the pictures used by two bikes, bake them
into the shared pack once under a name without the car prefix). That is a game change for 150 MB, so it is **not built**. The sound
already does it: 293 bikes use 98 engines, and each engine's layers are written once ([sound](ride6-bikes-sound.md)).

## Streaming from RIDE 6's own files

RIDE 6 keeps its content in Unreal Engine 5.6 IoStore containers (`.utoc`/`.ucas`, 45 of them, readable without a key) and
encrypted `.pak` files (the sound banks). A bike's model is a skeletal mesh package, its pictures are texture packages, and
everything is described by unversioned property data that is read with a mapping file (`.usmap`) the user makes by running the game
with a dumper. To draw a bike straight from those files the game would need, in Rust: the container reader (table of contents,
chunk offsets and the compression of the blocks: Oodle, a closed codec that the exporter's reader downloads as a DLL), the package
reader (name map, import and export tables, unversioned properties against the mapping), the skeletal mesh reader (UE 5.6 LOD
buffers, skin weights, material slots), the texture reader (platform data, mip bulk data, BC formats), the material instance
resolution (the chain of parents and static switches that decides which picture is the base colour), and then **the conversion
this importer does anyway** (the occlusion bake, the light material per triangle, the front end split, the 16-bit solids, DXT
encoding). The repository has none of the readers (`libs/` has no IoStore, Unreal package or Oodle code, and Oodle cannot be a
dependency of a project under MIT or Apache-2.0). The conversion is the larger half, and what a stream saves is only the disk
(3.8 GB). **Streaming is not feasible as a small piece of work and is not implemented.**

What is done instead, and what stays:

- the exporter is one console tool around the reader the user already needs for the data (`tools/ride6-exporter`), its output
  is scratch and can be deleted after the import (`C:\r6work\bikes` is 34 GB; the importer needs it only while converting);
- the converter is fast enough to run in the background (the native mod converts every bike with half the cores while the game
  runs; the stamp of its output tells it when nothing changed, so it converts again only after an importer change);
- a smaller mod on disk is a matter of converting fewer bikes: the native mod's `config.toml` takes a source folder, and a source
  folder holding only the bikes the user drives is a mod of a few hundred MB (12 MB a bike).

## What was cut

- the normal maps from the exporter's output (40 of 125 MB a bike, 32 percent), off unless `--normals`;
- nothing from the converted cars: the levels of detail C and D of a body are the same chassis level 3 (the game looks up the
  nearest level a pack has for a customization model, but nothing says it does so for the body solids, so both stay) and the
  textures are at the sizes of the quality complaint (2048 body, 1024 wheels, 512 discs).

## The finest level of the frame (completeness work)

`A` is chassis level 0 for every bike now (the frame as several solids, [completeness](ride6-bikes-batch.md#completeness)). Measured
on the 288 bikes with the same exports: `files/CARS` **4.0 GB before, 4.9 GB after** (+22 percent: 4 to 7 MB a bike, the 690 from
14.1 to 18.1 MB, of which its `GEOMETRY.BIN` from 9.7 to 14.0 MB), `files/PARTS` 161 MB, the whole import 7 minutes with 4 threads
on 8 cores (the machine was building other things). `B` stays at level 2 and `C`, `D` at level 3: level 1 would cost about a third
of the file again.
