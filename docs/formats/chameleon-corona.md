# Chameleon coronas: the flares, glows and flashing sirens of a lamp

How the 2012 game draws a lamp's light: the `Corona` objects a lamp record names, their flare, glow and beam layers, the curves that
size and dim them by angle and distance, the pictures, and the flash patterns of a police car's siren lamps. The lamp records and
the locator groups are in [chameleon-lamps.md](chameleon-lamps.md); the police cars in [chameleon-police.md](chameleon-police.md).
Reader: [`libs/chameleon-effects/src/corona/`](../../libs/chameleon-effects/src/corona) (objects, curves, flash patterns) and
`chameleon-vehicle::read_corona_records` (the lamp records with their references). Used by
[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content) (`src/lamp_coronas.rs`), which turns it into the sprites of
[a mod car's lamps](../modding/car-lamps.md) and the light bar of [a mod cop car](../modding/cop-cars.md#the-light-bar).

Tags: **[verified]** is read from the 2012 install (v1.4.0.0) on all 93 `VEH_<n>_HI.BNDL` and the global dictionaries;
**[exe]** is from `NFS13.exe` (Ghidra; function addresses are given, no code is reproduced); **[infer]** is a reading of the data
or of the code that the executable was not traced further for; **[unconfirmed]** may be wrong.

## Where the data is [verified]

| What | Where |
|---|---|
| The types `Genesys.Gen.Corona`, `.Flare`, `.Glow`, `.Beam`, `Genesys.Gen.VfxGlobals.VehicleCoronaScalesForNight` | `GLOBALEFFECTS.BNDL`, as `GenesysType` resources (kind `0x14`) only. **The file holds no corona object.** |
| The `Corona` objects (kind `0x15`) | each car's own bundle: 1047 lamp records of the 93 cars name 998 distinct objects, 8 to 13 a car; the ids (`0x2519A`, `0x251BB`, ...) repeat across cars, the bytes are the car's |
| The lamp records | the car's `VehicleVfxBehaviour` objects: the ordinary one and, on a police car, `<Car>_Cop_Coronas` ([police](chameleon-police.md#the-second-corona-object-verified)) |
| The flash patterns and their sequences | the car's bundle (63 records of the five police cars name one) |
| The pictures | a layer's reference is a material of `GLOBALMATERIALDICTIONARY.BNDL` (kind `0x2`); the material's first texture is in `GLOBALTEXTUREDICTIONARY.BNDL` |

## The lamp record [verified]

A `Genesys.Gen.VehicleVfxBehaviour.Corona` is 0x50 bytes ([chameleon-lamps.md](chameleon-lamps.md#the-vehiclevfxbehaviour-object-verified)
has the first fields). With the references of the object's import table:

| Offset | Type | Field |
|---|---|---|
| +0x10 | f32 x4 | colour factor (the record's tint) |
| +0x20 | reference | the **flash pattern** object of a siren; none on an ordinary lamp |
| +0x28 | reference | the `Corona` object |
| +0x30 | u32 | the locator group id |
| +0x34 | f32 | intensity factor (1; 0.7 on a few tail lamps) |
| +0x38, +0x3C | f32 | size factors (1 to 2.4) |
| +0x40 | u16 | the light-state bit; `state_word & (1 << bit)` lights the record **[exe]** (`FUN_004A6630`) |
| +0x42 | u16 | mode (0, 1, 2) |
| +0x44 | u8 | 1 |

State bits over the 1047 records: 0 (492), 1 (239), 7 (126), 2 (90), 3 (63), 6 (37). The 63 records of bit 3 are the siren
lamps: the same 63 records have a flash pattern (19 on the Crown Victoria, 20 on the Explorer, 8 on each of the Charger, the Corvette
and the SWAT van), and the converter reads a bit-3 record with a pattern as a siren.

## The `Corona` object [verified layout, [exe] for the use]

The main block (`Genesys.Gen.Corona`, 0xE0 bytes, the offsets count the 12-byte head):

| Offset | Field |
|---|---|
| +0x10 | the object's **curve block** (0x90 bytes, next section) |
| +0xA0 | the object's own id (u32) |
| +0xA4 | f32: a corona farther than this from the camera is not drawn (99,999 in nearly all) **[exe]** (`FUN_004A6490`) |
| +0xA8 | f32: a lamp farther than this is skipped before it is drawn (100, 500 or 1,000) **[exe]** (`FUN_004A5F70`) |
| +0xAC | f32: -0.5 in all **[infer]** a bias |
| +0xB0, +0xB4, +0xB8, +0xBC, +0xC0, +0xC4 | six offsets of **layer lists** (0 for none): the object's layers are inline, 0xE0 bytes each |
| +0xC8 | u16 (no use found) |
| +0xCA, +0xCC, +0xCE, +0xD0, +0xD2, +0xD4 | the layer count of each list |

The type resource names the slots `Beam`, `Flare`, `Glow` x4, and the counts as seven u16 fields from +0xC8; the code reads the
counts from +0xCA on, in this pairing **[exe]** (`FUN_0049D950`), which the data bears out (an object with a glow at +0xBC has a
1 at +0xD0):

| List (offset, count) | Is | Drawn by view |
|---|---|---|
| +0xB0, +0xCA | beams (a strip along the lamp's axis) | 0 |
| +0xB4, +0xCE | flares (a ghost sprite along the line through the screen centre) | 0 |
| +0xBC, +0xD0 | glows (a camera-facing sprite at the lamp) | 0 |
| +0xB8, +0xCC | beams drawn the way the second kind of view does | 1 |
| +0xC0, +0xD2 | glows | 2 |
| +0xC4, +0xD4 | glows | 3 |

The view number is chosen by the caller (`FUN_004A73B0`): 0 for the normal view, 1 for the views numbered 3 to 8, 2 and 3 for
the views 2 and 1 **[exe]**; which of them are mirrors, cube maps or shadow passes was not traced **[unconfirmed]**. The first
list of each kind (view 0) is the one the player's camera draws and the only one the converter reads. Over the 93 cars, the
view 0 lists hold 93 beams, 1538 flares and 719 glows; the +0xB8 list is always empty, the +0xC0 list has 635 layers and the
+0xC4 list 88.

### A layer [verified layout]

A layer (`Genesys.Gen.Corona.Beam`, `.Flare`, `.Glow`: the three types have the same layout and size, 0xE0):

| Offset | Field |
|---|---|
| +0x10 | the layer's **curve block** |
| +0xA0 | f32 x4: the colour multiplier (red, green, blue, alpha) |
| +0xB0 | f32 x4: a beam's start and end along the axis in metres and two more numbers (`(-0.4, 5, 1, 0)` on the Porsche's headlamp beam); a flare's ghost offsets (`(1, 1, 1, 0)` to `(-2.16, -2.16, 1, 1)`); a glow's `-0.25` to `-1`, no use found |
| +0xC0 | reference: the picture's **material** |
| +0xC8 | f32: copied into the sprite as its fourth coordinate (`-1`, `-0.25`, `1.145`, `-2.162`): a depth bias **[infer]** |
| +0xCC | f32: a start angle added to (flare) or taken from (glow) the camera's roll **[exe]** (`FUN_0049D4F0`, `FUN_0049D710`); 0 in nearly all, `-99.8` on one flare |
| +0xD0 | u16: where the colour comes from: 0 the lamp's first colour, 1 its second, anything else white **[exe]**; 0 on glows, 1 on flares |

## The curve block [exe]

Each corona and each layer has a 0x90 byte block whose first 16 bytes are the object head and whose rest is **eight rows of four
floats**, evaluated four lanes at a time (`FUN_004AB450`, the SSE routine). Naming the rows by their offset in the block (+0x10 to
+0x80) the evaluator computes, for each lane `i` and an input `x`:

```
below = x < threshold[i]                       (row +0x50)
slope = below ? row +0x30 : row +0x40
icept = below ? row +0x10 : row +0x20
u     = min(slope * x + icept, 1)
gain  = below ? row +0x60 : row +0x80
out   = (u * gain + row +0x70) * scale[i]      (scale: 1, set by the caller)
```

A lane is a two-piece ramp that is clamped to 1 from above and has no lower clamp. The data uses it as "go from `a` to `b`
between two thresholds", often with huge slopes (±10,000) to make a step. Example: the Porsche's brake lamp, lane 2 of the
corona: `x < 100`: `u = 1 - 0.01 x`, `out = 0.5 - 0.425 u`; above: `u = 0.0025 x - 0.25`, `out = 0.5 + 0.5 u`: a brightness that is
0.075 at the lamp, 0.5 at 100 m and 1 from 500 m **[verified]** (`corona/tests.rs` has the lane arithmetic).

### What the lanes are [exe] for the chain, [infer] for the words

`FUN_004A5790` (the routine that queues one lamp's layers) does this per lamp:

1. The corona's curve block runs on `(a, d, d, d)`, `a` the angle between the lamp's axis and the direction to the eye (radians,
   from an arc cosine of their dot product) and `d` the distance in metres.
   - lane 0 is **how visible the lamp is at that angle**: the brake lamp above is 1 on the axis, 0.8 at 45 degrees, 0.53 at 90
     degrees and 0 behind. A lamp whose lane 0 is 0 on the axis (the Porsche's headlamp object `0x251BB`, which holds only a beam)
     is seen from the side;
   - lane 1 is **a size factor by distance** (4.0 at the lamp falling to 1.0 at 150 m on the brake lamp);
   - lane 2 is **a brightness by distance**.
2. The lamp's intensity is `i = lane2 * lane0` and its size vector the record's size factors times lane 1.
3. Each layer's curve block runs on `(i, i, i, d)`: lanes 0 and 1 are the sprite's width and height, lane 2 the colour
   intensity, lane 3 a distance factor that scales a **glow's** size (`FUN_004A4CF0`; a flare's and a beam's size ignores it,
   `FUN_004A52E0`, `FUN_004A5040`). A glow's lane 3 is 0 within about 5 m of the lamp and grows to full at 150 m, so a glow
   stays about the same size on the screen.
4. The sprite's colour is the layer's colour times the record's tint times lane 2 times the intensity factors; its alpha is the
   layer's alpha times the tint's.

The evaluation is `Corona::seen` in the reader. Its sizes are in the executable's units, which were not traced to the screen
(metres is the reading **[unconfirmed]**), and the absolute brightness is **not** known: the executable multiplies in a per-vehicle
scale and a flash value, and the colours come out of the curves as 0.02 to 1. The converter therefore carries the brightness
as a relative number.

## The pictures [verified]

13 distinct pictures serve every layer of the 93 cars: `Bc1` 128x128 (2), 128x64 (1), 256x128 (4), 256x256 (2), and uncompressed
`Rgba8` 128x128 (2) and 64x64 (2). All are bright shapes on black (a radial glow, a ring, a horizontal streak): intensity in the colour
channels, drawn additively; the alpha of the `Rgba8` ones was not read as a mask. The glow `0x6D0F` is a soft round bloom with a
cross, `0x6D11` a blue-white round glow.

## Flash patterns [verified]

A siren record's +0x20 reference is a pattern object (type id `0x2805`):

| Offset | Field |
|---|---|
| +0x10 | u32: the offset of the first segment (0x18) |
| +0x14 | u16: the number of segments (1 or 2 in the data) |
| per segment, 0x24 bytes from the first | +0x0C a reference to a sequence object; +0x18 f32 weight, +0x1C f32 rate, +0x20 f32 offset |

A sequence object (type `0x2808`) holds at +0x10 the offset of its steps (0x18) and at +0x14 the step count (u16, 9 to 20), then one
f32 per step, 0 or 1 in all the data.

The evaluator (`FUN_0049EF00`) adds the weights, takes `frac(t / sum) * sum` to pick the segment whose cumulative weight holds it, and
steps that segment's sequence by `rate * t + offset` turns **[exe]**. Where the executable rounds to a step (floor or nearest) was not
read out of the decompilation, and `t` is the lamp's clock times a per-vehicle factor: the reader takes `t` in seconds and floors
the step **[unconfirmed]**. Read that way, one step lasts `1 / (rate * steps)` seconds, which is **0.125 s for every segment of
the long patterns** (rate 0.4 over 20 steps, 0.5 over 16): that agreement is the evidence for the reading.

The five police vehicles use two families, the same data in each bundle:

| Family | Segments | Steps | Reads as |
|---|---|---|---|
| long, one per lamp | (8, 0.4, 20 steps) and (8, 0.5, 16 steps), offset 0 to 0.125 | 0.125 s | three flashes in a row, a pause, three more, then a seven-step gap and single flashes: the lamp of the three red and the three blue lamps; the lamps differ in the starting offset, so the flashes run along the bar |
| short, two lamps | (1, 3.3, 9 steps) | 0.034 s | `.#..#.#.` and its complement `#..#.#..`: a rapid alternating strobe of the two small lamps |

The cycle of a long pattern is 16 time units, but the segments turn their sequences a non-whole number of times in it
(0.4 x 8 = 3.2 turns), so the sound repeat is after **five cycles** (80 s). The converter bakes that.

### The siren lamps of the police vehicles [verified]

| Bundle | Lamps | Notes |
|---|---|---|
| `VEH_122706_HI` Charger SRT8 cop | 8 records of bit 3: 3 blue (`0x251AD`), 1 blue and 1 red small strobes (`0x251A8`, `0x251A9`), 3 red (`0x251B3`) | each lamp has its own locator group |
| `VEH_122713_HI` Crown Victoria | 19 records: the Charger's patterns on more lamps (a second siren object) | |
| `VEH_510072_HI` Explorer Interceptor | 20 records, the same patterns | |
| `VEH_122699_HI` Corvette Z06 | 8 records, the Charger's | |
| `VEH_1467242_HI` SWAT van | 8 records, the Charger's patterns, in the second of its two lamp objects | |

The colour of a lamp is the colour of its object's glow layer: blue `(0.21, 0.58, 1)`, red `(1, 0.005, 0)` or `(1, 0, 0)`; its flares carry
tints near white, blue-white and light red. This settles the question [the police page](chameleon-police.md) left open (which are red and which blue).

## What the converter carries over

`mods/nfsmw-2012-content/src/lamp_coronas.rs`, from each car's own records:

- the lamp kind from the state bit and the end of the car (front bits 0, 4, 6, 7: head; rear bit 1: brake; rear bits 0, 7: tail; rear
  bit 2: reverse; bit 3 with a flash pattern: siren; the fog lamps and the second switch of an ordinary car are left out);
- the place from the locator group (each locator and its mirror image for a lamp, the locators themselves for a siren of mode 0);
- a glow layer (at most two, a flare when there is no glow) of the view-0 lists as the sprite, with the layer's picture converted to the
  car's own texture (DXT1 for the plain ones), its hue from the layer's colour, and its radius and brightness evaluated at 8 m and
  100 m (the brightness relative: the brighter of the two is 1);
- the angle falloff as a cosine power fitted to lane 0 from the axis to 60 degrees; a lamp invisible on its axis (a beam) has no flare;
- a siren's pattern baked at its shortest step over as many cycles as it takes to repeat (at most 16, at most 4096 steps), one digit
  0 to 9 a step.

Not carried: the beam layers, the flare ghost chain (only one flare is used, and only when there is no glow), the view 1 to 3
lists, the night scales (`VehicleCoronaScalesForNight`, a table of six factors the executable applies to the modes 1, 4, 6 and 7 of a
record), the global corona that the record whose object is `0x251BD` also queues, the bloom, the road lights.

## Not decoded

- the units of a layer's sizes on the screen and the absolute brightness (the per-vehicle factor in the queue entry, the HDR scale);
- the views other than 0 and what the +0xC8 u16 of the object is;
- the rounding of the pattern step and the time scale of the pattern's clock;
- the meaning of a record's mode and the +0x44 byte, the second colour (`+0xD0 = 1`) beyond "the lamp's second colour";
- how a beam is meant to be drawn in view 0 (a strip from the start to the end along the axis, whose width the layer's curves give).

## Tests

`cargo test -p chameleon-effects corona` runs the curve arithmetic, the object and pattern readers and the evaluation chain on
synthetic bytes. `CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-vehicle corona_records -- --ignored` reads the 1047
records of the 93 cars and checks each names a corona object of its bundle and that 63 have a flash pattern.
`MW2012_GAME_DIR=<2012 install> cargo test -p nfsmw-2012-content lamp_coronas -- --ignored` converts the Charger (eight siren
lamps) and the Porsche. `MW2012_GAME_DIR=<2012 install> cargo run -p chameleon-effects --example corona -- coronas
VEHICLES/VEH_122706_HI.BNDL Cop_Coronas` prints a car's records with their layers, evaluated sizes and flash patterns; `survey`
prints the counts above and `picture <material id> <folder>` writes a layer's picture as a PNG.
