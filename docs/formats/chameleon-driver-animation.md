# Chameleon driver animation

How the people in the cars of NFS: Most Wanted (2012) are animated: the skeleton, the 28 clips, how a pose is made from a
clip, how the game picks the clip, and what this repo does with it. Reader: [`libs/chameleon-anim`](../../libs/chameleon-anim)
(skeleton, clips, posing, a two-bone reach) and `libs/chameleon-vehicle/src/driver_rig.rs` (the figure's skin, the driver's
poses). The characters and the car's driver setup are in [chameleon-vehicle.md](chameleon-vehicle.md#the-driver); containers
in [chameleon-bundle.md](chameleon-bundle.md). Evidence tags: [README](../README.md#evidence-tags); **[exe]** is read in
`NFS13.exe` (v1.4.0.0; addresses are given, no code is reproduced).

## Short answer

- The three characters share **one skeleton of 46 bones** (type 0xB2) and **one animation list of 28 clips** (type 0xB0, the
  clips are inside the list, not resources of their own). Both are in `CHARACTERS/ALL_CHARS.BNDL`.
- A clip is a table of **keys at 30 a second**: a packed rotation per rotation track (up to 41 of the 46 bones) and, for
  the hands and feet, two small vector tracks. The rotations are **absolute, small turns in the rest world axes** of each bone.
  Posed that way the hand bones land within 5 mm (1 mm on average) of the hand positions the clips store, on every key of
  every clip **[verified]**.
- The game does not play clips as they are. A state machine of 29 states **[exe]** picks a clip by the car's inputs; the
  **steering** clip (index 3, 31 keys, one second) is **scrubbed**: full left is its first key, full right its last. The
  arms are then reached to the **hand grips on the car's wheel** (two locators the car's driver setup names) **[exe]**.
- The seat belt and harness bones are set from the records of the car's driver setup, not from clips.
- This repo bakes the steering clip at 7 inputs and the long look-back clip into pose meshes ([Used by](#used-by)).

## The skeleton (type 0xB2)

Main block: u16 version (2), u16 bone count (46), u32 offset of the bone table (0x20), u32 offset of a **list of marked
bones** (the executable looks the hands up by name through it), u32 offset of the hash table, u16 number of marked bones (2: bones
14 and 28, the hands). A bone is 0x30 bytes, as in a car's skeleton ([chameleon-hinged-parts.md](chameleon-hinged-parts.md#the-skeleton-type-0xb2)):
f32 x, y, z, pad; an f32 quaternion (x, y, z, w); four i32: parent (-1 for the root), two links, own index. Names are not stored;
the hash is the bit-inverted CRC-32 of the name. A parent always comes before its children **[verified]**.

| Bones | Names (all matched by hash **[verified]**) |
|---|---|
| 0 to 4 | `Root`, `SeatbeltBottomRight`, `SeatbeltBottomLeft`, `SeatbeltTop`, `HarnessTopRight` (the belt and harness anchors, children of the root) |
| 5 to 10 | an unnamed bone (hash `9f79aea0`, the base of the spine), `Spine1`, `Spine2`, `Spine3`, `Neck`, `Head` |
| 11 to 14 | `LeftShoulder`, `LeftArm`, `LeftForeArm`, `LeftHand` |
| 15 to 23 | the left fingers: `LeftHandMiddle1` to `3`, `LeftHandPinky1` to `3`, `LeftHandThumb1` to `3` |
| 24 | an unnamed helper under the left forearm (`1864d8b9`); 38 is its twin on the right |
| 25 to 37 | the same on the right: `RightShoulder` ... `RightHand`, `RightHandMiddle1` ... `RightHandThumb3` |
| 39 to 45 | `Hips`, `LeftUpLeg`, `LeftLeg`, `LeftFoot`, `RightUpLeg`, `RightLeg`, `RightFoot` |

- **Positions are absolute** in the figure's space (x left, y up, z forward, metres), the seated pose (the hips 0.96 m above
  the origin, which is the seat). **Rotations are absolute orientations, not local**: the offset of a child from its parent, taken
  into the parent's frame by the inverse of its rest quaternion, is along +x for every arm, spine and finger bone (the upper arm
  0.241 m, the forearm 0.281 m) **[verified]**. A bone's +x axis is the way to its child.
- The three characters share this skeleton, so the figure with the belt has the harness bone too (a car's setup sets 3 or 4 belt bones).

## The animation list (type 0xB0) and a clip

List main block (201,584 bytes): u16 version (2), u16 count (28), u32 total size, u32 offset of the table of clip offsets
(0x2C0); three tables of 28 eight-byte ids at 0x20, 0x100 and 0x1E0, **all zero** in the file (filled by the loader
**[unconfirmed]**). Each clip follows at its offset; the last ends at the total size.

A clip (offsets from its start) **[verified]** on all 28:

| Offset | Type | Field |
|---|---|---|
| 0x04 | u32 | number of keys (2 to 112) |
| 0x08 | u32 | number of bones of the skeleton (46) |
| 0x0C | u32 | number of rotation tracks (35 to 41) |
| 0x10 | u32 | number of vector (slide) tracks: 2, or 5 in the last clip |
| 0x18 | u32 | number of position tracks (2: the hands) |
| 0x1C, 0x20, 0x28 | u32 | bytes per key of the rotation table (4 per track), the slide table (16 per track) and the position table (16 per track) |
| 0x2C | u32 | size of the clip |
| 0x30 | f32 | **keys per second: 30.0** in all 28 (the executable divides by it: the clip's length is `(keys - 1) / 30` seconds **[exe]**) |
| 0x34 | u16 | rotation format: 1, 32-bit packed |
| 0x36 | u16 | a key index: 0, and 10 for the steering clip (a reference key for use as an addition **[unconfirmed]**; the reading below does not need it) |
| 0x38 | u32 x 9 | offsets of the tables below |
| 0x5C | u32 | the clip's own id (low word; class 1) |

| Table | Holds |
|---|---|
| 0 | 16 bytes per key, **all zero** in the 28 clips |
| 1 | slides: per key and slide track, f32 x, y, z and a pad word; tiny values (3 mm) on the feet |
| 2 | positions: per key and position track, the world position of a **hand** (`LeftHand` then `RightHand`, the order of the marked-bone list) |
| 3 | 48 f32, 1.0 or 0.0 (a per-bone weight, mostly 1.0; not the track mask) **[unconfirmed meaning]** |
| 4 | bone map of the slides: one byte per bone, the track or 0xFF. The feet (42 and 45) in all clips; bones 1 to 3 and the feet in the last |
| 5 | packed rotations: per key and rotation track, a u32 |
| 6 | a bone map with no entry in any clip |
| 7 | bone map of the rotations |

A **packed rotation** is the car animations' format ([chameleon-active-aero.md](chameleon-active-aero.md#the-animation-type-0xb3)): the
top two bits say which component of the quaternion is left out, the three 10-bit fields at bits 20, 10 and 0 hold the others as
`(field - 511) * 0.001382`, and the left-out one makes the quaternion unit. Keys are evenly spaced in time. A rotation track
holds the turn of a bone; a bone with no track keeps its rest.

## Making a pose from a clip [verified]

For each bone, the clip's rotation `d` at the time (a blend of the two keys round it, spherical) is a turn **in the rest world
axes**: the bone's local rotation is `inverse(parent_rest) * d * rest`, its world orientation is its parent's posed
orientation times that, and it sits at its parent's position plus its rest offset (taken into the parent's frame) turned by
the parent's posed orientation. The first two readings tried (a turn in the bone's own frame, a turn in the parent's) put the
hands 6 to 100 cm off the stored positions; this one, 0.1 cm on average (`install_tests::clips_posed_put_the_hands_where_the_clips_say`).
It also reads right: the steering clip rolls the head (about the forward axis) and the spine into the corner, and the look-back
clip turns the head about the up axis by 55 degrees.

A vertex is skinned with four bone indices and four weights (usage 13 and 14 of the vertex, the layout of
[chameleon-hinged-parts.md](chameleon-hinged-parts.md#the-skin)); the indices are bones of this skeleton, the weights add to about 255
(the bytes round): `point' = sum w * (P_posed + R_posed * inverse(R_rest) * (point - P_rest))`. With no turn the figure is exactly the
model (the rest pose is the seated pose).

## The 28 clips

The names are not in the data. What each moves, from the bones that turn more than 8 degrees against the first clip, and from
the machine below. **[unconfirmed]** for the names; the numbers are read.

| Clip | Id (low) | Keys | Seconds | What it moves |
|---|---|---|---|---|
| 0 | 1777ED | 3 | 0.07 | the driving pose, hands on the wheel; the base of every other clip (the steering clip's middle key is the same pose) |
| 1, 2 | 177835, 177837 | 21, 2 | 0.67, 0.03 | right arm to 99 degrees, left arm 67, torso and head 45: a reach with one hand; 2 is its end pose |
| 13, 14 | 177839, 17783B | 21, 2 | 0.67, 0.03 | the same for the other arm (left arm 79, right shoulder 42) |
| 3 | 177B00 | 31 | 1.00 | **steering**: the shoulders and arms 10 to 21 degrees, the spine and head lean; scrubbed by the steering input |
| 4 | 177AFA | 43 | 1.40 | both forearms 44 to 54 degrees, head 25 |
| 5, 15 | 17784E, 177850 | 41, 41 | 1.33 | both arms 70 to 100 degrees, head 47 to 51, torso 25 to 32 (15 pairs with 13, 5 with 1) |
| 6 | 177824 | 31 | 1.00 | the right hand 84 degrees and its fingers (a grab) |
| 7, 8, 10, 11, 12 | 177AAB, 177AA9, 177AA5, 177AA7, 177AA3 | 69, 68, 74, 71, 71 | 2.2 to 2.4 | the head 52 to 64 degrees with the arms 20 to 60: glances and idle motions |
| 9 | 177B02 | 64 | 2.10 | both arms 64 to 91 degrees with the fingers: the hands off the wheel |
| 16 | 17784C | 17 | 0.53 | the fingers of both hands only (a grip) |
| 17, 18 | 17780F, 177811 | 2, 2 | 0.03 | no bone turns more than 8 degrees from the base pose: the poses of two look-around states |
| 19 | 177AAF | 112 | 3.70 | **look back**: the head 55 degrees about the up axis, the right arm and torso turning with it, then back |
| 20 to 27 | 177817, 17781D, 17782A, 1779AD, 1779C7, 1779D3, 177A8D, 22F541 | 2 or 3 | 0.03 to 0.07 | **a pose per car**: the fingers and the arms round that car's wheel, the legs on the pedals (`LeftUpLeg`, `LeftFoot` turn 8 to 23 degrees); the last also turns the belt bones |

The car's driver setup names one of clips 20 to 27 at +0x50: 1779C7 in 62 of the 93 cars, 177A8D 8, 1779AD 10, 1779D3 4, 22F541 5,
177817 1, 17781D 2, 17782A 1 **[verified]**.

## How the game uses them [exe]

The driver of a car is drawn by an animation job per player (the job builder at 0x00538270 <!-- leak-check: allow -->). Its steps:

1. **The grips.** From the driver setup it takes two locators of the car's own spec: the **grip group** (+0x30; two locators, the
   left and right hand on the wheel) and the **shifter group** (+0x2C; one locator, the gear lever, 0 in 28 cars)
   (0x00534990 <!-- leak-check: allow -->). They are the locator groups of [chameleon-lamps.md](chameleon-lamps.md#locator-groups-verified)
   under the ids the setup holds (a group per car). The locators are on the wheel: the middle of the two is within 20 cm of the
   car's `SteeringWheel` bone and they are 31 to 50 cm apart in all 91 cars that have them **[verified]**. From the pair it builds
   a circle (centre, radius, plus the f32s at +0x34 and +0x38 of the setup: 0.05 and 0.095).
2. **The clip.** The state machine (0x0047e430 <!-- leak-check: allow -->, 29 states in one switch) loads the **first 20 clips into
   slots** (slot n is clip n), steps the clock of the slot(s) of the current state, and appends commands to the job: play slot n
   with a weight and a time, play one of the car's pose clips (slot 20 + i), set the belt bones, reach the hands.
3. **The inputs** it reads are a 0x1C-byte record per player: the speed (f32), the **steering** (f32, -1 to 1), two more f32 and
   about eight flag bytes. The state 1 (driving) leaves for another state by the flags (a gear change, three looks) and the
   steering.
4. **Steering (state 2).** Slot 3 is scrubbed: the target time is `(steering * 0.5 + 0.5) * (end - start)`, the clip's own
   time moves toward it at one clip-second per second (so lock to lock takes one second at the fastest), clamped to the clip. Full
   left is key 0, full right key 30. The command played is slot 3 at weight 1.
5. **A look** (flags 0x81, 0x82, 0x83 of the machine): slots 17, 18 and 19, the last being clip 19, the long look back.
6. **Alternates.** A flag at +0x7E of the machine switches the three clip indices the shift and reach states use from (1, 2, 5)
   to (13, 14, 15): the same motions for the other arm (or the other side of the car) **[unconfirmed which]**.
7. **The belts.** For each belt record of the setup (+0x60, 0x30 bytes: an offset f32 x3 at +0x10 and three angles in degrees at +0x20),
   commands set a rotation (the angles times pi/180) and a position on the belt bones, found by the hashes of their names
   (the harness bone's only when the setup has four). The records of the 93 cars are mostly zero.

## Used by

- `chameleon_anim::Skeleton::pose` and `Clip::rotation` make the pose; `ik::reach` turns the shoulder and elbow so a hand is on a
  target; `Skeleton::skin_point` skins a vertex.
- `chameleon_vehicle::load_rig` reads the skeleton, the list and the skin of every vertex of the character;
  `DriverRig::pose(DriverPose::Steering(s), grips)` is step 4 with the arms reached to the grips (step 1) and
  `DriverPose::LookBack` is clip 19 at its widest (key 50, the head turned 52 degrees); `DriverRig::posed` moves the figure's
  vertices. `driver_grips` reads the two grips from the car's spec. Not done: the clock's one-second rate (the host steps from the
  input), the state machine's other clips, the per-car pose clips (the arms are reached to the grips instead), the shifter locator
  and the belts.
- The 2012 content mod bakes seven steering poses and the look back into solids
  ([mods/nfsmw-2012-content/README.md](../../mods/nfsmw-2012-content/README.md#the-drivers-poses)); the host draws the one the
  steering is nearest to ([car-customization.md](../modding/car-customization.md#the-drivers-poses)).

Install tests (`CHAMELEON_GAME_DIR` or `MW2012_GAME_DIR`, `--ignored`): `cargo test -p chameleon-anim -- --ignored` (46 bones, 28 clips at
30 a second, the hands within 5 mm of the stored positions on all keys), `cargo test -p chameleon-vehicle driver_rig -- --ignored`
(skin weights, the rest pose reproduces the model within a millimetre, the hands reach the grips of all 91 cars to 0.3 mm at both locks,
the head travels 2 mm to 12 cm between the locks).

## Not decoded, not verified

- **The names of the clips**, and which flag of the machine each state tests: the input record is built by code that was not
  found. The roles above are from the bones.
- **The reference key** (u16 at +0x36, 10 in the steering clip) and tables 0, 3 and 6 of a clip; the per-clip weights.
- **The per-car pose clips and the circle.** The game applies the clip of +0x50 and the hands move round the wheel circle as the wheel
  turns; the hands here are reached to the fixed grips instead, since the wheel of the converted car does not turn.
- **The belt records**: the command layout is read, the rotation order and frame are not.
- **The rate**: one clip-second per second is the executable's, at the physics tick; the host draws the pose of the steering
  input of the frame.
- Whether a pose looks right in a running game: nothing was drawn.
