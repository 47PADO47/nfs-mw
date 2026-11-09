# Rain and water in NFS: Most Wanted (2012)

What the 2012 install holds about rain, drops on the lens, splashes and a wet road, and what it does not. Evidence tags as in
[../README.md](../README.md#evidence-tags). The effect variants that use this: [effect-variants-marks-rain.md](../modding/effect-variants-marks-rain.md).

## The finding: there is no rain

The 2012 game has **no rain weather** that this reading could find **[data] [exe]**. Looked at, all of it:

- The 214 `BearEffect` resources of `GLOBALEFFECTS.BNDL` (129 textures) were listed with their emitters, operations and pictures
  (`libs/chameleon-effects`) and all 129 pictures were looked at. None is a rain streak, a drop on a lens or a puddle splash. 84 are named
  by the surfaces (`SurfaceVfx` and `VfxWheelEffects`: sparks, dust, smoke, mist of the tyres, debris); the other 130 are loaded by
  the executable or other bundles (nitrous, weapons, explosions, pursuit). Nothing outside `SURFACELIST.BNDL` and one
  `WCVfxBehaviour` of `GLOBALRESOURCES.BNDL` references an effect by id (`GLOBALCONFIG`, `POSTFX`, `ENVIRONMENT`, `AMBIENCES`,
  `GAMELOGIC/*`, `PHYSICS/*` do not).
- `HAWAII/ENVIRONMENT.BNDL` (27 textures) holds sky domes at several times of day, a moon and cloud noise: no rain layer.
- The strings of `NFS13.exe` (Ghidra, program `NFS13.exe`) with the words rain, splash, droplet, wet, weather, puddle or water are:
  `IsOnWetSurface` ("Is the race car on a wet surface at all?"), `IsWetSurface` ("Is this vehicle driving on a wet surface?"),
  `IsInWater` ("Is The Player in Water"), `WithinWaterSpray` ("Float representation (0->1) of whether the player is in water
  spray"), `InDustStorm`, the shader technique names `GBufferWet`, `GBufferWetNight`, `GBufferBlendWet`, `WetScaleParams` and the
  sampler `puddleMapSampler`. They are **inputs of the sound graphs and the vehicle effects and shader names, not a weather system**.
  The 2012 game's wet look is for sprinklers, hydrants, puddles of the world and a few wet scripted places.

The 2012 water that does exist:

| Thing | Where **[data]** | What it is |
|---|---|---|
| Water spray | the effects `0x80AFC` and `0x1D6FEC` (texture `0x69D24`, 256 x 256: a white spray column), driven by `WithinWaterSpray` **[exe]** | hydrants and sprinklers |
| A splash ring | the effect `0x24F39`, texture `0x2512D` (64 x 64, a soft white ring) | an impact splash |
| Tyre mist | the wheel effects `0x24F54`, `0x24F55`, `0x7C950` (texture `0xF577B`, 512 x 256: eight puffs of mist) | the tyres on the wet road |
| Water on a lens | `RenderSpriteUVDistortion` effects with the distortion picture `0x140375` and the noise `0x18EAF0` (the wheel effects `0xF565F`, `0xF5678`, `0xF57D7`, `0x117E0E`, `0x117E19`, `0x1403AA`) | a refracting spray at the tyres |
| Splatters | 13 `RenderSplatter` effects of the surfaces' `SurfaceVfx` (+0x58, +0x100): pictures `0x162070` ... `0x1621FA`, `0x162202`, `0xF579C` | **not water**: grass blades, dust and debris flung at the screen |

## What is in `SURFACELIST.BNDL` about weather

- `Genesys.Gen.SurfaceType.Surface_weather_conditions` (type `0x01000000_00003349`, 432 bytes): five `Contact_Type` structures of
  three handles each and two more handles; a `SurfaceType` carries two of these groups (its +0xE0 and +0x1B0 slots, see
  [chameleon-tyre-marks.md](chameleon-tyre-marks.md)). The handles are filled by the loader and name the effects and mark sets of
  each weather condition; their meaning beyond the marks is **not decoded**.
- `Genesys.Gen.SurfaceReflection` (type `0x01000000_000B377A`) is **not a wet road**. Five objects hold five floats each
  (`0.7, 6000, 0.7, 1, 0.2` and four more rows) **[data]** and are referenced only by `Genesys.Gen.SurfaceSounds` at +0x2C **[data]**
  (23 + 7 + 3 times), so it is the **sound** reflection of a surface (a reverb or filter, the 6000 and 10000 look like cut-off
  frequencies in Hz) **[infer]**.
- 31 of the 373 shaders of `SHADERS.BNDL` list `WetScaleParams` (the world's driveable surface, decal and plot materials, `GBufferWet`
  techniques); 21 of them also `puddleMapSampler`, and `PlotPBR_AO_StandingWater_Opaque_Lightmap_Singlesided` is a standing-water
  material **[data]**. The engine's constant `WetScaleParams` is a four-float global registered as constant number `0x3E` (a table at
  `0x0118DE00`) by the function at `0x008C34B0` (the push of its name is at `0x008C4270`) **[exe]**; **what the engine writes into it
  and when is not decoded**, and which texture `puddleMapSampler` is bound to is not known. So there is **no decoded wet road
  look** to carry.

## What the mod does with this

`mods/nfsmw-2012-content/src/rain_assets.rs`, honest about the above:

- **Rain splash**: the ring (`0x2512D`) and the spray column (`0x69D24`) as the two layers of the splash profile. The picture ids were
  found by looking at the pictures and the effects that use them; every number of the profile (rates, sizes, lives, opacities) is
  **[chosen]**.
- **Rain drops**: the round dots (`0x7A956`, 256 x 256, the bokeh sheet used by the lamp glow effects `0x7A95F`, `0xAF168`, `0xEF943`) are
  read as connected blobs of bright opaque pixels, the largest is the outline of one drop, and the drop's displacement picture (64 x 64:
  green and blue 0.5 at the centre, bending to the opposite side by up to 0.42 at the rim with the square of the distance, alpha the dot's
  own soft edge) is *computed* from it. The bend is **[chosen]**; the 2012 game has no such picture.
- **Wet road**: four chosen factors (`film` 1.25, `pooling` 1.3, `soak` 1.5, `dry` 0.7), because the decoded data has nothing to read.

The three pictures are read by ranges from `GLOBALEFFECTS.BNDL` (the bundle's table and three textures; see
`rain_assets::find`). Nothing of the pictures is in the repository.

## Not decoded

What `WetScaleParams` holds and what writes it; the puddle map; the weather slots of a surface; whether the game has any rain at all
in a part this reading did not reach (the speech and movie data, the executable's weather tables). If a later reading finds rain, the
variant kinds here take its pictures through the same profiles.
