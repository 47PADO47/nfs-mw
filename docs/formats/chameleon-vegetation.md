# The trees of the 2012 world (Chameleon) and of the 2005 world

Where the trees of NFS: Most Wanted (2012) are in its bundles, what a tree model is made of, and how the 2005 game draws
its own trees, so that the 2012 trees can stand in for them (the **trees variant** of
[`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md), format of its profile in
[effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees)). The bundle container is
[chameleon-bundle.md](chameleon-bundle.md), the model, material and texture resources are
[chameleon-gfx.md](chameleon-gfx.md). Evidence tags as in the [docs README](../README.md#evidence-tags).

Everything about the 2012 install (v1.4.0.0) was measured with a Python scratch reader of the `bnd2` container and the readers
of `libs/chameleon-gfx`; everything about the 2005 install (PC, v1.3) with a scratch reader of the chunk format
([models.md](models.md)). Nothing from either game is in the repository: the tables below are names, ids and measured sizes.

## What the 2012 trees are **[verified]**

The 2012 game has no tree-specific file format and no procedural trees. A tree is an ordinary **model** (type 0x51, a level-of-detail
set) whose renderables use one of two shaders of `SHADERS.BNDL` that the technique names tell apart:

| Technique | Shader id | Used by |
|---|---|---|
| `Tree_Translucent_1Bit_Doublesided` | `0x010000530010D4BB` | cards of leaves |
| `Tree_Translucent_1Bit_Doublesided_InstanceTint` | `0x0100005300007843` | cards of leaves with a per-instance tint |

Both have one sampler (`DiffuseTextureSampler`) and the same vertex layout. The 143 bundles of `HAWAII/` that hold such a material
(`GLOBALRESOURCES.BNDL`, the districts `DISTRICT_*`, the track units `TRK_UNIT*`, `ARENA_527908`) hold **27 distinct tree
materials** with **22 distinct textures**; the 373 shaders of `SHADERS.BNDL` have no other `Tree`, `Foliage` or `Leaf` technique.
Grass is separate: `GroundcoverCollection` (0x20F, 2,069 resources) and `GroundcoverInstances` (0x214, 1,663) are not read here.

A bundle of the world does not name its resources (the ids are hashes of names that the install does not keep; only the 85 `MAPS`
bundles have debug names, and only for textures), so a tree is found by what it is made of: the models that have a renderable
with a tree material. There are **40 such models** in the install, each present, with the same id and the same content, in every
bundle that uses it (a district or track unit holds its own copy of the models its streets need; `GLOBALRESOURCES.BNDL` holds three).

### The model **[verified]**

| Part | What it holds |
|---|---|
| Model | 2 to 5 levels of detail, most detailed first (27 of the 40 have 4, 11 have 3, one has 5 and one has 2); the switch distances are squared metres, for example 0, 40,000, 722,500, 1,000,000 (the third level takes over at 850 m, the fourth at 1,000 m) |
| Renderable of a level | 1 to 3 meshes: **foliage** (the tree shader) and **bark** (a trunk and branches) |
| Foliage mesh | cards of leaves: 4 vertices a card, triangle strips with `0xFFFF` restarts (primitive 5), so 5 indices a card; 9 to 1,800 indices a mesh |
| Bark mesh | a trunk and the thick branches, the material's shader is `World_Diffuse_Specular_Singlesided` or `WorldPBR_Diffuse_Normal_Specular_Reflective_AO_Singlesided`; its textures are a bark map (and, for the PBR kind, a normal map) |
| Coarse levels | the last one or two levels are a few crossed cards (`Diffuse_1Bit_Doublesided` technique), or only the bark |
| Shadow stand-ins | some last levels hold a mesh of the `WorldShadow_Opaque_Singlesided` technique: a coarse shape the shadow pass draws, not drawn in the picture |

Axes and size: the engine's space is x left, y up, z forward ([chameleon-gfx.md](chameleon-gfx.md#renderable-type-0x05)); a model's
**origin is the foot of its trunk** (the lowest point of the trunk is y = 0.0 on every model measured, a trunk centred on x = z = 0 to a
metre), in metres. Examples, the bounds of the foliage mesh of the finest level: the broadleaf `0xB4FAD391` runs from y = 3.5 to 22.1 and
spreads 14 m, the columnar `0xB0D173AF` from 1.1 to 16.2 and 9 m, the tall conifer `0xA97B9688` from 2.5 to 50.3 (the bark mesh reaches from
-6.3, roots, to 51.3), the round bush `0xFAC87374` from -0.2 to 3.1.

### The vertex layouts **[verified]**

The shader imports its vertex descriptor ([chameleon-gfx.md](chameleon-gfx.md#vertexdescriptor-type-0x03)).

| Technique | Stride | Elements |
|---|---|---|
| Tree | 32 | usage 1 position `R32G32B32_FLOAT` at 0; usage 13 `R10G10B10A2_UINT` at 12 (0 on all samples); usage 14 `R8G8B8A8` at 16 (255, 0, 0, 0); usage 4 colour at 20 (white); usage 5 texture coordinate `R16G16_FLOAT` at 24; usage 6 index 1 `R8G8B8A8` at 28 |
| World (bark) | 24 | usage 1 position float3 at 0; usage 5 texcoord half2 at 12; usage 6 index 1 half2 at 16; usage 7 index 2 `R8G8B8A8` at 20 |

The positions are plain floats, not the 16-bit snorm of the cars. `chameleon_gfx::VertexDescriptor::decode` reads both layouts
(position, colour, first texture coordinate). **Not decoded [unconfirmed]:** the four bytes at 28 of a leaf vertex: they differ
between the corners of one card (for example 102, 46, 42, 170 and 79, 35, 56, 144) and the fourth byte runs from about 40 to 250 from
card to card, which fits a packed normal with a baked brightness, but the vectors are not unit length when read as `c / 127.5 - 1`, so the
tree variant makes its own normals ([below](#how-the-2005-game-lights-a-tree)). Usage 13 and 14 hold constants.

### The textures **[verified]**

The 22 textures are BC1 (16) or BC3 (6), 64 x 256 to 1024 x 1024, with the full mip chain. The leaf atlases are cut-outs: BC1 uses its one-bit
alpha (the index that makes a texel transparent), BC3 its alpha channel. Looked at as pictures, each foliage texture is:

| Texture id (low 24 bits) | Size | Picture |
|---|---|---|
| `104F52` | 512 x 512 BC1 | a whole broadleaf tree on a transparent ground (the leaf cards of the broadleaf models use pieces of it) |
| `15AB07`, `15C6C7`, `15C6D4` | 1024 x 512 and 512 x 512 | columnar trees, shrub strips, a yellow-brown tree |
| `27FFE`, `2800F`, `117864`, `27FC7`, `13CF1B` | 1024 x 512 and 512 x 512 | conifers with branch cards, spruces, a willow-like tree, an autumn tree with its bark strip |
| `27EDF`, `27F7C`, `27ED3`, `27EFA`, `27EDA`.. | 256 to 512 square | round bushes, thin bushes, a bare shrub |
| `1BDBD6`, `E376F` | 1024 x 1024, 512 x 512 | a pink blossom tree, a dead tree with its bark |

The bark maps (`104F49`, `27FFC`, `71314`, `E3770`, ...) are opaque pictures of bark.

## The species of the variant

Eleven of the 40 models carry the trees variant, and eight more the bushes ([the bushes](#the-bushes); [`trees/species.rs`](../../mods/nfsmw-2012-content/src/trees/species.rs)). The others are
copies of the same species at other sizes, or models whose resources are not in the bundles around them (`0x4DA8A407`, the blossom
tree, and `0xF0C5A212` miss a material or a texture in every bundle that holds them), or stands of trees of 80 to 300 m radius
(`0xEE68B20`, `0x9A045F0A`, `0x944B51E8`, `0xDF534A75`, `0xDCCDD936`) that are whole stands, not trees.

| Key | Model id | Read from | Levels | Bounding sphere of the finest level | Picture |
|---|---|---|---|---|---|
| `BROADLEAF_S` | `0x9EB74001` | `DISTRICT_1024094` | 4 | r 5.2 | small broadleaf tree |
| `BROADLEAF_M` | `0xB4FAD391` | `DISTRICT_164408` | 4 | r 11.4 | broadleaf tree |
| `BROADLEAF_L` | `0xE0663335` | `DISTRICT_701575` | 4 | r 15.2 | large broadleaf tree |
| `POPLAR` | `0xB0D173AF` | `DISTRICT_701575` | 4 | r 8.1 | columnar tree |
| `CONIFER_TALL` | `0xA97B9688` | `DISTRICT_701574` | 4 | r 28.8 | tall conifer with branch cards |
| `SPRUCE` | `0xDC0BADEE` | `DISTRICT_701574` | 4 | r 7.3 | dark spruce |
| `SPRUCE_S` | `0xDEDA583A` | `TRK_UNIT113` | 4 | r 10.5 | small spruce |
| `AUTUMN` | `0x315FCB51` | `GLOBALRESOURCES` | 4 | r 14.5 | autumn broadleaf tree |
| `AUTUMN_THIN` | `0x2A4A616D` | `TRK_UNIT115` | 4 | r 6.4 | thin autumn tree |
| `DEAD` | `0x0C41AAF0` | `TRK_UNIT146` | 4 | r 11.8 | bare tree |
| `BUSH` | `0xFAC87374` | `TRK_UNIT1` | 4 | r 3.4 | round bush |

### The levels that stand

The coarsest level of some species is **not a tree that stands on the ground**: it is a crown alone, with no trunk and a lowest point
that is metres above the model's origin (measured on the converted models, lowest and highest z of the level, metres, in 2005 space)
**[verified]**:

| Species, level | Lowest to highest | What it is |
|---|---|---|
| `AUTUMN` L3 | 1.2 to 4.8 | a small crown blob, a fifth of the tree's 23 m |
| `BROADLEAF_L` L3 | 7.8 to 22.0 | a crown alone from 7.8 m up (its trunk is two triangles) |
| `SPRUCE` L3 | 3.0 to 13.6 | crossed cards from 3 m up |
| `DEAD` L3 | 2.3 to 15.1 | the branch cards, no trunk |

Fitted to the 2005 far-tree solids (`1Z`), such a level hovered: `XT_CT_FALLTREED_1Z_JG_00` (a tree 25 m tall and 17 m wide in the 2005
game) became a crown **70 m wide and 14 m tall, 4.8 m above the ground** (the model's 3.6 m blob scaled by the 4x limit), seen from afar as dark
flat discs floating over the countryside; `XT_TREEMAPLEX_1Z_LL_00` got a crown from 12 m up. The converter therefore stops a species at the
first level that does not stand (its lowest point at most a tenth of its own top, and its top at least 60 percent of the finest level's,
`trees/build.rs`, `stands`) and the far solids take the coarsest level that does, so no solid of the variant hovers
([effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees)). The pack is 12 KB smaller.

For each of them the model, its renderables and materials, the vertex descriptors and shaders (`SHADERS.BNDL`) and the textures resolve in a pool of the
named bundle, `HAWAII/GLOBALRESOURCES.BNDL` and `SHADERS.BNDL` **[verified]**. The converter reads the bundles in that order (by ranges,
[below](#what-the-conversion-costs)) and needs no other.

## The bushes

The census of the **models with a `Tree_*` material** (every `HAWAII/` bundle, the shaders of `SHADERS.BNDL`, a scratch Python reader built on
`tools/chameleon_bundle.py` and `tools/chameleon_export.py`) finds **42 models**, not 40: the 40 of the first census and two more that the
first one counted as one. 11 are the trees above, 8 are **low growth** (2 to 3.2 m tall, a footprint of 3 to 7 m) and are the bushes of
the variant, and the rest are other trees (the large round bush `0x228F2FBA`, 7.2 m, and the thin-crowned `0x437640D7` and `0xE343E2A6` are trees
for the eye), stands of 60 to 350 m, planters (`0xF86CFC31`, `0xFD63F5E8`: a street lamp with a hanging basket, a concrete trough with shrubs) and one
arena prop. The eight were told apart **by looking at them**: a scratch software renderer (numpy, Pillow: the model's cards with their cut-out
textures, two views and the levels of detail) drew a contact sheet of every model under 7 m. **[verified]** for the sizes and the pictures,
the species names are ours.

| Key | Model id | Read from | Levels | Finest level (w x d x h, triangles) | Texture | Picture |
|---|---|---|---|---|---|---|
| `BUSH` | `0xFAC87374` | `TRK_UNIT1` | 4 | 6.2 x 4.5 x 3.2, 38 | `27EDF` | dark round bush (already of the first trees) |
| `SHRUB_GREEN` | `0x697E287C` | `TRK_UNIT33` | 3 | 4.1 x 4.0 x 2.7, 123 | `104F52` | green deciduous shrub, a clump of stems |
| `HEDGE` | `0xA807609B` | `TRK_UNIT158` | 3 | 6.7 x 2.6 x 2.0 above its origin (its stems root 0.9 m below it), 220 | `104F52` | a hedge of upright leafy stems, 6.7 m of it |
| `SHRUB_STRIP` | `0x5EE210C0` | `TRK_UNIT33` | 3 | 6.8 x 3.6 x 2.9, 116 | `15AB07` | olive shrub in a strip of two humps |
| `DRY_SHRUB` | `0x8CA31079` | `TRK_UNIT113` | 3 | 3.6 x 4.3 x 2.3, 84 | `27ECA` | twiggy brown shrub, grass at its foot |
| `DARK_SHRUB` | `0x345185D4` | `TRK_UNIT115` | 3 | 4.5 x 4.3 x 3.1, 474 | `27ED9` | dark, spiky evergreen shrub |
| `PALE_SHRUB` | `0x7AC64A1F` | `TRK_UNIT68` | 4 | 3.0 x 3.2 x 3.1, 172 | `27EFA` | thin-stemmed shrub with pale, yellowish leaves |
| `GRASS_TALL` | `0x8F4BEEBD` | `TRK_UNIT158` | 3 | 3.3 x 7.3 x 2.6, 136 | `15C521` | a strip of tall grass tufts |
| `GRASS_CLUMP` | `0xCD874A5B` | `TRK_UNIT22` | 3 | 2.1 x 2.6 x 2.3, 8 | `15C521` | ornamental grass: 8 triangles of long blades |

Each is in its bundle with every resource it needs but the shared ones (`GLOBALRESOURCES.BNDL`, `SHADERS.BNDL`): the bundle named is the smallest that
passes that check, found by the same reader as above **[verified]**. The same rules as for the trees apply (`trees/build.rs`): a level that does not
stand is dropped. No model has bark. There is a second, near-identical hedge, `0xCCEA1CBC` (4 levels), that is **not** used: it stands in a concrete
trough (a bark-role mesh of `WorldPBR_*` with the texture `71965`), and in the game the trough came out as tan boards sticking out of the hedge (the first
screenshots, `hedge1`), so the plain hedge `0xA807609B` replaced it.
The 2012 game has **no fern, no flowerbed and no reed**: `XT_FERN` keeps the 2005 look.

**The 2005 side** (a report of the solids of `STREAML2RA.BUN` by name, `report_the_solids_of_some_families` in `replace/real_tests.rs`;
solids, of which world-space copies, the width of the long side of a copy at the median and the 90th percentile, metres):

| Family | Solids (copies) | Width p50 / p90 | Vertices p50 | Effect | Replaced by |
|---|---|---|---|---|---|
| `XT_CT_BUSH_1B_JG_00` | 176 (175) | 78 / 153 | 100 | 0 | `SHRUB_GREEN` |
| `XT_BUSH_FOREST_1A/2A_JG_00` | 125 (122) | 75 / 162 | 176 | 0 | `DARK_SHRUB` |
| `XT_GC_BUSH_1B_00` | 47 (46) | 47 / 115 | 124 | 0 | `SHRUB_STRIP` |
| `XT_CT_BUSHMULLB_1B_00`, `XT_BUSHGROUP`, `XT_BUSHMULLD` | 25, 44, 11 | 32 to 69 | 84 to 330 | 0 / 6 | `BUSH` |
| `XT_BUSHREDA_1B_LL_00`, `XT_BUSHB_1C_AL_00` | 18, 1 | 95 | 144 | 0 | `DARK_SHRUB`, `SHRUB_GREEN` |
| `XT_BUSHYELLOWFLOWERS` | 19 | 20 to 57 | 56 to 112 | 6 | `PALE_SHRUB` |
| the hedges: `XT_HEDGE*`, `XT_HEDGEA4M/A8M`, `XT_HEDGEMUL_*`, `XW_HEDGESTADIUM` | 137 | 20 to 77 | 270 to 820 | 0 | `HEDGE` |
| the grasses: `XT_GRASSA/B/C`, `XT_GRASSPATCH`, `XT_BEACHGRASS`, `XO_GRASSCLUMPCT` | 221 | 35 to 70 | 90 to 180 | 0 / 6 | `GRASS_TALL`, `GRASS_CLUMP` |

Almost all of them are **world-space copies of a stretch** (a patch of undergrowth along a slope, a hedge along a house) with a few hundred vertices
and a footprint of tens of metres, so the fit of the trees (one copy for each stretch of the old vertices, `fit = "row"`) applies as it is; the fit of
a hedge row is the hedge model 6.7 m long, repeated along the stretch at the old height. Measured on the real tiles with the pack
(`report_what_the_trees_do_to_the_installed_world`): of the 58 names (**832 solids, 774 of them world-space copies**: 474 bush, 137 hedge, 221 grass) **no solid keeps
its own model, and the new footprint is 1.0 to 1.5 times the old** (median 1.01: the 1.5 is the `spread` of the bushes' limits) and 0.8 to 1.4 times as tall
**[verified]** on the real tiles. What was looked at in the game, and where it differs from the 2005 look: [below](#what-was-looked-at).

### Clearance **[ours]**

The bushes' limits (`spread` 1.5) make a copy up to half as wide again as the old stretch, and playing showed it: grass clumps and
hedges grew through the guard rails and concrete barriers they stand beside. The bushes set therefore asks for **clearance**
(`[fit] clearance = true`, [`api-replace.md`](../modding/api-replace.md#clearance)): each copy is narrowed until it clears the walls, rails,
barriers and buildings (`XW_`, `XB_`) of its section, to no less than the width of the stretch it replaces, and a copy rooted inside one is
left out. The trees do not ask for it (a crown may overhang a rail). The mod's conversion number went up with it, so an installed 2012
mod converts again on the next start. The ignored test `report_how_many_bushes_are_made_smaller_to_clear_their_neighbours`
(`crates/nfsmw/src/modding/content/replace/clear_real_tests.rs`) rewrites every section of the installed world and prints how many
copies near an object were narrowed, held or left out.

### What was looked at

Offscreen screenshots of the real world (`nfsmw view-world --muted --wait-for-load --at x,y --height 2.5 --heading 0 --pitch -8`, the camera 2.5 m above the
ground, the places are the camera's `--at`; the same place with the Bushes setting Original and then NFS Most Wanted 2012). Where, and what was seen:

| Place (x, y) | 2005 family | Before | After |
|---|---|---|---|
| 2166, 1368 (a road median) | `XT_CT_BUSH` | one long clipped dark hedge block | three separate round green shrubs with gaps (a patch is a row of copies, not a block) |
| 2478, 4077 (under a viaduct) | `XT_BUSH_FOREST_1A` | thin dark twiggy bushes | lush spiky evergreen shrubs (`DARK_SHRUB`), larger and lighter than the old |
| 1992, 1070 (a roadside bank) | `XT_GC_BUSH` | brown-orange leaf mass | pale leafy `SHRUB_STRIP` clumps at the foot, the autumn trees unchanged |
| 1236, -55 (planters of a street) | `XT_HEDGE` | flat-topped clipped boxes | rounded leafy clumps, soil showing between them |
| 1067, 135 (a stadium walk) | `XW_HEDGESTADIUM` | a dark clipped box | a leafy hedge filling the planter |
| 460, 4206 and 4034, 3356 (a garden bed, a hedge along a lot) | `XT_HEDGEA4M`, `XT_HEDGEA8M` | clipped dark boxes | light green leafy hedges, bright against the dark old ones |
| 895, 25 (a park) | `XT_BUSHYELLOWFLOWERS` | blobby yellow-green bushes | thin-stemmed shrubs with pale leaves (`PALE_SHRUB`), a clear gain |

Findings: a 2012 bush is **brighter and leafier** than the 2005 ones (the 2012 leaves are lit by the game's tree shader like the trees' ones, with the grey ramp
of `trees/light.rs`), and a **patch is a few separate shrubs, not a solid block**: the old hedge boxes were continuous, the copies are one for each stretch of the old
vertices and leave gaps where the old stretch had fewer than three vertices. That is the look of the 2012 game's own hedges, but it is **less dense than the
old** (open question below). The grasses were fitted (221 solids, no solid kept) but **no place with visible grass was looked at** **[unconfirmed]**.

**Cost:** the pack grows by about 0.7 MB (3.5 MB in all: 0.9 MB of models, 2.5 MB of pictures) **[verified]** (it was 2.8 MB). The frame cost on dense tiles and the vertex totals
(`report_what_the_trees_do_to_the_installed_world` now prints the old and new vertex and triangle counts of every solid) were **not measured yet** **[unconfirmed]**: the one
`--bench-seconds` run taken was on a busy machine (p50 6.6 ms, p95 21 ms, with the bushes off).

## How the 2012 world places them **[not read]**

The world places models through `PropObject` (0x217, 1,446), `CompoundObject` (0x215, 671), `WorldObject` (0x205, 1,307) and
the instance lists of each track unit (`InstanceList` 0x50, `PropInstanceList` 0x218, `CompoundInstanceList` 0x216, 169 each). Their
layout was not decoded: the variant needs the models and textures only, as the 2005 game places its own trees. The `InstanceTint`
shader implies a colour chosen per instance; it is not applied by the variant (the leaves are drawn with the texture's colours).

## How the 2005 game draws its trees **[verified]**

In `TRACKS/STREAML2RA.BUN` (20,377 solids in 720 sections, [maps.md](maps.md)):

- A **tree solid** is one with a shading group of **effect 6** (the tree shader, [world-material-routing.md](../specs/world-material-routing.md)).
  1,004 solids have one, which are **99 base names** once the per-tile world-space copies (`<NAME>_DEINST<n>_<tile>`) are folded
  into theirs.
- The solids that the scenery instances place with a matrix are in the shared library section **`X0`** (84 of the 1,004 tree solids) and,
  for ten (the panoramas, the canopies, a stand of cypresses), in a single tile. The other 910 are **world-space copies** (`<NAME>_DEINST<n>_<tile>`, 178
  of them of `XT_CT_TREELINE1_1B_00` alone): a copy of a solid with an instance's matrix baked into its vertices, drawn without a
  matrix ([backface-culling.md](../specs/backface-culling.md)), with bounds in world coordinates. A replacement by name therefore
  reaches the instanced trees, and not the copies.
- The name says what it is: `XT_` and `XO_` trees and bushes, `PAN_*` the panorama backdrops (kilometres wide, not replaced), the
  letter after the number is the level of detail (`1A` and `1B` finest, `1C`, `1Z` coarsest). Examples: `XT_CEDARM_1B_JG_00` (49.6 m
  tall), `XT_CT_POPLARD_1B_JG_00` (21.4 m), `XT_CT_FALLTREEDEAD_1B_JG_00`, `XT_JPNMAPLE_1B_JG_00`, `XT_BUSHMULLD_1B_RB_00`,
  and the stretches `XT_CT_TREELINE1_1B_00` (88 x 44 m, from 7.4 to 45.9 m high), `XT_EVRGRN_TREELINEL_1B_00`, `XT_CT_TREECANOPY_1B_00`.
- A tree solid is, for most, a trunk group (**effect 0**, group flags `0x4000`, or the normal-mapped effect 3 with `0x224000`) and one or
  more leaf groups (**effect 6**, flags `0x4008`, or `0x224008`), 36-byte vertices, from a few to several thousand triangles. The solid's flags are 0 except
  `0x80` on 13 of them (they sway, [scenery-sway.md](../specs/scenery-sway.md)).
- Single trees stand on z = 0, centred on x = y = 0 to a few metres, up is +z; a row's canopy floats (its lowest point is 7.4 m on
  `XT_CT_TREELINE1_1B_00`).
- The leaf textures (`ORG_*`) are DXT3 or DXT5 with punch-through alpha (`AlphaUsageType` 1, blend 0), which draws both faces
  ([backface-culling.md](../specs/backface-culling.md)).

### How the 2005 game lights a tree **[verified]**

The leaf groups carry **normals that point away from the middle of the crown** (so a card is lit like a point of a ball) and a **grey
vertex colour that grows from about `0x2F` at the bottom of the crown to `0x46` at the top** (trunks are `0x35` to `0x3B`). The tree
effect multiplies ambient light by that colour and ignores vertex alpha (`2 * T * (A * V + D * L * E) + S`). The variant makes its
vertices the same way, because the 2012 vertices have no normals that can be read.

## What the variant does with them

| Step | Rule | Where |
|---|---|---|
| Axes | 2012 (x left, y up, z forward) to 2005 (x forward, y left, z up): `(x, y, z)` becomes `(z, x, y)`, a cyclic permutation, so winding and handedness are kept | `fit::to_car_space` |
| Leaf groups | effect 6, flags `0x4008`; texture `AlphaUsageType` 1 (punch-through), `Flags` bit 1, from BC1 or BC3 as they are | `trees/build.rs`, `Palette::add_foliage` |
| Bark groups | effect 0, flags `0x4000`; opaque texture that repeats | `Palette::add_bark` |
| Normals and colours | crown normals and the grey ramp above for leaves; smooth normals (area-weighted, shared at seams) and `0x38` for bark | `trees/light.rs` |
| Levels | `1A`/`1B` the finest level of a single tree, `1C` the second, `1Z` the last level **that stands** ([above](#the-levels-that-stand)); a row takes one level coarser (`A`/`B` the second, `C` the third) | `trees/stock.rs` (`level`), `trees/build.rs` |
| Meshes left out | the `WorldShadow_*` stand-ins | `trees/read.rs` (`classify`) |
| Species per solid | the families of `trees/stock.rs`: for example the cedars and the tall conifer, the poplars and the cypresses the columnar tree, the fall trees and the Japanese maple the autumn tree, the dead trees the bare tree, the bushes the round bush, the tree lines and canopies rows of the broadleaf trees, the evergreen lines rows of the small spruce | `trees/stock.rs` |

What is **not** replaced: the panoramas (`PAN_*`), `XT_CT_JUNIPERA_*` (a 135 x 120 m carpet of juniper), `XT_TREECYPRESS_*` and
`XT_TREEWIDE_*` (whole stands in a few tiles), the grass clumps and patches (`XO_GRASSCLUMPCT_*`, `XT_GRASSPATCH_*`), and the solids
of the 99 base names that no family lists. A world-space copy (`<NAME>_DEINST<n>_<tile>`, 910 of the 1,004 tree solids) **is** replaced
when `<NAME>` is: the game finds its swap by the name before `_DEINST`, and fits the model to the copy where it stands (its footprint,
floor and height), see [effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees). The 84 solids of the shared
library `X0` are replaced like the tiles' (by name, fitted to their own bounds).

The fit of a model to the solid it replaces (scale to the old height, stand on the old floor, one copy for each stretch of the old
geometry, never wider than the old footprint, or the solid keeps its own tree) is in
[effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees); it is done by the game, from the vertices of the solid
it replaces, so the mod never reads the 2005 install.

## What the conversion costs

The 2012 game's files are never converted whole: the conversion opens the nine bundles the species come from by **ranges**
(`chameleon_bundle::Bundle::parse_ranges`, a reader the mod supplies; the crate opens no files) and asks only for the table of each and for
the blocks of the resources a tree is made of (the model, its renderables, materials and the first texture of each material, and
the texture's pixels). The shaders are the 1.8 MB `SHADERS.BNDL` the car conversion holds anyway.

| What | Figure | |
|---|---|---|
| The nine bundles | 350,654,140 bytes (DISTRICT 28 to 51 MB, TRK_UNIT 8 to 37 MB, GLOBALRESOURCES 97 MB) | **[verified]** |
| Read from them, whole-bundle reading (before) | 350 MB, and as much memory while a bundle is open | **[verified]** |
| Read from them, by ranges (now) | **2.48 MB**: the tables 0.41 MB, the blocks 2.07 MB (0.7 % of the bundles) | **[verified]** with a Python replay of the same reads (header and table, then the blocks of the model, renderables, materials and textures) |
| Time to read them | 0.02 s of a Python replay on a warm cache, including the inflating; a cold disk adds the seeks of the small reads | **[verified]** (warm) |
| Textures the species use (diffuse pictures) | 17, 3.24 MB at the 1,024 pixel limit of `max_texture_size` (2.85 MB at 512, 0.72 MB at 256) | **[verified]** from the headers |
| Models | the vertex and index data of the species' renderables is 0.60 MB in the 2012 layout (the pack's solids, of the vertices the triangles use, with 36-byte vertices and without the shadow stand-ins, are about as big) | upper bound **[verified]**, the pack **[unconfirmed]** |
| Written to the mod's `files/TREES/MW2012/` | 2.8 MB in three files (`GEOMETRY.BIN` 739 KiB, `TEXTURES.BIN` 1,996 KiB once, `profile.toml` of 81 swaps, 7 KB) | **[verified]**: 0.69 s on the install, 2,668 KiB read of 342,435 KiB |

Nothing is duplicated: each texture is one entry of `TEXTURES.BIN` (named by the 2012 resource id), one decoded copy in memory
(the pack's compressed pictures, until the world drops the pack) and one on the GPU (the shared sets), whatever number of tiles
use it; the sections of the stream carry none ([effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees),
"Textures once"). The mod writes files at all because the host reads packs through the install's layers; streaming the models in
memory at the start of a drive would need a host interface for mods that hand over bytes, which is not built, and the cost above is
what that would save: about 4 MB of disk and a conversion of a fraction of a second at the first start (later starts convert
nothing: the mod's fingerprint says the files are right).

## Open questions

- The four bytes at offset 28 of a leaf vertex, and whether the 2012 shader lights with them.
- The placement records (`PropObject`, `CompoundObject`, instance lists): where the trees stand, how they are rotated and scaled, and what
  the instance tint is.
- The resources of the blossom tree `0x4DA8A407` and of `0xF0C5A212` are not in the bundle of any unit that holds the model: another
  bundle must hold their material or texture (not searched).
- The 2005 world's own placement of the replaced solids is unchanged, so a 2012 tree stands where the 2005 tree stood; the crowns are
  fitted to the cells of the old vertices, not to the old silhouette ([effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees)).
  The real ground under a tree-line's crowns is not read: a collision query of the tile found it under only 186 of the 881 replaced
  solids (the terrain is not in a tile's collision), and where it was found the old crowns' floor was at the ground or below it (median
  3 m below). Trying to hang the crown of a replacement on the old floor and let its trunk reach down put the trunks through a wall in a
  screenshot, so a replacement's foot stands on the old floor of its stretch.
- The bushes' density: a hedge row is a row of separate copies; a per-set overlap (more copies, narrower cells) would close the gaps in planters but is a
  new field of the replacement API (`copies`, `overlap`), not built. The grasses were not looked at in the game (`XT_GRASS*`, `XT_BEACHGRASS`, `XO_GRASSCLUMPCT`).
- Whether every name of the 99 base names that is a tree (and not a panorama or a stand) is in `trees/stock.rs`: the families were
  listed from the names of the tree solids that were looked at; a name the list lacks keeps the game's look.
- The conversion's output size and time on the real install: the figures below are estimates from the headers; the log line `trees variant:
  read ... KiB of ... KiB ... ms` of a conversion, and the ignored test `the_trees_are_read_by_ranges_and_written_in_a_few_megabytes`, give them.
