# Tyre marks in NFS: Most Wanted (2012)

How the 2012 game draws the marks its tyres leave, as far as it was read, and how the mod `nfsmw-2012-content` turns it into the
**NFS Most Wanted 2012** value of the skid marks variant ([effect-variants-marks-rain.md](../modding/effect-variants-marks-rain.md)).
Evidence tags as in [../README.md](../README.md#evidence-tags): **[data]** read from the install's files, **[exe]** from
`NFS13.exe`, **[infer]** an inference, **[chosen]** a number the mod picked.

## Where the marks are

- The marks are **not per car.** The `SkidMarkParams` group of a car bundle's `TyreVfxBehaviour` is an empty structure (a head only)
  **[data]**; what a car does decide (the slips at which marks and smoke come on) is the `LatParams`/`LongParams` of the same object
  ([chameleon-drift-effects.md](chameleon-drift-effects.md#smoke-and-skid-marks-tyrevfxbehaviour-verified-layout-and-numbers-roles-unconfirmed)),
  which the game does not use (the host's own slip and skid intensities decide, and for bikes the slide of `bike_slide.rs`).
- The pictures are **per road surface.** `HAWAII/SURFACELIST.BNDL` has 65 `Genesys.Gen.SurfaceType` objects (the road surfaces) and 38
  `Genesys.Gen.SurfaceVfx` objects **[data]**. A `SurfaceType` names two `SurfaceVfx` objects through the structures at its +0xE0 and
  +0x1B0 (their slot offsets in the import table are `0x8000_0000 | 0xE0` and `| 0x1B0`; each structure of the type
  `Genesys.Gen.SurfaceType.Surface_weather_conditions` is a group of weather slots, whose own contents are not decoded) **[data]**.
  Which slot is the dry road and which the wet is **[infer]**: on the 27 paved surfaces the two slots name two different sets, on the 13
  loose ones (and 5 and 3 others) the same set twice, and the second paved set differs from the first only in its run normal map (a
  streaked water pattern) and its `Thermal` constant (0.6, against 0) .
- A `SurfaceVfx` object (type id `0x01000000_00002A51`, 464 to 960 bytes of data, 7 to 38 references) keeps the marks in a structure of the type
  `Genesys.Gen.SurfaceVfx.TyreMarksMaterials` at +0x0C: five 8-byte resource handles, at +0x0C, +0x14, +0x1C, +0x24 and +0x2C of it (so
  at +0x18, +0x20, +0x28, +0x30 and +0x38 of the object) **[data]**. Each names a **Material** in `GLOBALEFFECTS.BNDL`. The type's field
  names are hashes; what each of the five slots is for is **unknown**: slots 0, 2 and 3 name one material and slots 1 and 4 each
  another (slot 1: a larger tread print; slot 4: another print and a different alpha), so the mod reads slot 0, the most common.
- 14 of the `SurfaceType`s name `SurfaceVfx` objects with **no** mark materials (no marks: water and the like) **[data]**.

## The three sets

Counting the `SurfaceType`s by the first mark material of the object each slot names (65 surfaces) **[data]**:

| Set (slot 0 material) | Colour of its marks | Named by | Used for (**[infer]**) |
|---|---|---|---|
| `0x01000000_00117F16` | black | 27 + 5 surfaces in the first slot, 5 in the second | the dry paved road |
| `0x01000000_00117F1B` | black | 27 + 3 in the second slot | the wet paved road |
| `0x01000000_00117F02` | brown (117, 57-79, 18-65) | 13 surfaces, both slots | the loose road (dirt, sand, grass, gravel) |

(The mod finds the sets by the counts and the colour, not by these ids.)

## A mark material

A mark material (Material `0x02`, 464 bytes, 7 textures) runs the shader `0x01000053_00049296` of `SHADERS.BNDL`, technique
`Vfx_TyreMarksNew` **[data]**. Its samplers (read with `chameleon_gfx::Shader::sampler_slot`) are, in the order the material lists its
textures **[data]**: `SlipDiffuseSampler`, `SpinDiffuseSampler`, `SlipNormalSampler`, `ThermalSampler`, `RunDiffuseSampler`,
`RunNormalSampler`, `SpinNormalSampler`. So a mark has **three looks**, for a tyre that slides sideways (*slip*), one that spins or
locks (*spin*) and one that rolls (*run*), each a diffuse and a normal map. The shader's parameters are, in name order, `Animation`,
`WidthScale`, `Specular` and `Thermal`, four 16-byte slots **[data]**:

| Set | Animation | WidthScale | Specular | Thermal |
|---|---|---|---|---|
| dry paved (slot 0 material) | (0, 0, 0, 0) | (1.0, 0.2, 0, 0) | (0.65, 0, 0, 0) | (0, 0, 0, 0) |
| wet paved | (0, 0.25, 0, 0) | (1.0, 0.2, 0, 0) | (0.65, 0, 0, 0) | (0.6, 0, 0, 0) |
| loose | (0, 0, 0, 0) | (1.0, 0.2, 0, 0) | (0.65, 0, 0, 0) | (0.04, 0, 0, 0) |

What the numbers do is not decoded; the mod uses none of them (the width of the mark is the host's tread width times 1, the opacity 1).

The pictures (BC3 `fmt 77` or BC1 `71`, one mip or seven, **[data]**):

| Set | Slip diffuse | Spin diffuse | Run diffuse |
|---|---|---|---|
| dry paved | `0x7A92A`, 32 x 32, black, alpha 15 to 255 (soft horizontal streaks) | `0x117F0E`, 32 x 32, black, alpha 0 to 178 (a chevron tread print) | `0x7A932`, 8 x 8, empty |
| wet paved | the same slip and spin | | `0x7A932`, empty |
| loose | `0xF58CA`, brown (117, 79, 65), alpha 0 to 255 (smeared dirt) | `0x117F26`, orange-brown (117, 57, 18), alpha 0 to 178 (tread print) | `0x49291`, 8 x 8 |

The columns run along the track and the rows across it (the streaks of the slip picture are horizontal, the chevrons of the tread
print point along them) **[infer]**. A bigger family of normal maps (`0x7A92E`, `0x117F13`, `0x117F11`, `0xF58C9` ...) lights them; the
host's marks are unlit, so the mod does not carry them.

## What the mod writes

`mods/nfsmw-2012-content/src/marks_assets.rs`: opens `SURFACELIST.BNDL` (75 KB) and `GLOBALEFFECTS.BNDL` (4.8 MB) **by ranges**
(`trees::ranges`, the table of each bundle and the blocks of the resources it asks for), tallies the surfaces, decodes the slip and
spin diffuse pictures of the candidate materials, picks the most named non-brown material of each slot (the dry and the wet set, the
wet one only when it differs) and the most named brown one (the loose set), and writes up to six PNGs (`PAVED_SLIP.PNG` ...) and a
`profile.toml` under `files/SKIDMARKS/MW2012/`. The run pictures, the normal maps and the parameters are not carried. `slip_tile` 4 m,
`spin_tile` 2 m are **[chosen]** (a tyre's circumference is about 2 m).

## Cost

Measured with `cargo test -p nfsmw-2012-content --lib -- --ignored real_install --nocapture` (the two real-install tests): see the
numbers in the pull request description; the test asserts the mod reads under a quarter of the two bundles' bytes. Disk: six
tiny PNGs (the pictures are 32 x 32 to 64 x 64) and one profile.

## Not decoded

The meaning of the five slots of `TyreMarksMaterials`; which weather the two slots of a `SurfaceType` are; the `Animation`,
`WidthScale`, `Specular` and `Thermal` numbers; how the game chooses between the slip, spin and run looks (the `Genesys` inputs it
binds); the width and fade of a mark (the shader has a width scale, no length or time: a mark's life is the engine's); the normal maps.
