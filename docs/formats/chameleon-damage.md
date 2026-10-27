# Chameleon damage: the clips that crumple and break a 2012 car

What NFS: Most Wanted (2012) keeps about damage, and how this repo reads and plays it. The hinged parts and their ladders are in
[chameleon-hinged-parts.md](chameleon-hinged-parts.md), the skeleton and the skin there too; containers in
[chameleon-bundle.md](chameleon-bundle.md); the animation sampler in [chameleon-active-aero.md](chameleon-active-aero.md#the-animation-type-0xb3).
Evidence tags: [README](../README.md#evidence-tags); **[exe]** is a string, table or function of `NFS13.exe` (v1.4.0.0). Everything
was measured on the 93 `VEHICLES/VEH_<n>_HI.BNDL` of the install with the readers in
[`libs/chameleon-vehicle`](../../libs/chameleon-vehicle/src/damage_clip.rs) unless tagged otherwise. Axes are the file's: x left, y up,
z forward, metres.

## Short answer

- **There is no vertex morph and no mesh swap.** A 2012 car is damaged by **bone animation of its skinned body**: the
  `VehicleDamageBehaviour` object imports an `AnimationList` (resource 0xB0) of **8 clips, one per damage zone** (front, rear, left, right,
  front left, front right, rear left, rear right: the zone codes 1 to 8), each of **21 keys over the zone's damage 0 to 1**
  (every 0.05) on the 37 or so bones of the skeleton **[verified]** (91 cars have 8 clips; the bundle 122706 has 7, none for the rear right; the SWAT van has none).
- A clip moves a bone with a **slide** and a **turn** (small: a few centimetres and up to about 20 degrees, a tilt of the panel
  and of the bone's skinned neighbourhood) and a **scalar** that is 1 while the piece is whole and 0 once it is broken. A scalar going
  to 0 is accompanied by the bone's slide jumping away (a metre or two: the piece leaves or hides) **[verified]**.
- **Which pieces break, and at what damage, is the same in every car**: lamp glasses at 0.25, a lip of a bumper at 0.10 and one at
  0.35, window glass at 0.40, mirrors at 0.25, corner pieces of the bumpers at 0.60, the bumpers, the spoiler and the rear lamps'
  surround at 1.0 **[verified]** (table below).
- The ladders of the `Bodypart`s (`DamageThreshold`) are **not** what the clip is played against: 731 of the 737 parts of the 93 cars have a
  track in their zone's clip (730 turn by half a degree or more), and the keys at which pieces break are the same whatever the ladder of the part they sit on
  (142 of 805 pieces break at a value that is also a target of the ladder above them, about chance) **[verified]**. The ladder's second
  value is probably the share of the part's angle range its physical hinge may open to **[unconfirmed]**.
- A zone's damage is a float from 0 to 1 per zone (the HUD shows it times 100, and `DamageBarState` is documented as 0.0 to 3.0)
  **[exe]**. The 2012 game has **no smoke, fire or spot effects in the car data**: no `SpotEffect` in any of the 93 damage objects, every
  `DamageSequence` empty. What a "wreck" is lives in the gameplay tables below, not in the cars.

## The animation list (type 0xB0)

All little endian, offsets in bytes. The damage list is the 0xB0 resource the damage behaviour imports (a car may hold a second,
one-clip 0xB0 list that is not read).

| Offset | Type | Field |
|---|---|---|
| 0x00 | u16 | version, 2 |
| 0x02 | u16 | clip count, 8 (7 on the bundle 122706) |
| 0x04 | u32 | block size less 8 (0x4C28 on the 911) |
| 0x08 | u32 | offset of the clip table (0xE0) |
| 0x0C to 0x14 | u32 x3 | 0x20, 0x60, 0xA0: three zeroed 64-byte blocks **[unconfirmed meaning]** |
| table | u32 x count | the offset of each clip in the block |

A **clip** is the layout of the animation (type 0xB3) of [chameleon-active-aero.md](chameleon-active-aero.md#the-animation-type-0xb3)
with one more kind of track. Offsets from the start of the clip:

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | 2 |
| 0x04 | u32 | keys, 21 |
| 0x08 | u32 | bones of the skeleton |
| 0x0C, 0x10, 0x14 | u32 x3 | tracks: rotations, slides, scalars |
| 0x1C, 0x20, 0x24 | u32 x3 | stride of one key in the rotation table (4 per track), the slide table (16 per track), the scalar table |
| 0x30 | f32 | length in seconds, 6.0 (21 keys, 0.3 s apart; the damage fraction is the key over 20) |
| 0x34 | u16 | format, 1 (a rotation packs into 32 bits, [chameleon-active-aero.md](chameleon-active-aero.md)) |
| 0x38 | u32 x10 | table offsets, word *n* below |

| Word | Table |
|---|---|
| 0 | a block of 16 bytes per key, all zero on the 911 **[unconfirmed]** |
| 1 | the slides: a row per key of 16 bytes (x, y, z, pad) per slide track |
| 2, 3 | 1.0 for every bone, padded to a multiple of 4: per-bone weights **[unconfirmed]** |
| 4 | the bone map of the slides: a track number per bone, 0xFF for none |
| 5 | the packed rotations, a u32 per rotation track per key |
| 6 | the bone map of the scalars |
| 7 | the bone map of the rotations |
| 8 | the scalars: an f32 per scalar track per key |

Every clip has 21 keys over 6 s but three: the clips 5 and 6 of the bundle 122814 have 5 keys over 1.2 s and the clip 8 of the bundle 1551590 has
202 keys over 6 s; the reader takes the key count from the clip, and the damage fraction is the key over (keys - 1). The break fractions below
are for the 21-key clips.

The values are additive to the bone's rest pose, in the car's frame, as for the speed animations: a key less the reference key (key 0, the u16 at 0x36), which is
all zero on most cars and a small constant offset of some bones on 46 of the 93 (the same in all 8 clips; the reader subtracts it).
The scalar is 1.0 at key 0 on every track but three of one car (bones 9, 35 and 36 of the bundle 122699, 0 throughout: pieces that are never whole, read as not breaking) **[verified]**. The rotations use the sign of the right-handed quaternion
(a bonnet rising is a negative angle about +x, the sign of its range in [chameleon-hinged-parts.md](chameleon-hinged-parts.md))
**[verified geometrically]**. Spacing of the keys is uniform **[unconfirmed]**, as for the speed animations.

## What the clips do

The zone of a clip is its position in the list **[verified]**: the 911's clip 1 moves the front bumper (bones 1 to 6), the bonnet (13 to
17) and a front lamp (18); clip 2 the rear bumper, the spoiler (36) and the rear window (34); clips 3 and 4 the door hinge, handle,
mirror and window glass of the left and right doors; clips 5 to 8 two or three bones each, on the corner of the bumper (positions
at x > 0 are the left, as everywhere in the file).

The first key at which a bone's scalar is below one half (or, for a bone with no scalar, its slide steps by more than 0.25 m), over
the 93 cars, in damage fractions (key over 20) **[verified]**:

| Zone | Breaks at (cars) |
|---|---|
| front | 0.10 (89), 0.25 (69), 0.35 (21), 1.0 (88) |
| rear | 0.10 (82), 0.25 (20), 0.35 (65), 0.40 (69), 0.60 (24), 1.0 (88) |
| left, right | 0.25 (86, 85: the mirror), 0.40 (84, 85: the window glass) |
| front corners | 0.60 (89, 91) |
| rear corners | 0.25 (69, 68: the rear lamp), 0.60 (87, 86) |

The bumpers (bones 1 and 7) and the spoiler (36) drop by a metre at key 20 only. The bonnet and the doors never
break: they tilt (a bonnet by about 9 degrees at full damage, a door by 4 and its handle slides 15 cm). A piece that "breaks" is
drawn no more by the game: the scalar is probably what hides the bone's mesh **[unconfirmed: no consumer found]**.

## The ladders (`Bodypart.DamageThreshold`)

`DamageThreshold` is 24 bytes: the damage fraction at +0x10 and a second fraction at +0x14. The 911's bonnet: (0.04, 0), (0.05, 0.25), (0.5, 1);
bumpers (0.04, 0), (0.05, 0.1), (0.5, 0.75), (0.75, 1); mirrors (0, 0), (0.09, 0), (0.1, 0.5), (0.19, 0.5), (0.2, 0.7); doors (0.4, 0), (1, 1)
**[verified]**. The second values across the 93 cars are 0, 0.01, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.7, 0.75, 0.9 and 1. What a
step does is **[unconfirmed]**: the target of the part's physical hinge (its range, spring and damper are in the same `Bodypart`), not a
position in the clip, is the reading the numbers favour (above). `HingedPart::targets` reads them.

## The damage bar [exe]

`RaceCarEntity` binds `GetGlobalDamageBar`, `GetFrontDamageBar`, `GetRearDamageBar`, `GetLeftDamageBar`, `GetRightDamageBar`,
`GetFrontLeftDamageBar`, `GetFrontRightDamageBar`, `GetRearLeftDamageBar` and `GetRearRightDamageBar` (the binder is 0x0057e280; the <!-- leak-check: allow -->
documentation string of the bar is "The state of the damagebar going from 0.0f to 3.0f." at 0x00fc0cdc). A zone's getter (0x005a1df0 <!-- leak-check: allow -->
front, 0x005a1e50 rear) returns the float of the entity at +0xBD0 (front), +0xBE0 (rear), ... times 100, **held at 0.99 while a byte of the <!-- leak-check: allow -->
entity (+0xC20) is below a byte of the damage profile (+0x74 of the object at +0xC18)**: a bar cannot read full before that count is
reached. The global bar is two floats (+0xBBC and +0xBB4) multiplied by 100 (0x0059ed20). So the zone damage the ladders and clips use <!-- leak-check: allow -->
is a 0 to 1 float **[exe]**; the shape of the 3.0 scale is not seen in the bindings.

## The global tables (not decoded)

Fields of these objects are hashed, so only values can be read **[verified: present; meaning unconfirmed]**:

- `PHYSICS/PHYSICSCONFIG.BNDL`: `DamageBarProfiles` (0x7F100) lists four `Physics.DamageBarProfile`. Each has an
  `ImpactLocationDamageScale` of floats (profile 0x1028BF: 1, 1, 1, 0.85, 0.85, 1.5, 1, 0 then 0.75, 2, 4, 2, 4, 0.5, 1000,
  1000, 1000, 1000; profile 0x1028C7: 1, 1, 1, 1, 1, 1.5, 1, 0; profile 0x1028C8: 1, 1, 1, 1.125, 1, 1.65, 1, 0) and
  segments (0x1028BF: four 200s and a 500; 0x1028C7: zeros and a 100; 0x1028C8: a 100 and a segment with 1, 1, 30, 30 and a 50; 0x1028CB: four
  100s and a 500). Each segment names a `CrashingRules` object (0x102928, 0x6A5BE, 0x6A582, 0x10292B) of `ImpactRules` (a speed-like
  threshold: 33, 20, 30, or 50, 300, 50, 5, 25) with a `DamageToDeal` (50 on 0x10292B).
- The same types are in `GAMELOGIC/GAMEPLAY.BNDL`; `ImpactDamageGraphs` (0x7F0E0) holds two graphs (160, 100, 80, 33, 5 and 120, 100, 60, 32, 0, 6);
  `CollisionResponses_*.DamageParams` hold pairs of fractions per pair of car classes (player vs traffic: 0.4 to 0.7 and 0.7 to 1);
  `Physics.LandingDamage`; `CollisionInfo` (0x5D7CE: 50, 100, -0.9, 20, 0.7, 0.5).
- In each car: `Vehicle_Gameplay.DamageStats` (three of 28 bytes: 126.6 and seven small integers, 50, 30, 15 ... 100) and `WheelDamage` (one per
  wheel: 0, -15, 1, 1, 1, 0.25, 30, 7: a suspension damage, not a body one).
- `GLOBALCONFIG.BNDL`: `DamageImpactConfig` (0x461F1) links eight objects of other bundles (one per zone, **[unconfirmed]**);
  the `SmokeColour` objects there are tyre smoke colours.
- `Gameplay.BlacklistShutdownData.DamageThreshold` is the rival shutdown of the Most Wanted list (3.23, 10; speed control 300, 100, 90),
  not a car damage number.

So a wreck is decided by the gameplay (the segments' hit points and the crash rules), not by anything in a car's damage object. The
host's hit points belong to the same idea (`CarEventSpec::hit_points`).

## What the repo does with it

- `chameleon_vehicle::damage_clips(bundle)` reads the clips (`BoneTrack::at` interpolates, `break_key` finds where a piece goes);
  `hinged_parts` also reads each ladder step's second value.
- The 2012 content mod ([`damage_model.rs`](../../mods/nfsmw-2012-content/src/damage_model.rs)) turns them into manifest tables
  ([car-damage-parts.md](../modding/car-damage-parts.md#the-2012-cars)): the turn of each panel level (a third, two thirds, all) is a
  part's `stage_angles`, the first key at which its piece is gone is its `detach_at` (a part whose clip never breaks it gets `detach = false`),
  and every bone that breaks but is not a hinged part of the damage object becomes a part of its own (`Piece<bone>`), turning with the
  part it sits on. The 93 cars list at most 25 parts in all.
- Not played: the slides of the bones (centimetres), the physical hinge of a `Bodypart` (range, spring, damper, mass), a piece that
  hides instead of falling (it falls as the host's fragment), the global tables above, vertex-level dents (the data has none).

## Not decoded, not verified

- The consumer of the clips and the ladders: no function of the executable that samples the damage list was found; the speed-animation sampler
  (0x009F3250) reads the same format. Whether a piece falls or hides in the 2012 game is not known. <!-- leak-check: allow -->
- Whether the key is the zone damage itself (assumed) or a smoothed value, and the time of the 6 seconds.
- The tables above; `DamageStats`; the words 0 to 3 of a clip; the blocks of the list header.
