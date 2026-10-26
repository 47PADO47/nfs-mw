# Chameleon active aero: the spoilers and wings that move with the car

What the car bundles and `NFS13.exe` (v1.4.0.0) of NFS: Most Wanted (2012) say about the spoilers that go up and down with
the car's speed (the Porsche 918 Spyder's wing, the 911's spoiler), and how this repo reads it
([`libs/chameleon-vehicle/src/active_aero.rs`](../../libs/chameleon-vehicle/src/active_aero.rs) and
[`hsm.rs`](../../libs/chameleon-vehicle/src/hsm.rs)). The skeleton, bone names and skin that say which triangles move are in
[chameleon-hinged-parts.md](chameleon-hinged-parts.md); the containers in [chameleon-bundle.md](chameleon-bundle.md).
Evidence tags: [README](../README.md#evidence-tags); **[exe]** is read from the executable's code or tables (a Ghidra
session, nothing of it is in the repository). Addresses are of `NFS13.exe` v1.4.0.0.

**Rule of this document:** a number the game plays is a number the data or the executable states. Where one could not be
decoded, the part stays still and it is listed under [What is not decoded](#what-is-not-decoded) with the reason.

## Short answer

- A car with moving aero has an **`HSMData`** resource (type 0x501), a state machine named `<car>_SpoilerAnimation` (or
  `_AirVentsAnimation`, `_AileronAnimations`): conditions on inputs of the car (`gas`, `brake`, `steering`, `speedinmph`,
  `coplights`) and **animation states** that play an animation (type 0xB3) of the skeleton **[verified]**: 18 of the 93 bundles.
- **10 bundles of 8 models** have a spoiler state that is decoded completely: Porsche 911 Carrera S 2012 (two bundles), Lexus
  LFA, Porsche 918 Spyder Concept Study, Mercedes-Benz SL65 AMG Black Series (two), Mercedes-Benz SLS AMG, Porsche 918 Spyder
  2013, Lamborghini Aventador LP700-4, Porsche Panamera Turbo S (its spoiler and its two side flaps).
- The others are **not driven** (the part stays at rest): the air vents of the 918 Concept, the LP700-4, the Marussia B2 and
  the Aventador J; the Agera R; the Veyron Super Sport and Grand Sport Vitesse; the McLaren MP4-12C; the Venom GT Spyder; the
  Huayra. The behaviour of the Veyron's wing is decoded from the executable but not implemented; for the rest, the exact
  reason is below.

## The input names and the speed [exe]

- The inputs are bound to the car by the routine at 0x005a7400 <!-- leak-check: allow -->. It compares the hash of an input's
  name (CRC-32, bits read most significant first) with the hashes the static initialisers make of the strings at
  0x00fae7dc, 0x00fae7e0, 0x00fae7e8, 0x00fae7f4 and 0x00fae800 <!-- leak-check: allow -->. Result:
  **`gas` is the float at `+0x192c` of the vehicle object, `brake` is `+0x1930`, `steering` is `+0x193c`**,
  `speedinmph` comes from a routine, `coplights` is 1 when the word at `+0x1954` is 2.
- **The speed is `|velocity| x 2.236936`**: the update routine of the animation behaviour (0x005ad7d0 <!-- leak-check: allow -->)
  takes the length of the vehicle's velocity vector (`+0x120`, `+0x124`, `+0x128`, metres per second) and multiplies it by a
  vector constant that the start-up code (0x00f79260 <!-- leak-check: allow -->) makes from `1 / 0.44704`, which is
  2.236936 miles per hour per metre per second. So `speedinmph` is the plain speed in mph; the thresholds of the data are
  compared with it as they are.

## The animation state behaviour [exe]

The behaviour is built from a native object whose floats are read from the data: **`+0x24` the open number, `+0x28` the
close number, a short at `+0x2c` the kind** (0 or 1); the constructor is at 0x005adad0 <!-- leak-check: allow -->, the update
at 0x005ad7d0 <!-- leak-check: allow -->. No operator is left in the native object: the data's `>` and `<` become "open
number" and "close number".

**Kind 0** (every spoiler read here), update at 0x00489050 <!-- leak-check: allow -->: four states (0 at rest, 1 playing
forwards, 2 held at the end, 3 playing backwards). At rest it goes to 1 when the speed is **above** the open number; while
playing forwards it goes to 2 when the clock reaches the animation's length, or to 3 when the speed is **below** the close
number; held it goes to 3 below the close number; playing backwards it goes to 0 when the clock reaches 0. The clock moves by
the frame time, so the whole way takes the animation's length, either way, at a constant rate; nothing eases it. Between the
two numbers the state keeps its direction. A state change posts an event, which is for sounds.

**Kind 1** (the Veyron's wing machine), update at 0x00487a00 <!-- leak-check: allow -->: 14 states over five animations, each
with its own clock (slot 0 to 4). Decoded, not implemented:

| Constant | Value | Source |
|---|---|---|
| speed to open the wing | 140 mph | the vector constant made at 0x00f4b700 from 0x00fa14b0 <!-- leak-check: allow --> |
| speed of the brake wing | 125 mph | 0x00f4b720 from 0x00fa14d0 <!-- leak-check: allow --> |
| top speed wing | 200 mph | 0x00f4b710 from 0x00fa14c0 <!-- leak-check: allow --> |
| brake held before it counts | 0.4 s | `brake` (`+0x1930`) is zeroed until it has been held that long |
| cross-fade between two animations | 0.25 s | weights `1 - 4t` and `4t` |

The states: 0 rest; above 140 mph to 1 (slot 0 forwards) and 2 (held); above 200 mph from 2 to 6 (slot 3 forwards), 7 (held),
8 (back); the brake above 125 mph at rest to 4 (slot 1 forwards) and 5 (back); the brake while the wing is out to 9 and 10
(slot 2 cross-faded with slot 0), from the top-speed wing to 11 and 12 (slot 4); state 13 plays slot 0 at a rate taken from two
other vehicle floats (`+0x1a98`, `+0x1a9c`, meaning not decoded). The five slots are the five animations of the Veyron's
bundle in the order of the state's animation list (3.0 s, 1.58 s, 3.0 s, 1.5 s, 1.5 s). The poses of the slots are added with
the weights by the sampler at 0x009f22a0 <!-- leak-check: allow -->.

## The state machine's data

A block of objects. An **object record** is eight words: type hash, id, three offsets (properties for kinds, the **child
list**, the **property table**), a zero, the offset of the name and a flags word **[verified]**. Type hashes: machine
`24c422b2`, animation state `849c8ce8`, condition `0ec6f1b1`, timed transition `809acb7d`. A **child list** has 16 bytes per
child (id, table, count, name). A **property table** has rows of 16 bytes: a 64-bit key (low six bits 0), the offset of a
value cell, a type (6 for a nested object, otherwise a text) and a flag. A text cell holds the offset of a NUL-terminated
string; an object cell holds a table offset and a row count.

- An **animation state** holds its animation (key `4f883ba0b3049400`, a decimal number: the animation is the resource whose id
  ends in it), and a nested object under `684563f7fd6c4600`: the **close condition**.
- A **condition** record holds a nested object with three rows: the number (`bcdaa6b634a9aa00`), the input's name
  (`9952fc5819000000`) and the operator (`5b55835f4e5a7080`). Operator `2` is greater than and `3` is less than **[verified]**
  (every speed condition is labelled `Speed above N` or `Speed under N`); `0` (the Agera's `gas`) is **[unconfirmed]**.
- A machine lists its children: **the condition that opens a state is listed just before it** (the 911: `Speed above 70mph`,
  then `Spoiler Anim`), and the state's own object is the close condition **[verified on the 10 bundles; the exe's own loader
  that links them was not read]**. A state with **children of its own** (conditions and `Wait for Ns` timed transitions) is
  not read.

| Car | Opens above | Closes below | Takes | Slide (up, back) | Turn |
|---|---|---|---|---|---|
| Porsche 911 Carrera S 2012 (2 bundles) | 70 mph | 40 | 0.612 s | 2.0 cm up, 7.0 cm back | 30.46 deg, lateral axis |
| Lexus LFA | 70 | 50 | 0.508 s | 11.9 cm up, 13.5 cm back | 18.30 deg |
| Porsche 918 Spyder Concept Study | 100 | 80 | 0.612 s | 8.6 cm up, 7.9 cm back | none |
| Mercedes-Benz SL65 AMG Black Series (2) | 75 | 75 | 0.612 s | 9.2 cm up, 6.2 cm back | none |
| Mercedes-Benz SLS AMG | 74 | 64 | 0.769 s | 0.2 cm down, 9.9 cm back | 26.21 deg |
| Porsche 918 Spyder 2013 | 100 | 80 | 0.612 s | 9.5 cm up, 7.5 cm back | none |
| Lamborghini Aventador LP700-4 | 80 | 50 | 0.612 s | 0.5 cm up, 0.9 cm back | 12.54 deg |
| Porsche Panamera Turbo S | 70 | 50 | 3.000 s | 5.1 cm up, 5.3 cm back | 15.25 deg |

The SL65 opens above 75 and closes below 75: at exactly 75 it keeps its direction. The Panamera's animation has nine keys on
which the spoiler moves in nine equal steps (checked), so a linear clock over the 3 s is its motion; the key times themselves
are not read (uniform spacing assumed **[unconfirmed]**). The same animation moves two more bones that hang from the spoiler,
the side flaps (14 cm to each side); they are parts of their own that ride on the spoiler's movement.

## The animation (type 0xB3)

All little endian. Layout as the sampler of the executable reads it (0x009f3250 and 0x009f9d60 <!-- leak-check: allow -->)
**[exe]**, and checked on the 18 bundles **[verified]**:

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

- A **bone map** has one byte per skeleton bone: the bone's track number, or 0xFF for none.
- A **translation** key is a row of 16 bytes (x, y, z as f32, a pad word) per track.
- A **rotation** key is a packed quaternion of 32 bits per track: the top two bits say which component is left out, the
  three 10-bit fields at bits 20, 10 and 0 hold the other three as `(field - 511) * 0.00138`, and the left-out one makes the
  quaternion unit **[exe]**.
- The sampler adds, to the bone's rest pose, the interpolation of two keys **minus the reference key**: the movement at the
  end is the last key minus key 0 **[exe]**. The values are in the engine's space (x left, y up, z forward, metres) and in
  the frame of the bone's parent; on the 10 bundles that frame is the car's own (the 911 turns exactly about the lateral
  axis) **[verified]**, the general case is **[unconfirmed]**.

The turn is about the bone's own position, then the slide; a flap's movement is added to its parent's. The converter writes
`rotation_axis`, `rotation_deg`, `offset` and `parent` of [car-damage-parts.md](../modding/car-damage-parts.md).

## What is not decoded

Each of these cars keeps its part still. No number is guessed for them.

- **The air vents** of the 918 Concept (animation 1478700), the LP700-4 (1122608), the Marussia B2 (1343122) and the Aventador J
  (2091666): each vent state has a close condition and a `Wait for 5s` transition with a condition of its own, and the
  machine has a `Wait for 3s` transition with another. The native animation object holds only an open number and a close
  number (above), so the vents may well open and close by the two numbers (918 Concept 75/75, LP700-4 100/100, Marussia
  120/100, Aventador J 100/100), but what the timed transitions do to them was not found: the generic transition evaluator that
  reads those objects was not located (the animation behaviour's constructor 0x005adad0 and update 0x005ad7d0 never read a
  time). Not driven until that is known.
- **Agera R**: its state is closed by a condition on `gas` with operator `0`, and its opening condition is a composite
  `Speed 70mph + Gas`. The operator codes `0` and `1` have no label to read them from and the evaluator was not found
  (the native object has no operator). Its animation moves two child bones of the wing, not `SpoilerHinge`.
- **Veyron Super Sport and Grand Sport Vitesse**: the machine is the kind 1 behaviour decoded above, with five animations
  that blend and move six to eight bones of the wing and its arms (parent chains). Implementing it needs a pose sampler with
  blend weights and a manifest that carries animation keys, which the host does not have; the `+0x1a98` and `+0x1a9c`
  inputs are not named.
- **McLaren MP4-12C and Venom GT Spyder**: three animations (spoiler up, brake, reverse; 1.6 to 3.75 s) on three bones in a
  chain. The behaviour class that runs them (candidates called from 0x005a82e0, 0x005a8670 and 0x005ad410
  <!-- leak-check: allow -->) was not decompiled.
- **Pagani Huayra**: four animations of 0.612 s, one rotating bone each (two front and two rear flaps); their behaviour
  class is the same open question as the McLaren's.
- **Speed key times of the Panamera** and the rest pose frame, as marked above.
- Whether the physical downforce changes with the spoiler: nothing in the machine says so. The repo leaves it unchanged.
