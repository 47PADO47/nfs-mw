# RIDE 6 rider files

The formats the RIDE 6 rider travels in: the two intermediate files the user's export tool writes from their own install
(`R6RA` animations, `R6RM` skinned meshes, plus two skeleton JSON files), and the runtime files `ride6-rider-import`
(`mods/ride6-bikes/src/rider/`) turns them into. None of these files is committed: they hold the user's game data and
live in the mod's git-ignored `files/` folder ([licensing.md](../licensing.md)). The behaviour is in
[ride6-rider-animation.md](../specs/ride6-rider-animation.md); the runtime that plays the pack is
[skeletal-animation.md](../specs/skeletal-animation.md).

## Where the data comes from

RIDE 6 is Unreal Engine 5.6.1. Its `.utoc`/`.ucas` containers open without a key and need a `.usmap` mapping. The export
tool is a small C# program on top of CUE4Parse (outside the repository; its source is the user's scratch tool
`R6ToolRider`). It has six modes: `skeleton` (a `USkeleton` to JSON), `anims` (a list of `UAnimSequence` packages to one
`R6RA`), `acl` (the ACL-compressed sequences of a list, raw, below), `mesh` (a `USkeletalMesh` to `R6RM`), `textures` (PNG) and `bp` (a Blueprint class to pseudo-C++, used to read the
animation blueprint). Animations are decoded by CUE4Parse and **sampled at every source frame**; the tool does not
invent keys.

Assets used (all under `ride6/Content/assets/graphics/characters/riders/`):

| What | Package |
|---|---|
| Skeleton (81 bones, 9 sockets) | `riderShared/bodies/skeletons/suit_game_Skeleton` |
| Suit | `riderShared/bodies/skeletal_meshes/suit001_male` (6 LODs, 4,458 vertices at LOD 0) |
| Arms and hands | `riderShared/bodies/skeletal_meshes/SuitRacing_Male_arms` (1 LOD, 7,423 vertices) |
| Boots | `boots/boots001/skeletal_meshes/boots001` (6 LODs, 2,130 vertices) |
| Helmet and visor | `riderShared/default_rider/skeletal_meshes/helmetRacing_default`, `visorClosedRacing_default` (6 LODs each) |
| Pictures | `suits/suit001/textures/T_suit001_BC`, `boots/boots019/textures/boots001_d`, `gloves/gloves002/textures/T_gloves002_BC`, `riderShared/default_rider/textures/helmetDEfault_d`, `visorDefault_d` |
| Animations | `riderShared/Animations/motard/**`, `BikeSpecificCorrection/*`, a few `race/` clips |

The choice of pieces is the game's stock rider: `default_rider/customization_data/Default_Suit|Boots|Gloves|Helmet|Visor`
and the male suit data `bodies/customization_data/suit001_male` (`SuitMeshDataSource`: `SuitMesh`, `ArmsMesh`).

## Axes and units of the intermediate files

Unreal's: **x forward, y right, z up, left-handed, centimetres**; rotations are quaternions `(x, y, z, w)` that rotate a
vector by the usual sandwich product on those coordinates. The importer converts to the car frame (x forward, y left,
z up, metres): position `(x, -y, z) / 100`, quaternion `(-x, y, -z, w)`, triangle winding flipped.

## `R6RA`: animations

Little endian. A string is `u16` byte length then UTF-8.

```
"R6RA"  u32 version = 1  u32 bone_count  u32 clip_count
per clip:
  string name
  f32 length (seconds)   u32 frames   f32 rate_scale
  u8  additive (0 none, 1 local-space additive, 2 mesh-space additive)
  string base_name       (the clip whose frame is the additive base, "" if none)
  i32 base_frame
  u8  root_motion
  u32 notify_count, then per notify: string name, f32 time (seconds)
  f32[3] x bone_count    scale of each bone (constant over the clip: the first frame's)
  frames x bone_count x ( f32[4] rotation xyzw, f32[3] translation )   local to the parent, bone order of the skeleton
```

`frames` keys are evenly spaced from 0 to `length` (UE: `length = (frames - 1) / fps`; 61 frames, 2 s, 30 fps). Bones
without a track hold the skeleton's reference pose, so the file is dense. A sequence the source marks additive was
decoded by CUE4Parse into full poses (base plus difference); the importer takes the difference again against
`base_name` at `base_frame`.

`riding.r6ra` holds 74 sequences: the 93 `motard/` clips minus the starting-grid variants and minus the 8 stored with the ACL codec
(below), `BikeSpecificCorrection`, and the `race/` finger and wrist clips. `falls.r6ra`, made the same way from the 21 fall and
get-up sequences of `riderShared/Animations/Ragdoll/` that Unreal's own codecs store, is read in addition: the importer reads
every `*.r6ra` of the export folder.

## ACL sequences: `acl/<name>.acl` and `acl/<name>.meta.json`

158 of the 700 rider sequences are stored with the Animation Compression Library; the exporter's `acl <list> <folder>` mode
(`R6ToolRider`) writes, for each sequence of a list of packages, the compressed buffer (`FACLCompressedAnimData.CompressedByteStream`,
the format of [acl-decoder.md](../specs/acl-decoder.md)) and a small JSON file with what the sequence has besides it:

```text
{ "name", "package", "num_frames", "length" (s), "rate_scale", "additive" (AAT_None | AAT_LocalSpaceBase | ...), "base_name",
  "base_frame", "root_motion", "interpolation", "track_to_bone": [bone index of each track],
  "notifies": [ { "name", "link" (Absolute | Relative), "time" } ], "codec" }
```

The importer decodes them itself (`mods/ride6-bikes/src/rider/acl_clips.rs`, the generic decoder `libs/blackbox-acl`), with the rider
skeleton's reference pose for the tracks a clip does not animate, into the same clips the `R6RA` file holds. The folder holds the
nine the rider pack needs: the eight riding clips listed under Gaps and `race_ragdoll_FaceDown_GetUp`. Additive ACL sequences are not supported yet.

## `R6RM`: skinned mesh

```
"R6RM"  u32 version = 1  string name
u32 bone_count, per bone: string name, i32 parent, f32[4] rotation, f32[3] translation, f32[3] scale   (the mesh's own bind pose, local)
u32 material_count, per material: string slot_name, string material_asset ("None" if unset)
u32 lod_count, per LOD:
  f32 screen_size   u32 vertex_count   u32 index_count   u32 section_count
  per section: i32 material, u32 first_index, u32 face_count
  per vertex: f32[3] position, f32[3] normal, f32[2] uv, u8 n, n x (u16 mesh_bone, f32 weight)    (n <= 8)
  u32[index_count] indices (triangle list)
```

Bone indices are into the mesh's own bone list; a bone is matched to the skeleton **by name** (the boots have 10 bones, the
helmet one: `helmet`). Normals and positions are in the bind pose.

## Skeleton JSON

`{ "bones": [ { "name", "parent", "t", "q", "s", "retarget" } ], "sockets": [ { "name", "bone", "t", "r", "s" } ],
"virtual_bones": [] }`. `t` in cm, `q` xyzw, `r` an Unreal rotator `(pitch, yaw, roll)` in degrees. All 81 bones of the
rider skeleton use translation retargeting mode `Animation` (the animation's own translations apply), so no retarget step
is needed. `bike_skeleton.json` is the same export of `vehicles/bikes/chassis/chassis369/skeletons/chassis369_Skeleton`.

## Runtime pack `.bbsk` (written by `blackbox_skeleton::Pack::to_bytes`)

Little endian. Strings as above. A bone index is `u32`, `0xFFFFFFFF` meaning none. A transform is `f32[4]` rotation
xyzw, `f32[3]` translation, `f32[3]` scale.

```
"BBSK"  u32 version = 1
u32 bone_count, per bone: string name, u32 parent, transform rest
u32 socket_count, per socket: string name, u32 bone, transform
u32 clip_count, per clip:
  string name, f32 length, u8 flags (bit 0 additive, bit 1 root_motion)
  u32 notify_count x (string, f32)
  u32 track_count, per track: u32 bone, then three channels: rotation, translation, scale
    channel: u8 kind
      0 none     (the pose keeps its value)
      1 constant (one value)
      2 uniform  (u32 n, n values; evenly spaced from 0 to length)
      3 keyed    (u32 n, n times f32, n values)
    a value is f32[4] (rotation) or f32[3]
u32 mesh_count, per mesh:
  string name
  u32 bone_count, bone_count x u32 skeleton bone (or none), bone_count x f32[16] inverse bind matrix (column major)
  u32 material_count x string
  u32 lod_count, per LOD: f32 screen_size, u32 vertex_count,
     per vertex: f32[3] position, f32[3] normal, f32[2] uv, u8 n, n x (u16 mesh_bone, f32 weight)
     u32 index_count x u32, u32 section_count x (u32 material, u32 first_index, u32 index_count)
```

The reader checks every count against the bytes left, every bone index against the skeleton and every vertex index, and
rejects trailing bytes.

## Other files `ride6-rider-import` writes (`files/RIDER/`)

| File | Content |
|---|---|
| `rider.bbsk` | The pack above, in the car frame (metres). |
| `graph.json` | The blend tree, `blackbox_skeleton::Graph` serialised with serde (parameter names, then the node tree). |
| `attach.json` | Seat, hubs, grip and peg points, the limbs to solve with IK and the numbers that drive the animation (below). |
| `parts.json` | Which mesh wears which picture, which is translucent, which is drawn (below). |
| `CARS/<ID>/RIDER/contacts.json` | Not written by `ride6-rider-import`: one bike's seat, hubs, grip and peg points (below). |
| `ragdoll.json` | Not written by `ride6-rider-import` either: `ride6-rider-ragdoll` writes the rider's bodies and joints for a fall (below). |
| `textures/*.png` | The pictures. |

The game reads them from the mod's `files/CARS/<ID>/RIDER/` or `files/RIDER/`, the first folder that has an `attach.json`.

### `attach.json` (version 2)

The game reads it without knowing the bike or the game it comes from: targets and limbs are listed by role, bones by the
rider skeleton's own names. Frames are the car's (x forward, y left, z up), metres; "bike frame" is the bike skeleton's origin
(the mesh origin) at the bike's rest pose; "rider space" is the rider skeleton's component space.

| Key | Content |
|---|---|
| `frame`, `reference_clip` | Text; the clip whose first frame the tip offsets were measured on. |
| `rider_root_in_bike` | `{translation, rotation (x y z w)}`: the rider's root bone on the seat socket. Rider space to bike frame. |
| `hubs` | `{front, rear}`: the hubs in the bike frame. The game moves the bike frame to its model frame (`Attach::frame_offset`: the mean difference of the hubs in x and z; the model has them from the car's `[wheels]` table or its wheelbase). |
| `targets[]` | `{name, position, rotation, steers}` (`rotation`, x y z w: the turn of the socket's frame; older files have none): the points the limbs reach for (bike frame); `steers` grips turn about the front wheel's steering axis with the handlebar, pegs do not. |
| `limbs[]` | `{name, root, mid, tip, pole, target, tip_offset, tip_frame}`: two-bone IK on the rider bones; `tip_frame` `{translation, rotation}` is the tip bone's place and turn in the target's own frame in the reference pose, so the tip is brought to the target's frame composed with it and takes that turn (the authored placement of the hand on the grip is kept, and follows a grip socket that another bike turns differently). Without `tip_frame`, or when the target has no `rotation`, the tip goes to the target plus `tip_offset` (rider space) and keeps the forearm's turn. The middle joint bends towards the pole bone's place in the animated pose. |
| `inputs` | `tuck_speed_kmh` (speed from which the rider tucks), `tuck_rate`, `lean_rate`, `wheelie_rate`, `stoppie_rate` (1/s smoothing). |
| `torso` | `{lean, head, head_share}`: the spine bones (root to tip) the body leans forward with, in equal shares, to bring the shoulders within reach of the grips, and the neck and head bones that turn back by `head_share` of the lean in all (`reach.rs`). Absent: the body does not lean. |
| `seat` | `{bones, sink}`: the rider bones whose skin rests on the seat (the vertices that mostly follow them, carried by the first) and how far (metres) the lowest of them sinks below the seat's top. Absent: not sat on a surface. |
| `inertia` | `bone` (the rider bone that is shifted and turned), `min`/`max` (the range the rider's displacement from the physics is held to, car axes forward, left, up, metres: the blueprint's `InertiaDisplacement_RidingMin/Max`), `rate` (1/s). `braking_m` and `full_acceleration` of older files are ignored. |

Where each number comes from is in [ride6-rider-animation.md](../specs/ride6-rider-animation.md), section 8.

### `contacts.json` (version 1): one bike's contact points

`files/CARS/<ID>/RIDER/contacts.json`, written by `ride6-bike-import` for each bike, read in addition to the rig's `attach.json`
(a car's `RIDER/` folder needs no other file). The game takes `rider_root_in_bike`, `hubs` and `targets` from it and keeps
everything else (`limbs`, `tip_offset`, `inertia`) from the rig: a target of the same name replaces the rig's, a limb's
`tip_offset` (where the hand bone sits against the grip point in the authored pose) is the rig's.

| Key | Content |
|---|---|
| `version` | `1`. Another number: the file is ignored. |
| `vehicle`, `frame` | Text. |
| `rider_root_in_bike`, `hubs`, `targets[]` | As in `attach.json`: seat, the two hubs, and the four targets `left_grip`, `right_grip`, `left_foot`, `right_foot` (`steers` true for the grips), in the bike skeleton's frame at rest, car axes, metres. |
| `seat_surface` | Optional: `[[x, z], ...]`, the top of the bike's seat along its centre line in the bike skeleton's frame (x ascending, 2 cm apart, from 30 cm behind the seat socket to 10 cm ahead), measured by the converter from the chassis mesh (`seat_surface.rs`). The game sits the rider on it (`seat`). Without it he sits at the socket. |
| `correction` | Optional: the bike's row of the animation correction table: `rider_max_lean`, `aero_tank`, `lean_knee`, `lean_elbow`, `knee`, `elbow`, `muffler`, `spine_1_deg`, `spine_2_deg`, `arms_ik_spine_alpha`, `pole_elbow_cm` (3), `bike_type`, `quick_shift` (text). Read and kept; the game does not apply it yet. |

The game refuses a file whose numbers are not finite, whose seat has no rotation, whose front hub is not ahead of the rear one, or
that has a target more than 1.5 m from the seat (a bound for damaged files; the 293 imported bikes have grips 0.50 to 0.74 m and
pegs 0.46 to 0.62 m from the seat).

**Where the numbers come from** (RIDE 6): every bike's skeleton has the sockets `SOCKET_RIDER` (on `BONE_CHASSIS`),
`SOCKET_L_HANDLE_POSE` and `SOCKET_R_HANDLE_POSE` (on `BONE_L_GRIP` and `BONE_R_GRIP`), `SOCKET_LEFT_FOOT` and `SOCKET_RIGHT_FOOT`
(on `BONE_CHASSIS`); all 293 bikes exported have all five, at places that differ from bike to bike (grips 33 to 67 cm ahead of the
seat, pegs from 21 cm behind to 28 cm ahead of it), and nothing else in the data places the rider on a bike. The importer takes
them from the part table's `bones{}` (the world place and turn of every bone of the chassis skeleton, glTF axes, metres) and
`sockets{}` (a place in centimetres and a rotator on a bone, as Unreal stores them): glTF to Unreal is the swap of y and z (a turn
`(x, y, z, w)` becomes `(-x, -z, -y, w)`), then the converter's usual Unreal to car frame. Checked on the KTM 450 SMR: all five
sockets agree with the Unreal skeleton's to 1e-7 m (`SOCKET_L_HANDLE_POSE` at (11.76, -33.35, 113.76) cm in the chassis frame),
so a KTM's `contacts.json` equals the seat, hubs and targets of the shared `attach.json`.

### `parts.json`

`{ "parts": [ {mesh, slot, picture, translucent, drawn} ], "gloves_picture": ... }`. `mesh` is the mesh name inside the pack
(not the file name: `suit001_male`, `SuitRacing_Male_arms`, `boots001`, `helmetRacing_default`,
`visorClosedRacing_default`), `slot` the material slot name of the mesh, `picture` a file of `textures/`. `drawn: false`
(the arms mesh, whose sleeves the suit already has) is not loaded. A translucent part is drawn alpha blended.

### `ragdoll.json` (version 1): the rider in a fall

Written by `ride6-rider-ragdoll <RiderGP_PhysicsAsset.json> <RiderBP.json> <folder>` (two JSON files of the package reader's `json`
mode), read by `nfsmw/.../rider/fall_file.rs`. The rider's frame (x forward, y left, z up), metres, kilograms; positions, axes and
frames are in the frame of the bone a body is on. Without the file the rider stays on a bike that falls. How it is used:
[ride6-rider-fall.md](../specs/ride6-rider-fall.md).

```text
bodies[]   { bone, mass, radius, half_length (half the distance between the end spheres), centre [x,y,z], axis [x,y,z] }
joints[]   { child, parent, frame_child [x,y,z,w], frame_parent [x,y,z,w], swing_z_deg, swing_y_deg, twist_deg }   (0 locked, 180 free)
contact_friction, linear_damping, angular_damping, joint_damping (1/s)
falls      { velocity_multiplier, high_side {additional [m/s], scale, world [m/s], angular}, low_side {...} }
```

A body is a capsule on its bone. A joint's shared point is the child bone's origin; its twist axis is the X of the constraint frames,
`swing_z` the swing about their Z axis and `swing_y` about Y. `falls`: `v' = v + R additional + world + R (scale * R^-1 v)` in the
bike's frame, the spin times `angular`, the velocity first times `velocity_multiplier`.

## Gaps

- **ACL clips** (resolved): the eight riding clips `motard_drift_pose_L/R`, `motard_old_school_standard_pose`,
  `motard_wheelie_Entry_anim`, `motard_wheelie_Standing_anim`, `_L`, `_R` and `motard_wheelie_StandingInterpolation_anim`, and the
  get-up `race_ragdoll_FaceDown_GetUp`, are decoded by `blackbox-acl`. The pack has **104 clips** (74 riding, 21 fall and get-up in
  `falls.r6ra`, 9 from ACL; it had 74). The blend space `motard_Wheelie_Standing_BS` (steering -1..1 in 2 cells, entry 0..0.3 in
  1: the standing pose at (0, 0.3), left and right at (-1, 0.3) and (1, 0.3023), the interpolation pose at (0, 0)) is a graph node
  of the rider's blend tree; the drift poses are the ends of its three-way drift blend.
- **Gloves** are a picture composited onto the hands by a material function; the importer copies the picture and does not
  composite it.
- Face, eyes and goggles are not exported (the full-face helmet hides them).
