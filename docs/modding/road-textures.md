# The 2012 road, ground and sign textures (`nfsmw-2012-content`, setting `road_textures`)

The mod [`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md) replaces textures of the 2005 world by pictures of NFS: Most
Wanted (2012): the asphalt, the painted lines and the crosswalks, the sidewalks, the grass, concrete, the storm grate and the round
cover, and the STOP and speed limit signs. It uses the replacement API of the game ([api-replace.md](api-replace.md)): one set of kind
`road_textures`, which the game shows as the Gameplay option **Road textures** (`road_textures_variant`: *Original*, *NFS Most Wanted
2012*). Where the pictures are and why the 2005 road is built the way it is: [chameleon-road-textures.md](../formats/chameleon-road-textures.md).
This page is the table of what replaces what, and what it costs.

## How it works

1. `src/road_textures/table/` lists the textures to replace by **name hash** (the 2005 pack keeps 23 characters of a name, so three
   textures of different roles share a name) and says for each one which recipe makes its picture, at what size, how bright, and with
   what alpha.
2. At every start the mod writes `replace/road_textures-mw2012.toml` (the set, one `[[texture]]` with `target_hash` and `file` for each
   row) and a **stub** of 41 bytes for each file under `files/REPLACE/ROADTEX/`. That is all that goes to the disk: about 3 KiB of
   stubs and 7 KiB of manifest. With the setting off (`road_textures = false`) both are removed and the option disappears.
3. When the game first loads a section that holds one of the textures, it reads the stub, and the mod ([`road_textures::Maker`], the
   same `provide_lazy_files` maker as the streamed cars) builds the picture: it opens `HAWAII/GLOBALRESOURCES.BNDL` by byte range (only
   the table, 100 KiB of a 97 MB file), reads the compressed blocks of the textures the recipe names (0.2 to 3 MB each), decodes the mip
   it needs, crops, turns, resizes and layers it (`layer.rs`), compresses it with the game's own DXT encoder and a full mip chain
   (`recipe.rs`), and returns a `.dds`. The game keeps that texture for every section.
4. The replacement takes the **draw state** of the texture it replaces (the game's default `state = "keep"`): tiling, alpha usage and
   blend stay, as the materials of the 2005 world were authored for them.

## Size, format and mips

- **Resolution.** A 2012 picture is larger than the 2005 one, so a replacement is **twice the size of the texture it replaces** on each
  side (128 x 1024 becomes 256 x 2048, 512 x 512 becomes 1024 x 1024) unless a side would pass 2048, and keeps its shape, so the game's
  mapping still fits. The recipe reads from the smallest mip of the 2012 picture that still has the pixels, so a reduced picture is
  never taken from the full size.
- **Format.** DXT1 where the 2005 texture is opaque. DXT5 with a **constant alpha of 64 (25 %)** where the 2005 texture is DXT3 with
  alpha usage 2: its alpha is a specular mask ([textures.md](../formats/textures.md#alpha)), and a picture without alpha would read as a
  fully shiny road. Normal maps are DXT1 and keep the 2005 convention (RGB, blue up) **[the green channel was not compared]**.
- **Mips.** A full chain down to 1 x 1, made by the game's encoder (`blackbox_tpk::mip_chain`), so distance filtering behaves as for the
  2005 textures.
- **Brightness.** The 2005 textures are dark (the road averages 43 to 57 of 255, the grass 46); the 2012 ones are lit differently
  (asphalt 115, grass 66). Each row has a `gain` that is the ratio of the 2005 texture's mean to the 2012 picture's, so the road is as
  dark as it was and the new detail shows.

## What replaces what

Hashes are those of the texture records of `STREAML2RA.BUN` (`bStringHash` of the full name); "solids" counts the world solids that name
the texture. The rows were measured with scratch scripts over the stream (`tools/chameleon_texture_roles.py` for the 2012 side).
Roles (diffuse, normal, specular) were read from the images; **specular maps are left alone** (a 2012 specular map is packed
differently).

### Road

| 2005 texture (name as the pack keeps it) | hashes | solids (summed) | 2012 source | out size | notes |
|---|---|---|---|---|---|
| `TRNS_ALL_LAB_PAVEMENTA_` | `06EE3A6D`, `D5B76C7D` | 1737 | asphalt `0x0006175B` (1024x1024), square tiles | 256x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LAB_PAVEMENTA_` | `158FF03B`, `6FB1FA2B` | 1737 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LAB_PAVEMENTB_` | `10F7A61E`, `422E740E` | 1083 | asphalt `0x0006175B` (1024x1024), square tiles | 256x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LAB_PAVEMENTB_` | `21C5161C`, `7BE7200C` | 1083 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LB_PAVEMENTA_N` | `257795EC` | 645 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LB_PAVEMENTA_N` | `B767F1EE`, `DB34B2A4` | 1290 | asphalt `0x0006175B` (1024x1024), square tiles | 256x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LB_PAVEMENTB_N` | `282EEF65`, `E85B4F4F` | 324 | asphalt `0x0006175B` (1024x1024), square tiles | 256x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LB_PAVEMENTB_N` | `60B7CF8D` | 162 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LB_PAVEMENTA_W` | `DBD32C25`, `DBD78F2D` | 995 | asphalt + white paint `0x0006F394` | 256x2048 | white edge line in columns 2 to 5 of 128, as measured on the 2005 strip |
| `TRNS_ALL_LB_PAVEMENTB_W` | `28CD68E6`, `28D1CBEE` | 496 | asphalt + white paint `0x0006F394` | 256x2048 | white edge line in columns 2 to 5 of 128, as measured on the 2005 strip |
| `TRNS_ALL_LA_PAVEMENTA_Y` | `491D7B26`, `4921DE2E` | 1037 | asphalt + yellow paint `0x0006F393` | 256x2048 | yellow centre line in columns 2 to 4 of 128, as measured |
| `TRNS_ALL_LA_PAVEMENTB_Y` | `9617B7E7`, `961C1AEF` | 611 | asphalt + yellow paint `0x0006F393` | 256x2048 | yellow centre line in columns 2 to 4 of 128, as measured |
| `TRNS_ALL_LZ_PAVEMENTA_W` | `19035965`, `63AD9AFB` | 175 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LZ_PAVEMENTA_W` | `A0405C3D` | 108 | asphalt + white paint `0x0006F394` | 256x2048 | edge lines at both sides (columns 2 to 4 and 117 to 126 of 128) |
| `TRNS_ALL_LZ_PAVEMENTB_W` | `54439306`, `F6877CFC` | 465 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LZ_PAVEMENTB_W` | `ED3A98FE` | 153 | asphalt + white paint `0x0006F394` | 256x2048 | edge lines at both sides (columns 2 to 4 and 117 to 126 of 128) |
| `TRNS_ALL_LZ_PAVEMENTA_P` | `09F8D204`, `1DB6FD23` | 173 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LZ_PAVEMENTA_P` | `97FBD9BB`, `B7661F65` | 340 | asphalt + white paint turned a quarter turn | 256x2048 | 20 bars, one every 51 rows of 1024, as measured |
| `TRNS_ALL_LZ_PAVEMENTB_P` | `06C528E6`, `843DAE9C` | 104 | asphalt + white paint turned a quarter turn | 256x2048 | 20 bars, one every 51 rows of 1024, as measured |
| `TRNS_ALL_LZ_PAVEMENTB_P` | `C0FE6AE4` | 52 | aggregate normal `0x001286AF` (512x512 BC3) | 256x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LW_PAVEMENTA_I` | `25627894`, `BE10F98A` | 22 | asphalt `0x0006175B` (1024x1024), square tiles | 512x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LW_PAVEMENTA_I` | `FC2E5E12` | 11 | aggregate normal `0x001286AF` (512x512 BC3) | 512x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LW_PAVEMENTB_I` | `74C18215`, `AA52CE6B` | 460 | asphalt `0x0006175B` (1024x1024), square tiles | 512x2048 | plain asphalt; the 2012 asphalt is lighter, so the gain brings it to the 2005 brightness |
| `TRNS_ALL_LW_PAVEMENTB_I` | `9F75CBD3` | 229 | aggregate normal `0x001286AF` (512x512 BC3) | 512x2048 | normal map of every asphalt strip (the old strip's cracks are not kept) |
| `TRNS_ALL_LZ_SIDEWALKA_0` | `7E7B5460`, `92397F7F` | 996 | slab normal `0x00042128` (1024x2048 BC3) | 256x2048 | same part of the picture |
| `TRNS_ALL_LZ_SIDEWALKA_0` | `916C5AC1`, `93C18362`, `B7BC8B97`, `B7BC8B98` | 2022 | concrete slabs `0x00042129` (2048x4096), a strip of a quarter of its width | 256x2048 | slabs keep their shape |

### Ground

| 2005 texture (name as the pack keeps it) | hashes | solids (summed) | 2012 source | out size | notes |
|---|---|---|---|---|---|
| `MD_GRASSYTILE1024` | `9717031D` | 48 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 256x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TER_CAMP_GRASS1` | `1EA99F5A` | 1 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 1024x1024 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GRASSA_LD_GRASSA_0` | `156E7175` | 2180 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GRASSA_LE_GRASSA_C` | `10BBF63A`, `10BBF63B`, `10BD0EFC`, `10BD9B5D` | 562 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 256x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GRASSA_LE_GRASSA_C` | `367E5485`, `92AA09A3`, `A66834C2` | 394 | grass normal `0x000A97AC` (1024x1024 BC3) | 256x2048 |  |
| `TRNS_GRASSB_LD_GRASSB_0` | `50AF3777` | 30 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GRASSC_LD_GRASSC_0` | `8BEFFD79` | 255 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRN_TPARKGRASS1` | `5A588F65` | 2 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x512 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GCGRASS_LD_FAREWAY` | `074BC2D0`, `633B764C` | 156 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 256x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GCGRASS_LD_ROUGH_0` | `0319B1A2` | 135 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GCGRASS_LE_ROUGH_C` | `319561A7`, `319706CA` | 97 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 256x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRNS_GCGRASS_LE_ROUGH_C` | `E5390352` | 92 | grass normal `0x000A97AC` (1024x1024 BC3) | 256x2048 |  |
| `TRNS_GCGRASS_LX_PUTTING` | `896E0AAD` | 35 | grass `0x000A97AB` (2048x2048), 1024x1024 crop in square tiles | 512x2048 | no seams at the tile edges to the eye; the 2005 mowing stripes of the golf green are lost |
| `TRN_OLDCONCRETE_SIMPLE` | `032EA50D` | 290 | concrete slabs `0x00042129`, a 2048x2048 square | 1024x1024 | stands for the cracked old concrete |
| `TRN_OLDCONCRETE_SIMPLE_` | `9D8D9835` | 200 | slab normal `0x00042128` | 1024x1024 |  |
| `ARC_CAMPUS_MBWALL_CONC1` | `179F44B8`, `43309F2E` | 581 | concrete wall `0x000D4974` (1024x512), cut to the shape of the 2005 texture | 512x512 | also the barrier walls of the campus |
| `ARC_CAMPUS_MBWALL_CONC2` | `43309F2F` | 294 | concrete wall `0x000D4974` (1024x512), cut to the shape of the 2005 texture | 512x512 | also the barrier walls of the campus |
| `ARC_CAMPUS_MB_WALL_CONC` | `49F2C938`, `B59F53AE` | 475 | concrete wall `0x000D4974` (1024x512), cut to the shape of the 2005 texture | 512x512 | also the barrier walls of the campus |

### Covers

| 2005 texture (name as the pack keeps it) | hashes | solids (summed) | 2012 source | out size | notes |
|---|---|---|---|---|---|
| `OBJ_STORMGRATE` | `91557381` | 8 | storm grate `0x0006D245` (256x256), the middle 128 columns | 128x256 | the 2005 grate is 1:2, the 2012 one square |
| `OBJ_STORMGRATE_NORMAL` | `0C151C29` | 8 | grate normal `0x0006D247` | 128x256 |  |
| `OBJ_GASFILLCOVER` | `678A28FA` | 1 | round cover `0x0006D22D` (512x512) | 256x256 | on concrete grey; the 2005 gas-fill cover is the only round cover of the 2005 world |
| `OBJ_GASFILLCOVER_NORMAL` | `9669CF82` | 1 | cover normal `0x0006D22E` | 256x256 |  |

### Signs

| 2005 texture (name as the pack keeps it) | hashes | solids (summed) | 2012 source | out size | notes |
|---|---|---|---|---|---|
| `SGN_STOP` | `3B1AD9AC` | 12 | STOP sign `0x0006D4E6` (256x512), the octagon | 256x256 | black corners are clear, the metal grey shows |
| `SGN_SPEEDLIMIT50` | `80F4681B` | 1 | speed limit signs `0x0008C366`, the lower sign turned upright | 256x256 | the only 2012 limit that the 2005 world has (50); the face keeps its shape (portrait) on a square |
| `SGN_SPEEDLIMIT55` | `80F46820` | 1 | speed limit signs `0x0008C366`, the '5' of the 50 copied over its '0' | 256x256 | the 2012 pictures have a 2, a 5 and a 0 only, so the 55 is made; the sign stands about 60 m from the first intersection (at about 2045, 1593 of the map) |


### What is not replaced, and why

- **Arrows and `ONLY`** (2012 textures 41, 50, 53): the 2005 data has **no ground arrow** texture; `SGN_*ARROW*` are freeway signs and
  traffic signal arrows. Painting one needs a decal on the road, which is a model, not a texture
  ([api-replace.md](api-replace.md): `[[add]]` is not placed by the host yet).
- **Hazard and chevron strips** (`LZ_PAVEMENT*_Y` and the wide `W` ones, diagonal stripes), the boardwalk, `SIDEWALKA_D`, the cobbles, the
  cliffs, the golf `LC_PAVEMENT` transitions and every texture of a building keep their 2005 pictures.
- **Speed limits 35, 45, 60 and 75**: the 2012 game has the numbers 2, 5 and 0 only; the 50 is its own and the 55 is made from it.
- **The 2012 round "BWW" square cover** (`0006D26A`) and the second grate texture have no 2005 slot.
- **Shadow overlays** (`SHD_*`, 4,072 solids) are not roads (see [the format page](../formats/chameleon-road-textures.md)).

## Cost

**Disk.** Stubs of 41 bytes for 81 pictures (3 KiB) and a manifest of about 7 KiB: 10 KiB, written at every start only when
they differ. Nothing of the 2012 game is written; the pictures exist only in memory.

**Memory and the GPU.** The 81 pictures are 37.7 MiB with their mips (road strips 24.3 MiB, ground 13.1 MiB, covers and signs 0.3 MiB);
the 81 textures they replace were 10.8 MiB. A texture is made the first time a section that has it loads and is kept once for every
section, so the cost is paid as the player meets each kind of ground: the first asphalt strip makes the asphalt, and the other strips
that use it share the decoded picture. The game's texture detail setting drops top mips of a mod's textures as of its own.

**Reading the install.** The bundle is opened by byte range: the table (about 100 KiB of 97 MB) and, for each picture the recipes use,
the compressed graphics block (0.2 to 3 MB): the asphalt, the slabs, the grass, the paint, the covers and the signs are 15 textures in all.

**Time and frames.** MEASURED_PLACEHOLDER

**What is lost.** The 2005 road strips carry cracks, wheel tracks and the grime of their lane; the replacement is the 2012 asphalt,
which is uniform, so the cracks go (the leaves and the decals of the 2005 road are separate and stay). The golf green's mowing
stripes are lost on the grass.


## Screenshots

Before and after (the option *Original* against *NFS Most Wanted 2012*), offscreen, in an isolated profile (`C:6work\profile-roadtex`,
the mod's `config.toml` with only `road_textures` and one car on, the choice saved in `[mod_values.nfsmw]`):

```sh
BLACKBOX_PROFILE_DIR=C:6work\profile-roadtex nfsmw view-world --muted --wait-for-load --at 2080,1504 --height 4 --heading 90 --pitch -55 --screenshot road_close.png
```

The places (`--at X,Y --height H --heading D --pitch P`): road close `2080,1504 4 90 -55`; road far `2060,1504 9 90 -6`; crosswalk and
lines `2102,1476 6 90 -40`; speed limit 55 `2050,1575 2.5 100 0`; STOP face `3160,1845 2.5 90 0`; grate `3700,700 3 90 -60`; grass
`2187,1334 6 90 -40`; concrete lot `2075,1548 3 90 -4`; slabs of the sidewalk `2060,1535 3 105 -2`. The pictures are in
`C:6workoadtex-shots` (`<place>_before.png`, `<place>_after.png`); they show the game's textures, so they are not in the repository.

## Using and changing it

- Setting: `road_textures` (default `true`) in the mod's `config.toml`. The player's choice is the Gameplay option **Road textures**
  (texts: the message ids `nfsmw2012-road-textures` and `nfsmw2012-road-textures-value`, in `lang/en`, `lang/it` and the `pt-BR` language
  pack). A change of the option made during a drive streams the resident map tiles again; the textures of the shared sets follow at the
  next drive ([api-replace.md](api-replace.md#textures)).
- A new row is a line of `src/road_textures/table/<group>.rs`: `Entry::new(hash, "NAME", &RECIPE, width, height, alpha, gain)`. A new
  look is a `Recipe` of layers in `table/recipes.rs`; a layer is a crop, a quarter turn, a destination rectangle, a tiling and a blend
  (`layer.rs`). Add the 2012 picture to `table/sources.rs` with its index and what it is.
- Tests: `cargo test -p nfsmw-2012-content road_textures` (the tables, the layers, the files); with `MW2012_GAME_DIR` set and
  `--include-ignored`, every row is built from the install and read back by the game's DDS reader, and with `ROADTEX_OUT=<folder>` every
  picture is also written as a PNG to look at.
