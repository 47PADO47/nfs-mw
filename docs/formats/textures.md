# Textures (TPK texture packs)

Textures are stored in **TPK** ("texture pack") chunks. They show up everywhere: car `TEXTURES.BIN` /
`VINYLS.BIN` / `PREVINYL.BIN`, the first chunks of each streamed map section, NIS bundles, HUD files and
`GLOBAL/*.BIN`. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Chunk tree **[verified]**

```
B3300000 TexturePack
├─ B3310000 TexturePackInfo
│  ├─ 33310001 TexturePackInfoHeader     124 B; contains the pack's pipeline path
│  ├─ 33310002 TexturePackInfoKeys       u32 bStringHash per texture name
│  ├─ 33310003 TexturePackInfoEntries    (some packs)
│  ├─ 33310004 TexturePackInfoTextures   per-texture records with names, e.g. "FCOP01_GEAR", "HERO_BODY"
│  └─ 33310005 TexturePackInfoComps      per-texture compression info
└─ B3320000 TexturePackData
   ├─ 33320001 TexturePackDataHeader     24 B
   └─ 33320002 TexturePackDataArray      raw pixel data for all textures
```

Pack header paths show where the artists' files lived:

- `Global\Pipeline\CarTemplateTextures_BMWM3GTR.tpk`
- `Global\Art\Characters\FCop\FCop01.tpk`
- `Global\Art\Characters\Props\Cuffs01.tpk`

Related chunks:

- `B0300100 TextureAnimPack` (animated textures: header, entry, frames)
- `0003A100 CompTPKBlock`: the payload is a bare JDLZ blob that decompresses to a complete small
  `TexturePack` (17,152 B per minimap tile in `TRACKS/L2RA/MINI_MAP*.BIN`) **[verified]**; see
  [world.md](world.md#minimap)
- `0003BD00 TPKSettings` (1,480 B in `GlobalB.lzc`; the decomp's list calls this ID
  `SPEED_XENON_TEXTURE_PAGE`) **[unconfirmed purpose]**
- `30300200 DDSTexture`

## Linking textures to models

`SolidTextures` (`0x00134012`) in a solid lists texture **name hashes**. Shading groups pick textures by
index into that list (see [models.md](models.md)). The hashes are matched against `TexturePackInfoKeys`
in whichever packs are loaded.

## Two pack forms **[verified]**

A TPK stores its textures in one of two ways. Both appear in this install (491 plain packs, 180
compressed packs across `CARS/`, `GLOBAL/`, `FRONTEND/`, `NIS/` and the L2RA stream).

| Form | Info chunks | Where the pixels are |
|---|---|---|
| **Plain** | `…04` TexturePackInfoTextures (one 0x7C `TextureInfo` per texture) + `…05` TexturePackInfoComps (one 0x20 platform record per texture) | `TexturePackDataArray`, at `ImagePlacement` from the array's payload aligned to 0x80 |
| **Compressed** | `…03` TexturePackInfoEntries (one 24-byte streaming entry per texture) | One compressed blob per texture, anywhere in the file |

Car `TEXTURES.BIN` files and `CARS/TEXTURES.BIN` use the compressed form; `GlobalB.lzc`, `FrontB.lzc` and
the world stream use the plain form.

### Streaming entry (`0x33310003`, 24 bytes) **[verified]**

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | NameHash (= the key in `TexturePackInfoKeys`) |
| 0x04 | u32 | ChunkByteOffset: **file offset** of the texture's compressed blob |
| 0x08 | i32 | ChunkByteSize: blob size (its 16-byte wrapper header included) |
| 0x0C | i32 | UncompressedSize |
| 0x10 | u8, u8, u16 | UserFlags, Flags (1 in every entry seen), RefCount (0) |
| 0x14 | u32 | ChunkData (runtime pointer, 0 on disk) |

Field names from the decomp's `eStreamingEntry` **[decomp]**. All 12,865 entries in the install point at a
blob that starts with `HUFF` (10,738) or `JDLZ` (2,127). See [huff.md](huff.md) and
[bchunk.md §3](bchunk.md#3-jdlz-compression).

A blob inflates to exactly `UncompressedSize` bytes laid out as:

```
pixels (ImageSize bytes, all mips) | [palette] | TextureInfo (0x7C) | platform record (0x20)
```

The `NameHash` in the trailing `TextureInfo` equals the entry's hash. Checked on every texture of
`CARS/BMWM3GTR/TEXTURES.BIN` and `CARS/TEXTURES.BIN` (475 textures). Where the palette sits in a compressed
palettized texture is **[unconfirmed]** (no car texture is palettized).

### TextureInfo (0x7C bytes)

Field offsets are from the decomp's `TextureInfo` (`Ecstasy/Texture.hpp`) **[decomp]**; the ones marked ✔
are **[verified]** against the install (name, hash, size, format and mip count give correct images in
`nfsmw view-car`, and the hash matches `bStringHash(name)` for every untruncated name).

| Offset | Type | Field |
|---|---|---|
| 0x00 | 12 B | runtime pointers (zero on disk) |
| 0x0C | char[24] | DebugName, NUL-padded, **truncated** to 23 characters ✔ |
| 0x24 | u32 | NameHash = `bStringHash(full name)` ✔ |
| 0x28 | u32 | ClassNameHash |
| 0x2C | u32 | ImageParentHash |
| 0x30 | i32 | ImagePlacement (offset in the data array) ✔ |
| 0x34 | i32 | PalettePlacement |
| 0x38 | i32 | ImageSize (all mip levels) ✔ |
| 0x3C | i32 | PaletteSize |
| 0x40 | i32 | BaseImageSize |
| 0x44 | i16, i16 | Width, Height ✔ |
| 0x48 | i8, i8 | ShiftWidth, ShiftHeight (log2 of the size) ✔ |
| 0x4A | u8 | ImageCompressionType (`TEXCOMP_*`: 0x22 DXT1, 0x24 DXT3, 0x26 DXT5, 0x20 32-bit, 0x08 / 0x80 / 0x81 8-bit palettized) ✔ |
| 0x4B | u8 | PaletteCompressionType |
| 0x4C | i16 | NumPaletteEntries |
| 0x4E | i8 | NumMipMapLevels ✔ |
| 0x4F–0x57 | i8 ×9 | TilableUV, BiasLevel, RenderingOrder, ScrollType, UsedFlag, ApplyAlphaSorting, **AlphaUsageType** (0 none, 1 punch-through, 2 modulated), **AlphaBlendType**, Flags |
| 0x58 | i16 ×7 | ScrollTimeStep, ScrollSpeedS/T, OffsetS/T, ScaleS/T |
| 0x68 | 20 B | runtime pointers and reference count |

### Platform record (0x20 bytes, PC) **[verified]**

| Offset | Value |
|---|---|
| 0x00–0x13 | `0, 0, 1, 5, 6` (u32 ×5; the last is 2 in some packs; meaning unknown) |
| 0x14 | u32 **D3DFORMAT**: FourCC `DXT1` / `DXT3` / `DXT5`, or 21 (`A8R8G8B8`), 41 (`P8`) |
| 0x18–0x1F | zero |

Formats counted in the plain packs (`ImageCompressionType`, D3DFORMAT, count):

| Comp. type | D3DFORMAT | Textures |
|---|---|---|
| 0x24 | DXT3 | 3,070 |
| 0x22 | DXT1 | 1,710 |
| 0x08 | P8 | 154 |
| 0x81 | P8 | 37 |
| 0x26 | DXT5 | 35 |
| 0x20 | A8R8G8B8 | 17 |
| 0x80 | P8 | 2 |

Mip levels follow the base level back to back; each DXT level is `ceil(w/4) × ceil(h/4) × (8 or 16)`
bytes. Reader: [`libs/blackbox-tpk`](../../libs/blackbox-tpk).

The decomp's `SpeedChunks.hpp` names the TPK children `SPEED_TEXTURE_PACK_HEADER` (`33310001`),
`…_INDEX_TABLE` (`…02`, the keys), `…_STREAM_TABLE` (`…03`), `…_INFO_TABLE` (`…04`), `…_PLAT_INFO_TABLE`
(`…05`) and `SPEED_TEXTURE_VRAM_DATA_HEADER` / `…_TABLE`
(`33320001` / `…02`) **[decomp]**.

## Alpha

How a texture's alpha is drawn, as implemented in `blackbox-scene::blend_mode`. Field names are from the
decomp's `TextureInfo`; the interpretation is **[verified]** on the PC world and cars by rendering it.

| `AlphaBlendType` (0x56) | Meaning (`TEXBLEND_*`) | Drawn as |
|---|---|---|
| 0 | SRCCOPY | opaque; cut out at alpha 0.5 if `AlphaUsageType` (0x55) is 1 (punch-through) |
| 1 | BLEND | alpha-blended after opaque geometry |
| 2 | ADDITIVE | additive (lights, glows) |
| 3, 4 | SUBTRACTIVE, OVERBRIGHT | approximated as blended **[unconfirmed]** |

Notes:

- **"Modulated" alpha (`AlphaUsageType` 2) does not mean transparent.** On copy-mode textures it is a mask
  for another effect. Road and brick textures average 15–30% alpha with a maximum around 60%, which matches
  a specular or reflection mask. They must draw opaque.
- **`SHD_` textures are shadow overlays.** Their colour is a flat dark purple (32, 4, 32) and their alpha is
  the shadow strength. They are blend-mode textures on separate `SHD_…` solids laid over roads and terrain;
  drawing them opaque turns every road black.
- **`ApplyAlphaSorting` (0x54) marks blended textures the engine depth-sorts** (some glass, leaf cards).
  It is not set on the shadow overlays.

In the world stream, 774 road textures (`SHD_CP_BE_ROAD_A_02A`, …) use usage 2 / blend 1; 1,388
architecture textures use usage 0 / blend 0; and the tree foliage (`ORG_*`) uses usage 1 / blend 0 with
DXT3 or DXT5.

## Animated textures

`B0300100 TextureAnimPack` chunks (top level, next to texture packs in streamed sections and global
files) make one texture cycle through others **[verified]**:

| Child | Record | Fields |
|---|---|---|
| `30300101` | 16 B header | u32 animation count, then zero |
| `30300102` | 0x34 B `TextureAnim` per animation | char[24] name, u32 name hash (the first frame's texture), i32 frame count, i32 frames per second, i32 time base, then runtime pointers and state |
| `30300103` | 16 B per frame | u32 texture name hash, then runtime pointers; all animations' frames back to back, in animation order |

Field names are from the decomp's `TextureAnim` / `TextureAnimEntry` **[decomp]**. The stream has 6
animations (29 frames) and the global packs have 2. For example, `ANM_WATERA_` has 14 frames
(`ANM_WATERA_` then `ANM_WATERA_001`…`013`) at 17 fps; others include `ANM_BILLBOARDLITES_A_LL` and
`SGN_SIGNAL_CROSS_`.

Models reference the first frame's texture. At draw time the engine substitutes frame
`(time × fps) mod count`. The Rust viewer does the same with `Renderer::redirect_texture`
(`blackbox-tpk::read_texture_anims`).

