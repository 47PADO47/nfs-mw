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
- `0003A100 CompTPKBlock` (JDLZ-compressed TPK pieces, used by the minimap tiles)
- `30300200 DDSTexture`

## Linking textures to models

`SolidTextures` (`0x00134012`) in a solid lists texture **name hashes**. Shading groups pick textures by
index into that list (see [models.md](models.md)). The hashes are matched against `TexturePackInfoKeys`
in whichever packs are loaded.

## Pixel formats

D3D9-era block-compressed formats (DXT1/DXT3/DXT5) plus uncompressed ARGB, described per texture in
`TexturePackInfoComps`. **[unconfirmed in this project]** The exact record layout is implemented in
[NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) `Common/Textures/` (`TpkManager.cs` and the
`Version*Tpk.cs` readers, one per TPK revision across the Black Box games), which can dump them to DDS.
