# The road signs and traffic lights of the 2012 world (Chameleon) and of the 2005 world

Where the road signs and the traffic lights of NFS: Most Wanted (2012) are in its bundles, how they are made, how the 2005 game's own
are made, and how the one stands in for the other (the **road signs** and **traffic light models** variants of
[`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md), declared through
[the replacement API](../modding/api-replace.md)). Bundle container: [chameleon-bundle.md](chameleon-bundle.md); models, materials, textures:
[chameleon-gfx.md](chameleon-gfx.md); the same method for trees: [chameleon-vegetation.md](chameleon-vegetation.md). Evidence tags as in the
[docs README](../README.md#evidence-tags).

Everything about the 2012 install (v1.4.0.0) was measured with the Python scratch readers of `tools/chameleon_bundle.py` and
`tools/chameleon_export.py` (a software renderer written for the purpose drew every model quoted below, with back faces culled, to look at
them); everything about the 2005 install (PC, v1.3) with an ignored test of the host that dumps solids by name
(`crates/nfsmw/src/modding/content/replace/real_dump_tests.rs`). Nothing from either game is in the repository: the tables are ids, names,
boxes and measured sizes.

## How the 2012 models were told apart **[verified]**

A 2012 bundle does not name its resources (the ids are hashes), so a sign is found by what it is made of and by looking at it:

1. **The shader.** `Sign_RetroReflective` (shader `0x0100005300007840`, samplers Diffuse, Specular, Reflection) is used by **5 materials**, and
   they are on **12 models** of 10 track units and 2 arenas, every one a whole building front with a sign face among its meshes (for example
   `0x12825570` of `TRK_UNIT110`: 33 meshes, 7 techniques). The standalone street signs do **not** use it: they are drawn with the ordinary
   world shaders (below).
2. **The placement lists.** The 12,290 props of the world are 112 `PropObject`s (the `PropInstanceList` of each unit, a model each; read as
   [chameleon-world.md](chameleon-world.md) says), 4,477 dynamic objects are 271 `WorldObject`s, and 35,431 plain instances are 3,287 models.
   The signs, lamp posts and hydrants are among the props and the dynamic objects (a sign is a smackable prop in the 2012 game too).
3. **Looking.** Every distinct model with a bounding sphere under 9 m (872 of the install's 3,808 models) was drawn and read off contact
   sheets. A street sign is one mesh (a plate or two on a pole, 72 to 220 triangles) of one picture, which is how the 40 or so of them were
   found; the traffic lights were found among the 159 models whose shader lights a surface (`Emissive`, `Illuminance`).

The signs come in **bent twins** (`32adb500` is `e55b2ad8` with its pole bent: the prop the game shows after a crash), and the warning
diamonds (`8a7a07a1` and 40 others of the `World_Diffuse_Specular_Singlesided_InstanceAdditive` / `InstanceTint` shaders) draw **grey**
without an instance colour, so they are not used: the colour of the face is the instance's, which the 2005 scenery cannot give.

### The signs the variant carries **[verified]**

| Key | Model id | Bundle | What it is | Picture | Triangles | Front |
|---|---|---|---|---|---|---|
| `STOP` | `0xACCB7DE9` | `TRK_UNIT6` | clean stop sign on a pole with a base plate, 2.81 m | 256 x 128 BC1 | 168 | +z |
| `STOP_OLD` | `0xDE9FDED6` | `GLOBALRESOURCES` | weathered stop sign, rusted pole, 2.71 m (placed twice in the world) | 256 x 512 BC1 | 218 | +z |
| `SPEED_45` | `0xE55B2AD8` | `TRK_UNIT103` | "SPEED LIMIT 45", 2.31 m, plate 0.59 wide (59 placements) | 256 x 256 BC1 (an atlas: the 15 shares it) | 72 | -z |
| `SPEED_55` | `0x712AF22E` | `TRK_UNIT110` | "SPEED LIMIT 55" (66 placements) | 256 x 256 BC1 | 72 | -z |
| `NO_ENTRY` | `0xA98F0117` | `DISTRICT_1024094` | "DO NOT ENTER" (45 placements) | 128 x 256 BC1 | 72 | -z |
| `TOW_ZONE` | `0xCF2C6DD3` | `DISTRICT_1024094` | two plates: "TOW ZONE, NO STOPPING ANY TIME" over a no-parking disc (235 placements) | 1024 x 1024 BC1 | 132 | -z |

The 2012 speed limits are 15, 20, 45, 55 and 70 (no 35, 50, 60 or 75, which the 2005 game has); the other models seen and **not** carried:
the route and interstate shields and the green highway panels (Fairhaven names and numbers), "keep right", "no trucks", "right lane must turn
right", the yellow "PAVED" parking sign, the warning diamonds, the blank sign backs (`79bdccfe`, `eff79db5`).

Every model has **four levels of detail** (`lods 4`); the variant takes the finest. The geometry is a **single mesh** with the one picture;
`World_Diffuse_Specular_Singlesided` (and the `Deflicker_WorldPBR_*` and `WorldPBR_*` variants for the lit ones) have one diffuse sampler,
which is the first texture. The texture coordinates **leave the unit square**: the pole is a strip of the picture repeated down its length
(v from -3.57 to 4.57 on the speed and no-entry signs), so the picture must repeat (`TilableUV` 3 in the 2005 texture record).

**Which side the picture is on** was read off renders with back-face culling (the engine winds triangles counter-clockwise outward): the
stop signs show their picture towards +z of the 2012 axes (x left, y up, z forward), every other sign towards -z. It is data in the table
(`Front`); the converter turns the model about the vertical axis so that the picture looks towards -y, where the 2005 signs show theirs.

### The traffic light's parts **[verified]**

| Part | Model id | Bundle | What it is |
|---|---|---|---|
| Mast arm | `0xBB97A2C5` | `GLOBALRESOURCES` | one mesh, 1,458 triangles, 6 m tall with an arm of 5.9 m towards -x: the **pole, its base, the arm and its struts**, **and** two signal heads hanging from the arm and a pedestrian head on the pole (62 placements). Its twins: `0x7BD98686` (62; the `Skin` shader, the breakable form), `0x1BACE0D2` and `0xE80628E4` (24 each, the other-handed pair) |
| Signal head | `0xF6135E80` | `TRK_UNIT182` | a pole 3 m tall (mesh one, picture `0x6D21A`, 256 x 512) and **a head** (mesh two, 402 triangles, the picture of the arm, `0x9DBF8`, 256 x 256 BC1). `0x4B04296D` is the same with the `Skin` shader |
| Picture | `0x9DBF8` | `GLOBALRESOURCES` | the atlas that the arms' and the heads' meshes share |

The arm and the heads' shader is `Deflicker_WorldPBR_Emissive_Diffuse_Normal_Specular_Reflective_AO_*`: an **illuminance map** lights the
lenses, and the game lights them (red, amber, green) from its own state; the 2012 files hold no separate lamp models. The mesh of an arm
falls into surfaces that share vertex positions, so it can be cut (the measured components of `0xBB97A2C5`, 2012 axes, metres):

| Surface | Triangles | Box |
|---|---|---|
| the pole | 294 | y 0.73 to 5.46, 0.2 across |
| the pole's foot | 60 | 0.5 wide, y -0.02 to 0.77 |
| the arm's tubes and struts | 28 each | up to 5.9 long, at y 4.7 to 5.96 |
| **a signal head's housing** (two of them) | 180 | 0.29 wide, 0.9 tall, 0.21 deep, at y 4.66 to 5.56 |
| **its three lens visors** | 70 each | 0.22 x 0.2 x 0.18, centres 0.272 m apart (red on top, amber, green below) |
| its hanger | 38 | 0.25 tall, above the housing |
| the pedestrian head on the pole | 48 | 0.34 tall, y 1.95 to 2.27 |

The head of `0xF6135E80` has the same housing, visors and two small visors (6 triangles each) between them. **A head faces +z** (its lenses
protrude to z 0.26 to 0.43 in front of a housing at z 0.05 to 0.26) **[verified]**; the lens centres of the standalone head are at y 2.905,
2.63 and 2.36: **0.272 m apart**.

## How the 2005 game builds them **[verified]**

### The signs

`XS_*` solids (538 names) are placed from the shared library **`X0`** (every generic sign below is there, once, and is placed by a matrix),
a few in a tile (`XS_DONOTENTER_1B_LZ_00` in `O53`, `XS_NOPARKINGA_1B_CK_00` in `R72`). A pole sign is **three shading groups**, all effect 0,
flags `0x4000`: the **front plate** (its normal -y, the picture), the **back plate** (normal +y, a plain grey) and the **pole** (normal around -y,
a strip of the pole picture). The vertex colours are greys from 47 to 83 (the baked light). 16 to 54 triangles, **one level of detail**
(`_1B_00`), texture coordinates from -1.1 to 2.1 down the pole. The box of each, in the solid's axes (x across the plate, y through it, z up;
the pole is buried 0.3 to 0.4 m below the origin) and the placements the scenery has (instances, counted from the sections' scenery lists):

| Solid | Box, x | y | z | Placements |
|---|---|---|---|---|
| `XS_STOPSIGNA_1B_00` | -0.40 to 0.40 | -0.055 to 0.047 | -0.01 to 2.23 | 5 |
| `XS_STOPSIGNB_1B_00` | -0.56 to 0.50 | -0.082 to 0.096 | -0.01 to 3.57 | 100 |
| `XS_SPEED45_1B_00` | -0.51 to 0.51 | -0.050 to 0.089 | -0.29 to 3.24 | 67 |
| `XS_SPEED55_1B_00` | -0.67 to 0.67 | -0.063 to 0.115 | -0.39 to 4.31 | 29 |
| `XS_DONOTENTER_1B_00` | -0.70 to 0.71 | -0.053 to 0.115 | -0.39 to 4.22 | 54 |
| `XS_NOPARKINGA_1B_00` | -0.21 to 0.21 | -0.050 to 0.050 | -0.03 to 2.23 | 60 |
| `XS_NOPARKANYTIME_1B_00` | -0.25 to 0.23 | -0.036 to 0.037 | -0.01 to 2.35 | 14 |

(The 2005 signs are larger than the 2012 ones for their pole height: the plate of the 45 sign is 1.0 m wide on 3.5 m.) The signs the
variant does **not** replace, and why: the speed limits 35, 50, 60, 75 (the 2012 game has no such sign, and another number would be wrong);
yield, do-not-pass, no-exit, no-motor-vehicles, the warning signs (`XS_WARN*`, `XS_SIGNALAHEAD`, ...), `XS_CHEVRON*` (no matching 2012 model with
a colour of its own); the `NP` solids (plates with no pole: `XS_SPEED45NP_1B_00` is 1.66 m tall); the lane signals `XS_TRAFFICSIGNALARROW_JK`
and `XS_TRAFFICSIGNALCROSS_JK` (12 and 10 baked copies in the world, with a two-frame animated texture); the Rockport shop, billboard and
highway signs (names and texts of another city).

### The traffic lights

| Solid | Box (x, y, z) | Vertices | What it is |
|---|---|---|---|
| `XO_TRAFFICLIGHTA_1B/1C/1Z_00`, `TRAFFICLIGHTA_FRAG01` | 0.49 x 7.22 x 8.72 (y from -6.98) | 171, 128, 83; 137 | a pole 8.7 m tall with an arm of 7 m towards **-y**: the pole and the arm only, no signal, no marker (104 placements) |
| `XO_TRAFFICLIGHTB_1B/1C/1Z_00`, `TRAFFICLIGHTB_FRAG01` | 0.49 x 3.77 x 8.72 (y from -3.53) | 164, 128, 79; 131 | the same with an arm of 3.5 m (99 placements) |
| `XO_TRAFFICLIGHTC_1B/1C/1Z_00` | x -0.24 to 0.54, y -0.64 to 0.63, z -2.40 to 0.10 | 116, 76, 16 | a **signal head** 2.5 m tall, hung from its origin (z 0 is the top); **one position marker** (53 placements) |
| `XO_TRAFFICLIGHTD_1B/1C/1Z_00` | x -0.17 to 0.38, y -0.44 to 0.45, z -1.75 to 0.03 | 116, 76, 16 | the small head, 1.8 m (287 placements, the commonest) |
| `XO_TRAFFICLIGHTCABLEA/B_1B_00` | 10 m and 24 m long lines | 14 | the cables the heads hang from (kept) |

The solids' **flags are 0**, their groups effect 0 `0x4000` with one picture (the head's is `0x81792362`), so **a head is a flat slab with the
picture of three lenses on it**, not a model. The heads are **placed separately** from the poles: in the junction of tile `A47` a
`XO_TRAFFICLIGHTD_1B_00` stands 6.47 m along the arm of an `XO_TRAFFICLIGHTA_1B_00` (its local -y) and 7.59 m above its base, so its top
is at the arm. Each head carries **one position marker of parameter 4** (a red blinking light, [environment-prop-lights.md](../specs/environment-prop-lights.md)):
on `C` at (0.22, 0.00, -0.73) from the origin and on `D` at (0.16, 0.01, -0.59) (the marker's own translation minus the solid's pivot).
The head faces **+x**, as the host assumes.

### What the host does with them (the `traffic_lights` option)

The 2005 game has no signal system ([ai-traffic-world.md](../specs/ai-traffic-world.md)); the rewrite's extension draws a **lit box and a glow at
a lamp head point that the host computes from the solid's name** (`scenes/world/ai/signal_props.rs`), not from the solid's own surfaces. The
points before this variant were guesses: 4.5 m above the origin for every `A`, `C` and `D`. A head hangs **below** its origin, so the lamp
was floating 4.5 m above it. They are now the head: `HEAD_C` at 1.15 m below the origin and `HEAD_D` at 0.86 m, 0.3 m in front, and the lit
lamp moves up for red and down for green by the pitch of the lenses (0.42 m and 0.35 m), amber at the point. The numbers are the middles
of the old heads (a lens every third of the slab) and are checked by the screenshots of the two games' heads side by side
([the variant's pictures](#what-was-looked-at)); the **marker's** own heights (`-0.73` and `-0.59`) agree with a red lens a pitch above the
middle for the 2005 heads **[verified]**. `signals force red|amber|green|off` shows every lit lamp in one colour (a debug aid for looking at a
light in each state; the cars follow the clock).

## What the variant does

| Step | Rule | Where |
|---|---|---|
| Models read | by ranges, with the global bundle and the shaders behind each model's own bundle | `signs.rs` |
| Axes | 2012 to 2005: `(x, y, z)` becomes `(z, x, y)` (a cyclic permutation, winding kept) | `signs/geometry.rs` |
| A sign | turned about the vertical so the picture looks towards -y; scaled evenly to the old height (width at most 1.15 times the old plate); centred on the old box across and through; its lowest point on the old lowest point; squeezed on any axis where it would leave the room the game allows | `geometry::{face_minus_y, stand_in, squeeze_into}` |
| An arm | the 2012 mast arm with its heads cut away (surfaces higher than 1.5 m and no longer than 1.1 m are signal equipment), **stretched to the old box**: 7 m or 3.5 m of arm, 8.7 m tall; the coarser levels of detail of the 2005 pole take the coarser levels of the 2012 model | `geometry::{cut_heads, fill}` |
| A head | the 2012 head hung from its origin with the **middle lens at the host's lamp point** (`HANG_C`, `HANG_D` in `signs/table.rs`) and **the lens pitch of the host**: scaled by `pitch / 0.272`, its back at x -0.12 (C) or -0.10 (D), facing +x; a hanger rod up to the arm (the head's top is 0.3 to 0.5 m below the origin); **one red blinking marker** on the front of the top lens, like the 2005 head's | `geometry::{hang, rod}`, `build.rs` |
| Fit | every model is authored in the frame of the solid it replaces, so the sets say `fit = "exact"` (the game uses it as it is when its box is inside the old box grown by half of `spread - 1` and 5 cm; the converter squeezes it to fit, so it is always taken) | manifests |
| Texture | each 2012 picture is cut to 512 pixels at most and stored as DXT1 (BC1 passes through), named `MW12_S<id>`, with the **repeat** flag (the pole strip) or, for a cut-out picture, punch-through alpha from both sides | `build.rs`, `textures.rs` |
| Normals and colour | smooth normals from the triangles, grey vertex colour 66 (a world value) | `trees/light.rs` |

The marker of a replaced head is the model's, placed like its vertices (the solid keeps the pivot of the one it replaces, so the host adds
the pivot to a marker it keeps, the way the files store it); a model with no markers keeps those of the solid it replaces
(`replace/fit.rs`, `markers_of`).

## What the conversion costs **[verified]**

| What | Figure |
|---|---|
| The install's bundles read | by ranges: the table of each bundle that holds a model (`TRK_UNIT6`, `103`, `110`, `182`, `DISTRICT_1024094`, `GLOBALRESOURCES`) and the blocks of the models and their pictures |
| Time | about 0.3 s on the install (warm cache) |
| Written | `SIGNS/MW2012`: `GEOMETRY.BIN` 61 KiB, `TEXTURES.BIN` 321 KiB (pictures cut to 512 pixels; 811 KiB at the 1,024 limit); `LIGHTS/MW2012`: `GEOMETRY.BIN` 220 KiB, `TEXTURES.BIN` 41 KiB; two manifests of about 1 KiB: **about 645 KiB** |
| Read from the install | the log line `signs: read ... KiB of ... KiB of the bundles, ... ms` has the figures of an install (the reads are the tables of six bundles and the blocks of 8 models and their pictures) |
| In the game | `view-world` offscreen at a junction, 12 s, vsync off, `--bench-seconds`: **p50 3.02 ms without the variants, 2.61 ms with them** (p95 41 and 12 ms, the machine was shared with other builds, so the tails are noise); resident set 555.5 MiB without, 568.0 MiB with (**+12 MiB**: the models and 21 pictures in the shared sets). The replaced solids have more triangles than the old ones (an arm about 1,000 against 171 for the old pole), so the cost is the extra geometry of 104 + 99 arms and 340 heads in view range, which the bench does not tell from noise |

## What was looked at

Before and after pictures were taken offscreen, one run for each, in an isolated profile (`BLACKBOX_PROFILE_DIR`) whose config sets
`[mod_values.nfsmw] option_road_signs_variant` and `option_traffic_lights_variant` to `original` or `mw2012` and `traffic_lights = true`:

```
nfsmw view-world --at X,Y --height 2 --heading DEG --pitch P --muted --wait-for-load --screenshot out.png --screenshot-delay 3     [--exec "signals force red"]
```

The camera is put in front of a placement of the tables above (the position and axes of the scenery instance give the heading). 17 places:
the stop signs A and B (4), speed 45 and 55 (4), do not enter (2), the two no-parking signs (2), the signal heads in each state (red, amber
and green at one junction, green and amber at two more) and the mast arms (2). In every pair the 2012 model stands where the 2005 one did,
on its pole's foot and at its height; the lit lamp is on the red lens at the top, the amber one in the middle and the green one at the
bottom. The shared library is loaded at the start of a world, so a choice made inside a drive shows at the next drive.

## Open questions

- The lens positions of the 2005 heads are taken to be evenly spaced about the middle of the slab. The marker heights agree; the picture's
  own lens rows were looked at on screenshots only.
- The 2012 placement records (`PropInstanceList`'s second model, the broken form of a prop, `WorldObject`'s state) were not decoded: the
  bent signs and the `Skin` twins of the arms are not used.
- The lane signals (`XS_TRAFFICSIGNALARROW_JK`, `XS_TRAFFICSIGNALCROSS_JK`) have a 2012 counterpart (a lit arrow strip over the road,
  `0xD33A27C6`, 33 placements, an illuminance-lit mesh) that was not carried: the 2005 sign switches between two pictures of one texture
  animation, and a mesh of one picture cannot.
- Whether the 2012 `Sign_RetroReflective` faces (a specular texture that answers the headlights) have a 2005 equivalent: the 2005 world has
  no such shader.
