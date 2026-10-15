# Car underside material (BOTTOM)

Status: **data traced; in-game comparison still needed**. Earlier drafts read a "cullable part" table here.
That was a data-overlap false positive and is withdrawn (see [docs/reversing.md](../reversing.md)).
Evidence tags: **[decomp]** from the binary, **[verified]** from game data or the port's code, **[unconfirmed]**.

## Chain for the BMW M3 GTR body shell

1. The body shell solid is `BMWM3GTR_KIT00_BODY_A` in `CARS/BMWM3GTR/GEOMETRY.BIN`. Its material-name list
   includes `BOTTOM`. [verified: chunk dump]
2. `bstring_hash("BOTTOM")` = `0x52BF4C34`. It is entry 2 of the solid's light-material list. [verified]
3. Shading group 5 of that solid has light material 2. It draws 78 triangles (140 vertices), using texture
   slot 2. [verified: the port's reader, scratch dump]
4. Texture slot 2 is `CARBOTTOM`, a 256x128 P8 texture in `GLOBAL/GLOBALB.LZC`. Its decoded pixels vary:
   256 distinct colours, luma standard deviation about 32 of 255. So the texture is not flat. [verified]
   The palette is read as 4-byte B G R A entries, which the decoder marks as unconfirmed. The palette block is
   1024 bytes, which matches 256 entries of 4 bytes, so the entry size is consistent with the data. The channel
   order is still unconfirmed. [verified: palette size; channel order unconfirmed]
5. The light material `BOTTOM` is in `GLOBALB.LZC`: diffuse scale 0.833 to 0.840, specular power 3.888,
   environment-map power 2.0. [verified: the port's light-material reader]
6. The renderer shades a group by its light material: `CarMaterials::shading` in
   `crates/nfsmw/src/scenes/car/materials.rs`. Glossy shading uses the material if it is loaded, and falls
   back to the default material otherwise. [verified: code read]

## What this says about the flat underside

- The material's diffuse range is almost zero (0.833 to 0.840). Under the port's diffuse scale, the
  underside's brightness barely changes with light angle. That is what "flat" looks like in shading, and it
  comes from the data, not from a missing part. [verified data; the shading effect is [unconfirmed]]
- The texture has detail, so the lack of visible texture is more likely a shading or sampling question than
  missing data. [unconfirmed]
- The environment-map power of 2.0 is high for a matte surface. Check it against the original's look.

## What the original names

- The original's shader-parameter table includes the names `DIFFUSEMIN`, `DIFFUSERANGE`, `ENVMAPPOWER`,
  `ENVMAPMIN` and `ENVMAPANGE`, next to `DIFFUSECOLOUR`, `SPECULARPOWER` and `SPECULARRANGE`. The table is built
  by one constructor that registers each name and stores a handle for it. [decomp: strings and constructor]
- These are shader constants, not light-material file fields. The light material's values (`0.833`–`0.840`,
  `2.0`, and so on) are most likely written into these constants before drawing. The step that does the
  writing is not yet found. [unconfirmed]

## Resolved: how the values reach the shader

- The port maps a light material to shader constants in `crates/nfsmw/src/scenes/car/shading.rs`
  (`glossy_material`, spec `docs/specs/car-assembly.md` §8). The constants are `diffuse_min` and `diffuse_range`,
  which match the binary's `DIFFUSEMIN` and `DIFFUSERANGE` names. [verified: code read, names match]
- Formula: `diffuse_min = colour_min × min_scale`, `diffuse_range = colour_max × max_scale − diffuse_min`.
- For `BOTTOM`, the colours are `[1, 1, 1]` for both min and max. So `diffuse_min = 0.833` and
  `diffuse_range = 1 × 0.840 − 0.833 = 0.007`. The shading ramp is only 0.007 wide, so the underside is nearly
  uniform. [verified: data read and formula]
- This is the port's formula. Whether the original uses the same formula is still [unconfirmed]; the binary
  has the constant names but the writer of the values is not yet located.

## Still open

- Compare the underside with the original game at the same camera angle. Only that can show whether the
  near-flat diffuse range is intended. [unconfirmed]
- Confirm how the original uses the diffuse range and environment power for this material, and whether the
  port applies them the same way. Needs decompiled shading code and a spec.
- The P8 palette layout is an assumption in the decoder. If it is wrong, the texture colours are wrong, not
  its detail. Check against a known P8 texture.
