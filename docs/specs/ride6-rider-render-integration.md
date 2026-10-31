# Drawing the RIDE 6 rider in the game (wave 2)

**Status: implemented** (`crates/nfsmw/src/scenes/world/drive/rider/`, `RenderBackend::update_mesh_vertices`), not yet built
or run. The text below is the design it follows; where the code differs: the update is `update_mesh_vertices` with a
re-create fallback (section 2), the rider is drawn at level 0 with the arms mesh off, two-sided, the grips follow the
steering (section 5, the bars are now a separate steered solid) and the data is read from `files/CARS/<ID>/RIDER/` or
`files/RIDER/` (section 6). The user-facing summary is [bikes.md](../modding/bikes.md#the-rider).

What the renderer and the game have to do to put the animated rider of [ride6-rider-animation.md](ride6-rider-animation.md)
on the two-wheel bike. The data side is done (`ride6-rider-import`, `blackbox-skeleton`); nothing here is built. The
files are in [formats/ride6-rider.md](../formats/ride6-rider.md), the runtime in [skeletal-animation.md](skeletal-animation.md).
The code this touches (`libs/blackbox-render`, `libs/blackbox-gfx`, `crates/`) was being edited by others when this was
written and was only read.

## 1. What exists

- `blackbox_gfx::Vertex` is 36 bytes: position, normal, D3DCOLOR `B, G, R, A`, one UV. `MeshDesc` takes `&[Vertex]`,
  `&[u16]` indices and `DrawRange`s (textures and shading per range). `RenderBackend` has `create_mesh` and
  `destroy_mesh` and no way to change a mesh (`libs/blackbox-gfx/src/backend.rs`).
- A solid (`blackbox-solid`) can carry skin: effects listed in `Layout::skinned_effects` (version 16: effect 2, "WorldBoneShader")
  have 60-byte vertices with three blend weights and three bone slots, resolved through the group's 16-byte
  `bone_table`. The reader decodes them (`SkinVertex { weights: [f32; 3], bones: [u8; 3] }`) but the scene splits skinned
  solids into rigid parts (`upload_rigid_part`) and the renderer does not skin. So **there is no GPU skinning to reuse.**
- The game draws a car as a body plus four copies of one wheel solid. The bike mod puts the whole bike in the body solid.

## 2. Recommended: CPU skinning into a dynamic mesh

The rider is small: the stock rider at LOD 0 is the suit (4,458 vertices) + arms (7,423) + boots (2,130) + helmet (1,109) +
visor (169), about 15,300 vertices and 28,000 triangles; at LOD 2 or 3 it is a fraction of that. Skinning 15,000 vertices
with up to 8 influences is well under a millisecond. So:

1. **Load once** (when the bike is chosen): read `rider.bbsk` with `blackbox_skeleton::Pack::parse`, `graph.json` with
   serde into `Graph`, `attach.json`, `parts.json`. Upload each picture once with `create_texture` (the pictures are PNG,
   up to 2048 pixels: reduce them to the game's texture limits as the bike importer does for the livery).
2. **Per part, per LOD:** build the static data: indices as `u16` (all parts fit: the largest has 7,423 vertices),
   `DrawRange`s from the sections (`Section { material, first_index, index_count }`), the colour `0xFFFFFFFF`, UVs as they
   are (Unreal's V is top-down like Direct3D's; check against the first screenshot, flip V if the suit is upside down).
3. **Per frame**, for the nearest rider only (others use a low LOD or no animation):
   - `params` from the bike state (section 4), `graph.evaluate(skeleton, clips, params, time)` gives a `Pose`.
   - IK on the four limbs (section 5) with `blackbox_skeleton::apply_two_bone`.
   - `pose.component(skeleton)` then `skin_matrices` + `skin_vertices` per part, into reused `Vec<Vec3>`.
   - write the skinned positions and normals into the mesh's vertex buffer.
4. **Draw** the meshes with the transform `body_transform * rider_root_in_bike` (`attach.json["rider_root_in_bike"]`:
   translation and rotation in the car frame). The rider's component space is the bike's space, so it leans, pitches and
   bounces with the body automatically.

### The renderer change this needs (not done)

An `update_mesh_vertices(handle, &[Vertex])` on `RenderBackend`, writing the vertex buffer of an existing mesh with the
index buffer and draw ranges untouched (the new vertex count equals the old). Without it, a frame can `destroy_mesh` and
`create_mesh` each rider part (5 meshes a frame): it works, it allocates; use it only to get a first picture.
Translucency: the visor (`parts.json`, `translucent: true`) needs the renderer's alpha-blended `BlendMode`; the mesh has
no picture of its own beyond `visorDefault_d`, an 11 KB tint.

### Alternative: GPU skinning through the solid format

Convert the rider to a version-16 solid with the skinned effect (60-byte vertices) and teach `blackbox-render` the
bone-table palette. Limits: three influences per vertex (the RIDE 6 data has up to 8; the extra weights would be dropped
and the rest renormalised, visible at shoulders and hands), 16 bones per group (the suit uses all 78: it would be split
by bone set), and a vertex shader change. Not recommended before the CPU path is shown to cost too much.

## 3. Levels of detail

`MeshLod.screen_size` is Unreal's screen-size threshold (the fraction of the screen height at which the level starts to
be used). Pick the level from the projected size of the bike. The suit has 6 levels, the arms 1. Unverified: what the
arms mesh (`ArmsMesh` of the suit's data asset, 7,423 vertices, one level) is for. It is probably the close-up or
first-person arms, and drawing it with the suit would double the sleeves. At the first stage draw the suit, boots,
helmet and visor, and add the arms mesh only if the suit's own arms and hands look wrong (both meshes share the same
78-bone set, so both are skinned to the fingers).

## 4. Bike state to graph parameters

Table in [ride6-rider-animation.md](ride6-rider-animation.md) section 5. In `blackbox-vehicle::bike` terms: `lean` from
the body roll angle over the steepest lean the model allows (the controller already has the limit), `speed_kmh` from the
forward speed, `wheelie` and `stoppie` from the pitch of the body past the wheel lift (0 when both tyres are down),
`tuck` from speed and throttle. Smooth each at the speeds listed there before using it; the graph clamps its inputs and
tolerates any value.

## 5. Hands, feet and the steering

Per frame, after the graph:

1. Component transforms of the pose (rider space).
2. Targets in rider space: take each bike socket in `attach.json` (`SOCKET_L_HANDLE_POSE`, `SOCKET_R_HANDLE_POSE`,
   `SOCKET_LEFT_FOOT`, `SOCKET_RIGHT_FOOT`) in bike space, move the grip sockets with the steering (rotate them about the
   steering axis through `BONE_HANDLE`, which tilts back with the rake: its rest rotation is in `bike_bones`), subtract
   the rest offset between the hand/foot bone and the socket in the authored centre pose, and bring the result into
   rider space with the inverse of `rider_root_in_bike`.
3. `apply_two_bone(skeleton, &mut pose.local, chain, target, pole, 1.0, tip_rotation)` for each limb of
   `attach.json["limbs"]`; the pole is the component position of the pole bone (`PV_arm_l/r`, `PV_leg_l/r`) in the pose.
   `tip_rotation` for the hands is the pose's own hand rotation (pass `None`).
4. Recompute the component transforms (they changed) and skin.

The steering angle is the bike model's handlebar angle. The front wheel and the fork are part of the baked bike body
today (the bike importer bakes them), so the grips move relative to a body that does not show the steering: the hands
will turn with the bars only when the bike's fork and bars are animated too. Until then pass a zero steering angle to
the grips.

## 6. Wiring into the car pose structures

The game's car model (`nfsmw-data::car::CarModel`, drawn by the car scenes) knows a body solid and a wheel solid and
poses four wheels. For a mod car whose manifest says it is a bike, add an optional `rider: Option<RiderRig>` next to the
wheel poses:

- `RiderRig` holds the `Pack`, `Graph`, the attach data, the uploaded meshes and a scratch buffer, and is built by a
  mod-content reader in `crates/` (the bike manifest names `RIDER/` of the same mod; a bike without the folder has no
  rider). The reader belongs in `crates/`, not in the generic crate.
- The vehicle step produces the body transform and the bike state each fixed step; the render step calls
  `RiderRig::update(&BikeState, dt)` once and draws the meshes with the body transform. Animation time advances by the
  frame time (the clips loop; `time` is seconds).
- Shadows and reflections: draw the rider into the same passes as the body (same mesh, same transform).
- Replays, other players, cutscene bikes: share one `Pack`, one `RiderRig` per visible bike.

## 7. Checks for the first run

Offscreen screenshots only, muted: the bike at rest (rider sits, hands on the grips, boots on the pegs, no part of the suit
inside the tank); steady speed with a lean left and right (the head moves to the inside); a wheelie ramp 0 to 1; the
visor translucent; every LOD of the suit. If the rider is mirrored or inside out, the two suspects are the winding flip
and the sign of `y` in `ride6-rider-import`'s `convert.rs` (documented there); if it faces sideways, the
`rider_root_in_bike` rotation was not applied.

## 8. Contact points per bike (`contacts.json`, `rider/contacts.rs`, `rider/reach.rs`)

Sections 5 and 6 aim the hands, feet and seat at the points of one bike's `attach.json`. With one shared rig that was the
KTM's: on any other bike the hands, feet and seat stayed where the KTM has them. Now each bike has its own contact points.

**What the original uses.** The rider is attached to the bike's own skeleton: every one of the 293 bikes exported has the sockets
`SOCKET_RIDER` (the seat: the rider's root bone sits there), `SOCKET_L_HANDLE_POSE` and `SOCKET_R_HANDLE_POSE` (on the grip bones
`BONE_L_GRIP` and `BONE_R_GRIP`, so they turn with the handlebar), `SOCKET_LEFT_FOOT` and `SOCKET_RIGHT_FOOT` (on `BONE_CHASSIS`),
and each has its own place (data: the `SkeletalMeshSocket` objects of the bike skeleton asset). The animation blueprint's control
rigs `CtrlRig_HandsIK` (inputs `hand_L/R_position`, `hand_L/R_rotation`, `hand_L/R_alpha`) and `CtrlRig_FeetCorrection` solve the
limbs with the engine's full-body IK; where they read those inputs from is a native binding of the blueprint's property access
that was not traced. The binary also holds the names `L_HANDLE_POS`, `R_HANDLE_POS`, `BONE_LEFT_FOOT_POS`, `BONE_RIGHT_FOOT_POS`,
`HANDLEBAR`, `LEVER_SYS_L/R` and `SOCKET_LEFT_FOOT_GEARSHIFT`/`SOCKET_RIGHT_FOOT_GEARSHIFT` (`ride6-Win64-Shipping.exe`, build
5.6.1-1697004, strings at `14a3fce90` to `14a3fcf00` and `14a3e6630`); no bike skeleton has bones or sockets of those names, so they
are names of the native code's own targets (not resolved). Per vehicle the game has one more table, `RiderAnimationCorrection_DT`
(row struct `BikeCorrection`, 364 rows, one per vehicle): `RiderMaxLean` (1.0 for all), `AeroTankCorrection`, `LeanKneeCorrection`,
`LeanElbowCorrection`, `KneeCorrection`, `ElbowCorrection`, `MufflerCorrection`, `SpineCompensation1/2` (degrees, added to the
spine), `ArmsIKSpineCompensationAlpha`, `TransformPVElbowCorrection` (a transform; non-identity only for `Yamaha_FZ-750-1985`: -7 cm
in x, -10 cm in z), `BikeType` (`NOOVERRIDE` for 338 rows, `RACING` 7, `STREET` 6, `ENDURO` 6, `SCOOTER` 3) and `QuickShiftType`.
These are **not** contact points: the blueprint turns the knee and elbow numbers into per-side values by a fuzzy blend of the lean
(`RiderLeaning_IsLeft/IsRight/IsCenter`: the centre takes `KneeCorrection`, the leaning side `LeanKneeCorrection`; `ElbowCorrection_L/R`
likewise) and the spine numbers into rotators (`SpineCompensation1/2_BikeSpecific`). So RIDE 6 uses **one rider rig and one pose set
for all bikes**; the seat, grip and peg points come from the bike skeleton's sockets, and the table only adjusts knees, elbows and
spine per vehicle.

**What the importer writes.** `rider/contacts.rs` reads the part table's `bones{}` and `sockets{}`, turns the five sockets into the
car frame and writes `files/CARS/<ID>/RIDER/contacts.json` with the seat, the hubs, the four targets and the vehicle's correction row
([formats/ride6-rider.md](../formats/ride6-rider.md)). `attach.json` of the shared rig is written by the same code from the
reference bike, so the two agree (the KTM's `contacts.json` equals it to 1e-7 m).

**What the game does.** `contacts::lay_over` reads the car's `contacts.json` after `attach.json` and replaces the rig's seat, hubs
and same-named targets (`Attach::with_contacts`); the limbs' `tip_offset` stays the rig's, since it is the offset of the hand bone
from the grip *socket*, a property of the rig and of how the socket is authored, not of the bike. The frame offset that joins the
bike's skeleton to the model uses the bike's own hubs. A missing file keeps the rig's points (a log line says so once); an unusable
one is logged and ignored.

**What was wrong (measured on the exported meshes, 2026-10).** Three things put a rider on a bike other than the KTM wrong:

1. *The seat.* `SOCKET_RIDER` is the rider's root bone, not a point on the seat. Compared with the top of the chassis mesh's seat
   at the socket's place it is 2.9 cm above on the BMW R 1300 GS, 5.9 on the KTM 450 SMR, 9.1 on the Honda X-ADV, 12.6 on the Yamaha
   R7 and 14.6 on the Ducati 998R. The rider's root is also 5 cm under his pelvis bone, so the R7's hips hovered 17 cm over the seat.
   RIDE 6's own sockets are not what it sits him by (nothing in the data of the physics, the rows of `RiderAnimationCorrection_DT`
   or the blueprint does: the physics rider's `SaddleHeight` is 20 for all 317 bikes with a rider block); the animation's own
   full-body solve is the part that was not traced.
2. *The reach.* The pose is authored for the KTM, upright. The R7's grips are 10 cm above its seat and 53 cm ahead, so the shoulders
   were 0.65 m from the grip with arms 0.53 m long. The old lean (one linearised step about the pelvis, at most 0.5 rad) closed
   only part of it: the arm ended straight with the wrist 12 cm from the grip socket (hand bone at (0.19, 0.26, 1.03) for a socket at
   (0.26, 0.29, 0.93)), the glove hung under and behind the grip.
3. *The turn of the hand and foot.* The tip was aimed at `target + offset` in the rider's space and kept the forearm's turn, so a
   bike whose grip socket is turned differently (clip-ons) got the KTM's hand placement.

The eight unreadable ACL clips are not the cause: the riding poses (`motard_CenterPose_*`, `Leaning_*`) are decoded; the missing
ones are the standing-wheelie blend space and the drift poses ([ride6-rider-animation.md](ride6-rider-animation.md), Gaps), which
drive the rider only in those tricks (they stay replaced by the nearest decoded wheelie clips).

**The fix** (`drive/rider/{seat,reach,pose}.rs`, `mods/ride6-bikes/src/rider/{seat_surface,attach,contacts}.rs`):

- *Sat on the seat.* The converter measures the seat's top along the bike from the exported chassis mesh (vertical rays down on the
  body parts at the centre line and 7 cm to each side, the highest hit below the root plus 3 cm, averaged, 2 cm apart from 30 cm
  behind the socket to 10 cm ahead) and writes it to `contacts.json` as `seat_surface`. The game takes the vertices of the rider's skin
  that mostly follow the seat bones (`pelvis`, `gluteus_Vol_l/r`, from `attach.json` `seat.bones`; 200 on the suit), fixes them to the
  pelvis, and each frame lowers (or raises) the body along the bike's up axis until the lowest of them is `seat.sink` below the
  surface. `sink` is a **choice calibrated on the KTM**: with the root on the KTM's socket the game's own points are 4.0 cm under
  its seat, and that is taken for every bike, so the KTM stays where the original placement has it (its pelvis moves 0.07 mm) and
  the others sit as deep. Result on the scratch set (pelvis shift from the socket's placement): R7 -6.9 cm, Ducati -6.3, X-ADV -2.7, Indian -0.8, BMW +3.7.
  The shift is held to 20 cm; a bike with no `seat_surface` keeps the old placement.
- *Reach by the spine.* `reach.rs` leans the spine forward about its own three joints (an equal share each) by the smallest angle at
  which every shoulder is within 0.93 of its arm's length of the hand's goal (found by stepping 0.05 rad and bisecting, up to 1.25
  rad), and turns the neck and head back by 0.8 of it in all. The pelvis does not turn, so the legs and the seat contact are
  unaffected. 0.93 leaves the elbow bent 20 degrees; the KTM needs no lean (arms at 0.79). On the R7 the head ends about 40
  degrees ahead of the pelvis's vertical with the arms bent. These constants are choices (the original solves the body with a full-body IK).
- *Hand and foot frames.* `contacts.json` targets carry the socket's turn. `attach.json` limbs carry `tip_frame`: the tip bone's place
  and turn in the target's frame, measured on the reference pose, so the tip is at `target frame * tip_frame` and takes that turn
  (`apply_two_bone` with a tip rotation); for the KTM this is exactly the old `target + offset` (hands and feet agree to 0.1 mm).
  Measured against the KTM's own sockets the hand bone's frame is 12.6 to 15.4 degrees and 3.5 cm from `SOCKET_*_HANDLE_POSE`'s and
  the foot bone's 9.5 to 13 degrees and 8 cm from the peg socket's, which suggests the original aims the bones at the sockets with
  small offsets (the blueprint's `HandIKLocationOffset` are 1 cm); putting the wrist exactly on the socket was tried
  (`HANDS_ON_SOCKET`) and moved the KTM's hands off its grips, so the authored relation is kept.
- Every limb reaches its goal to under 0.1 mm on the KTM, R7, Ducati 998R RM, X-ADV, BMW GS, Indian Challenger and Italjet Dragster
  (`probe_tests.rs`, `fit_tests.rs`: hands and feet within 1 cm, buttocks `sink` into the seat within 5 mm, on a supermoto, a sport
  bike and a cruiser).

**Still imperfect.** The glove is where the authored relation puts it, 3.5 cm from the socket, not wrapped round the bar by a
finger solve; the elbow and knee directions follow the rig's pole bones (no per-bike `elbow`/`knee` corrections); the spine bends
in equal shares (no per-bone limits); the seat profile is the centre of the seat only (a bike whose tank rises under his knees
is held by the profile's range); the tuck pose of the original (`RACING` poses) is not exported.

**Not done.** The correction row is carried and not applied (knees and elbows by lean, the tank and muffler corrections, the spine
rotators, `pole_elbow_cm`): they need the control rigs' solve, which this runtime replaces with two-bone IK. The rider does not
change pose set by `BikeType`.
