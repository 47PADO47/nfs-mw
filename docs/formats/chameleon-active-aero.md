# Chameleon active aero: the spoiler that rises with speed

What the car bundles and `NFS13.exe` (v1.4.0.0) of NFS: Most Wanted (2012) say about the spoilers that go up and down with
the car's speed (the Porsche 918 Spyder's wing, the 911's spoiler), and how this repo reads it
([`libs/chameleon-vehicle/src/active_aero.rs`](../../libs/chameleon-vehicle/src/active_aero.rs)). The skeleton, bone names
and skin that say which triangles move are in [chameleon-hinged-parts.md](chameleon-hinged-parts.md); the containers in
[chameleon-bundle.md](chameleon-bundle.md). Evidence tags: [README](../README.md#evidence-tags); **[exe]** is read from the
executable's code or tables (a Ghidra session, nothing of it is in the repository). Addresses are of `NFS13.exe` v1.4.0.0.

**Rule of this document:** a number the game plays is a number the data or the executable states. Where one could not be
decoded, the part stays still and it is listed under [What is not decoded](#what-is-not-decoded) with the reason.

## Short answer

- A car with moving aero has an **`HSMData`** resource (type 0x501): a state machine named `<car>_SpoilerAnimation` (or
  `_AirVentsAnimation`, `_AileronAnimations`). Its conditions test **inputs** of the car (`speedinmph`, `gas`, `brake`,
  `steering`, `coplights`) against numbers; its states play an **animation** (type 0xB3) of the skeleton **[verified]**:
  18 of the 93 bundles have one.
- The executable's state behaviour opens the animation when the input is **above** the open number and closes it when the input
  is **below** the close number, playing the clock **forwards or backwards at real time** (linear, no easing, no speed-up)
  **[exe]**.
- 8 bundles of 7 models are exactly that, one animation and two conditions on `speedinmph` (`>` and `<`), and are decoded
  completely: **Porsche 911 Carrera S 2012** (two bundles), **Lexus LFA**, **Mercedes-Benz SL65 AMG Black Series** (two),
  **Mercedes-Benz SLS AMG**, **Porsche 918 Spyder 2013**, **Porsche Panamera Turbo S**. Their speeds, time, slide and turn
  are read from the data (table below).
- The others are **not driven** (their part stays at rest): 918 Spyder Concept Study, Aventador LP700-4, Agera R, Veyron
  Super Sport and Grand Sport Vitesse, McLaren MP4-12C, Venom GT Spyder, Huayra, Marussia B2, Aventador J. The reasons are
  exact and in the last section.

## The input names and the state behaviour [exe]

- The machine's inputs are bound to the car by a routine at 0x005a7400 <!-- leak-check: allow -->. It hashes the input name
  (CRC-32, bits read most significant first, table at 0x010e5a10 <!-- leak-check: allow -->) and compares it with the hashes
  of `brake`, `gas`, `steering`, `speedinmph` and `coplights`; each gives a float of the vehicle object (the speed one is a
  float of the vehicle's speed record). The names are stored at 0x00fae7d8 to 0x00fae800 <!-- leak-check: allow -->.
- The **animation state** behaviour at 0x00489050 <!-- leak-check: allow --> reads its input every frame. It has four states
  (0 at rest, 1 playing forwards, 2 at the end, 3 playing backwards): above the open threshold it moves to playing forwards
  until the clock reaches the animation's end, below the close threshold it moves to playing backwards until the clock reaches
  the first key. The clock moves by the frame time in seconds, either way; nothing eases it. Between the two numbers it keeps
  its direction.
- The speed input is the car's speed in miles per hour as the vehicle object holds it **[unverified: the conversion from the
  physics unit was not measured independently; the host uses 2.236936 mph per m/s]**.

## The state machine's conditions

The resource is a Genesys-style object graph (type descriptions are in the bundle). A condition is an `HSM.AnimationCondition`
object whose properties sit in a **property table**: rows of 16 bytes, a 64-bit key (low byte 0 or 0x80), the offset of a value
cell, a count and a flag word of 0 or 1 **[verified]**. The cell holds the offset of a NUL-terminated string. Three keys
matter **[verified]**:

| Key | Holds |
|---|---|
| `0xbcdaa6b634a9aa00` | the number the input is compared with, as text (`70`) |
| `0x9952fc5819000000` | the input's name (`speedinmph`) |
| `0x5b55835f4e5a7080` | the operator code, as text |

Operator `2` is **greater than** and `3` is **less than**: every speed condition of the install is labelled `Speed above N`
or `Speed under N` (and the bigger/smaller layout matches) **[verified]**. The codes `0` and `1` (on the Agera's `gas`
conditions) are **[unconfirmed]**: the evaluator that reads the operator was not found in the executable.

A machine is read as a plain speed spoiler when it holds exactly two conditions, both on `speedinmph`, one `>` (the open
speed) and one `<` (the close speed), and one animation. The animation is the resource whose id ends in the decimal
number the machine's strings hold (904078 on the 911) **[verified]**.

| Car | Opens above | Closes below | Takes | Slide (up, back) | Turn |
|---|---|---|---|---|---|
| Porsche 911 Carrera S 2012 (2 bundles) | 70 mph | 40 | 0.612 s | 2.0 cm up, 7.0 cm back | 30.46 deg, lateral axis |
| Lexus LFA | 70 | 50 | 0.508 s | 11.9 cm up, 13.5 cm back | 18.30 deg |
| Mercedes-Benz SL65 AMG Black Series (2) | 75 | 75 | 0.612 s | 9.2 cm up, 6.2 cm back | none |
| Mercedes-Benz SLS AMG | 74 | 64 | 0.769 s | 0.2 cm down, 9.9 cm back | 26.21 deg |
| Porsche 918 Spyder 2013 | 100 | 80 | 0.612 s | 9.5 cm up, 7.5 cm back | none |
| Porsche Panamera Turbo S | 70 | 50 | 3.000 s | 5.1 cm up, 5.3 cm back | 15.25 deg |

The SL65 opens above 75 and closes below 75: at exactly 75 it keeps its direction. The Panamera's animation has nine keys on
which the spoiler moves in nine equal steps (checked: slide and turn grow linearly key by key), so a linear clock over the
3 s is its motion; the key times themselves are not read (uniform spacing assumed **[unconfirmed]**).

## The animation (type 0xB3)

All little endian. Layout as the sampler of the executable reads it (the pose routines at 0x009f3250 and 0x009f9d60
<!-- leak-check: allow -->) **[exe]**, and checked on the 8 cars **[verified]**:

| Offset | Type | Field |
|---|---|---|
| 0x04 | u32 | number of keys (2 on the plain spoilers, 9 on the Panamera) |
| 0x08 | u32 | number of bones of the skeleton |
| 0x1C | u32 | stride of one key in the rotation table, bytes |
| 0x20 | u32 | stride of one key in the translation table, bytes |
| 0x30 | f32 | **length in seconds** |
| 0x34 | u16 | format; 1 packs a rotation into 32 bits |
| 0x36 | u16 | the reference key (0) |
| 0x38 | u32 x n | offsets of the tables; word 1 the translations, word 4 their bone map, word 5 the packed rotations, word 7 their bone map |

- A **bone map** has one byte per skeleton bone: the bone's track number, or 0xFF for none. The skeleton's `SpoilerHinge`
  (index from its name hash, see chameleon-hinged-parts.md) picks the tracks.
- A **translation** key is a row of 16 bytes (x, y, z as f32, a pad word) per track, keys stride apart.
- A **rotation** key is a packed quaternion of 32 bits per track: the top two bits say which component is left out, the
  three 10-bit fields at bits 20, 10 and 0 hold the other three as `(field - 511) * 0.00138` (the float 0x3ab53240), and the
  left-out one makes the quaternion unit **[exe]**.
- The sampler adds, to the bone's rest pose, the interpolation of two keys **minus the reference key**: the movement at the
  end of the animation is the last key minus key 0, the movement half way is half of it **[exe]**. The values are in the
  engine's space (x left, y up, z forward, metres) and in the frame of the bone's parent; on the 8 cars that frame is the
  car's own (the 911 turns exactly about the lateral axis) **[verified]**, the general case is **[unconfirmed]**.

The turn is about the bone's own position (the hinge pivot of the skeleton), then the slide. The reader gives a unit
quaternion and the translation; the converter writes the axis and angle and the slide in the car's space
(`rotation_axis`, `rotation_deg`, `offset` of [car-damage-parts.md](../modding/car-damage-parts.md)). The cyclic change of the
axes (engine x, y, z to the game's y, z, x) keeps the handedness, so the angle's sign carries over.

## What is not decoded

Each of these cars keeps its part still. No number is guessed for them.

- **918 Spyder Concept Study (2010) and Aventador LP700-4**: two stages (`Speed above 75mph` and `above 100mph`; `above 80mph`
  and `above 100mph`) with an animation each and waits (`Wait 3s`, `Wait 5s`) between states. The machine has more than two
  conditions and a graph of states and transitions; the objects that join a state to its condition and its animation, and
  the order of the stages, were not decoded (the parent and transition objects are not read). The earlier version took the
  largest constants as speeds; that was a guess and is removed.
- **Agera R**: the machine reads `gas` as well as the speed; the operator codes `0` and `1` of its conditions cannot be
  named (the evaluator was not found), and its animation moves child bones of the wing (flaps), not `SpoilerHinge`, which a
  per-bone part would need and the skin does not give.
- **Bugatti Veyron Super Sport and Grand Sport Vitesse, McLaren MP4-12C, Hennessey Venom GT Spyder**: brake and speed machines
  with several states (`Brake Anim`, `Top Speed Anim`, `Reverse ...`) and arms with their own bones. The state graph (which
  condition leads to which state, the priority) is not decoded; the wing's motion cannot be assigned to a single input without
  it.
- **Pagani Huayra** (four flaps by speed, steering and brake), **Marussia B2** and **Lamborghini Aventador J** (air vents):
  one machine per flap or vent, several states, per-bone parts under the car's skeleton. Same reason: no state graph, and
  the vents and flaps are not skin parts of a single hinge.
- **Panamera's side flaps** (two more tracks of the same animation, 14 cm to each side): the tracks are read but the flaps
  are not a hinged part (no solid of their own is cut), so they do not move.
- **Speed unit** of `speedinmph` and the uniform key spacing of the Panamera are as marked above, **[unconfirmed]**.
- Whether the physical downforce changes with the spoiler: nothing in the machine says so. The repo leaves it unchanged.
