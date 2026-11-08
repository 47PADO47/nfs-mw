# Road, ground and sign textures: the 2012 pictures and the 2005 textures they can replace

Where the road, ground and sign pictures are in NFS: Most Wanted (2012), what each is used as, how the 2005 game textures its roads
and ground, and which 2005 texture each 2012 picture fits. The mod that does the replacement is
[`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md) (module `road_textures`); the table of what replaces what is
[road-textures.md](../modding/road-textures.md). Container: [chameleon-bundle.md](chameleon-bundle.md); texture and material layout:
[chameleon-gfx.md](chameleon-gfx.md); 2005 texture packs: [textures.md](textures.md). Evidence tags:
[README](../README.md#evidence-tags). Everything was measured on the 2012 install (v1.4.0.0) and the 2005 install (PC, v1.3)
unless tagged; nothing of either is in the repository.

## The 2012 side: `HAWAII/GLOBALRESOURCES.BNDL` **[verified]**

The bundle (97 MB) holds the world's shared resources: **261 textures**, 101 renderables, 27 models, 96 materials. A texture has no
name; what it is comes from the materials that use it. A material imports its textures in the order of its shader's sampler table
([chameleon-gfx.md](chameleon-gfx.md#shader-type-0x53)), and the sampler's name is the role.
`tools/chameleon_texture_roles.py roles <game dir> HAWAII/GLOBALRESOURCES.BNDL` lists every texture with its roles; `sheet` draws them.
The textures of the 2,842 distinct ones of the 169 `TRK_UNIT*` bundles that the road materials use come from here too: the
ColourMap of the roads that 111 units draw is texture 76 of this bundle.

Formats are BC1 (200), BC3 (60) and one RGBA16F; sizes 4 x 4 to 2048 x 8192 with full mip chains. The graphics block of a texture is
one zlib stream, so reading a texture costs the stream's size (a 1024 x 1024 BC1 asphalt is about 0.6 MB compressed), never the
bundle's.

### The materials of the ground

| Shader family | Samplers (in order) | What it draws |
|---|---|---|
| `DriveableSurface_*`, `RoadDepth` (road) | `ColourMap`, `SpecColour`, `Normal`, `DetailMap_Normal`, `RoadDepth`, `DetailMap_Diffuse`; with paint: `Line_Diffuse`, `Line_NormalPlusSpec` or `Decal_Diffuse`, `Decal_NormalPlusSpec` | a road: the colour map covers the road's width, a detail map repeats over it, a line or a decal is painted on top |
| `WorldPBR_FloatingDecal_*` | `Normal`, `Diffuse`, `Specular`, `Overlay`, `Reflection` | pictures laid on the road: arrows, `ONLY`, covers, the lane signs |
| `PlotPBR_DriveableGrass_*`, `TerrainPBR_*` | `Mask`, `Tiling1..3`, `Tiling1..3Normal`, `Noise` | ground and grass in three tiled layers blended by a mask |
| `WorldPBR_Diffuse_Normal_Specular_*` | `Normal`, `Diffuse`, `Specular`, `Overlay`, `Reflection` | props and walls |

### The pictures the mod uses (index in the bundle, low 32 bits of the id, size)

| Picture | Index | Key | Size | Is |
|---|---|---|---|---|
| asphalt | 30 | `0006175B` | 1024 x 1024 BC1 | `ColourMap` of 85 units: grey asphalt with chips and wear |
| aggregate normal | 205 | `001286AF` | 512 x 512 BC3 | `DetailMap_Normal` of the road |
| concrete slabs | 21 / 20 | `00042129` / `00042128` | 2048 x 4096 BC1 / 1024 x 2048 BC3 | `ColourMap` and `Normal` of the paved road |
| grass | 158 / 159 | `000A97AB` / `000A97AC` | 2048 x 2048 BC1 / 1024 x 1024 BC3 | the grass layer and its normal map |
| white road line | 81 | `0006F394` | 128 x 512 BC1 | `Line_Diffuse`: worn white paint on black, columns 10 to 113 |
| yellow road line | 80 | `0006F393` | 128 x 512 BC1 | the same in yellow |
| arrows, `ONLY` | 50, 53, 41 | `0006D253`, `0006D256`, `0006D239` | 256 x 512, 256 x 512, 512 x 512 | white paint on black (diffuse), with a normal and a specular map each |
| STOP sign | 68 | `0006D4E6` | 256 x 512 BC1 | the octagon at x 4 to 174, y 7 to 177, its rusty back, the pole |
| speed limit 25 | 71 | `0006D4EA` | 256 x 512 BC1 | the face at the top left, the pole at the right |
| speed limits 20 and 50 | 143 | `0008C366` | 256 x 256 BC1 | two faces, **turned a quarter turn**, the pole at the left |
| round cover "Fairhaven Drain" | 33 / 34 / 35 | `0006D22D`.. | 512 x 512 BC1 on black | diffuse, normal, specular |
| square cover "BWW" | 59 / 58 / 57 | `0006D26A`.. | 256 x 256 BC1 | diffuse, normal, specular |
| slotted grate | 47 / 49 / 48 | `0006D245`.. | 256 x 256 BC1 | diffuse, normal, specular |
| concrete wall | 168 | `000D4974` | 1024 x 512 BC1 | wall panel with a seam |

The sign faces are parts of atlases with the pole next to them, so the converter crops them. Normal maps are RGB (blue is up) in BC1 or
BC3, not a two-channel format **[verified by looking]**; the green-channel convention was not checked against the 2005 shader
**[unconfirmed]**.

## The 2005 side: how the road is textured **[verified]**

`TRACKS/STREAML2RA.BUN` holds 477 texture packs (3,644 texture records, 3,101 distinct hashes, 2,491 distinct names). A texture is
found by the hash of its name, and **the pack keeps the first 23 characters of the name** ([textures.md](textures.md#textureinfo-0x7c-bytes)),
so `TRNS_ALL_LAB_PAVEMENTA_NORMAL`, `..._SPEC` and the plain one all show as `TRNS_ALL_LAB_PAVEMENTA_` with three different hashes. A
replacement must therefore be keyed by **hash**, which the replacement API takes (`target_hash`).

- **The road is not one texture.** The 1,122 road solids (`TRN_*_ROAD_*_CHOP_*`) are made of **strips 128 x 1024** (some 256 x 1024)
  named `TRNS_ALL_L{AB,A,B,Z,W}_PAVEMENT{A,B}_{N,W,Y,P,I}` and `..._SIDEWALK{A,B}_0`: a strip holds one lane's width of asphalt with its
  paint at fixed columns. `W` is a white line (columns 2 to 5 of 128, a second at 117 to 126 on some), `Y` a yellow line (2 to 4), `N` none,
  `P` a **crosswalk** (bars about every 51 rows), `I` an intersection (256 wide), the wide `Y` and `W` ones diagonal hazard stripes.
- **The `SHD_` textures are not roads.** 4,072 solids and 1,411 textures (256 x 256 or 512 x 512, all DXT3) are shadow overlays: a flat
  dark purple whose alpha is the shadow. They are left alone.
- **Each look has a diffuse, a normal and a specular map** with three hashes. The order in a solid's `SolidTextures` list is not a
  reliable role (the list repeats a hash, and an unresolved hash stands for a missing map); the roles were read from the image: the
  normal map is blue (mean B above 170, R and G near 128), the specular map is a grey with no colour. DXT3 diffuse maps with alpha usage
  2 carry a **specular mask in alpha** (15 to 30 % on average); a replacement that has no alpha would read as 100 % specular, so the
  mod writes DXT5 with a constant alpha of 64 where the original had one.
- **The textures are dark.** The 2005 road averages 43 to 57 of 255 and the grass 46; the 2012 asphalt is 115 and the grass 66 (the 2012
  game lights its textures differently), so the mod multiplies by the ratio (the `gain` of each row).
- **What the 2005 game has no texture for:** road arrows and `ONLY` (nothing in the 2005 data is a ground arrow: `SGN_*ARROW*` are
  freeway signs and traffic signals), and a manhole cover **by that name**: the manholes of the world are the solids `XO_MANHOLE_1B_CK_*`
  (8 instances) textured with `OBJ_STORMGRATE` (a 64 x 128 slotted grate); the one round cover, `OBJ_GASFILLCOVER` (128 x 128), is on the
  solid `AGASS_GASFILLCOVER01`, of which no scenery instance was found.
- **Signs.** `SGN_STOP` (128 x 128, the whole octagon) is on 12 solids: the sawhorses (`*XO_SAWHORSE*`), `XS_DONOTENTERGATEB_1A_00` (instances
  at (3156, 1863), (1807, 346) and others), the sign groups (`XS_SIGNGROUPA_1B_LL_00` and its world-space copies) and the stop sign
  solids `BXS_STOPSIGNB_1B_00` and `CXS_STOPSIGNA_1B_00`, of which no instance was found. The speed limit faces are 128 x 128 squares (`SGN_SPEEDLIMIT35, 45, 50, 55, 60, 75`); the only speed limit
  solids that scenery places by name are world-space copies of `XS_SPEED60NP_1B_00` (3); the 55 is on `bAXS_SPEED55_1B_00`, which a sign about
  60 m from the first intersection of the city shows (seen in the world, its instance was not found by the scan), and the 50 is on
  `cAXS_SPEED50_1B_00`, of which none was seen.

## Measuring a 2005 strip

The columns of paint of a strip were found by a Python scratch script: decode the diffuse, take the median brightness of each row
and column, and call a column painted where more than 15 % of its pixels are 45 levels above the median. The numbers above are those
measurements; they are the only thing of the 2005 pictures the mod keeps (as positions of the paint, not as pixels).
