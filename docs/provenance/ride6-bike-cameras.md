# Provenance: the cameras of a bike

Where each number of `docs/specs/ride6-bike-cameras.md` comes from. The data is the user's own RIDE 6 install; none of it is in the
repository.

## Data

- **Camera containers.** The physics dump (`C:\r6\physdump-all` here; `--physics` of `ride6-bike-import`) holds, below
  `resolved/content/assets/data/cameras/`, the Blueprint class of each bike's cameras in `Containers/<class>/<Vehicle>.json` (344 files),
  each class's base (`GameBaseCAmeraContainer_<Class>.json`, 14 files) and the root base (`Containers/GameBaseCAmeraContainer.json`),
  plus `CamerasCurve/` (acceleration, deceleration, gamma, noise modulation and shutter curves, not used). A file is the JSON dump of the class
  and its default object's components: each object has `Name` (`<Component>_GEN_VARIABLE`), `Properties` and `OwnProperties`. The values
  read are listed in section 1 of the spec. A bike's own file has only what the bike changes, so a value is the first of the bike's, the
  class base's and the root's.
- **Skeleton frame.** The positions are in the frame of the bike's skeleton, as `RIDER/contacts.json` (`hubs`, the seat) is; the model of
  the game is that frame moved by the mean difference of the two hubs (`attach.json` `frame_offset`; KTM 450 SMR: 1.6 mm forward,
  79.7 mm up). That the containers are in the skeleton's frame, and not on the ground under it, is an assumption [C]: the
  difference is 8 cm on the KTM and is not told by the data. [M] The stock rider's eye bones (the probe of `rider/probe_tests.rs`, the
  rider at 140 km/h with no tuck) are at x 0.31 and z 1.45 m of the Panigale's skeleton frame and x 0.08, z 1.55 m of the KTM's, against
  the containers' 0.06 and 1.20 (Panigale) and -0.15 and 1.50 (KTM): the container's helmet point is a fixed camera of a tucked rider; the
  game uses the live rider's eyes (which follow the tuck) and the container's point only for a bike with no rider.
- **Rider rig.** The eye bones `eye_l` and `eye_r`, the head bone `head` and the meshes `helmetRacing_default` and
  `visorClosedRacing_default` are those of the stock rider the rider converter writes (`mods/ride6-bikes/src/rider/parts.rs`,
  `docs/formats/ride6-rider.md`).

## Reading the data

The ambiguities, with what was done:

- `RelativeRotation.Pitch` of the helmet camera is -10 for every Racing, Sport, naked and bagger bike, -20 for two scooters, absent
  (zero) for most enduros and some motards, and -50 or -60 for the supermotard class and two of its bikes. The class base
  (`GameBaseCAmeraContainer_Supermoto.json`) holds the -60. It cannot be a look direction relative to a bike's axis (the road would be
  out of the picture), and nothing in the data says what it is measured from; the importer caps the tilt at 10 degrees down
  (`MAX_HELMET_DOWN_DEG`).
- `GameHelmetCameraBehavior` has `FollowCameraRoll` true and `MaxAngle` 5; a second one (`GameHelmetCameraBehavior1`) has 2.5.
  Which camera each belongs to is not in the data (an SCS node lists them in order, with no parent). The first is taken for the helmet
  (it also carries `DeactivateWidget`); the reading of `MaxAngle` is in section 6 of the spec.
- The field of view is `IgnitionCameraComponent.FieldOfView`: Unreal's horizontal field of view at the screen's aspect; 16:9 is
  assumed.
- `FrontCamInternalCameraBehavior` has no rotation: RIDE 6's front camera looks ahead. The spec's front camera looks back (the
  request) from a place of its own choosing.
- The 26 bikes with `FrontCamNoGyroInternalCameraBehavior` (roll factor 0, z 78 to 84 cm) have it for `FrontCamReplay`; the replay
  camera's data is not used.

- The helmet camera's `Lean` slots are the two `Lean` entries of `HelmetCameraModifierBehaviorVariable.Slots` (the bike's own
  array when it has one, as the class or root array otherwise; an array is replaced whole, not merged by slot). The root has 25 to 70
  degrees with 28, 16, -35 cm and pitch -5, and 30 to 70 with yaw 20 and roll 12. 81 of the 344 containers carry their own; the
  Panigale V4 R has 30, 15, -35 and the same turn, the KTM 450 SMR 20 to 70 with 10, 15, -10 and pitch -3, and 0 to 70 with yaw 3 and
  roll 8. The slot's input is `CameraModifierInputType::Lean`, its range `InputMin`/`InputMax`. The data has no description of the
  frame of the offsets, the sign of a lean, or whether `GameHelmetCameraBehavior.MaxAngle` caps the slot's roll too; the choices are
  in section 6.1 of the spec. The other slots of that behaviour (`Velocity`: a field of view of +20 over 250 km/h and 10 cm ahead
  below it; `Query`: the brake, 15 cm back and 10 up; the base behaviour's `Wheelie` and the tuck, a `QueryClampedRemap` of
  `BikeInputHandler.TuckInLevel` faded out by `CurrentLeanAnglePercentage` between 0.8 and 0.4: 15 ahead, 13 down, pitch -8) are not
  used: the live rider's eye already holds his tuck.
- That the helmet camera's mount does not roll with the bike is the reading of `RollFactor` 0 with the 5 degree `MaxAngle`
  (`GameHelmetCameraBehavior`): nothing else in the containers places the camera, so the container's point is a place in a frame whose
  roll is the camera's own. Spec section 6.1.
- The helmet look (spec 6.2): `GameBaseCAmeraContainer.json` has `HelmetIgnitionSideViewCameraBehavior` with `YawModifier`
  (`AxisName` `LookRight`, `OffsetLimits` -11 to 11), `PitchModifier` (`LookUp`, -10 to 10), `RotatorFilterSpeed` 4 and
  `RotateSpringArm` false; `CockpitIgnitionSideViewCameraBehavior` is the same with 13 and 10. Only `Sport/BMW_M-1000-RR-2023.json`
  overrides it (filter speed 5, the same limits). No container has a free-look, head or pan object besides these, and none
  has a side view for the front camera. How the filter speed filters is not in the data: spring and exponential are both
  possible; the port takes the exponential approach (Unreal's `RInterpTo`, whose step is the speed times the frame time).
  The limits and the speed are host constants (`viewer/camera/head_look.rs`), not table values: the data is the same for every bike.
- The helmet's straight tilt: `HelmetInternalCameraBehavior.RelativeRotation.Pitch` is -10 on 312 of 344 containers, absent on 23
  and -5, -7, -15, -20, -50 and -60 on 9; `PitchFactor` is 0.25 on 339 and a share of the bike's pitch, not a tilt. Nothing in the
  data looks further down for a normal bike, so the 8 degrees more (`HELMET_VIEW_DOWN_DEG`) are the user's request and a modelling
  choice (spec section 5).

## Tools

`ride6-bike-import cameras <exporter output> <mod folder> --physics <dump>` writes the `[bike.camera]` tables of bikes already imported
(from the manifest's `[wheels]` and the car's `RIDER/contacts.json`); a new import writes them after the contact points
(`pipeline.rs`). A bike whose container is not found is listed and keeps no table.
