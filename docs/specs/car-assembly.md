# Car assembly (stock parts, wheels, brakes, paint)

How NFS: Most Wanted builds the car you see from its pieces: which solids make up a stock car, where the
four wheels and brakes go, and how the body gets its colour. Static pose only (front-end garage); the
in-game additions (suspension, spin, steering, body roll) are listed but not specified here.

- **Sources read:**
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; mostly the GameCube
    build): `src/Speed/Indep/Src/World/CarInfo.{hpp,cpp}` (parts database, `RideInfo::SetStockParts`,
    `SetPart`, `GetUsedCarTextureInfo`), `CarPartID.h`, `CarRender.cpp` (`CarRenderInfo` constructor,
    `UpdateCarParts`, `UpdateWheelYRenderOffset`, `Render`, `FrontEndRenderingCar::LookupWheelPosition`),
    `CarRenderConn.cpp`, `CarSkin.cpp` (`CompositeSkin`, `CompositeSkin32`, `CompositeWheel`),
    `Frontend/MenuScreens/Safehouse/FEPkg_GarageMain.cpp` (`UpdateRenderingCarParameters`),
    `Ecstasy/Ecstasy.hpp`, `EcstasyData.hpp`, `eLight.hpp`, `Generated/AttribSys/Classes/ecar.h`,
    `Generated/Hash.hpp`. Read for understanding; no code copied.
  - [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) (MIT): how legacy VPAK pointer fix-ups,
    class definitions and collection layouts are read (used for a scratch reader of `ecar`).
  - [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) (MIT): `Support.MostWanted/Class/CarTypeInfo.cs`
    to cross-check the `CarTypeInfo` field order.
  - `speed.exe` v1.3 resource `IDI_CAR_FX` (see [shaders.md](../formats/shaders.md)), disassembled
    locally with the system's `D3DCompiler_47.dll` (`D3DDisassemble`) to read the car shader's math.
    Nothing from it is in the repo.
- **Data inputs:** `GLOBAL/GlobalB.lzc`: `CarTypeInfos` (`0x00034600`), `CarPartPack` (`0x80034602`),
  `SlotTypes` (`0x00034607`), `PresetRides` (`0x00030220`), `LightMaterials` (`0x00135200`); AttribSys
  `GLOBAL/attributes.bin` class `ecar`; `CARS/<CAR>/GEOMETRY.BIN` solids and their `SolidMarkers`
  (`0x0013401A`); texture packs `CARS/<CAR>/TEXTURES.BIN`, `CARS/TEXTURES.BIN`, `GLOBAL/GlobalB.lzc`.
  All layouts are in [cardata.md](../formats/cardata.md).

Evidence tags as in the [docs README](../README.md#evidence-tags). "[verified]" here means the rule was
reproduced on the install with the scratch probes (stock parts of all 91 car types, wheel arches of
five cars, a rendered side/front/top preview of the M3 GTR).

## Car space

Car solids use **+x forward, +y left, +z up**, metres [verified: `LEFT_HEADLIGHT` markers sit at +x, +y].
Wheel index order everywhere: **0 = front left, 1 = front right, 2 = rear right, 3 = rear left**.
Matrices below use the engine's row-vector convention: `p' = p · A · B` applies `A` first.

## 1. Stock part selection

A car is a table of **139 slots** (`CAR_SLOT_ID`, names in [cardata.md](../formats/cardata.md#slots)),
each holding one part of the parts database or nothing. Slots 0–75 are model slots (they name solids);
the rest are paint, vinyl, decal-texture, window-tint, HUD and misc choices.

**Finding a part for a slot** [decomp; verified]:

1. The slot maps to a **part id** (`CAR_PART_ID`): slots 0–65 map 1:1; 66 and 67 (front/rear wheel)
   both map to 67 `WHEEL`; 68 → 68 spinner, 69 → 69 plate, 70–75 → 70–75 decal models, 76 → 76 paint,
   77 → 79 vinyl, 78 → 78 rim paint, 79–82 → 77 vinyl paint, 83–130 → 80 decal texture, 131 → 81 window
   tint, 132 → 82 HUD, 133–135 → 83 HUD paint, 136 → 84, 137 → 85, 138 → 86.
2. The slot has up to **two type names** to search, in order: an override from `SlotTypes` if one
   matches (car, slot), else the slot's default pair, where `0xFFFFFFFF` means "this car's own type name"
   and 0 means "none". In this install every model slot searches only the car itself, except: 44 spoiler
   → (car, `SPOILER`) or a per-car override (`SPOILER_PORSCHES`, `SPOILER_CARRERA`, `SPOILER_HATCH`), 62
   roof → `ROOF`, 66/67 wheels → (car, `WHEELS`), 68 → `SPINNER`, 69 → `PLATES`, 76–82 → `PAINT`/`VINYL`,
   131 → `WINDOW_TINT` [verified].
3. Search the parts array in order for the first part whose part id matches, whose type-name hash
   (through the type-name table) equals the searched type, whose name hash matches if one is asked for,
   and whose upgrade level matches if one is asked for. Try the first type, then the second.

**The stock car** [decomp; verified against the `CE_GTRSTREET` preset, which lists exactly the same 81
part hashes]:

- Every slot gets its first part with **upgrade level 0**, except:
  - slot 76 (base paint): the part whose name hash is `CarTypeInfo.DefaultBasePaint` (any level);
  - slots 77 (vinyl layer) and 83–130 (decal textures): left empty;
  - slots 79–82 (vinyl colours): the parts named `VINYL_L1_COLOR01`, `…COLOR03`, `VINYL_L2_COLOR11`,
    `VINYL_L1_COLOR01` (only matter when a vinyl is applied);
  - HUD colours 133–135: `ORANGE`, `ORANGE`, `WHITE`.
- Setting slot 23 (body) also fills slots 46–51 (`DAMAGE0_*` corner overlays) and 72–75 (door and quarter
  decal models) with parts named `<CAR>_KIT<nn>_DAMAGE0_FRONT` etc., where `nn` is the body part's
  `KITNUMBER` attribute (0 for stock). Slot 67 (rear wheel) is never set: the rear wheels reuse the
  front wheel model (§3).
- Presets (`PresetRides`) override slot by slot: a part hash replaces the stock part, 0 clears the slot,
  1 keeps the stock part [decomp].

## 2. From part to solid

A part's model table gives one name per LOD `A`–`E` [decomp; verified on 13,080 parts]:

- **Plain** table: the entry is already the solid's `bStringHash`, or `0xFFFFFFFF` for "no model at this
  LOD".
- **Templated** table: build the hash incrementally (`bStringHash` continues from a previous hash):
  start from a base hash chosen by the part's selector (0 → `0xFFFFFFFF`, i.e. empty; 1 → the part's own
  type name, which for car parts is the car type name; 2 → the part's `BRAND_NAME` attribute, which already
  holds a hash), then append the table's middle string (if any), the
  entry's string, and `_A`…`_E`. Example: `BMWM3GTR` + `_KIT00` + `_BODY` + `_A` =
  `BMWM3GTR_KIT00_BODY_A`; aftermarket rims: `BBS` + `_STYLE01_18_25` + `_A`.
- If no loaded solid has that hash, the slot simply draws nothing at that LOD. Many stock parts name
  solids that do not exist (the M3 GTR's `KIT00_HOOD`, `KIT00_SPOILER`, side mirrors and attachments),
  which is how "part of the body" pieces are expressed [verified].

Solids are searched in the car's own `GEOMETRY.BIN` and the shared packs `CARS/WHEELS`, `BRAKES`,
`PLATES`, `ROOF`, `SPOILER*` (all loaded as one name-hash pool). For stock cars every model comes from
the car's own file [verified: all 91 types].

**Stock wheel = `<CAR>_KIT00_FRONT_TIRE_<LOD>`**, a single solid holding tyre *and* rim (materials
`RUBBER`, `MAGSILVER`, …). 91 of 92 car folders have it (not `COPHELI`); no `REAR_TIRE` solid exists
anywhere [verified]. Aftermarket rims (`CARS/WHEELS`, 164 parts, upgrade levels ≥ 1) are also full
wheels named `<BRAND>_STYLE<nn>_<rim inches>_25_<LOD>` [verified].

**LOD range** [decomp, GameCube build; PC unconfirmed]: player cars use LOD B, AI/traffic C–E, NIS cars A.
Decal models always use LOD A; interior and driver have their own ranges. A viewer can simply use A.

## 3. Wheel placement

Wheel positions come **only from AttribSys** (`ecar` collection keyed by the lower-case base model name,
e.g. `bmwm3gtr`, with inheritance through parents up to `default`). No solid carries a wheel marker
[verified: census of every car `GEOMETRY.BIN`]. One car type, `BMWM3`, has no `ecar` collection (no
gameplay vehicle uses it) and so no wheel placement [verified]. Inputs per car:

| `ecar` field | Use |
|---|---|
| `TireOffsets[4]` (vec4) | x, y, z of each wheel; w = tyre radius |
| `FECompressions[2]` | front/rear suspension compression added to z in the front end |
| `TireSkidWidth[4]` | wanted tyre width per wheel |
| `TireSkidWidthKitScale[7]` (vec2) | width multiplier per kit number (x front, y rear) |
| `KitWheelOffsetFront[6]`, `KitWheelOffsetRear[6]` (u8, mm) | extra track per kit number |
| `CamberFront`, `CamberRear` | camber amount |
| `WheelSpokeCount` (i8) | bit 7 set → mirror the left wheels instead of rotating them |
| `ExtraRearTireOffset` | trucks: draw each rear wheel a second time, shifted by this in x |

The wheel solid is modelled at its own origin: axle along y, the **rim face at y ≈ 0 and the tyre
extending towards +y** (a right-hand wheel) [verified: `MAGSILVER` vertices average y = 0.03, tyre back
cap at y = 0.23].

Per wheel `i` (`end` = 0 front, 1 rear; `kit` = body part's `KITNUMBER`):

```
model_w   = solid AABB size in y          model_r = solid AABB size in x / 2
ws        = TireSkidWidth[i] * KitScale[kit].(x or y) / model_w   (1 if either is 0)
rs        = TireOffsets[i].w / model_r                             (1 if either is 0)
pivot     = 0.5 * model_w * ws                                     (half the drawn width)
track     = |TireOffsets[i].y| + KitWheelOffset{Front|Rear}[kit] * 0.001
z         = TireOffsets[i].z + FECompressions[end]                 (front end; in game: suspension)
camber    = (CamberFront or CamberRear) * 7 degrees,  sign + for wheels 0 and 3, - for 1 and 2
push_down = (CamberFront or CamberRear) * 0.03 m                   (camber only at LOD A/B)
```

Wheel transform (static: no spin, no steering):

```
W_i = Scale(rs, ws, rs) · Translate(0, -pivot, 0) · Side_i · RotX(camber) ·
      Translate(TireOffsets[i].x, 0, z) · Translate(0, axle_y, -push_down) · Body
Side_i = identity for right wheels (1, 2)
       = RotZ(180°) for left wheels (0, 3); Scale(1, -1, 1) instead if WheelSpokeCount bit 7 is set
axle_y = -(track - pivot) for right wheels,  +(track - pivot) for left wheels
```

Without camber the pivot cancels: a right wheel's model point `y_m` lands at `y_m·ws − track`, a left
wheel's at `track − y_m·ws`, so the rim face sits at the track width and the tyre runs inwards. The
pivot (half the drawn width) is the point the camber rotation (and steering, in game) turns about. A
mirrored left wheel flips triangle winding: swap the cull mode for it. The `TireOffsets.y` value itself
is only used through `track`.

In game, spin is a rotation about y and steering a rotation about z (front wheels only), both inside the
`RotX` position of the chain; z then follows the suspension:
`z = TireOffsets.z + compression + (render radius − physics radius)` [decomp].

**Placing the car on the floor (front end)** [decomp]: the car origin is put at height
`mean(radius) − mean(z) − 0.025` above the floor, so the tyres sink 2.5 cm into it. In game the body is
also lowered by `ecar.RideHeight` inches and shifted so the centre of its parts' AABB is at the physics
body's origin; those are out of scope here.

## 4. Brakes

Front brakes use slot 24's solid (`<CAR>_KIT00_FRONT_BRAKE`), rear brakes slot 34's; if the rear has no
model the front one is reused [decomp]. 54 of 92 car folders have a front brake solid, 52 a rear one
[verified]. The brake solid is modelled like the wheel (disc in the y = 0…0.08 band at the origin).

The wheel solid's `SolidMarkers` give the brake's depth inside the wheel: marker `FRONT_BRAKE` of the
front wheel solid, marker `REAR_BRAKE` of the rear one (falling back to the front wheel's
`FRONT_BRAKE`); only the marker's y translation is used [decomp; markers verified]. Brakes are not scaled:

```
B_i = Translate(0, marker_y * ws - pivot, 0) · Mirror_i · RotX(camber) · Translate(x, 0, z) ·
      Translate(0, axle_y, -push_down) · Body          (first factor is identity if marker_y = 0)
Mirror_i = Scale(1, -1, 1) for left wheels (0, 3) (swap the cull mode), identity for right wheels
```

So a brake origin sits `marker_y · ws` inboard of the rim face. Left brakes also swap texture
`BRAKE_GLOBAL` for `BRAKE_GLOBAL_LEFT` (mirrored caliper lettering) [decomp].

## 5. Other parts and what is drawn

| Slot(s) | Stock content | Drawn |
|---|---|---|
| 0 `BASE` | `<CAR>_BASE` (window mask shell; carries light/exhaust/spoiler markers) | body frame |
| 23 `BODY` | `<CAR>_KIT00_BODY` | body frame |
| 25–27, 35–37 windows | `<CAR>_KIT00_*_WINDOW` | body frame, in the transparent pass |
| 28 `INTERIOR`, 29–32, 38–41 lights | `KIT00_INTERIOR`, `*_HEADLIGHT(_GLASS)`, `*_BRAKELIGHT(_GLASS)` | body frame |
| 33, 42 side mirrors | `KIT00_LEFT/RIGHT_SIDE_MIRROR` when the solid exists | body frame, not in reflections |
| 43 `DRIVER` | `KIT00_DRIVER` | in game only, never in the front end |
| 44 `SPOILER` | `KIT00_SPOILER` when it exists | body frame at upgrade level 0; at the `SPOILER` (or `SPOILER2` if the part has `USEMARKER2`) marker of the base solid for aftermarket ones |
| 45 universal spoiler base | hidden while the spoiler's upgrade level is 0 | |
| 46–51 `DAMAGE0_*` | corner damage overlays | hidden until damaged |
| 52–61 attachments, 63 hood | `KIT00_ATTACHMENTn`, `KIT00_HOOD` when they exist | body frame |
| 62 `ROOF` | `NO ROOF SCOOP` (no model) | aftermarket: at the base solid's `ROOF_SCOOP` marker |
| 66 wheels, 24/34 brakes | §3, §4 | four corners |
| 69 plate | `LICENSE_PLATE_STYLE01` | only at `LICENSE_PLATE_*` markers, which no car has [verified] → never |
| 70–75 decal models | `<CAR>[_KIT00]_DECAL_*` | drawn, but every decal texture is replaced by transparent `DEFAULTALPHA` (§6) → invisible |
| 1–22 `DAMAGE_*` | `KIT00_DAMAGE0_<PANEL>` | cop cars build their body from these panels; racers have none |

## 6. Texture replacement

Solids name *placeholder* textures; a per-car table swaps them before drawing [decomp; existence
verified]:

| Placeholder in the solid | Replaced by (stock) |
|---|---|
| `<CAR>_SKIN1`, `GLOBAL_SKIN1` | skinnable cars: the composite paint texture (§7); others: unchanged (`<CAR>_SKIN1` exists in the car's pack) |
| `HEADLIGHT_LEFT/RIGHT` | `<CAR>_KIT00_HEADLIGHT_ON` (headlights always lit) |
| `HEADLIGHT_GLASS_LEFT/RIGHT` | `WINDOW_FRONT` |
| `BRAKELIGHT_LEFT/RIGHT/CENTRE`, `BRAKELIGHT_GLASS_*` | `<CAR>_KIT00_BRAKELIGHT_OFF`, `…_GLASS_OFF` (`_ON` while braking) |
| `DUMMY_DECAL1…6`, `DUMMY_NUMBER_LEFT/RIGHT`, 26 `*_DECAL` names (`HOOD_DECAL`, `LEFT_DOOR_DECAL`, …) | `DEFAULTALPHA` (32×32 DXT3, alpha 0) |
| `<CAR>_TIRE` | itself; `TRAFFIC_TIRE` for traffic cars |
| `WINDOW_FRONT`, `WINDOW_REAR`, `WINDOW_{LEFT,RIGHT}_{FRONT,REAR}` | `WINDOW_FRONT` while undamaged, `WINDOW_DAMAGE0` when damaged [decomp] |
| `REAR_DEFROSTER` | itself (damaged: `WINDOW_DAMAGE0`) |
| `CARBONFIBRE` skin set | only for carbon hoods (`CARBONFIBRE` attribute) |

## 7. Paint

**Colour.** The base-paint part (slot 76) has attributes `RED`, `GREEN`, `BLUE`, `GLOSS` (0–255) and
`LIGHT_MATERIAL_NAME`. For a skinnable car (`CarTypeInfo.Skinnable` = 1; 47 types) the body's
`<CAR>_SKIN1` does not exist in any pack [verified] and is replaced by a runtime texture (`DUMMY_SKIN1`…
in `CARS/TEXTURES.BIN`, 512×512 A8R8G8B8) that the game **fills with the paint colour**: every texel
`(R, G, B, A = GLOSS)`, then a vinyl layer blended on top through its mask, then four swatch texels
[decomp]. Stock cars have no vinyl, so the body texture is a flat colour. Non-skinnable cars (cops,
traffic) use their own painted `<CAR>_SKIN1` [verified: 38 of 44 have one].

**Material.** Body groups use light material `CARSKIN`; at draw time `CARSKIN` is swapped for the
paint part's light material, and `WINDSHIELD` for the window-tint part's (stock: `WINDSHIELD`). Rim
materials `MAGSILVER/MAGCHROME/MAGGUNMETAL` are swapped for the rim-paint part's material only when the
wheel's upgrade level ≠ 0, so stock rims keep their own materials; the caliper swap
(`CALIPER/CALLIPER`) is wired up but never fed a part in the decompiled build [decomp]. Spoiler and roof
scoop use the body's paint material unless painted separately.

**BMW M3 GTR stock paint** [verified]: `DefaultBasePaint` = `0xC7F2884E` → part `METAL_L1_COLOR02`:
RGB **(79, 79, 79)** = `#4F4F4F`, gloss 128, light material `METPAINTSILVER`: diffuse 0.75 → 1.0,
specular power 3 (0.7 → 0.3), env-map power 0.15 (3.5 → 0.2), all colours white. The blue livery seen in
the game's story is a vinyl, not the base paint.

## 8. The car shader

All car groups use effect id 4 [verified on 6 car files; mapping to `IDI_CAR_FX` unconfirmed]. The
shader's constants and math, read from the disassembly [verified as a reading of `IDI_CAR_FX`]:

Per vertex (model space; `N` normal, `V` unit vector to the eye, `vc` vertex colour):

```
f      = max(N·V, 0.01)                          facing term
s      = saturate(2 * vc.r)                      baked occlusion (vertex colours ≥ 0.5 → 1)
light  = Σ_{k=0..2} saturate(N·L_k + 0.1) * C_k  three directional lights (LocalDirectionMatrix rows,
                                                 LocalColourMatrix columns)
diff   = DiffuseMin  + DiffuseRange  * f         (rgba)
spec   = SpecularMin + SpecularRange * f^SpecularPower
env_k  = EnvmapMin.x + EnvmapRange.x * f^EnvmapPower
R      = 2 f N − V                               reflection vector (to view space for the cube map)
```

Per pixel (`tex` = diffuse texture, `env` = cube map at `R`, `sh` = 1 when not shadowed, else
`0.2 + InvShadowStrength`):

```
glint = 2 * saturate(reflect(L_0, N)·V)^16       sun highlight (light 0 only)
rgb   = (s*light*diff.rgb * tex.rgb + glint * diff.a * s*spec * sh) * sh
        + env.rgb * s*env_k * diff.a * 0.5
alpha = tex.a * diff.a                           paint: GLOSS/255 (consumer of this alpha unconfirmed)
```

The light material maps to the constants as `…Min = MinScale × (MinR, MinG, MinB[, MinA])` and
`…Range = MaxScale × Max… − …Min` (the min is the grazing value, the max the facing value)
[unconfirmed: the constant setup code is not decompiled; the names and the plausible Fresnel-like values
support it]. `MetallicScale` and `SpecularHotSpot` are not read by this shader. The alpha ramps
(`DiffuseMinA`/`MaxA`) are used as they are, without a scale.

### Where the inputs come from

| Input | Source | Status |
|---|---|---|
| `DiffuseMin/Range`, `SpecularMin/Range/Power`, `EnvmapMin/Range/Power` | the `LightMaterials` chunk of each shading group's light material (`ShadingGroup.light_material` indexes the solid's `light_material_hashes`) | [verified] all 156 materials read; M3 GTR groups use 21 of them |
| `CARSKIN` | not a `LightMaterials` entry: hash `0xD6D6080A` is named by every body group and swapped for the paint part's `LIGHT_MATERIAL_NAME` (§7) | [verified] |
| `WINDSHIELD` | stays as is; the window-tint part's own material swap (§7) is not applied | approximation |
| Unlisted hash | a plain default material (diffuse 1, specular 0.2, env 0.1) | [guess] |
| Light rig | `eShaperLightRig`: a name hash and **four** `eShaperLight` entries of {mode, theta, phi, red, green, blue, scale}, mode = world space, camera space, sun direction, opposite sun direction or world position; named rigs exist for the car lot, safehouse, back room, shop, in-game cars, … [decomp] | the shader reads three of them |
| Rig values | **not in any file of the install**: no `ShaperLightRig` chunk (`0x0003B650`) exists in any `.bun`, `.lzc` or `.bin` (all scanned, inflated) [verified]; angle units and the values are in the executable only [unconfirmed] | chosen, see below |
| Environment | the game renders a cube map of the surroundings (`g_CarEnvironmentMapEnable`, [shaders.md](../formats/shaders.md)); no static cube map for cars was found in `CARS/` or `GLOBAL/` | procedural sky |

### What is implemented

`Shading::Glossy` in `blackbox-render` (`shaders/glossy.wgsl`) draws every car group. With the `car_shading = simple`
setting (docs/low-end.md) the game maps every group to `Shading::Lit` instead and creates no glossy material,
light rig or environment, so the renderer builds none of the glossy resources:

- the §8 maths per pixel, in world space (the lights are given in world space instead of being
  rotated into each model's space; the result is the same), with `sh = 1` (no shadow map) and the
  reflection looked up with the world-space reflection vector, not a view-space one;
- the highlight is the sun mirrored about the normal against the view vector, and is zero when the
  normal faces away from the sun (`step(0, N·L₀)`; the sign convention of the original `reflect` call
  is not known) [guess];
- the alpha is `tex.a × DiffuseAlpha` only when blending; the alpha test and opaque draws ignore the
  diffuse alpha (a windshield at 0.4 would vanish under the test);
- the paint texture's alpha (gloss) is not read; the flat paint texture is opaque;
- fog as the other shadings; no tone mapping, so sums above 1 are clamped by the target.

Game side (`scenes/car/`): `shading.rs` turns each `LightMaterial` into a `GlossyMaterial`
(min/range as above); `materials.rs` creates one renderer material per light material of the car and
the `CARSKIN` swap; the floor keeps the placeholder shading.

**Rig defaults** [guess, not game values]: key light = the scene's sun (viewer: from (0.4, 0.3, 1);
world: from (0.35, 0.45, 1)), colour (0.95, 0.92, 0.84); fill 150° around the sun's azimuth at 25°
elevation, colour (0.38, 0.43, 0.52); back light −110° at 35°, colour (0.30, 0.32, 0.38); ambient 0.
**Sky** [guess]: zenith (0.30, 0.48, 0.78), horizon (0.78, 0.84, 0.90), ground (0.22, 0.23, 0.25),
64×64 faces, blended by height (zenith over `z^0.6`, ground over 0.35 below the horizon).

Still approximate: the rig values and the sky (above), the reflection of the car's own surroundings
(the game renders them), shadows, per-vertex evaluation of the facing ramps (the game evaluates them
per vertex; here per pixel), window tints, rim and caliper paint swaps, and the gloss in the skin's alpha.

## Constants

Hard-coded in the code, not in data [decomp]: pivot fraction 0.5; camber 7° per unit and push-down
0.03 m per unit, applied at LOD A/B only; kit wheel offset unit 0.001 m; fallback wheel width 0.225 m and
radius 0.32 m when a wheel solid is missing; front-end floor offset −0.025 m; shader constants 0.01,
0.1, 16, 2, 0.5, 0.2 as above [verified in the disassembly].

## Worked example: BMW M3 GTR, front end [verified]

| | FL (0) | FR (1) | RR (2) | RL (3) |
|---|---|---|---|---|
| `TireOffsets` x, y, z, r | 1.615, 0.88, 0, 0.33 | 1.615, −0.88, 0, 0.33 | −1.12, −0.89, 0, 0.33 | −1.12, 0.89, 0, 0.33 |
| centre z (+ `FECompressions` 0.21) | 0.21 | 0.21 | 0.21 | 0.21 |
| width scale (skid 0.235 / 0.255 ÷ 0.2469) | 0.952 | 0.952 | 1.033 | 1.033 |
| radius scale (0.33 ÷ 0.3416) | 0.966 | 0.966 | 0.966 | 0.966 |
| tyre spans \|y\| | 0.653–0.888 | 0.653–0.888 | 0.644–0.899 | 0.644–0.899 |
| brake origin \|y\| (marker 0.046 × ws) | 0.836 | 0.836 | 0.843 | 0.843 |

Wheel solid `BMWM3GTR_KIT00_FRONT_TIRE_A` (all four), brakes `BMWM3GTR_KIT00_FRONT_BRAKE_A` (front) and
`…_REAR_BRAKE_A` (rear), `WheelSpokeCount` 8 (left wheels rotated, not mirrored), camber 1.96° / 1.4°
with 8.4 / 6 mm push-down, kit offsets 0. Wheelbase 2.735 m. Checks: the body's wheel-arch openings at
centre height span x 1.238–1.981 and −1.491…−0.728 (centres 1.61 and −1.11), arch tops at z 0.527/0.531
against tyre tops at 0.54, and no body vertex lies inside 0.9 × the tyre radius. The same check fits the
911 Turbo, 997 S and Gallardo.

## How to check it

1. **Placement:** in the garage, screenshot the M3 GTR from the side and front, render the same pose with
   our viewer (car origin 0.095 m above the floor, wheels per §3) and overlay: rim faces flush with the
   arches, tyres 2.5 cm into the floor. Repeat for a `WheelSpokeCount` < 0 car (911 Turbo, Corvette,
   Monaro, RX-8, SLR) to see mirrored directional rims, and a truck (`ExtraRearTireOffset`).
2. **Stock parts:** compare our slot table with the `PresetRides` entry of the same car (`CE_GTRSTREET`
   for the M3 GTR; `BL*` presets for the Blacklist cars differ from stock on purpose).
3. **Paint:** the M3 GTR body without its vinyl should be dark metallic grey #4F4F4F; garage paint
   swatches show the `RED/GREEN/BLUE` of each `PAINT` part.
4. **Shader constants:** capture a garage frame (e.g. apitrace or PIX for D3D9) and read vertex shader
   constants c18–c25 while the body is drawn; compare with `METPAINTSILVER` to settle the min/range
   mapping, and the light rig in c11–c16.
