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

## Tools

`ride6-bike-import cameras <exporter output> <mod folder> --physics <dump>` writes the `[bike.camera]` tables of bikes already imported
(from the manifest's `[wheels]` and the car's `RIDER/contacts.json`); a new import writes them after the contact points
(`pipeline.rs`). A bike whose container is not found is listed and keeps no table.
