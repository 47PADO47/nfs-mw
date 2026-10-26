# Chameleon active aero: the spoiler that rises with speed

What the car bundles of NFS: Most Wanted (2012) say about the spoilers that go up and down with the car's speed (the
Porsche 918 Spyder's wing, the 911's spoiler), and how this repo reads it
([`libs/chameleon-vehicle/src/active_aero.rs`](../../libs/chameleon-vehicle/src/active_aero.rs)). The skeleton, bone names
and skin that say which triangles move are in [chameleon-hinged-parts.md](chameleon-hinged-parts.md); the containers in
[chameleon-bundle.md](chameleon-bundle.md). Evidence tags: [README](../README.md#evidence-tags); **[exe]** is a string or
table of `NFS13.exe` (v1.4.0.0).

Everything below was measured on the 93 `VEHICLES/VEH_<n>_HI.BNDL` with the Rust reader and throwaway scripts outside the
repository, unless tagged otherwise.

## Short answer

- A car with moving aero has an **`HSMData`** resource (type 0x501): a hierarchical state machine named
  `<car>_SpoilerAnimation` (or `_AirVentsAnimation`, `_AileronAnimations`). Its conditions read the car's **speed**
  (`speedinmph`) and, on some cars, the brake, the gas and the steering; its states play an **animation** (type 0xB3) of the
  skeleton **[verified]**: 18 of the 93 bundles have one.
- The animation moves the bone **`SpoilerHinge`** by a translation between its first and last key **[verified]** on the 11
  bundles read here; that bone is also the root of the spoiler's triangles (the skin).
- 11 bundles of 9 models have a plain speed-up, speed-down spoiler: **Porsche 911 Carrera S 2012** (two bundles), **Lexus LFA**,
  **Porsche 918 Spyder Concept Study 2010**, **Mercedes-Benz SL65 AMG Black Series** (two), **Mercedes-Benz SLS AMG**, **Porsche
  918 Spyder 2013**, **Lamborghini Aventador LP700-4**, **Porsche Panamera Turbo S**, **Koenigsegg Agera R**.
- Not read here: the **Bugatti Veyron 16.4 Super Sport and Grand Sport Vitesse**, the **McLaren MP4-12C** and the **Hennessey
  Venom GT Spyder** (their wing also reacts to the brake: states `Brake Anim`, `Top Speed Anim`, `Reverse ...`), the **Pagani
  Huayra** (four flaps that follow speed, steering and brake) and the vents of the **Marussia B2** and the **Lamborghini
  Aventador J** (`AirVents Anim` above 100 or 120 mph). The Agera's `gas` input is not read either.

## The state machine

The resource is a Genesys-style object graph (type descriptions are in the bundle; 1,888 bytes on the 911, 13,520 on the
Veyron). Its strings are the readable part:

| String | Meaning |
|---|---|
| `<car>_SpoilerAnimation` | the name of the machine **[verified]** |
| `Speed above 70mph`, `Speed 75mph`, `Speed 70mph + Gas` | the name of the condition; it holds the speed it tests **[unconfirmed: a designer's label; the number in it matches the largest constant on 9 of the 11 bundles]** |
| a number (`40`, `70`, `100`) | the constants of the conditions, a speed in mph each; the bigger one is the deploy speed, the smaller the retract speed **[unconfirmed]** |
| `speedinmph`, `gas`, `brake`, `steering` | the **inputs** the engine feeds the machines; the list is in the executable at 0x00fae7d8 to 0x00fae800 (`gas`, `brake`, `steering`, `speedinmph`, `coplights`) **[exe]** |
| `Spoiler Anim` | the name of the state that plays the animation |
| `904078` | the animation's **id as a decimal number**: the animation is the resource whose id ends in 0x0DCB8E (904078 on the 911) **[verified]** |
| `False`, `0.25` | a flag (not looping) and a time (0.25, probably a blend in seconds) **[unconfirmed]** |
| `Wait 3s`, `Wait 5s` | a delay between states, on the two cars with two stages **[unconfirmed]** |

What this repo takes from it (`active_aero::speeds`): the **deploy speed** is the lowest speed of a condition that says
`above` (or the only one named); the **retract speed** is the biggest other constant below it, and ten mph under the deploy
speed when there is none. Read that way:

| Car | Deploys at | Retracts below | Travel (up, forward) | Takes |
|---|---|---|---|---|
| Porsche 911 Carrera S 2012 | 70 mph | 40 | 2.0 cm up, 7.0 cm back | 0.61 s |
| Lexus LFA | 70 | 50 | 11.9 up, 13.5 back | 0.51 s |
| Porsche 918 Spyder Concept Study | 75 | 65 (default) | 11.4 up, 4.9 forward | 0.51 s |
| Mercedes-Benz SL65 AMG Black Series | 75 | 65 (default) | 9.2 up, 6.2 back | 0.61 s |
| Mercedes-Benz SLS AMG | 74 | 64 | 0.2 down, 9.9 back | 0.77 s |
| Porsche 918 Spyder 2013 | 100 | 80 | 9.5 up, 7.5 back | 0.61 s |
| Lamborghini Aventador LP700-4 | 80 | 50 | 0.5 up, 0.9 back | 0.61 s |
| Porsche Panamera Turbo S | 70 | 50 | 5.1 up, 5.3 back | 3.00 s |
| Koenigsegg Agera R | 70 | 60 | 2.7 down, 2.7 forward | 0.34 s |

The 918 Spyder Concept Study and the Aventador LP700-4 have two stages (`Speed above 75mph` and `above 100mph`; `above 80mph`
and `above 100mph`), each with its animation; the first animation that moves is used, so the first stage. The Agera reads
`gas` as well. The SL65 and SLS numbers are a guess where the data names one constant.

## The animation (type 0xB3)

496 bytes on the 911; all little endian **[verified]** unless tagged:

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | version, 2 |
| 0x04 | u32 | **number of keys** (2 on the simple spoilers; 5 and 9 on the Veyron and the Panamera) |
| 0x08 | u32 | number of bones of the skeleton |
| 0x0C | u32 | a count (1 on the 911; 3 on the Agera; 4 on the Veyron) **[unconfirmed meaning]** |
| 0x30 | f32 | **length in seconds** (0.612 on the 911) |
| 0x38 | u32 x9 | offsets of the tables, in this order: 0x70, 0x90, 0xB0, 0xB0, 0x150, ... (the 911) |
| 0x5C | u32 | the animation's own id (0x0DCB8E) |
| 0x70 | 16-byte rows | a table with two rows on the 911, all zero **[unconfirmed: rotations]** |
| 0x90 | 16-byte rows | the **translations**: one row (x, y, z, 0 as f32) per animated track and key, key by key; the 911's second row is (0, 0.0203, -0.0700) **[verified: the rows are small movements of the right size and direction, e.g. the 918's wing 9.5 cm up and 7.5 cm back]** |
| then | rows of (1, 1, 1, 1) | scales |
| then | bytes | maps of 0xFF with a few bone numbers, and bit tables **[unconfirmed]** |

The translation table is read as `keys` blocks of `tracks` rows (`tracks` is the row count over the key count): the move of
a track is its row in the last block minus its row in the first. The cars with two or six tracks (SL65, Agera, SLS) have
tracks that stay still; the first track that moves is taken. The Panamera has nine keys and three tracks (the centre and
two side flaps that move 14 cm sideways): the first, central track is taken.

The values are in the engine's space (x left, y up, z forward, metres) and are a movement of the bone, so they are a
translation of the whole spoiler; the data holds no rotation that this reader found **[unconfirmed]**. The animation maps
each track to a bone by tables this reader does not decode; the bone is taken from the name `SpoilerHinge`
**[verified]** by the skeleton (the names `Root`, `BonnetHinge`, `BootHinge`, `RearWindow`, `SteeringWheel`, `SpoilerHinge` hash
to bones of the 911: 0, 13, 33, 34, 35, 36; the same names are found in the bones of the other ten bundles).

## What is not decoded

- The consumer in `NFS13.exe`: only the input names are found there ([exe], see above); the code that evaluates the machine
  was not located. The deploy and retract speeds, the 0.25 and the waits are read from strings.
- The bone maps and the other key tables of the animation, so rotations (if any), the side flaps of the Panamera and the
  per-track bones.
- The brake, gas and steering conditions, and the Veyron, McLaren, Venom, Huayra and vent animations.
- Whether the physical downforce changes with the spoiler: nothing in the machine says so. The repo leaves it unchanged.
