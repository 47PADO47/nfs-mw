# 2012 tyre marks, water and wet road provenance

Restricted sources read for [formats/chameleon-tyre-marks.md](../formats/chameleon-tyre-marks.md) and
[formats/chameleon-rain-water.md](../formats/chameleon-rain-water.md), which the mod `nfsmw-2012-content` (`marks_assets.rs`,
`rain_assets.rs`) and the variants of [modding/effect-variants-marks-rain.md](../modding/effect-variants-marks-rain.md) follow.

- **The 2012 install's data files** (measured, nothing copied): `HAWAII/SURFACELIST.BNDL` (the `Genesys.Gen.SurfaceType`,
  `SurfaceVfx`, `SurfaceVfx.TyreMarksMaterials`, `SurfaceReflection` and `Surface_weather_conditions` types and objects: their names, the
  offsets of their reference slots and the objects' import tables, read with a scratch Python reader of the `bnd2` tables and the
  `chameleon-bundle` crate), `GLOBALEFFECTS.BNDL` (the materials of the marks, the 214 effects, the 129 textures: sizes, formats, pictures
  looked at), `SHADERS.BNDL` (the `Vfx_TyreMarksNew` shader's sampler and parameter names; the shaders that name `WetScaleParams` and
  `puddleMapSampler`) and `HAWAII/ENVIRONMENT.BNDL` (its 27 textures looked at). The scratch readers and pictures stay outside the
  repository.
- **`NFS13.exe`** (SHA-256 `9571280bb8868cd254e95442181d7bf1d6eafb2156a95d0c38337c7470762c0f`, 13,672,448 bytes; Ghidra, program
  `NFS13.exe`, read-only): only strings and one cross reference. The strings named in the format page
  (`IsOnWetSurface` at `0x00FACA78`, `IsWetSurface` at `0x00FB1CB8`, `IsInWater` at `0x00FC1EF0`, `WithinWaterSpray` at `0x00FC2D2C`,
  `InDustStorm` at `0x00FC23EC`, `puddleMapSampler` at `0x01001488`, `GBufferWet,GBuffer` at `0x010001E0`, `GBufferBlendWet,GBufferBlend` at
  `0x010002A8`, `WetScaleParams` at `0x01000928`), and the single reference to `WetScaleParams`: the push at `0x008C4270` in the
  function at `0x008C34B0`, one of a run of calls (`0x00A5D490` on the table object at `0x0118DE00`) that register the engine's global
  shader constants by number and size (`WetScaleParams` is number `0x3E`, 16 bytes). Nothing was decompiled; what writes the constant was
  not followed.

No decompiled code, table or asset is in the repository: the mod reads the player's own install at run time, by ranges, and writes
small PNGs and profiles into the mod folder on the player's machine. The numbers the variants carry (tile lengths, rates, sizes, lives,
opacities, the lens bend, the wet road factors) are chosen and marked **[chosen]** in the format pages.
