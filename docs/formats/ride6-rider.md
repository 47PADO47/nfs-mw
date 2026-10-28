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
`R6ToolRider`). It has five modes: `skeleton` (a `USkeleton` to JSON), `anims` (a list of `UAnimSequence` packages to one
`R6RA`), `mesh` (a `USkeletalMesh` to `R6RM`), `textures` (PNG) and `bp` (a Blueprint class to pseudo-C++, used to read the
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

74 sequences are in the file: the 93 `motard/` clips minus the starting-grid variants and minus 8 clips stored with the
ACL codec (see Gaps), `BikeSpecificCorrection`, and the `race/` finger and wrist clips.

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
| `targets[]` | `{name, position, steers}`: the points the limbs reach for (bike frame); `steers` grips turn about the front wheel's steering axis with the handlebar, pegs do not. |
| `limbs[]` | `{name, root, mid, tip, pole, target, tip_offset}`: two-bone IK on the rider bones; the tip is brought to the target plus `tip_offset` (rider space), the vector from the target to the tip in the reference pose, so the authored placement of the hand on the grip is kept. The middle joint bends towards the pole bone's place in the animated pose. |
| `inputs` | `tuck_speed_kmh` (speed from which the rider tucks), `tuck_rate`, `lean_rate`, `wheelie_rate`, `stoppie_rate` (1/s smoothing). |
| `inertia` | `bone` (the rider bone shifted), `min`/`max` (range, bike axes forward, left, up), `braking_m`, `full_acceleration`, `rate`. |

Where each number comes from is in [ride6-rider-animation.md](../specs/ride6-rider-animation.md), section 8.

### `parts.json`

`{ "parts": [ {mesh, slot, picture, translucent, drawn} ], "gloves_picture": ... }`. `mesh` is the mesh name inside the pack
(not the file name: `suit001_male`, `SuitRacing_Male_arms`, `boots001`, `helmetRacing_default`,
`visorClosedRacing_default`), `slot` the material slot name of the mesh, `picture` a file of `textures/`. `drawn: false`
(the arms mesh, whose sleeves the suit already has) is not loaded. A translucent part is drawn alpha blended.

## Gaps

- **ACL clips.** Eight sequences use the ACL compression plugin and need the native `CUE4Parse-Natives` library, whose
  sources (the ACL and RTM submodules) are not in the scratch copy: `motard_drift_pose_L/R`, `motard_old_school_standard_pose`,
  `motard_wheelie_Entry_anim`, `motard_wheelie_Standing_anim`, `_L`, `_R` and `motard_wheelie_StandingInterpolation_anim`.
  The standing wheelie (its blend space `motard_Wheelie_Standing_BS`: axis Steer -1..1 in 2 cells, axis 0..0.3) and the
  drift poses are therefore not in the pack. Building the library needs the ACL sources (a download the user has not
  approved).
- **Gloves** are a picture composited onto the hands by a material function; the importer copies the picture and does not
  composite it.
- Face, eyes and goggles are not exported (the full-face helmet hides them).
