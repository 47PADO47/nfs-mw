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
- Driven from the data:
  - **spoiler states** (speed above / speed below): 911 Carrera S x2, LFA, 918 Spyder, 918 Spyder Concept Study, SL65 AMG x2, SLS AMG,
    LP700-4, Panamera (spoiler and two side flaps);
  - the **Agera R**: two flaps that open above 70 mph with the gas pressed and close when it is released;
  - the **air vents** of the 918 Spyder Concept Study, the LP700-4, the Marussia B2 and the Aventador J, with their waits;
  - the **Veyron Super Sport and Grand Sport Vitesse** wing machine: five keyed animations, cross-fades, the brake;
  - the **McLaren MP4-12C and Hennessey Venom GT Spyder** wing: the Veyron's machine without the top-speed states, numbers from their own data (assumption, see below);
  - the **Pagani Huayra** flaps: guarded animations (assumptions, see below).

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
| 4 | at least (>=) | the end of a left turn: `Front Left Flap Turning Anim` closes when `steering` is at least -0.3 |
| 5 | at most (<=) | `Brake OFF and Speed under 120mph` (`speedinmph` 5 120); the end of a right turn: closes when `steering` is at most 0.3 |

The codes 4 and 5 are read from the labels of the Huayra and the McLaren (the first reading of this table had them the other way round; a right turn that ends at `steering` 5 0.3 only makes sense as "at most"). A condition that is several tests at once (`Speed 125mph + Brake ON`, `Speed
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
from the generic condition evaluator; no car was tied to them.

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

Nothing is driven that the data does not state; the open questions are:

- **The class of the McLaren, the Venom and the Huayra.** No native class was tied to their data: the registration table at
  0x00fae0ac has nine entries (constructor / vtable / update): 0x005aae60 / 0x00faf168 / 0x005a9d10 (bone controllers),
  0x005adad0 / 0x00faf1c8 / 0x005ad7d0 (speed spoiler and Veyron machine), 0x005a8900 / 0x00faf0d8 / 0x005a8670 (timed vents),
  0x005a84a0 / 0x00faf0a8 / 0x005a82e0 (flag), 0x005ad5e0 / 0x00faf198 / 0x005ad410 (vehicle flags), 0x005ae8d0 / 0x00faf078 /
  0x005b62f0 (lights and dirt), 0x005ac8c0 / 0x00faefe8 / 0x005ac790 (a scrubbed animation), and 0x005af090 / 0x00faf208 and
  0x005b6230 (not read) <!-- leak-check: allow -->. The data names no class (every animation state has the same type hash;
  the top byte of the record's flags word differs: 00, 02 for vents, 03, 06 and 07 for the Veyron and McLaren states) and
  the loader from data to native objects was not found. These three cars are therefore driven from the **graph in their data**,
  with the assumptions below.

## The McLaren, the Venom and the Huayra

**Link semantics read from the data** (the child lists of a machine: each child has links of 24 bytes to other objects of the
machine; a link with the key `684561a168000000` joins a condition and what it guards, both ways; a `Wait` transition in the chain
is the seconds the condition has to hold; a condition that is `OR` has the row `5b562d0c8c7b8000` = 1). The McLaren's and the
Venom's graph is the Veyron's without `Top Speed`: the root has an `OR` group (`Speed above 120mph`, `Brake ON and Speed above
80mph`), `Wait 0.4s` with `Brake OFF`, and the states `Spoiler Up Anim`, `Brake Anim` and the nested `Brake From Speed Anim`
with its `Reverse` copies; it exits by `Speed under 120mph OR Brake`, `Brake OFF and Speed above/under 120mph`.

**Played as the wing machine** (the Veyron's code, native class unknown): slot 0 is `Spoiler Up Anim`, slot 1 `Brake Anim`, slot 2
`Brake From Speed Anim`; the open speed (120 mph, the condition that is a single speed test), the brake speed (80 mph, the speed
test of `Brake ON and Speed above 80mph`) and the hold (0.4 s, the timed transitions) come from the data; there is no top-speed
state. **Assumptions, not data:** that the three-animation graph is played by the same state table as the Veyron's (the
Veyron's code takes its speeds from static constants, so another class must hold the McLaren's numbers; the graphs are
isomorphic), that the 0.25 s cross-fade of the Veyron's code applies, that `Speed under 10` (and the Veyron's `Speed under 20`)
is not tested (the Veyron's code does not), and that a clock runs linearly over the animation's length between evenly spaced keys.

**Guarded animations** (the Huayra, any other machine of animation states directly in a group machine): each state is played
by the condition that guards it (looked up through the links, and through the group it is in: the group's condition starts all
its states, the group's free condition ends them) and, if it has its own condition list, ends by that. The Huayra has ten
(`hsm::drives`): the four `Turning` states (speed above 50 and below 137 mph with `steering` beyond 0.3 to the right or -0.3 to
the left; they end when the speed leaves the band or the wheel comes back to 0.3), the two front speed bands (50 to 93 mph),
the two rear speed bands (50 to 155 mph) and the two rear brake states (speed above 155 mph with the brake, held 0.4 s; they
end when the brake is off or the speed under 10). The four animations move one bone each (the two front and two rear flaps),
so each flap has two or three drives. **Assumptions, not data:** a drive plays its animation **to the end** at a constant rate
over the animation's length (the labels `Rotate 30°` and `Rotate to 50°` are two states playing the same animation, whose end
pose is about 48°; no field limits the 30° state), a flap is out as far as the furthest of its drives, a drive that is out stays
until its closing tests hold, the brake states nested inside a speed state of the front flaps (`Brake Animation` when the speed is
above 62 mph with the brake) add nothing (they play the animation the speed state already plays), a positive `steering` of the
game is a turn to the right, and the speed of the Huayra's front flaps ends at 93 mph as its data says.

- **The key times** of the animations (uniform spacing assumed) and the rest-pose frame, as marked above.
- Whether the physical downforce changes with the wings: nothing in the data says so. The repo leaves it unchanged.
