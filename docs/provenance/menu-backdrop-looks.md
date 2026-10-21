# Provenance: the menu backdrop looks

Addresses, file offsets, method and dead ends for [menu-backdrop-looks.md](../specs/menu-backdrop-looks.md). Ghidra auto
names (`FUN_*`, `DAT_*`) are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3). Every
reading is static (decompile and disassembly); the pictures were checked with offscreen renders of this rewrite only.

## 1. Method

1. Started from the stage chain in [menu-backdrop.md](../specs/menu-backdrop.md) sections 6 and 7: `FUN_007B9EA0` (per-frame
   update of the front end's car scene) calls the car stand routine `FUN_007A96C0` and the stage loader `FUN_007B40E0`.
2. The solids of the stage files were listed with a scratch reader on `blackbox-solid` (name, version, bounds, pivot,
   markers). No marker is present in any of the six files, so the placement had to come from the executable.
3. The stand routine's float constants were read from the image (`read_memory`) and decoded by hand.
4. The camera was followed from the scene constructor `FUN_007BEED0` to its mover object (`FUN_00476870`, base `FUN_00470890`)
   and its update `FUN_004769E0`; the attribute lookups by decoding the `frontend` class of `ATTRIBUTES.BIN` and the
   collections of `FE_ATTRIB.bin` with a scratch program on `blackbox-attrib` and `nfsmw-data`.
5. The pose conventions (eye, roll) were closed by rendering and looking at the picture, not by a second reading.

## 2. The car stand (`FUN_007A96C0`, `0x007A96C0`)

| What | Address | Reading |
|---|---|---|
| Entry guard | `0x007A96D0`..`0x007A96EF` | car id at `+0x8` absent or -1, or scene `+0x6C` set: the car is hidden (`+0x574` = 0) |
| Wheel loop | `0x007A9740`..`0x007A97AA` | four times: `FUN_00737AD0` (wheel position, from the car's wheel table at model `+0x318`) and `FUN_00737B60` (wheel size, default 1.5); sums of the sizes at `[esp+0x1C]` and of the heights at `[esp+0x14]` |
| Car height | `0x007A97AC`..`0x007A97E5` | `(sum sizes * 0.25) - (sum heights * 0.25) - 0.025`; `0x008910F0` = 0.25, `0x008916CC` = 0.025 |
| Car x, y | `0x007A97CF`, `0x007A97D5` | read from `0x009B9E50` / `0x009B9E54`, both 0, never written (a Ghidra xref lists only these two reads) |
| Platform switch | `0x007A98A4`..`0x007A98CE` | `FUN_00516E10` gives the front-end manager (`0x0091CAE0`), `FUN_00516E20` its `+0x18` platform id; id 2 loads `0x008B719C`, ids 3 and 5 load `0x00890968`, others `0x008B7180` (the angle of the first rotation, which turns the position, which is (0, 0): no effect) |
| Car turn | `0x007A99B5`..`0x007A9AB5` | the car matrix is `RotZ(a) * RotX(b) * RotY(c)` with `a` = `0x008B71E0` (340.0) for platform 5, else `0x008B71A8` (304.97); `b` = `0x008B71D8` (-0.379) for 5, else `0x00890968` (0); `c` = `0x008B71DC` (-0.000189) for 5, else 0 |
| Angle units | `0x00893348` = 65,536.0 | each angle is multiplied by it, rounded (`FUN_007C4B80`, a float to int) and divided by 360 (`0xB60B60B7` is the reciprocal trick): degrees to 16-bit units |
| Front wheel steer | `0x007A9ADB` | `0x00905E20` = 21.67 degrees, applied to the first two of four wheel matrices (`iVar4 < 0x41`) |

Constants (little-endian floats): `0x008B7180` = 134.4, `0x008B719C` = 302.85, `0x008B71A8` = 304.97, `0x008B71E0` = 340.0.
The 304.97 and 340.0 also appear as `CarZ` presets (304.96979 for presets 1 to 4, 340 for preset 5) in a community
loader's ini ([../mods/nfsmw_feloader.md](../mods/nfsmw_feloader.md) section 3), which agrees with the platform split
(ids 1 to 4, and 5). No code was taken from that loader.

## 3. The camera

### 3.1 Objects

- `FUN_007BEED0` (`0x007BEED0`) builds the front-end scene object: view `0x009BA3A0` (an `eView`, `FUN_00739A70`), the car
  display object (`FUN_00760D50`, 0x590 bytes) and the camera mover (`FUN_00476870`, 0x140 bytes, vtable `0x00894E90`, base
  `FUN_00470890` with vtable `0x00894BC8`). The name `BACKDROP` is a scene string at `+0x38`.
- The mover stores its state at `+0x90`..`+0xB8` (pitch, yaw, distance, roll, field of view, target x, y, z), the move's start
  at `+0xC0`.. and its goal at `+0xF0`.., and the move's decay (`+0x12C`), order (`+0x130`) and time (`+0x134`, `+0x138`).
- `FUN_004770B0` (`0x004770B0`) sets a goal: `param_1` = (pitch, yaw, distance) to `+0xF0`..`+0xF8`, `param_2` to `+0xFC` (roll),
  `param_3` to `+0x100` (field of view), `param_4` = target to `+0x110`..`+0x118`, `param_5` to `+0x138` (duration),
  `param_6` to `+0x12C`, `param_7` to `+0x130`.
- `FUN_007B9EA0` calls it (`0x007BA085`) with values read from the pose collection (`local_40 + ...`), which came from
  `FUN_0051E1A0` (an `Attrib::Instance` of the class with hash `0x85885722`, the class `frontend`). The mapping of the
  arguments to layout offsets: `param_1` = `[0x3C]`, `[0x54]`, `[0x50]`; `param_2` = `[0x44]`; `param_3` = `[0x24]`;
  `param_4` = `[0x2C]`, `[0x30]`, `[0x34]`; `param_5` = `[0x20]`; `param_6` = `[0x1C]`; `param_7` = `[0x48]`.

### 3.2 The update (`FUN_004769E0`, `0x004769E0`)

State 0 blends start and goal with `1 - e^(-k t^2) * cos((n + 0.5) pi t^2)` (`t` the time over the duration, `k` and `n` the decay
and order), wraps the angles into 0..360, and ends in state 2. State 1 is the user's rotate and zoom: the yaw and pitch move
with the stick and are clamped to 94.5..110 (pitch) and 4.65..6.65 (distance) while `DAT_00926134` is clear. After the state
code it calls `FUN_0046B7E0(matrix, state + 0x90)`, then sets the view's field of view to `[state + 0xA0] * 65536 / 360`
(`0x00476E10`..`0x00476E36`, word at view `+0xC4`) and the view's `+0xB0` to the distance, and hands the matrix to the view
(`FUN_004700D0`).

### 3.3 The matrix (`FUN_0046B7E0`, `0x0046B7E0`)

Row-vector (Direct3D) convention. `M = RotZ(yaw) * RotX(pitch) * Translate(0, 0, distance)`; `V = inverse(M)` through
`FUN_006BDE60` (translation `-t`, rotation transposed); the "eye" is read from `V`'s translation row at `[esp + 0x9C]`
(`0x0046B8AB`..`0x0046B8C9`), which is `-distance * M's third column`:
`-d * (sin(yaw) sin(pitch), cos(yaw) sin(pitch), cos(pitch))`. The up vector is `FUN_00468960`: world up turned about the
(target - eye) direction by the roll (`FUN_006BD980`, an axis-angle matrix). `FUN_006CF0A0` is the look-at: forward =
normalise(target - eye), x = forward cross up, y = forward cross x; the camera looks down +z.

The rotation helpers: `FUN_006BDD20` is rotation about z (`FUN_006BDB50`: `[[c, s], [-s, c]]`), `FUN_006BDBB0` about x,
`FUN_006BDC60` about y, `FUN_006BDD50` a translation; sine and cosine are `FUN_0045DB30` / `FUN_0045DBF0` over 16-bit units.

### 3.4 Which pose: `FUN_007A8C70` (`0x007A8C70`) and `FUN_007A8FE0` (`0x007A8FE0`)

`FUN_007A8C70(name, index)` builds the pose collection's name from the front-end context word (`*(0x0091CF90 + 300)`):
bit 5 (customize): `customize_<name>` and `customize_<name>_<index>`; bit 15: `carlot_`; bit 0: `career_`; bits 2 and 10:
`quickrace_`; bit 2: `quickracemain_`; bits 3 or 6: `online_`; bit 4: `options_`; bit 8: `career_manager` (the name is replaced);
else the name as is. It hashes the string (`FUN_004549B0`) and returns the hash if the collection exists, else `0x3B5AEA62`
(`default`). `FUN_007A8FE0(hash)` follows the collection's reference field (layout offset 0, a `RefSpec` to the `angle*`
collection) with `FUN_0056D0D0` and returns its key, `0xF907E767` when absent. `FUN_007B9EA0` stores the name hash at scene
`+0x74` and re-sets the goal when it changes, which is why a screen change moves the camera. `*(frontend + 0xE0)` (`FUN_00516AC0`)
is the current screen record whose `+0xC` is the file name string.

### 3.5 The `frontend` class

Layout (`ATTRIBUTES.BIN`, `frontend`, size 91 bytes): offset 0 reference (`0x28B5440B`), 12 reference (`0x851874EB`), 24 text
(`0x9CA1C8F9`), 28 `0x2581B8BA` (decay), 32 `0xF624544D` (duration), 36 `0xC220D5AB` (field of view), 44/48/52 `0xA8D9A1F4`,
`0xFE9B5900`, `0x7C9B89D3` (target), 56 class (`0x0DE009BE`), 60 `0xC8F96E5A` (pitch), 68 `0x05B56526` (roll), 72 `0x798624CA`
(order), 76 price, 80 `0xA10C312C` (distance), 84 `0x803FA14C` (yaw), 88 for sale, 89 unlock rank, 90 `0x998DD0DB`. The 446
collections of the install's `FE_ATTRIB.bin` are the cars of the lot, the screen collections (parents `career_master`,
`quickrace_master`, `option_master`, `customize_master`, `backroom_master`, `online_master`, `mc_master`, `demo_master`, all under
`screens`) and the `angle*` pose collections (under `angles`).

Poses read (pitch, yaw, distance, roll, field of view, target):

| Collection | Pitch | Yaw | Dist | Roll | FOV | Target |
|---|---|---|---|---|---|---|
| `angle10` (`career_mainmenu_sub`) | 97.3 | 110 | 4.25 | 16 | 65 | -0.2, -0.5, 0.3 |
| `angle_final_career_safehouse` | 97.3 | 110 | 4.25 | 16 | 65 | -0.2, -0.5, 0.3 (move 3.15 s) |
| `angle_entry_career_safehouse` | 94.4 | -58 | 9 | -13.1 | 55 | 2.3, -0.3, 1 |
| `angle3` (`mainmenu`) | 97.7 | -3 | 5.5 | 7.3 | 55 | 1.2, -0.3, 0.3 |
| `angle_career_manager` (`career_manager`) | 95.8 | 64 | 4.5 | 24.7 | 65 | 1.2, -1.3, 0.5 |
| `angles_bounty` (`career_safehousebounty`) | 116.1 | 99 | 7 | 0 | 65 | 1.5, -3.7, -2.6 |
| `angle21` (`career_safehousemilestones`) | 106 | -46 | 6 | 0 | 65 | 1.1, -2.6, 0.6 |
| `angle_racesheet` (`career_safehouseracesheet`) | 101.6 | -41 | 5 | 20.3 | 55 | -0.6, -0.6, -0.7 |
| `angle20` (`career_safehousereputationoverview`) | 104.5 | 21 | 6 | -17.4 | 49 | -5, -0.7, 1.1 |
| `angle22` (`career_safehouserivalchallenge`) | 94.4 | -134 | 4.5 | -3.1 | 35 | 2.9, -3.7, 1.4 |
| `angle4_d` (`career_car_select`) | 101.6 | -75 | 6 | 0 | 49 | -1.7, 0, 0.4 |

## 4. The cars rendered to pick the default

`nfsmw view-car <folder> [--preset P] --yaw 35 --screenshot out.png`:

- `BMWM3GTR`: dark grey metallic, no arches, small rear spoiler (free roam's car).
- `BMWM3GTRE46`: red, wide arches, large rear wing, GTR front bumper. The `PresetRides` entry `M3GTRCAREERSTART` (car type
  `BMWM3GTRE46`, front-end key `0x23CC4A4E`, vehicle key `0x39DAE0F8`) renders white with the same kit.
- `BMWM3`: the road M3. Not the car.

The red car is the stock `BMWM3GTRE46`, so no paint option is needed. In the safe house render its plate reads NFSMW.

The vehicle key of a stock car is `vlt_hash(lower-case type name)` (checked for the racer types against `CarTypeInfos`); the
front-end keys differ for six lot cars. A preset ride's handle is `bstring_hash(preset name)`:
`bstring_hash("M3GTRCAREERSTART")` is `0x38A3C42C`, the starter handle (a unit test checks it).

## 5. The stage files

`CAREER_SAFEHOUSE.BIN` (9,433,536 bytes): `GeometryPack` at 0 (1,599,224 bytes, 16 children: a `MeshContainerInfo` and 14
`SolidPack`s), `TexturePack` at `0x18CA00` (7.8 MB), `TextureAnimPack` at `0x8FEE00`, an `EmitterLibrary` at `0x8FEF70`. The solids
are version `0x16` (the crib's are `0x18`). Names, polygons and bounds (x, y, z):

| Solid | Polys | Bounds |
|---|---|---|
| `BACKDROP_CRIB` | 2,023 | -15..12.7, -12.1..11.3, -0.13..8.5 |
| `BACKDROP_REFLECTION` | 3,378 | -15..13, -11.8..12, -9.06..0.03 |
| `BACKDROP_CURRGEN` | 74 | -21.9..19.9, -13..13.1, -11..11 |
| `BACKDROP_CAST_SHADOW_MAP`, `..2`, `..3` | 389, 2,127, 284 | inside the room |
| `BACKDROP_CAST_SHADOW_MAP_NEXTGEN` | 8,568 | z 22.7..24.5 (above the room) |
| `STEAM EMITTER01`..`07` | 8 each | 0.6 m wide, 1.76 m tall, placed by their pivots |

Renders (offscreen, `view-screen MainMenu_Sub.fng --look safehouse`): with `BACKDROP_CRIB` alone the windows show the clear
colour and the floor is dark; adding `BACKDROP_CURRGEN` makes the windows white and puts a pale patch on the floor (the
reflection of the windows); adding `BACKDROP_REFLECTION` mirrors the room's frame in the floor. All three together are what is
drawn. `CAR_LOT.BIN` has `BACKDROP_CRIB` (9,039 polygons), `BACKDROP_CURRGEN`, `BACKDROP_CAST_SHADOW_MAP`, `OBJECT01` and `FX_*_EMITTER_*`
markers. No file has a solid with position markers.

## 6. Dead ends

- The `FrontB.lzc` markers and the `PlatformCrib.BIN` solids carry no car position: the stand is code.
- The two position globals `0x009B9E50` and `0x009B9E54` have no writer; they are the constant zero.
- The pose collections of customize screens are keyed by part ids (`customize_customizeparts_258`); only the safe house screens
  were read for this look.
- Not read: the code path that draws the car's reflection (`FUN_004AF820`, called at the end of the stand routine with the
  scene's reflection object), and the shadow map pass.
