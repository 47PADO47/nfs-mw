# RIDE 6 rider gear: the data model and the files

What RIDE 6's rider editor offers (helmet, suit, gloves, boots, the body of the rider), where that is in the game's data, the
catalogue the user's export tool makes of it, and the runtime files `ride6-rider-gear` (`mods/ride6-bikes/src/rider/gear/`)
turns that into. Nothing here is committed: the files hold the user's game data and live outside the repository (the catalogue)
or in the mod's git-ignored `files/` folder (the runtime files; [licensing.md](../licensing.md)). What the game does with them is
in [ride6-rider-gear.md](../specs/ride6-rider-gear.md) (the spec); the rider that wears them is
[ride6-rider.md](ride6-rider.md). Tags: **[measured]** read from the user's install on 2026-11-07, **[estimate]** computed from
measurements, **[unverified]** not run (no cargo run was allowed while this was written), **[ours]** a choice of this port.

## Where the data is

RIDE 6 is Unreal Engine 5.6.1; the files open with CUE4Parse and the game's `.usmap` ([ride6-rider.md](ride6-rider.md#where-the-data-comes-from)).
Everything is under `ride6/Content/assets/graphics/characters/riders/` (9,151 files **[measured]**), in one folder per piece of
gear and, below it, `customization_data/` (data assets), `material_instances/`, `textures/` and `skeletal_meshes/`:

| Folder | Files | Folders of gear |
|---|---|---|
| `helmets/` | 3,194 | 61 (`helmet001` ... `helmet164`, `Helmet030` ...) |
| `suits/` | 1,066 | 56 (`suit001` ... `suit070`, `suit333`) |
| `gloves/` | 573 | 35 |
| `boots/` | 617 | 25 |
| `goggles/` | 605 | 18 (not imported: [Gaps](#gaps)) |
| `riderShared/` | 3,080 | the default rider, the bodies and their skeleton, the visors, the faces (1,683), the animations (1,162) |

### The tables: what the editor lists

The rider editor's lists are the rows of the data tables `graphics/Bindings/Rider/*` (class `IgnitionDataAssetTableRow`: one
`DataAsset` reference per row; the row name is the item's id) **[measured]**:

| Table | Rows | Row ids look like | The data asset is a |
|---|---|---|---|
| `HELMETS` | 300 (299 and `EMPTY`) | `HL_AGV_Corsa-R_Multi-Jack-2017` | `HelmetDataSource_C` |
| `SUITS` | 144 | `SU_Alpinestars_GP-Tech-V4_Black-White` | `SuitDataSource_C` |
| `GLOVES` | 110 | `GLO_Alpinestars_GP-Plus-R-V2_Black` | `GlovesDataSource_C` |
| `BOOTS` | 104 (103 and `EMPTY`) | `BO_Alpinestars_Supertech-R_Black-White` | `BootsDataSource_C` |
| `VISORS` | 8 | `VS_Standard_Gold` | `VisorDataSource_C` or a collection of them |
| `GOGGLES` | 67 | `GO_FOX_Vue-Stray_Black-Yellow` | `GogglesDataSource_C` |
| `BUTTPATCHES` 21, `FACES` 66, `HAIRS` 66, `EYES` 2, `CAPS` 1, `RIDING_STYLE` 5 | | | not gear: not imported |

A row id is `<prefix>_<brand>_<model>_<variant>`: a helmet model comes in many liveries, each a row (`HL_HJC_i50_Livery-2`,
`HL_HJC_i50_ERASED-MC24SF`), and the rows with `Custom` in the name (`..._Custom`) are the model before the colour sliders;
the rows named after riders (`..._CaseyStonerCustom_Default`) are replicas. So a **livery is a row**: helmets 299 rows over 110
different meshes, suits 144 rows over one body mesh. `ColorsTable` and `PbrColors/*` are the game's paint colours and not used here.

### The data assets (`customization_data/`)

| Class | Properties that matter | Meaning |
|---|---|---|
| `HelmetDataSource_C` | `HelmetMesh` (SkeletalMesh), `HelmetMaterial` (MaterialInstanceConstant), `VisorClosedMesh`, `VisorOpenMesh`, `VisorMaterial`, `VisorColorMask` | the shell, its picture, the visor (219 of 300 have a visor material, 212 a visor mesh); the helmet mesh is not the visor's |
| `SuitDataSource_C` | `SuitMaterial`, `SliderColorMask` | the picture of the leathers; the suit's shape is not here (below) |
| `GlovesDataSource_C` | `GlovesBaseColor`, `GlovesNormal`, `GlovesPbr`, `GlovesAlpha` | four pictures, composited onto the hands (below) |
| `BootsDataSource_C` | `BootsMesh`, `BootsMaterial` | the boots' mesh and picture |
| `SuitMeshDataSource_C` (`riderShared/bodies/customization_data/`) | `SuitMesh`, `ArmsMesh` | a **body**: `suit001_male`, `suit001_female`, `suit002_male`, `suit002_female`; the `NFS` and `jacket`, `street`, `casualHome` ones are the cinematic rig's and street clothes |

The picture of an item is the **base colour** of its material instance: `TextureParameterValues` `BaseColor - Texture`, found by
walking the instance's `Parent` chain (`BASE_Helmet3`, `BASE_RiderSuit3_NoLeg`, `BASE_Boots3`, ... up to the masters
`MASTER_Character3` and `MASTER_Rider3_Suit` of the `ignitionrendering` plugin; `tools/ride6-exporter/materials.py` does it). The
instances also set the normal map, the PBR picture (`S_R_M`), a clear coat layer (298 of 616 helmet and boot materials), and two
**customization passes** (`ColorCustomization` and `TextureCustomization - Pass1/Pass2`: a mask picture, a picture and a modulation
colour, on 442 materials) that the editor's colour sliders drive. **[ours]** Only the base colour picture is imported: the rows are
the presets of the editor, and what its sliders add on top is not ([Gaps](#gaps)).

### Which body wears which suit

The suit pictures (`T_suit001_BC`, `suit002_d`, `T_suit902_BC` ...) share one texture layout: the same 144 pictures were drawn on both
`suit001_male` (4,458 vertices at level 0) and `suit002_male` (4,437) with the same result **[measured]** (a software rendering of
five suits on both bodies). The female bodies are 3,901 vertices (`suit001_female`) with the same layout. All four use the rider
skeleton's 78 bones (`root` ... `headRef`; the game's `suit_game_Skeleton` has 81, with the three extras it does not skin to), so the
animation of the existing rider drives them. The helmets are one-bone meshes on the bone `helmet` with the identity as their
bind pose (297 of 299; two are 23 cm forward), the boots 10 bones (`root`, `pelvis`, `thigh_l` ... `ball_r`) with the same bind pose as the
body's legs **[measured]**, so a mesh of any item is skinned by the rider's own pose by bone **name**.

### Gloves

A glove is four pictures in a layout of their own (two hands, palm and back, in a 1,024 pixel square), not the suit's. RIDE 6
composites them into the suit's picture at run time with material functions (`data/characters/Compositing/MF/MF_CompositingGloves`,
`MF_MakeGlove`, `SuitBaseColorCompositing`); the functions are cooked without their node graphs, and the suit's own
coordinates of the hand vertices (a strip at the top of the suit's picture, 0.29 by 0.15 of it, one for each hand) are **not** the
gloves' layout nor a second texture coordinate set (the mesh's second set equals the first on the hand vertices **[measured]**), so the
mapping is not recoverable from the data. **[ours]** A glove is imported as the average colour of its base colour picture over its
alpha mask (`gear_colors.py`) and the hand triangles of the body are drawn in that colour ([the importer](#the-runtime-files-ride6-rider-gear-writes)).

## The catalogue: what the export writes

`tools/ride6-exporter/gear.py` (with the tool's `gmesh` and `gtex` modes, `tools/ride6-exporter/R6ToolParts/Gear.cs`):

```
python -I tools\ride6-exporter\gear.py --out C:\r6work\rider-gear [--max 1024]
```

It reads the tables and data assets (through the JSON cache every exporter script shares), resolves each item's pieces and writes,
into `C:\r6work\rider-gear` (persistent, outside the repository):

| File | Content |
|---|---|
| `catalogue.json` | every item with the keys of its meshes and pictures, below |
| `meshes/<key>.r6rm` | each skeletal mesh once, all its levels of detail, as the existing `R6RM` ([ride6-rider.md](ride6-rider.md#r6rm-skinned-mesh)) |
| `tex/<key>.r6tx` | each picture once, as the game stores it (`R6TX`, below) |

A file's `<key>` is the first 12 hexadecimal digits of the SHA-1 of its **content**: two items (or two packages) that have the same
mesh or picture share one file, and the catalogue lists the packages each came from (`sources`). Result **[measured]**: 138
meshes (4 bodies, 24 boots, 110 helmets and visors) and 597 pictures from 604 packages (7 packages hold the content of another);
40.8 MB of meshes with all six levels of detail and **370.8 MB** of
pictures at the 1,024 pixel cap (355 MB on disk with the cluster rounding of Windows), in **3.5 seconds** with the JSON dumps
cached (the first run, which dumps the 700 data assets and materials, took under 2 minutes).

`catalogue.json`:

```text
{ "version": 1,
  "bodies":  [ { "id": "suit001_male", "gender": "male"|"female", "shape": 1|2, "mesh": <mesh key> } ],
  "helmets": [ { "id": <row id>, "name", "brand", "mesh": <key>, "color": <picture key>, "visor": { "mesh": <key>, "color": <key> } } ],
  "suits":   [ { "id", "name", "brand", "color": <picture key> } ],
  "gloves":  [ { "id", "name", "brand", "rgb": [r, g, b] } ],
  "boots":   [ { "id", "name", "brand", "mesh": <key>, "color": <key> } ],
  "meshes":   { <key>: { "file": "meshes/<key>.r6rm", "bytes", "sources": [package paths] } },
  "textures": { <key>: { "file": "tex/<key>.r6tx", "format", "width", "height", "mips", "bytes", "sources": [...] } } }
```

`name` is the row id without its prefix, the dashes made spaces (`AGV Corsa R Multi Jack 2017`); `visor` is absent for a helmet whose
visor is not a separate mesh (the Nexx, the motocross helmets with the goggles built in; 90 of 299). The items counted **[measured]**:
4 bodies, 299 helmets (209 with a visor), 144 suits, 110 gloves, 103 boots.

### `R6TX`: a picture as the game stores it

```
"R6TX"  u32 version = 1  u32 format  u8 srgb  u32 mip_count
per mip, largest first:  u32 width  u32 height  u32 byte_count  bytes
```

`format`: 1 BC1 (DXT1), 2 BC2 (DXT3), 3 BC3 (DXT5): the blocks exactly as RIDE 6 stores them, no decoding and no re-encoding, so
the picture costs what it costs in the game and loses nothing; 255: one mip that is a PNG (a format the renderer has no block
codec for, made by CUE4Parse's decoder; none of the 597 is one). The tool writes from the largest level of which both sides are
within `--max` (a 2,048 by 1,024 boot picture at the cap 1,024 starts at 1,024 by 512) down to the 4 by 4 level.
Formats **[measured]**: 438 pictures are 1,024 by 1,024 BC1, 136 are 1,024 by 512 BC1, 23 are BC3 (the visors, which carry
their alpha; one is 512 by 256).

### `R6RM` for gear

The existing `R6RM` is unchanged (version 1, one texture coordinate set). A mesh is written from CUE4Parse's `SkeletalMeshDto`: the
bones of the mesh in Unreal's axes and centimetres, the material slot names, every level of detail with up to eight bone weights.
The slot names say what a section is: a helmet's `helmet_mat`, a goggle band `goggles_mat` (`goggles_AB13`), the stray
`lambert1` and `Default_Material` of two helmet meshes.

## The runtime files `ride6-rider-gear` writes

`ride6-rider-gear <catalogue folder> <output folder> [--max-texture N]`
(`mods/ride6-bikes/src/rider/gear/`), run by the native mod's job for the `gear` setting
([README](../../mods/ride6-bikes/README.md)). The output is the mod's `files/RIDER/GEAR/`; the game reads it through its
virtual file system, one item at a time, when the player picks it (nothing is loaded until then):

| File | Content |
|---|---|
| `gear.json` | the list of items the game shows, below |
| `meshes/<key>.bbsk` | one mesh as a `BBSK` pack ([ride6-rider.md](ride6-rider.md#runtime-pack-bbsk-written-by-blackbox_skeletonpackto_bytes)): **the mesh's own bones** as its skeleton (1 for a helmet, 10 for boots, 78 for a body), no clips, one mesh, **level 0 only** |
| `tex/<key>.r6tx` | the picture, `R6TX` without the mip levels above `--max-texture` |

```text
{ "version": 1,
  "bodies":  [ { "id", "name", "gender", "mesh": "meshes/<key>.bbsk" } ],
  "helmets": [ { "id", "name", "brand", "mesh", "color": "tex/<key>.r6tx", "visor": { "mesh", "color" } } ],
  "suits":   [ { "id", "name", "brand", "color" } ],
  "gloves":  [ { "id", "name", "brand", "rgb": [r, g, b] } ],
  "boots":   [ { "id", "name", "brand", "mesh", "color" } ] }
```

The order of each list is the order of the catalogue (the table's own order). Ids are the table's row ids and are what the
player's choice stores. A mesh pack's sections carry **roles** as material names, not RIDE 6's slots:

| Role | Drawn with | Made from |
|---|---|---|
| `main` | the item's picture | every section but the two below |
| `goggles` | a dark grey (the goggle band of the motocross helmets has no picture of its own) | a section whose slot or material name contains `goggle` |
| `hands` | the glove colour of the chosen gloves, or the suit's picture when none | on a body: the triangles whose three vertices are weighted at least half to the hand and finger bones (`hand_*`, `index_*`, `middle_*`, `ring_*`, `pinky_*`, `thumb_*`): 1,498 vertices of the 4,458 of `suit001_male` |
| (left out) | | `lambert1`, `Default_Material` and every other section of a stray placeholder material |

Names: the importer names a helmet's mesh `helmetRacing_default` and its visor's `visorClosedRacing_default` (the names of the
stock rider's, which the bikes' `[bike.camera.helmet] hide_meshes` list: the first-person camera leaves them out
whatever helmet is worn, [ride6-bike-cameras.md](../specs/ride6-bike-cameras.md)).

### Size and time **[estimate]**

Computed from the catalogue with the layout above (the importer itself is **[unverified]**): meshes at level 0, 138 files, 16.5 MB;
pictures cut at `--max-texture`: 370.8 MB at 1,024, **92.9 MB at 512 (the default)**, 23.3 MB at 256. So the whole catalogue (299
helmets, 144 suits, 110 gloves, 103 boots, 4 bodies) is about **110 MB** at the default, against the 13 MB of the rider and the 3.8 GB
of the bikes. No picture is stored twice (content hashes), no item stores a picture of another's, the 110 gloves store three numbers
each. The conversion reads and writes about 110 MB and copies mip tails: seconds, no decoding.
**Streaming** from RIDE 6's containers is not possible for the reasons of [ride6-bikes-disk.md](../modding/ride6-bikes-disk.md#streaming-from-ride-6s-own-files);
what is lazy is the use: the game reads a mesh and a picture only for the items the player wears or looks at, not the catalogue.

## Gaps

- **Goggles** (67 rows, `GogglesDataSource_C`: `GogglesMesh`, `LensMaterial`, `GogglesMaterial`) are not exported: the full-face helmets cover
  them, and an open-face helmet shows the rider's face, which is not imported either.
- **Colour sliders**: the `ColorCustomization` and `TextureCustomization` passes (a modulation colour over a mask) are not applied;
  an item is its base colour picture. The `*_Custom` rows are the default colours of the sliders.
- **Gloves** are a flat colour per glove, not the picture ([Gloves](#gloves)).
- **Visors**: one closed visor mesh per helmet, drawn translucent with its base colour (the alpha of the BC3 picture); the open
  visor, the transmittance picture and the visor's tint `VISORS` table (8 rows) are not used.
- **Normal and PBR pictures, the clear coat** of the helmets are not imported; the game's renderer has no per-item material for the
  rider, only a colour picture.
- **Female bodies** share the male rider's skeleton proportions; the grips and pegs were measured for the male body: [unverified].
- **Jackets, street and casual wear, the buttpatch, faces and hair** are not rider gear for the bike and are not imported.
