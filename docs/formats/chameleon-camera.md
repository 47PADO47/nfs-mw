# Chameleon cameras: what a 2012 car says about its gameplay cameras

Where NFS: Most Wanted (2012) keeps the data of its chase, bonnet, bumper and rear-view cameras, per car, and how
[`libs/chameleon-camera`](../../libs/chameleon-camera) reads it and turns it into a camera that follows a car. The
containers are in [chameleon-bundle.md](chameleon-bundle.md); how Genesys types and objects are laid out is in
[chameleon-vehicle-stats.md](chameleon-vehicle-stats.md#genesys-types-and-objects-verified). How this was learned:
[provenance/chameleon-camera.md](../provenance/chameleon-camera.md). Evidence tags are those of
[README.md](../README.md#evidence-tags): **[verified]** is measured on the install (v1.4.0.0) with a scratch script or
the Rust reader, **[exe]** is read in `NFS13.exe`, **[unconfirmed]** is a reading that fits the values and is not proved.

## Short answer

* **Layout [verified].** Every car bundle (`VEHICLES/VEH_<n>_HI.BNDL`, 93 of 93) holds one Genesys object with a
  `Genesys.Gen.VehicleCameraContainer`: the chase camera (`CameraGameplayExternal`), the bonnet camera
  (`CameraGameplayBonnet`), the bumper camera (`CameraGameplayBumper`), the shared globals
  (`CameraGameplayExternalGlobals`), the gradient adjustments and the rear-view camera (`CameraRearView`). The shared
  `VEHICLES/TRAFFICATTRIBS.BNDL` holds eleven more containers, one per traffic vehicle. The type layouts are the same
  in all of them.
* **Field names are hashed** (the function is not known, see the provenance doc), so fields are addressed by offset and
  the meaning of each number is **[unconfirmed]**, inferred from the structure names (`Yaw_spring_modification`,
  `Camera_Roll`, `Deceleration_Pitch_Change`, `Speed_based_height_change`, `High_Speed_Shake`, `Impact_shake`,
  `LensProperties`), the values, and how they vary between cars.
* **The camera code was not found in `NFS13.exe`.** Ghidra was loaded and the camera entity's script bindings were
  found, not its update (see provenance). So the **behaviour model** in the crate is built from the data alone: its
  shape follows the data's structure names, every mapping from a number to a term is a reading, and the doc below marks
  each. The *data* is exact; the *motion* is a plausible camera that uses it.
* **What varies per car:** the chase position and look-at point, the speed movement, the height change, the roll, the
  deceleration pitch, the shake amplitude and speed, and (in 30 cars) the bumper placement. What is shared by all 93:
  the globals (impact shake, lens), the gradient adjustments, the yaw spring, the rear view and the field-of-view pairs
  (the SWAT van excepted).

## Axes and units

The file uses the mesh convention of the 2012 game (x left, y up, z forward, see [chameleon-gfx.md](chameleon-gfx.md)).
The crate's host space is the mod ABI's `BbVec3` space: right-handed, **x forward, y left, z up**, metres, seconds,
degrees. The reader converts every position and per-axis triple once (`from_data_axes`, a cyclic permutation:
`host = [data.z, data.x, data.y]`, so handedness is kept). Nothing downstream converts. A chase camera 4.6 m behind and
1.6 m above a car is `(0, 1.6, -4.6)` in the file and `[-4.6, 0, 1.6]` in the crate. The three **rotation-axis triples**
of `Impact_shake` are kept in the file's order `[pitch, yaw, roll]` (they are not positions). Speeds the file gives are
miles per hour **[unconfirmed units, fit by the Tesla Roadster: 125 mph is its limit]**.

## The types [verified]

Offsets are in the structure, counting its 12-byte head (two zero words and the type hash); sizes are the type's.
`inline` means the structure sits at that offset inside its parent; `ptr` means the field is a 4-byte pointer (the
type's alignment field is 0x104, which marks pointers) that the loader patches. In the object, every structure is also
listed in the import table with its absolute offset, which is how the reader finds them.

| Type | Size | Fields (offset: kind) |
|---|---|---|
| `VehicleCameraContainer` | 32 | `+0x0C` id (u32, the object's key); `+0x10` ptr bonnet; `+0x14` ptr bumper; `+0x18` ptr external |
| `CameraGameplayBonnet`, `CameraGameplayBumper` | 128 | `+0x0C`, `+0x18`, `+0x24` three floats each; `+0x30` id; `+0x34` to `+0x6C` fifteen floats; `+0x70`, `+0x74` two u32; `+0x78`, `+0x7A` two u16 |
| `CameraGameplayExternal` | 392 | `+0x0C` `Speed_based_movement` (72, inline); `+0x54` `Acceleration_movement` (60); `+0x90` `Camera_Roll` (44); `+0xBC` `Yaw_spring_modification` (44); `+0xE8` `Deceleration_Pitch_Change` (36); `+0x10C` `Speed_based_height_change` (24); `+0x124`, `+0x130`, `+0x13C`, `+0x148` three floats each; `+0x154` u64; `+0x15C` id; `+0x160` to `+0x170` five floats; `+0x174` ptr globals; `+0x178` ptr gradient adjustments; four u16 at `+0x17C`; u8 at `+0x184` |
| `…External.Speed_based_movement` | 72 | `+0x0C` `High_Speed_Shake` (36, inline); `+0x30` id; `+0x34`, `+0x38`, `+0x3C`, `+0x40` floats |
| `…Speed_based_movement.High_Speed_Shake` | 36 | `+0x0C` id; `+0x10`, `+0x14`, `+0x18`, `+0x1C` floats |
| `…External.Acceleration_movement` | 60 | `+0x0C` id; ten floats `+0x10` to `+0x34` |
| `…External.Camera_Roll` | 44 | `+0x0C` id; six floats `+0x10` to `+0x24` |
| `…External.Yaw_spring_modification` | 44 | `+0x0C` id; six floats `+0x10` to `+0x24` |
| `…External.Deceleration_Pitch_Change` | 36 | `+0x0C` id; four floats `+0x10` to `+0x1C` |
| `…External.Speed_based_height_change` | 24 | `+0x0C` id; one float `+0x10` |
| `CameraGameplayExternalGlobals` | 144 | `+0x0C` `Impact_shake` (96, inline); `+0x6C` `LensProperties` (28, inline); `+0x88` id |
| `…Globals.Impact_shake` | 96 | three float triples `+0x0C`, `+0x18`, `+0x24`; `+0x30` id; nine floats `+0x34` to `+0x54`; two u16 |
| `…Globals.LensProperties` | 28 | `+0x0C` id; `+0x10`, `+0x14` floats |
| `CameraGameplayGradientAdjustments` | 68 | `+0x0C` and `+0x24` two `Params` (24 each, inline); `+0x3C` id |
| `…GradientAdjustments.Params` | 24 | `+0x0C` id; `+0x10` float |
| `CameraRearView` | 40 | `+0x0C` three floats; `+0x18` id; `+0x1C` float; `+0x20` ptr rear-view globals |
| `CameraRearViewGlobals` | 32 | `+0x0C` id; `+0x10`, `+0x14`, `+0x18` floats |
| `CameraGameplayBonnet.Wind_sound` | 20 | `+0x0C` id only (no car references it) |

An *id* is the 32-bit key of a Genesys object (the `Id` member every structure has): non-zero for a root structure and 0
for a nested one. The `CameraRearView` has no pointer from the container; it sits in the same object, next to the
others, and the reader pairs them by that. The 352 types of a bundle repeat in the sound half; the first copy counts.

`CameraGameplayShakeEffect` (with `Rotation` and `Translation`, three `AxisParams` each), the `CameraTransform*` process
nodes and the `CameraSequenceItem`s are the feedback and cutscene cameras (`GAMELOGIC/WEAPONS`, `EN_US/FEEDBACKGROUPS`);
`CAMERAS.BNDL` holds 45 `CameraTake` cutscene resources. None is a gameplay camera and none is read by the crate.
Each car bundle holds two `CameraGameplayShakeEffect`s, one inside the container's object and one as an object of its
own; all 186 copies are identical (the first rotation axis 250, 0.4, 20, -20, 450, the second and third 0, 1, 1, -1, 1; the
translation axes 0, 0.1, -1, 1, 1, 1 / 0, 1, 0, 1, 1, 1 / 100, 0.5, -0.2, 0.3, 100, 0.2). The chase camera's u64 at
`+0x154` is 0 in all cars, so what refers to them is not known. The crate does not read them.

## The values

Tables give the Porsche 911 Carrera S (`VEH_1085007`), then the spread over the 93 cars. `host` is the crate's field.

### Chase camera: `CameraGameplayExternal`

| Offset | `ExternalCamera` field | Porsche | All 93 cars | Reading |
|---|---|---|---|---|
| +0x124 | `response` | 0.55, 0.09, 1.0 (file axes) | same in all | **[unconfirmed]** per-axis response; not used by the model |
| +0x130 | `focus` | (0, 1.2, 0.75) | up 1.0 to 1.6, forward 0.53 to 1.26 (59 values) | the look-at point relative to the car: about 0.75 m ahead and 1.2 m above the origin |
| +0x13C | `ground_pivot` | (0, 0, 0.75) | same forward as `focus`, up 0 | the focus on the ground; role **[unconfirmed]**, not used |
| +0x148 | `position` | (0, 1.6, -4.6) | up 1.45 to 3.0, back 4.0 to 6.1 (4.6 in 76 cars) | the rest position relative to the car; the follow distance and height |
| +0x160, +0x164 | `fov_deg` | 95, 70 | same in all | two fields of view, see below |
| +0x168 | `param_168` | 10 | 10 in 92, 5 in the SWAT van | role unknown |
| +0x16C | `param_16c` | 4 | 4 in all | role unknown |
| +0x170 | `lag_s` | 0.055 | 0.045 to 0.07 (5 values) | read as a short lag in seconds; not used |

`Speed_based_movement` (`speed_movement`): `+0x34` `back_at_speed` -1.0 (-1.0 in 84 cars; -1.2, -0.9, -0.8, -0.75,
-0.6, 0), `+0x38` `height_a` 1.5 (1.35 to 1.85), `+0x3C` `height_b` 1.7 (1.45 to 3.0), `+0x40` 0 in all.
`height_a` and `height_b` bracket `position` height in most cars (1.5 and 1.7 around 1.6; 1.525 and 1.725 around 1.625)
and `height_b` equals it in several (1.7, 3.0 in the SWAT van, 1.9 in `122713`); the crate does not use them.
`High_Speed_Shake`: `+0x10` amplitude 1.2 (1.3 in ten cars), `+0x14` rate 1.0 in all, `+0x18` 180 (125, 150, 160, 185,
200, 208 in the others: seven values), `+0x1C` 0 in all. The `+0x18` value is a speed in mph that scales with the car
(125 for the Tesla Roadster whose limit is 125) **[unconfirmed]**.

`Speed_based_height_change` `+0x10` (`height_at_speed`): 1.59 (1.35 to 1.7, 18 values). Reading: the camera height at
full speed **[unconfirmed]**; a value near the rest height is a camera that stays, 1.5 under the SWAT van's 3.0 is one that
comes down.

`Acceleration_movement` (`accel_movement`): `-2, 0.5, 2.5, 0, 50, 1.5, -1.2, 2, 0, 1.75` at `+0x10` to `+0x34`. `+0x10`
is -2 (-1.9, -2.5), `+0x30` is 0 in 59 cars and -0.9 in 34; the others are the same in every car. Readings
**[unconfirmed]**: pitch in degrees at full acceleration; a smoothing time in seconds; the acceleration (mph per second)
that counts as full; metres the camera drops back at full acceleration and moves forward at full braking.

`Camera_Roll` (`roll`): `2.5, 2.5, 30, 4.7, 0.2, 10`. Spread: `+0x14` 2.5 / 2.6 / 3.0, `+0x18` 30 / 35 / 40, `+0x1C`
4.7 (80 cars), 3.0, 5.6 and 6.0. Readings: the speed (mph) where the roll is on in full, the largest roll in degrees at
full steering, a smoothing time in seconds.

`Yaw_spring_modification` (`yaw_spring`): `0.01, 10, 100, 0.05, 10, 0`, identical in all 93 cars. Readings: a speed
range 0.01 to 100 m/s over which the spring's rate goes from the first 10 to the second 10 (so constant), a 0.05 s lead
along the yaw rate.

`Deceleration_Pitch_Change` (`decel_pitch`): `12, 4.5, 2, 2.25`. `+0x10` is 12 (11 in four cars), `+0x14` 4.5 (80 cars; 5.0,
6.0, 2.5). Readings: the fastest pitch change (degrees per second), pitch in degrees at full braking, a follow rate.

**Field of view.** Pairs of numbers: chase 95 and 70, bonnet 105 and 95, bumper 120 and 95 (the SWAT van's bonnet 100 and
100), rear view 60, `LensProperties` 90 and 1.4. **[unconfirmed]** The crate reads them as *horizontal* degrees at 16:9
and widens the view with speed from the smaller to the larger: chase 70 to 95 is 43 to 64 degrees vertical, a normal
chase camera; read as vertical they would give 95 degrees at speed and 120 for a bumper camera, which is not a view. The
`CameraOptions::aspect` field sets the aspect used for the conversion.

### Bonnet and bumper: `CameraGameplayBonnet`, `CameraGameplayBumper`

| Offset | Field | Bonnet (92 cars) | Bumper (63 cars) |
|---|---|---|---|
| +0x0C | `response` | (0.5, 0.7, 1.0) | same |
| +0x18 | `eye` | (0, 1.3178, 0.2077) | (0, 0.6, -1.5); 25 cars (0, 0.5, -1.55); 4 cars (0, 0.7, -1.75) |
| +0x24 | `target` | (0, 1.05, 0.8) | (0, 0.425, 1.5); 25 cars (0, 0.35, 1.4); 4 cars (0, 0.5, 2.15) |
| +0x34, +0x38, +0x3C | `smoothing_s`, `sway_m`, `wind_shake` | 0.3, 0.05, 0.25 | same |
| +0x58, +0x5C | `look_limit_deg` | 15, 15 | 0, 0 |
| +0x64, +0x68 | `fov_deg` | 105, 95 | 120, 95 |

Both are in file axes in the table (host axes in the crate). **The values are not car geometry [unconfirmed]**: 92
bonnets and 63 bumpers are identical on a Porsche and a pickup, an eye 1.32 m above the origin is above a sports car's
roof, and a bumper eye at z = -1.5 is behind the middle of the car. The four larger cars (`1085958`, `1467164`, `300097`,
`510072`) are the ones with the farther bumper target. So the numbers are offsets from a per-car anchor (likely a
locator or bone of the model) that this data does not name; `MountedRig::new` and `CameraOptions::bonnet_anchor` take
that anchor from the host. **The SWAT van (`1467242`) is the exception:** its bonnet is (1.617, 0.66), target (1.532, 0.11),
fov 100 and 100, and its *bumper* holds the numbers every other car has on its bonnet.

The `+0x40` to `+0x54`, `+0x60` and `+0x6C` floats (bonnet 0.15, 0.05, -1, 5, 0.1, 0, 0, 3; bumper 0.2, 0.25, -0.15, 0, 0.25,
0, 0, 3) are in `MountedCamera::other`; roles unknown. `Wind_sound` is a type with no member data and no user.

### Rear view: `CameraRearView`

`+0x0C` (0, 0.9, 0), `+0x1C` 0, and in the globals `+0x10` 3.3333, `+0x14` 400, `+0x18` 60, identical in 93 cars.
**[unconfirmed]** 3.3333 is the aspect ratio of the mirror image (400 pixels wide, 120 high) and 60 its field of view;
the first triple is the camera's offset, read as a height of 0.9 m.

### Globals and gradient adjustments

`Impact_shake`: frequencies (10, 10, 50), amplitudes (1, 1, 1), decay times (0.5, 0.5, 0.5), then 0.75, 0.165, 1, 60, 2,
30, 4, 5, 1, identical in all cars. Readings **[unconfirmed]**: per rotation axis (pitch, yaw, roll) the angular
frequency in rad/s, the amplitude in degrees and the decay time constant; the first scalar (0.75 s) as the duration.
`LensProperties`: 90 and 1.4. `GradientAdjustments`: two `Params` of 0.5 each, read as the share of the car's pitch that
the chase camera takes on when the car climbs and descends.

## The model **[unconfirmed]**

`CameraState` holds a `ChaseCamera`, two `MountedRig`s and a `RearViewRig`. The host builds it from `CarCameras`, calls
`reset(&CarMotion)` when the car appears, and `step(&CarMotion)` once per rendered frame after the car has moved; it
gets a `CameraPose` (position, forward, up, vertical field of view, a 0..1 speed shake). `dt` is cut to 1/20 s inside.
`CarMotion` holds the car's position (ground reference point), forward, up, velocity, speed, steer (positive left),
throttle, brake, yaw rate, grounded, impact (0..1) and dt. Everything below is a reading; **no term is from the
executable**. The speed fraction `s` is `mph / full_speed_mph` of `High_Speed_Shake` (0..1).

| Effect | Formula (host axes, heading frame) | Data used |
|---|---|---|
| Heading | critically damped spring (exact solution, stable for any `dt`) towards the car's heading plus `lead_s * yaw_rate`; rate `lerp(rate_low, rate_high, speed ramp)`; in the air the direction of travel replaces the car's heading | `yaw_spring` |
| Rest pose | camera at `position` rotated by the heading, looking at `focus` | `position`, `focus` |
| Pull-back | `x += back_at_speed * s` | `speed_movement.back_at_speed` |
| Height | `z = lerp(position.z, height_at_speed, s)` | `position`, `height_at_speed` |
| Acceleration | filtered acceleration over `full_accel`, clamped to -1..1: `-back_m * a` back when accelerating, `forward_m * -a` forward when braking, smoothed by `smoothing_s` | `accel_movement` |
| Pitch | towards `-max_deg * brake * ramp(speed, 0, 5 m/s) + pitch_deg * max(a, 0)`, at most `rate_deg_s`, with rate `response`; plus the car's own pitch times the gradient share (climb or descent) | `decel_pitch`, `gradient` |
| Roll | towards `steer * max_deg * ramp(mph, 0, full_speed_mph)` degrees, positive tilting the up vector left, smoothed by `smoothing_s` | `roll` |
| Field of view | `lerp(small, large, s)` horizontal at the option's aspect, converted to vertical | `fov_deg` |
| High-speed shake | three sines on a rate `base_hz * rate`, amplitude `amplitude * shake_scale_deg * ramp(mph, start, full)^2` degrees | `speed_movement.shake`, options |
| Impact shake | on `impact > 0`: per axis `amplitude * strength * exp(-t/decay) * fade * sin(frequency * t)` degrees for `scalars[0]` seconds | `globals.impact_shake` |
| Mounted | rigid in the car's frame at `eye` + anchor, looking at `target` + anchor; moved back by `sway_m * accel/10` under acceleration; field of view as above; `wind_shake(speed)` is `wind_shake * s^2` | `MountedCamera` |
| Rear view | `rear_extent_m` behind the car, `offset.z` above the origin, looking back; field of view 60 horizontal at the mirror aspect | `RearView` |

Constants **not in the data** (so options with defaults in `CameraOptions`): the screen aspect (16:9), the degrees of
the shake per unit amplitude (0.25), its base frequency (8 Hz), the acceleration filter (0.15 s), the car's rear extent
(2.2 m), the shake's axis phases (fixed offsets), the 5 m/s ramp of the braking pitch and the 10 m/s2 of the mounted
sway. Left unused because no reading was found: `response`, `ground_pivot`, `lag_s`, `height_a`, `height_b`,
`param_168`, `param_16c`, the unnamed `param_*` fields, `LensProperties`, and the mounted `smoothing_s`, `look_limit_deg`
and `other`.

## Per car

Measured on the 93 car bundles (script in the provenance doc). "Distinct" counts values over the 93 cars.

| Field | Distinct | Range | Most common |
|---|---|---|---|
| chase up (`position`) | 17 | 1.45 to 3.0 | 1.6 (31 cars) |
| chase back | 12 | 4.0 to 6.1 | 4.6 (76) |
| focus up / forward | 15 / 59 | 1.0 to 1.6 / 0.53 to 1.26 | 1.2 (31) / 0.765 (5) |
| `lag_s` | 5 | 0.045 to 0.07 | 0.055 (45) |
| `back_at_speed` | 7 | -1.2 to 0 | -1.0 (84) |
| `height_at_speed` | 18 | 1.35 to 1.7 | 1.4 (16) |
| `full_speed_mph` of the shake | 7 | 125 to 208 | 180 (43) |
| shake amplitude | 2 | 1.2, 1.3 | 1.2 (83) |
| roll max degrees | 4 | 3.0, 4.7, 5.6, 6.0 | 4.7 (80) |
| decel pitch max degrees | 5 | 2.5 to 6.0 | 4.5 (80) |
| `accel_movement` +0x30 | 2 | 0, -0.9 | 0 (59) |
| bumper eye back | 4 | -1.75 to 0.208 | -1.5 (63) |

The cars that depart from the usual cluster in the heavy and long bodies (`1085958`, `1467164`, `300097`, `510072`: roll
and braking numbers, farther bumper) and the SWAT van `1467242` (back 6.0, up 3.0, its own bonnet, bumper and shake
numbers). `122713` (back 6.1, up 1.9) is the third long chase. Cars that are variants of one model
(`2255901` to `2255934`, `2277210` to `2277252`) share every value. The 30 cars with a non-default bumper are 25 with
(0.5, -1.55), 4 with (0.7, -1.75) and the van.

## Not decoded

* The camera update, blend between views, the look-around of the bonnet and the transitions: `NFS13.exe`'s code was not
  found (the entity bindings `GameplayExternalCameraTransform`, `GameplayExternalFOV`, `IsCockpitCam` and
  `GameplayExternalImpactShakeAngles` are strings **[exe]**).
* The meaning of the unnamed floats listed above, and the real units of the speeds.
* The cutscene cameras (`CAMERAS.BNDL`, `CameraTake`) and the feedback shakes.

## Checks

`cargo test -p chameleon-camera` runs the synthetic reader tests and the model tests. With `CHAMELEON_GAME_DIR=<2012
install>`, `cargo test -p chameleon-camera -- --ignored` reads all 93 cars (every one has the three cameras with values
in the ranges above), checks what the cars share, the Porsche's numbers, the eleven traffic containers, and drives the
model for every car and view.
