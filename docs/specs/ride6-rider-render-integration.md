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
