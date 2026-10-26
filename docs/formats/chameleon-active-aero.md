# Chameleon active aero: the spoilers, wings and vents that move with the car

What the car bundles and `NFS13.exe` (v1.4.0.0) of NFS: Most Wanted (2012) say about the spoilers, wings and air vents that
move with the car's speed, brake and throttle, and how this repo reads it
([`active_aero.rs`](../../libs/chameleon-vehicle/src/active_aero.rs), [`hsm.rs`](../../libs/chameleon-vehicle/src/hsm.rs),
[`wing.rs`](../../libs/chameleon-vehicle/src/wing.rs)). The skeleton, bone names and skin that say which triangles move are in
[chameleon-hinged-parts.md](chameleon-hinged-parts.md); the containers in [chameleon-bundle.md](chameleon-bundle.md).
Evidence tags: [README](../README.md#evidence-tags); **[exe]** is read from the executable's code or tables (a Ghidra
session, nothing of it is in the repository). Addresses are of `NFS13.exe` v1.4.0.0.

**Rule of this document:** a number the game plays is a number the data or the executable states. Where one could not be
decoded, the part stays still and it is listed under [What is not decoded](#what-is-not-decoded) with the reason.

## Short answer

- A car with moving aero has an **`HSMData`** resource (type 0x501), a state machine named `<car>_SpoilerAnimation` (or
  `_AirVentsAnimation`, `_AileronAnimations`): conditions on inputs of the car (`gas`, `brake`, `steering`, `speedinmph`,
  `coplights`) and **animation states** that play an animation (type 0xB3) of the skeleton **[verified]**: 18 of the 93 bundles.
- Driven from decoded data:
  - **spoiler states** (speed above / speed below): 911 Carrera S x2, LFA, 918 Spyder, 918 Spyder Concept Study, SL65 AMG x2, SLS AMG,
    LP700-4, Panamera (spoiler and two side flaps);
  - the **Agera R**: two flaps that open above 70 mph with the gas pressed and close when it is released;
  - the **air vents** of the 918 Spyder Concept Study, the LP700-4, the Marussia B2 and the Aventador J, with their waits;
  - the **Veyron Super Sport and Grand Sport Vitesse** wing machine: five keyed animations, cross-fades, the brake.
- **Not driven:** McLaren MP4-12C, Hennessey Venom GT Spyder, Pagani Huayra (the exact blocker is below).

## The input names and the speed [exe]

- The inputs are bound to the car by the routine at 0x005a7400 <!-- leak-check: allow -->. It compares the hash of an input's
  name (CRC-32, bits read most significant first) with the hashes the static initialisers make of the strings at
  0x00fae7dc, 0x00fae7e0, 0x00fae7e8, 0x00fae7f4 and 0x00fae800 <!-- leak-check: allow -->. Result:
  **`gas` is the float at `+0x192c` of the vehicle object, `brake` is `+0x1930`, `steering` is `+0x193c`**, `speedinmph` comes
  from a routine, `coplights` is 1 when the word at `+0x1954` is 2.
- **The speed is `|velocity| x 2.236936`**: the update routine of the animation behaviour (0x005ad7d0 <!-- leak-check: allow -->)
  takes the length of the vehicle's velocity (`+0x120`, `+0x124`, `+0x128`, metres per second) and multiplies it by a vector
  constant that the start-up code (0x00f79260 <!-- leak-check: allow -->) makes from `1 / 0.44704`.
- The vehicle floats `+0x1a98`, `+0x1a9c`, `+0x1aa0` are written only by the vehicle constructors (0x00586810 and 0x00591350
  <!-- leak-check: allow -->, zeros): they are a scrub input that sets an animation clock directly (a preview), not gameplay.

## The condition operators

A condition's data object holds a number, an input and an **operator code**. Read off the labels of the 93 bundles
**[verified]**:

| Code | Meaning | Label that shows it |
|---|---|---|
| 0 | equal | `Brake OFF` (`brake` 0 0), the Agera's `gas` 0 0 (throttle released) |
| 1 | not equal | `Brake ON` (`brake` 1 0), `Speed 70mph + Gas` (`gas` 1 0) |
| 2 | greater than | `Speed above N`; `Turning Right` (`steering` 2 0.3, 5 0.3 on the Agera and Huayra flaps) |
| 3 | less than | `Speed under N`; `Turning Left` (`steering` 3 -0.3) |
| 4 | at most | `Front Left Flap Turning Anim` (`steering` 4 -0.3) |
| 5 | at least | `Front Right Flap Turning Anim` (`steering` 5 0.3) |

The codes 4 and 5 are read from two labels only. A condition that is several tests at once (`Speed 125mph + Brake ON`, `Speed
70mph + Gas`) has several data objects: the **flag word of its property row counts them**, and all have to hold. The
executable does not evaluate operators in the animation behaviours (below): the generic evaluator that turns conditions into
the flags and thresholds those behaviours read was not found.

## The animation behaviours [exe]

The executable has several hard-coded behaviours for the animation states of a car (the update routines at 0x00489050, 0x00487a00,
0x00488cc0, 0x0047bd00 and 0x0047c0b0 <!-- leak-check: allow -->). Their constructor reads a native object with floats (the
numbers of the data) and a short that picks the behaviour. Read so far:

**Speed state** (0x00489050 <!-- leak-check: allow -->), the spoilers: four states (0 at rest, 1 playing forwards, 2 held at the
end, 3 playing backwards). At rest it goes to 1 when the speed is **above** the open number; while playing it goes to 3 when the
speed is **below** the close number; at the end it holds; backwards it ends at 0. The clock moves by the frame time, so the whole
way takes the animation's length at a constant rate. The Agera's state has the same shape with the gas test in place of the speed
close test.

**Timed state** (the air vents: 0x00488cc0, fed by 0x005a8670 <!-- leak-check: allow -->). The feeder takes four numbers from the
state's native object: an opening speed, a duration, a closing speed and a delay. Closed, it counts the seconds the speed stays
**above** the opening speed (any dip resets the count); when the count passes the duration the vent plays forwards to its end and
holds. Open, when the speed is **below** the closing speed it loads the delay into a countdown and, when that reaches zero, plays
backwards; **the speed is not looked at again until it is closed**. In the data the opening speed is the `Speed above N` condition
listed before the state, the duration is the `Wait for Ns` transition of the machine whose aborting condition is `Speed under N`,
the closing speed is the state's own `Speed under M` condition and the delay is the `Wait for Ms` transition of the state
(**[unconfirmed]**: the executable's loader that fills the native object was not read; this is the only assignment of the four
numbers that fits their labels).

| Vent | Opens above | for | Closes below | after | Takes |
|---|---|---|---|---|---|
| 918 Spyder Concept Study | 75 mph | 3 s | 75 | 5 s | 0.51 s |
| Lamborghini Aventador LP700-4 | 100 | 3 s | 100 | 5 s | 0.77 s |
| Marussia B2 | 120 | 3 s | 100 | 5 s | 0.77 s |
| Lamborghini Aventador J | 100 | 3 s | 100 | 5 s | 0.77 s |

**Wing machine** (the Veyron; 0x00487a00 <!-- leak-check: allow -->): 13 states over **five animations**, each with its own clock.
The data names the animations by state (`Handling Anim`, `Brake Anim`, `Brake from Handling Anim`, `Brake from Top Speed Anim`,
`Top Speed Anim`, plus `Reverse ...` copies that are the same animations played backwards) and holds the numbers the code tests:
**140 mph** (`Speed 140mph`), **125 mph** (`Speed 125mph + Brake ON`), **200 mph** (`Speed 200mph`) and `Wait 0.4s`; the code
adds the 0.25 s cross-fade. The brake counts once it has been held 0.4 s (its float is zeroed before that). States:

| State | Plays | Goes on when |
|---|---|---|
| 0 rest | animation 0 at 0 | speed > 140 to 1; speed > 125 and brake to 4 |
| 1 / 2 open | animation 0 forwards / held | brake to 9; speed < 140 to 3; in 2 speed > 200 to 6 |
| 3 | animation 0 backwards | at 0 to rest |
| 4 / 5 brake | animation 1 forwards (held while braking) / backwards | brake released to 5; at 0 to rest |
| 6 / 7 / 8 top | animation 4 forwards / held / backwards | brake to 11; speed < 200 to 8; at 0 to 2 |
| 9 brake from open | animation 2 over animation 0, cross-faded over 0.25 s | brake released: speed > 140 to 10, else at the end to 5 |
| 10 | animation 0 over animation 2 (fading), cross-faded | at 0 to 2 |
| 11 brake from top | animation 3 over animation 4, cross-faded | brake released: speed > 200 to 12, else at the end to 10 or 5 |
| 12 | animation 4 over animation 3 (fading) | at 0 to 7 |

Slot 0 ends where slot 2 and slot 4 begin and slot 3 begins where slot 4 ends **[verified]** on the keys (the chain of poses is
closed). A pose is the **blend of absolute key poses** (0x009f22a0 <!-- leak-check: allow -->, the weighted sampler): the first
animation replaces the pose, the second moves it toward its own pose by a weight (1 - t/0.25 or t/0.25); translations are
linearly interpolated and rotations spherically. A bone that only the second animation moves is moved from rest.

**Flag classes** (0x0047bd00 and 0x0047c0b0 <!-- leak-check: allow -->): one-bit open and close behaviours that read booleans
from the generic condition evaluator; the McLaren, Venom and Huayra are of this family (not read, below).

## The state machine's data

A block of objects. An **object record** is eight words: type hash, id, three offsets (properties for kinds, the **child
list**, the **property table**), a zero, the offset of the name and a flags word **[verified]**. Type hashes: machine
`24c422b2`, animation state `849c8ce8`, condition `0ec6f1b1`, timed transition `809acb7d` (none of them is in the
executable: they are hashes of type names the bundle carries). A **child list** has 16 bytes per child (id, table, count,
name). A **property table** has rows of 16 bytes: a 64-bit key (low six bits 0), the offset of a value cell, a type (6 for a
nested object, otherwise a text) and a flag word (the number of nested objects). A text cell holds the offset of a NUL-terminated
string; an object cell holds, for each nested object, a table offset and a row count.

- An **animation state** holds its animation (key `4f883ba0b3049400`, a decimal number: the animation is the resource whose id
  ends in it) and, for a close condition kept in the state, a nested object under `684563f7fd6c4600`.
- A **condition** record holds nested objects with three rows each: the number (`bcdaa6b634a9aa00`), the input's name
  (`9952fc5819000000`) and the operator (`5b55835f4e5a7080`).
- A **timed transition** holds its seconds under `bd69b4a619000000` and a child list with the condition that aborts the wait.
- A machine lists its children: **the condition that opens a state is the nearest condition listed before it** **[verified on
  the 15 bundles; the exe's own loader that links them was not read]**.

| Spoiler state | Opens | Closes | Takes | Slide (up, back) | Turn |
|---|---|---|---|---|---|
| Porsche 911 Carrera S 2012 (2 bundles) | speed > 70 mph | speed < 40 | 0.612 s | 2.0 cm up, 7.0 cm back | 30.46 deg |
| Lexus LFA | > 70 | < 50 | 0.508 s | 11.9 cm up, 13.5 cm back | 18.30 deg |
| Porsche 918 Spyder Concept Study | > 100 | < 80 | 0.612 s | 8.6 cm up, 7.9 cm back | none |
| Mercedes-Benz SL65 AMG Black Series (2) | > 75 | < 75 | 0.612 s | 9.2 cm up, 6.2 cm back | none |
| Mercedes-Benz SLS AMG | > 74 | < 64 | 0.769 s | 0.2 cm down, 9.9 cm back | 26.21 deg |
| Porsche 918 Spyder 2013 | > 100 | < 80 | 0.612 s | 9.5 cm up, 7.5 cm back | none |
| Lamborghini Aventador LP700-4 | > 80 | < 50 | 0.612 s | 0.5 cm up, 0.9 cm back | 12.54 deg |
| Porsche Panamera Turbo S | > 70 | < 50 | 3.000 s | 5.1 cm up, 5.3 cm back | 15.25 deg |
| Koenigsegg Agera R (2 flaps) | > 70 and gas pressed | gas released | 0.337 s | none | 12.38 deg |

The SL65 opens above 75 and closes below 75: at exactly 75 it keeps its direction. The Panamera's animation has nine keys on
which the spoiler moves in nine equal steps (checked), so a linear clock over the 3 s is its motion; the key times themselves
are not read (uniform spacing assumed **[unconfirmed]**, the same for the Veyron's keys). The same animation moves two more bones
(the Panamera's side flaps, the Agera's second flap), which are parts of their own.

## The animation (type 0xB3)

All little endian. Layout as the sampler of the executable reads it (0x009f3250 and 0x009f9d60 <!-- leak-check: allow -->)
**[exe]**, and checked on the 18 bundles **[verified]**:

| Offset | Type | Field |
|---|---|---|
| 0x04 | u32 | number of keys (2 on the plain spoilers, 3 to 9 elsewhere) |
| 0x08 | u32 | number of bones of the skeleton |
| 0x1C | u32 | stride of one key in the rotation table, bytes |
| 0x20 | u32 | stride of one key in the translation table, bytes |
| 0x30 | f32 | **length in seconds** |
| 0x34 | u16 | format; 1 packs a rotation into 32 bits |
| 0x36 | u16 | the reference key (0) |
| 0x38 | u32 x n | offsets of the tables; word 1 the translations, word 4 their bone map, word 5 the packed rotations, word 7 their bone map |

- A **bone map** has one byte per skeleton bone: the bone's track number, or 0xFF for none.
- A **translation** key is a row of 16 bytes (x, y, z as f32, a pad word) per track.
- A **rotation** key is a packed quaternion of 32 bits per track: the top two bits say which component is left out, the three
  10-bit fields at bits 20, 10 and 0 hold the other three as `(field - 511) * 0.00138`, and the left-out one makes the
  quaternion unit **[exe]**.
- The additive sampler (0x009f3250) adds the interpolation of two keys **minus the reference key** to the rest pose (the
  speed states: the movement at the end is the last key minus key 0); the weighted sampler (0x009f22a0, the wing machine)
  blends the **absolute** keys. The keys of the spoilers start at zero, so both give the same poses. The values are in the
  engine's space (x left, y up, z forward, metres) and in the frame of the bone's parent; on the bundles read that frame is the
  car's own **[verified]**, the general case is **[unconfirmed]**.

The converter writes `rotation_axis`, `rotation_deg`, `offset`, `gas`, `open_after_seconds`, `close_after_seconds`, `parent`,
`[[damage_parts.track]]` and `[wing]` of [car-damage-parts.md](../modding/car-damage-parts.md).

## What is not decoded

Those cars keep their parts still. No number is guessed for them.

- **McLaren MP4-12C, Hennessey Venom GT Spyder and Pagani Huayra.** What was established and where the trail ends:
  - The behaviour classes are registered in a table of nine entries at 0x00fae0ac (stride 0x14: a small function that returns a
    category, a common word, the constructor, two more) <!-- leak-check: allow -->. Constructor, class vtable and update routine of
    each: 0x005aae60 / 0x00faf168 / 0x005a9d10 (bone controllers: wheels and suspension, not wings); 0x005adad0 / 0x00faf1c8 /
    0x005ad7d0 (the speed spoiler and the Veyron wing machine); 0x005a8900 / 0x00faf0d8 / 0x005a8670 (the timed vents);
    0x005a84a0 / 0x00faf0a8 / 0x005a82e0 (one-animation open/close flag); 0x005ad5e0 / 0x00faf198 / 0x005ad410 (the same with
    vehicle flags); 0x005ae8d0 / 0x00faf078 / 0x005b62f0 (lights and dirt, not animation states); 0x005ac8c0 / 0x00faefe8 /
    0x005ac790 (one animation scrubbed by a continuous vehicle value, `0.5 + 0.5 v`, e.g. a steering wheel); 0x005af090 /
    0x00faf208 and 0x005b6230 were not read <!-- leak-check: allow -->.
  - The data does not name the class: every animation state of every car has the same type hash. What differs is the **top byte of the
    record's flags word**: `00` for the plain spoilers, the Agera and the Huayra flaps, `02` for the vents, and `07`, `06`, `03`, `00`
    for the Veyron's `Handling`, `Top Speed`, `Brake from ...` and `Brake` states, `06`, `03`, `00` for the McLaren's and the Venom's
    `Spoiler Up`, `Brake From Speed` and `Brake`. Nothing in the executable's registration table reads it; the loader that turns
    these objects into the native ones the constructors read (0x005adad0 and the others) was **not located**, so the byte cannot
    be tied to an entry.
  - The McLaren cannot run the Veyron's machine: that one tests static constants 140, 125 and 200 mph (made at 0x00f4b700 to
    0x00f4b720), while the McLaren's data holds 120 and 80 and `Speed under 10`, and none of the static vector constants of the
    animation classes is 120 or 80 (the only static 120 and 80 belong to code outside this family: 0x00910790, 0x00589e20,
    0x0063dfa0). Its graph has the same shape as the Veyron's without the top-speed states, which is not enough to port the
    Veyron's code with other numbers: the thresholds of that code are not read from the data.
  - The Huayra's four flaps follow `steering` at -0.3 and 0.3 (operators 4 and 5), speed and brake; their states have the two
    flag properties set to 1 (the McLaren's `Brake` has one of them) and no flags byte. The class is not among the entries read.
- **The key times** of the animations (uniform spacing assumed) and the rest-pose frame, as marked above.
- Whether the physical downforce changes with the wings: nothing in the data says so. The repo leaves it unchanged.
