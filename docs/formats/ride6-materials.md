# RIDE 6 vehicle materials

How the bikes of RIDE 6 (Milestone, Unreal Engine 5.6.1, build `5.6.1-1697004`) describe their surfaces, and what the
[bike importer](../modding/ride6-bikes.md) takes from them. Read from the user's own install with a package reader
(CUE4Parse, the package `.usmap` mapping, no AES key); nothing here is copied from the game and no game data is in the
repository. Evidence tags as in the [README](../README.md#evidence-tags): **[data]** is a value read from an asset,
**[stats]** is inferred from texture statistics, **[inferred]** is read from parameter names only. The shader graph of the
master materials is cooked away in the shipped packages, so the lighting arithmetic itself is **not** known; only its inputs
are.

## The material chain **[data]**

Every surface of a bike mesh is a `MaterialInstanceConstant` (`mat_*`). It names a parent instance and overrides some of
its parameters; the parents are shared by all bikes and end in a master material:

```
mat_mechanics369_001  ->  BASE_Mechanics3  ->  MASTER_Vehicle3
mat_livery_0_369_001  ->  BASE_Livery_Cinematic3  ->  BASE_Livery3  ->  MASTER_Vehicle3
mat_fronttyre001      ->  BASE_Tyres3_Front  ->  BASE_Tyres3  ->  MASTER_Vehicle3_Tyres
```

A parameter's effective value is the first one found walking from the instance up to the master. An instance holds
`ScalarParameterValues`, `VectorParameterValues`, `TextureParameterValues`, `StaticParametersRuntime` (static switches),
`BasePropertyOverrides` and `TextureStreamingData` (which UV channel each texture is sampled with). The exporter script
resolves all of it into `parts.json` (`materials`).

`BasePropertyOverrides` of every vehicle material of the KTM 450 SMR: `BlendMode = BLEND_Masked`, `OpacityMaskClipValue =
0.3333`. `ShadingModel` is `MSM_ClearCoat` for painted and metal parts (livery, mechanics, cockpit, rim, muffler, exhaust
manifold, gear) and unset (default lit) for the chain, the brake disc, the glass decal and the tyres. The master's own
properties: `bTangentSpaceNormal = false`, used with skeletal meshes, Nanite and static lighting.

## Textures **[data]**

| Parameter | Content | Stored as |
|---|---|---|
| `BaseColor - Texture` | base colour, sRGB | DXT1, or DXT5 when it also carries opacity |
| `BaseColorOpacity - Texture` | base colour in RGB, opacity mask in alpha | DXT5 (`TC_Default`, sRGB) |
| `Normal - Texture` | tangent-space normal, two channels | BC5 (`TC_Normalmap`, linear) |
| `PBR - Texture` | packed material values, see below | DXT5 or DXT1 (`TC_Masks`, linear) |
| `DynamicDirt - Mask - Texture - Texture` | where dirt collects | per-part `*_dirt_mask` |
| `ColorCustomization - Mask - Texture` | which texels the player's colour palette recolours | per-livery `*_custom_mask` |
| `ClearCoat - LegacyMode - Ride - Texture` | clear-coat strength mask | per-livery `*_clearcoat_mask` |

The tyre master (`MASTER_Vehicle3_Tyres`) names its textures `BaseColor`, `BaseColorBlur`, `CleanNormal`, `CleanNormalBlur`,
`DirtNormal`, `DirtNormalBlur`, `Pbr`, `PbrBlur`: a sharp and a motion-blurred set for the spinning wheel, and a dirty
normal map. The engine's flat defaults (`T_Default_BC`, `T_Default_N`, `T_White_*`, `T_Black_*`) mean "no picture".

Which of `BaseColor - Texture` and `BaseColorOpacity - Texture` is read is the static switch `BaseColor - Source` against
`BaseColorOpacity - Source` (and their `- Texture` variants, which say the source is a picture and not a constant). A
material whose `BaseColorOpacity - Source` is on (brake disc, chain, gear, muffler, glass decal) takes its cut-out from
the picture's alpha, clipped at `OpacityMaskClipValue`; the others have alpha 255 everywhere.

**Texture sizes** on the KTM: livery, mechanics, cockpit, rim, tyres and exhaust 2048 px (the livery and cockpit colour
sources are 4096 and are stored at 2048 here: the exporter decodes the largest mip not above its limit); the disc and
sprocket 1024; the chain 256.

**UV channels.** `TextureStreamingData` lists `UVChannelIndex 0` for every picture of every material of the bike. The
exported meshes carry eight UV sets, only set 0 is used for these textures.

### The packed picture **[stats]**

The static switch pair `LegacyTexturesFormats - PBR - S_R_M` (off) and `LegacyTexturesFormats - PBR - S_R_M_O` (on)
names the layout: **R specular, G roughness, B metallic, A occlusion**. The statistics of the pictures agree:

| Channel | Observed | Reading |
|---|---|---|
| R | 123 to 134 on every picture (median 129) | specular 0.5, the engine's default dielectric reflectance (4 % at normal incidence) |
| G | 13 to 190; livery paint 50, plastics 75 to 154, rim about 114 | roughness, 0.2 for glossy paint |
| B | 0 or 255 almost everywhere | metallic mask: bare metal (bolts, discs, rims, chain) 255, paint and rubber 0 |
| A | 0 to 255, 50 % of the mechanics below 9, 255 on rim, brake, tyres, gear, chain | baked ambient occlusion (dark between engine parts); pictures stored as DXT1 have no alpha and mean "unoccluded" |

The occlusion channel is dark over a large part of the engine atlas. An Unreal renderer applies it to indirect light
only; the 2005 car shader has no ambient term, which is why the importer applies a softened version to the colour
([Baking](#what-the-importer-keeps-and-loses)).

## Static switches and parameter groups **[data, meaning inferred from names]**

The instances switch features of the master on and off. The groups seen on the bike materials:

| Group | Parameters (examples) | What it does |
|---|---|---|
| Sources | `BaseColor - Source`, `Normal - Source`, `PBR - Source` and `- Texture` | picture or constant for each input |
| Legacy | `LegacyMode - Enabled`, `LegacyMode - ClearCoat - Ride`, `LegacyTexturesFormats - Enabled` | the format of the earlier game's pictures: packed `S_R_M_O`, a clear-coat mask picture |
| Clear coat | `ClearCoat - Enabled`, `- Layer - CarbonFibers`, `- Layer - Flakes`, `ClearCoat - Flat - ClearCoatIntensity` (1.0), `- Flat - ClearCoatRoughness` (1.0) | a second specular layer over the base (`MSM_ClearCoat`); carbon-fibre weave and metallic flakes are optional layers with their own tiling (`- UV - Tiling` 50 to 200), normal intensity and roughness range |
| Colour customization | `ColorCustomization - Enabled`, `- Palette`, `- Mask`, `- BaseColor`, `CustomColor00_Map` | the player's palette colours replace the texels the custom mask selects; the default is the picture's own colour |
| Texture customization | `TextureCustomization - Pass1`, `- BaseColor/Metallic/Roughness/Specular - Enabled` | a second decal-like picture layer, off on the stock livery (`- Enabled = 0`) |
| Dynamic dirt | `DynamicDirt - Enabled`, `- Mask`, `- Pass1 - BaseColor` (0.22, 0.161, 0.09) | mud and dust collecting where the mask says, driven by the game |
| Weather | `Weather - Enabled`, `Weather - RainDrops`, `RainDrops - Normal - Intensity`, `- UV - Tiling` | drops on the surface in rain |
| Pearlescence | `Pearlescence - Mask`, `- Intensity - Multiplier` (0 on the livery) | a colour shift by view angle |
| Tank | `TankColorOverride`, `LiquidColor`, `Wave*`, `MotionAmplitude`, `RotationAmplitude` (cockpit material only) | a fuel/oil level window with a sloshing liquid |

Run-time-only inputs (dirt, rain, the palette, the liquid) have no value in the asset beyond their defaults; the importer
uses the defaults, which are the clean stock look.

## The KTM 450 SMR 2022 (chassis 369) **[data]**

| Surface (material instance) | Where | Base colour | Packed | Normal | Alpha |
|---|---|---|---|---|---|
| `mat_mechanics369_001` | engine, frame, fork, wheels' hubs, calipers | `mechanics369_001_d` | `_l` | `mechanics369_n` | opaque |
| `mat_livery_0_369_001` (LOD 0) / `mat_livery_1_369_001` (LODs 1 to 3) | plastics, tank, seat cover, fenders | `livery369_001_d` | `livery369_001_l` | `livery369_n` | opaque |
| `mat_cockpit369_001` | handlebar, grips, bar pad, levers | `cockpit369_001_d` | `_l` | `cockpit369_n` | opaque |
| `mat_gear369` | rear sprocket | `gear369_d` | `_l` | `_n` | cut-out |
| `mat_chain_1_001` | chain | `chain1_001_d` | `_l` | `chain1_001n` | cut-out |
| `mat_brake369` | disc rotors, front sprocket | `brake369_d` | `_l` | `_n` | cut-out |
| `mat_glassdecal369` | scratch decal on the screen (none on a supermotard) | `glassdecal369_d` | scratch map | none | cut-out |
| `mat_rim369` | both rims | `rim369_d` | `_l` | `_n` | opaque |
| `mat_fronttyre001`, `mat_backtyre001` | tyres | `fronttyre001_d`, `backtyre001_d` | `_l` | `_n` | opaque |
| `mat_muffler369_001` / `mat_exmanifold369_001` | silencer / header pipes | `muffler369_001_d` / `exmanifold369_001_d` | `_l` | `_n` | muffler cut-out, header opaque |

The mechanics and livery meshes are skinned atlases: one picture each covers every part of the frame, which is why the
frame, engine, calipers, levers, sprockets and chain are all in the chassis mesh.

## Levels of detail **[data]**

The chassis has eight levels. Levels 0 to 3 keep the seven sections above (level 0: 146,000 triangles in 135,000
vertices; level 1: 78,000 vertices; level 2: 44,000; level 3: 26,000); levels 1 to 3 swap the livery section for slot 7,
`mat_livery_1_369_001`, which reads the same pictures with a cheaper clear coat. **Levels 4 to 7 use one material,
`mat_bike_lods`, for the whole bike** (14,600 to 2,000 vertices). It is not an asset: the engine builds its picture at run
time from the other materials (`assets/data/vehicles/Compositing/LiveryLODBaseColorCompositing`, `LiveryLODPbrCompositing`).
The importer reads levels 0 to 3 of the chassis only.

## What the importer keeps and loses

The 2005 car shader draws a group with one diffuse picture and one *light material* (ramps for diffuse, specular and a
cube-map reflection, chosen by name from the install's `GlobalB.lzc`); it has no normal map and no per-texel gloss
([car-material-shading.md](../specs/car-material-shading.md)). The mapping, which is a set of modelling choices (the
thresholds are named constants in `shading.rs` and `bake.rs`):

| Unreal | 2005 car files |
|---|---|
| base colour (sRGB) | diffuse picture, up to 2048 px for body parts, half for wheels, a quarter for discs |
| occlusion | diffuse darkened in linear light by `floor + (1 - floor) * occlusion`, `floor = 0.3` |
| opacity mask, clip 0.3333 | diffuse alpha cut to 0 or 255, DXT5, alpha usage punch-through, two-sided texture |
| translucent surface (`BLEND_TranslucentGreyTransmittance`: screen, lamp lenses) | blended glass: diffuse alpha is the opacity (below), DXT5, alpha usage modulated, alpha-sorted, one-sided, light material `WINDSHIELD` or `BRAKELIGHTGLASS` per triangle ([Translucent surfaces](#translucent-surfaces)) |
| metallic and roughness | light material per **triangle** (its corners and centre sampled): bare metal `CHROME` below roughness 0.2, `MAGSILVER` below 0.4, else `ALUMINUM`; other surfaces `SHINYPLASTIC` below 0.4, else `DULLPLASTIC`; tyres `RUBBER`; discs `BRAKEDISC` |
| normal map | lost, unless the tangent-space path is on (below) |
| clear-coat layers, flakes, carbon weave | lost (the light material's gloss stands in) |
| dirt, rain, palette recolour, tank liquid, pearlescence | not used (run-time inputs; defaults are clean) |

The light-material names exist in the PC install (`RUBBER`, `MAGSILVER`, `CHROME`, `ALUMINUM`, `BRAKEDISC`, `SHINYPLASTIC`,
`DULLPLASTIC`, among 156), checked by reading the names out of `GlobalB.lzc`. A metal texel keeps its base colour in the
diffuse picture, but the metal's tint comes from the light material, so coloured metals (the gold of an anodised part) come
out silver.

### Translucent surfaces

**The material [data].** The screen, the headlamp lens, the tail lens and the indicator lenses of a bike are one material,
`mat_glass<chassis>` (`mat_glass151_001` on the Ducati 888 SP5): `BlendMode = BLEND_TranslucentGreyTransmittance`,
`ShadingModel = MSM_ThinTranslucent`, static switches `Transmittance - Source - Texture` and `LegacyMode -
OverrideBaseColorWithTransmittanceColor` on, `Modulation - Transmittance - Max` = 0.95 (a vector, the same in all three
channels), `Refraction - IOR - Raster` 1.5 with a refraction normal map. Its pictures: `BaseColor - Texture` and
`Transmittance - Texture` are the **same** picture (`glass665_001_d`, 2048 px DXT5, sRGB), `PBR - Texture` is the shared
scratch map `vehicle_glass_scratches` (not roughness and metal), plus a rain mask and the refraction pictures. A second
material, `mat_glassdecal`, lays the scratch decal over the screen and is skipped. The lamp bulbs and reflectors behind the
lens are `mat_light<chassis>`, an ordinary masked material (below).

**The picture [stats].** The transmittance picture is white (255, 255, 255) over most of its area, which is the clear
screen and the clear headlamp lens, and flat colour in the islands of the tinted lenses: on the Yamaha YZF-R7 2022 a red
(173, 0, 5) for the tail lens, on the Ducati 888 SP5 a red (203, 41, 41) and an amber (255, 123, 10) for the tail lens and the
indicators. Sampling the picture at the vertices of the glass primitive puts the screen and the headlamp lens on white (the
tail lens on red; the Yamaha's indicators are not glass). White means transmittance 1, so the sign is **transmittance, not
opacity**: a white screen is clear, not a white pane. The picture's alpha is small everywhere (0 to 37 on the Yamaha, 6 to 27
on the Ducati) and a little higher over the tinted islands; it is not the transmittance of the colour (the Ducati's red
has linear mean 0.21, alpha 0.11), the material has no use for it that its parameters show, and the importer reads the
colour only.

**The stock cars [data].** The 2005 cars draw a window as its own shading group, effect 4, flags `0x4180`, with the
placeholder texture `WINDOW_FRONT` (every window placeholder swaps to it; `swaps.rs`) and the light material `WINDSHIELD`.
`WINDOW_FRONT` in the install's global pack is a 128 px DXT3 texture with alpha blend type 1 (`TEXBLEND_BLEND`), alpha usage 2
(modulated), `ApplyAlphaSorting` set and the two-sided flag clear; its colour is a dark grey (about 49) and its alpha is 255
nearly everywhere. The body's window frame is a second group on the same solid with `WINDOW_MASK` (8 px, opaque, black).
The headlamp covers are groups with the light material `HEADLIGHTGLASS` (its placeholder swaps to `WINDOW_FRONT` too), the
tail lamp covers `BRAKELIGHTGLASS` with the car's own `<CAR>_KIT00_BRAKELIGHT_GLASS_OFF` and `_ON` textures. The light
materials' diffuse alpha, read from `GlobalB.lzc` (`diffuse_min_alpha` / `diffuse_max_alpha`): `WINDSHIELD` 0.40 / 0.40,
`WINDOWMASK` and `CLEARPLASTIC` 0.40 / 0.40, `WINDSHIELD_TINT_L1_*` 0.55 to 0.65, `_L2_*` 0.75, `HEADLIGHTGLASS` 1.00 / 0.35,
`BRAKELIGHTGLASS` 1.00 / 1.00; `TRAFFICWINDOWS` is 0.80 / 0.30; the rest of the car materials are not window materials (alpha 1, or a paint, pearl or character response). Rendered with
`view-car` the stock windows and headlamp covers are see-through with the cube-map reflection on top, so the pipeline
**does** draw smooth translucent car groups: nothing is missing in the engine.

**The mapping [choice, from the data above].** The shader outputs `texture alpha * light material alpha`, so a surface of
opacity *o* is a picture alpha of *o* divided by the light material's alpha.

- Opacity of a texel: `1 - mean(linear RGB) * Max`, the mean being the "grey" of the blend mode, linear because the picture is
  sRGB and the transmittance is a physical quantity. White gives 0.05, the red of the Yamaha 0.87, black 1.
- A triangle takes `WINDSHIELD` (alpha 0.4) when the mean opacity at its corners and centre is at most 0.4, otherwise
  `BRAKELIGHTGLASS` (alpha 1.0), because `WINDSHIELD` cannot show more than 0.4 (the thresholds are `CLEAR_ALPHA` and
  `TINTED_ALPHA` in `glass.rs`). The texel's alpha is stored for the material its own opacity selects, so a clear texel is
  alpha 32 (0.05 / 0.4) and the Yamaha's red lens 221 (0.87; the Ducati's red 0.80 and amber 0.62).
- The colour is the picture's colour as it is (white veils the scene behind the screen by its 5 percent, which is about the
  Fresnel reflection of glass; a red lens is red).
- The texture is DXT5, alpha usage modulated, blend type 1, alpha-sorted, the two-sided flag clear: like the stock windows. A
  blended group of the car effect culls back faces (`culls_blended`), so the screen is seen from the front only.

Not done: refraction, the rain and scratch layers, the lens colour of a lit lamp (the lamps do not light), and the
reflection that a thin translucent surface adds on top of its transmittance (the cube map is attenuated by the same alpha as
the diffuse, so a clear screen shows less reflection than in the game).

**Lamps.** `mat_light<chassis>` is `BLEND_Masked` with the base colour read from `BaseColor - Texture` (the *opacity source*
switch `BaseColorOpacity - Source` is off), so the cut-out of `cutout()` does not apply and the picture's alpha is 255
everywhere (checked: `light665_d` alpha 255 on every texel, `lights151_d` 240 to 255). Its colours are the reflector and the
housing (grey 128 to 206 on the front lamp, the tail lamp's red (173, 0, 5) and light grey), not a white lens; the emissive
colours are `Lights - Light<n> - EmissiveColor` (white and red) with the mask `lights_inc<chassis>`, which the importer does
not read. What looked like a white block was the **lens** in front of the lamp: the glass material baked as an opaque
white picture. With the lens translucent the lamp behind it shows as it is.

**Tangent-space path** (`--normal-maps`): groups with a normal map become effect 3 (`WorldNormalMap`: 60-byte vertices with a
tangent, textures diffuse, normal and specular), which `blackbox-render` already shades with `NormalModel::Bump` for the
world. The specular picture is `(1 - roughness)^2` times grey, blended to the base colour by metallic. The normal map is the
engine's two-channel picture rebuilt as RGB; Unreal's green channel points the other way from many tools' and **the
handedness against this shader is not verified** (`--flip-green` inverts it). A car solid with effect-3 groups draws them
like effect-4 groups (diffuse and light material) until the car scene routes such groups to normal materials; that routing
is a small change in the car scene's `MaterialLookup` (see [ride6-bikes.md](../modding/ride6-bikes.md#rendering)).

## Open

- The master materials' graphs are not in the shipped packages, so the clear-coat and flake arithmetic, the use of the
  occlusion channel and the palette recolour are inferred from parameter names. A binary reverse engineering of the shader
  permutation compiler is possible but would not recover the graph either.
- The stock tyre brand and compound (their sidewall picture; `FRONT_TYRE_COMPOUNDS` rows differ per brand and size, all
  size 120-60-17 rows point at one mesh) and the stock rim colour come from the player's customization state, not from a
  table found in the packages; the importer uses the tyre master's default pictures and the rim picture as shipped.
