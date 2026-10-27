# HD reflections: behaviour spec for optional port features

Status: **specification, not implemented.** Written before any code, from two community mods and our own
read of `speed.exe`. Each feature names its original default, the mods' options, a proposed setting, the
renderer backends it applies to and the target module in the port.

Sources and licence rule:

- NFS HD Reflections by Aero_ ([docs/mods/nfshdreflections.md](../mods/nfshdreflections.md), GPL-3.0). Cited as
  **[community]** for option names, defaults and feature descriptions only.
- NFSMWHDReflections, sniten mirror of the same mod ([docs/mods/nfsmwhdreflections.md](../mods/nfsmwhdreflections.md),
  GPL-3.0). Same rule.
- Neither mod's code, address tables, code caves or INI layout is used here. Their addresses were used only to
  locate facts in `speed.exe` for verification (see [provenance](../provenance/hd-reflections.md)).
- Original facts come from `speed.exe` through Ghidra, read only, and are tagged **[verified]** (matched in the
  executable's bytes or in game-behaviour notes) or **[decomp]** (read from the decompiled logic, not yet tested in
  game). **[community]** means a mod states it and we have not checked it. **[unconfirmed]** means a gap.
- Port facts come from the files named in each section.

Evidence tags in this file: [verified], [decomp], [community], [unconfirmed]. Setting names are proposals.

## 0. Summary

| # | Feature | Original (speed.exe) | Port default | Backends | Main target |
|---|---|---|---|---|---|
| 1 | Car environment cube size | 256 px faces when the car env option is on, 16 px when off [decomp] | medium = 128 | native only (bevy `local_reflections` is false) | `libs/blackbox-gfx/src/reflection.rs`, `libs/blackbox-render/src/gpu/reflections.rs` |
| 2 | Rear mirror size | 256 x 256 [verified bytes] | 384 x 131 (fixed) | native (`request_view_capture`); bevy falls back to a full `request_capture` (mirror use not checked) | `crates/nfsmw/src/app/mirror.rs` |
| 3 | Wet-road reflection | 320 x 240 target, enabled by the road option [decomp] | not implemented | native (new pass) | new `libs/blackbox-render/src/gpu/road_reflection.rs` (proposed) |
| 4 | Front-end car cube | same cube routine as car env, size unconfirmed [unconfirmed] | not implemented | native | `crates/nfsmw/src/frontend/` (proposed) |
| 5 | Auto size and power-of-two rounding | none (sizes are fixed) | off | native | `libs/blackbox-gfx/src/reflection/size.rs` (proposed) |
| 6 | Reflection LOD | reflection views use a 32 px threshold [decomp] | original | native | `crates/nfsmw/src/scenes/world/reflections.rs` |
| 7 | Render distance caps | cube far plane 100 m [decomp]; mirror far plane 192 m [verified, menu read] | no cap | native | `crates/nfsmw/src/scenes/world/reflections.rs`, `rear_view.rs` |
| 8 | Restored content | some object classes skipped in reflections [community] | original | native | `crates/nfsmw/src/scenes/world/reflections.rs` |
| 9 | Vehicle reflection brightness | 1.0 (no setting) | 100 percent | native | `libs/blackbox-render/src/gpu/glossy/uniforms.rs` |
| 10 | Mirror tint | none | off (255, 255, 255) | native (and bevy if mirror is shown there) | `crates/nfsmw/src/app/mirror.rs` |
| 11 | Object pool and slot limits | no fixed cap found in the port | no setting | native | `libs/blackbox-render/src/gpu/slots.rs` (check only) |
| 12 | Backface culling | original culling state [decomp, see section 12] | original | native | `libs/blackbox-render/src/gpu/pipelines.rs` |

## 1. Original baseline

What `speed.exe` does, independent of any mod:

- **Mirror target.** One render target of 256 x 256 is created for the rear mirror. The two dimensions are
  stored as initial values in the executable's data [verified bytes]. The same values are read by the code
  that builds the mirror and the RVM pass [decomp].
- **Road target.** The road reflection target is 320 x 240 when the road option is on. When it is off, the code
  creates a 1 x 1 placeholder [decomp]. The 320 and 240 values are immediates in the function that fills the road
  record [verified bytes].
- **Second target sizes.** A second mirror-related target of 320 x 200 is created when a counter is above zero
  [decomp, meaning unconfirmed]. A further 320 x 240 target is made only in one pass state [decomp].
- **Car cube.** Six faces. Each face is 256 px when the car environment option has a non-zero value, and 16 px
  when it is zero [decomp]. The face count is six; the cube is only built when a flag byte is set (initial value 1)
  [verified bytes].
- **Stock option values.** The initial values of the car environment enable and road enable options are 0 in
  the executable's data [verified bytes]. The option writer that creates the registry defaults writes these same
  values [decomp]. So a fresh stock install starts with both options at 0 unless the options screen changes them.
  The effective default chosen by the options screen is **[unconfirmed]**.
- **Camera planes.** The six car-cube cameras get near 0.5 m and far 100 m [decomp, from the initial data values
  read by the function that updates them]. The rear mirror camera has near 0.75 m and far 192 m [verified,
  menu read; see docs/specs/rear-view-mirror.md].

The port's own quality enum sets the car capture size: off 0, low 64, medium 128, high 256
(`libs/blackbox-gfx/src/reflection.rs`, `ReflectionQuality::size`). High is therefore the original size.

## 2. Car environment cube: resolution and scale

- **Original.** 256 px faces when on; the stock build uses 16 px when the option is 0 [decomp].
- **Mods.** Aero: a cube size option (default 256) and a scale multiplier (default 1.0) [community].
  sniten: a vehicle reflection scale (default 1.0) that sets the height of the vehicle reflection target, with
  the same role as the cube scale [community].
- **Proposed setting.** `reflection_car_scale`, percent, range 25 to 400, default 100. Applies to the existing
  `reflections` quality. The effective face size is `round(quality.size() * scale / 100)`, then rules of
  section 5.
- **Console.** `reflection_car_scale 150`, via `crates/nfsmw/src/devtools/console/settings_cmd.rs` and the key
  list in `settings_keys.rs`. Value check: `1..=400` clamped with a log line.
- **Menu.** Video options, row "Car Reflection Size", next to the existing "Car Reflections" row
  (`crates/nfsmw/src/frontend/options.rs`, new `GraphicsSetting` variant).
- **Backends.** Native (`blackbox-render`). Bevy reports `local_reflections: false`
  (`libs/blackbox-bevy-render/src/caps.rs`), so the row is hidden there through
  `crates/nfsmw/src/settings/availability.rs`.
- **Target.** `ReflectionQuality::size` gets a scaled variant in `libs/blackbox-gfx/src/reflection.rs` (or a new
  `reflection/size.rs` once the file is split, per the 500-line rule). The capture allocation in
  `libs/blackbox-render/src/gpu/reflections.rs` already rebuilds when `size` changes (`filter(|p| p.size == size)`).

## 3. Rear mirror: resolution and scale

- **Original.** 256 x 256 target [verified bytes]. The on-screen picture is shaped by the RVM mask, which the port
  does not use yet (see docs/specs/rear-view-mirror.md).
- **Port now.** Fixed 384 x 131 capture (`crates/nfsmw/src/app/mirror.rs`, `SIZE`). Its aspect follows the bezel,
  not the original square.
- **Mods.** Aero: mirror width and height scale with the mirror scale (default 1.0), and the auto rule gives
  (screen width / 2) x (screen height / 6) [community]. sniten: mirror width = ResY x scale, height = (ResY / 3) x
  scale, with a mirror scale default 1.0 [community].
- **Proposed setting.** `reflection_mirror_scale`, percent, 25 to 400, default 100. It scales `SIZE` in
  `mirror.rs`. The bezel aspect stays fixed, so the scale changes detail, not shape. Auto rule: height = output
  height / 6, width = height x (384 / 131), then the section 5 rules.
- **Console and menu.** `reflection_mirror_scale`; row "Mirror Reflection Size".
- **Backends.** The mirror is a HUD texture fed by `request_view_capture`. The native backend has its own reduced,
  non-blocking path. Bevy does not override it, so it falls back to a full `request_capture`
  (`libs/blackbox-gfx/src/backend.rs`). Whether the mirror works there is
  **[unconfirmed]**. Gate the row with `caps` once checked.
- **Target.** `crates/nfsmw/src/app/mirror.rs` (size and scale). The capture input is `scene.rear_view` in
  `crates/nfsmw/src/scenes/world/rear_view.rs`.

## 4. Wet-road reflection

### 4.1 What the original road reflection is

- **Shader side.** The world reflection effect (`fx\worldreflect.efx` in the install, `WorldReflectShader`) takes
  parameters named `REFLECTEDTEX`, `REFLECTEDTEXMIN` and `REFLECTEDPROJ`, plus `WINDOWREFLECTION` for windows
  [verified strings; binder function read, decomp]. The shader samples a reflected image projected from the
  camera, so it is a planar reflection, not a cube map.
- **Target.** The road image is the 320 x 240 target in section 1. Its state moves through a small machine (idle,
  one-view, two-view) driven by the road option and by whether the game is in its in-game state (the render state
  byte equal to 6) [decomp].
- **Scene content.** The road reflection pass checks a list of scene objects with a flag, and sets a per-frame
  flag when one of them is active [decomp, meaning unconfirmed].
- **Enable option.** The road reflection option (`g_RoadReflectionEnable` in the registry) is a DWORD. Initial
  value 0 [verified bytes]. Redux notes list a baseline of 2 for it, so the value set by the stock options screen
  is **[unconfirmed]**.
- **Water.** Water reflections are a separate shader contract and are not covered here.

### 4.2 How the mods change it

- **Aero.** RoadScale (default 1.0) scales the 320 x 240 target. A "removes road reflection on the mirror" change
  is made under RestoreShaders (default 1) [community].
- **sniten.** RoadScale (default 1.0) and HDRoadReflections (default 1) raise the road target. The README's exact
  size rule is not checked [community]. HDReflectionBlur (default 1) raises the blur buffer used with the
  road image [community]. RestoreShaders also removes the road reflection on the mirror and enables shader
  effects on reflections [community].
- **Both mods agree** that the road reflection is removed from the mirror image. Keep that in the port: the mirror
  shows the road without the wet reflection.

### 4.3 Port proposal

- **Port now.** No road reflection. `docs/renderers.md` says wet-road and water reflection shaders are separate
  work. Nothing in `crates/nfsmw` or `libs/blackbox-render` names a road reflection.
- **Proposed setting.** `reflection_road_scale`, percent, 25 to 400, default 100, applies to the 320 x 240 base.
  The toggle is `road_reflections` with values `off` and `on`, default off until the pass exists. Availability is
  false until the pass lands, so the menu row is hidden.
- **Backends.** Native only.
- **Target.** New `libs/blackbox-render/src/gpu/road_reflection.rs` (planar capture, projection, store); shader in
  `libs/blackbox-gpu-passes/src/shaders/`; driver in `crates/nfsmw/src/scenes/world/`. Keep the engine-generic
  projection in `libs/`, and the NFS:MW-specific object flags in `crates/`.

## 5. Auto-resolution, scale rules and power-of-two rounding

Shared by sections 2, 3, 4 and 6.

- **Default (auto off).** `size = clamp(round(base * scale / 100), min, max)`. Base values: car face 256; mirror
  384 x 131 (port) or 256 x 256 (original reference); road 320 x 240; front-end face 256 [decomp / verified].
  Minimum 16 for cube faces, 64 for planar targets. Maximum 2048 for planar, 1024 for cube faces (port choice).
- **Auto (`reflection_auto_size`, bool, default false).** Reference height H is the output height in pixels, after
  render scale. Car face = H. Mirror height = H / 6, width = height x 384 / 131. Road = W x H, with width capped
  at 2048. Then scale and clamp as above. Aero's rule is the source of the reference values [community]. Our
  caps are port choices.
- **Power-of-two (`reflection_pow2`, bool, default false).** Round cube faces down to the nearest power of two
  (floor of log2). Planar targets are not rounded. Aero's OldGPUCompatibility option does the same job for cube
  faces [community]. The original's cube values are powers of two already (256, 16), so the rule only matters
  when auto or a scale is used.
- **Order.** base, scale, auto (when on, replaces base), clamp, then pow2 for cube faces.
- **Target.** `libs/blackbox-gfx/src/reflection/size.rs` (proposed, pure functions, unit tested). It has no NFS:MW
  names, so it belongs in `libs/`.

## 6. Reflection level of detail

- **Original.** The reflection views select scenery models with a fixed 32 px threshold on slot 2, and some flags
  choose slot 0 or 3 [decomp, and docs/specs/scenery-lod.md]. The float 32.0 is stored in the executable's data
  and read by the cube camera update [verified bytes, decomp].
- **Port now.** `crates/nfsmw/src/scenes/world/reflections.rs` passes `quality.size()` as the reference height to
  `visibility::collect_for_height`. So the port's LOD for reflections follows the capture size, not the 32 px
  value.
- **Mods.** Aero: ImproveReflectionLOD (default 1). sniten: ImproveReflectionLOD with levels 0 off, 1 basic, 2 full
  (default 2), and level 2 also changes the mirror LOD and the road LOD [community]. Their defaults differ.
- **Proposed setting.** `reflection_lod`: `original` (default, the port's current behaviour) or `high`. `high`
  raises the reference height used for reflection LOD by a fixed factor (2) and lowers the detail threshold for
  the mirror. The exact numbers are to be set by a test scene.
- **Console and menu.** `reflection_lod`; row "Reflection Detail".
- **Backends.** Native.
- **Target.** `crates/nfsmw/src/scenes/world/reflections.rs` (height argument) and `visibility.rs` (threshold).

## 7. Render distance caps

- **Original.** Cube far plane 100 m, written to the six cube cameras [decomp]. Mirror far plane 192 m [verified,
  menu read, not in a race]. The mirror far-plane value is the same number as sniten's cap for the mirror.
- **Mods.** Aero: 100 m for cube types, 384 m for the mirror and the back of the cube, and a skybox distance of 0.5
  [community]. sniten: 100 m for most objects, 192 m for the mirror, and a skybox distance change [community].
  The 384 m mirror value conflicts with the 192 m in the original; the original value is taken here.
- **Port now.** No cap. The cube capture uses an infinite reverse projection (`face_frame`, PerspectiveInfiniteReverse),
  and the mirror lens uses a fixed near plane only (`viewer/camera/rear_view.rs`). So distance is limited only by
  streamed scenery.
- **Proposed setting.** `reflection_distance`: `original` (proposed default, a fidelity choice that changes the
  present behaviour) caps objects at 100 m for the car cube and 192 m for the mirror, using the visibility
  distance test; `unlimited` keeps the present behaviour.
- **Console and menu.** `reflection_distance`; row "Reflection Distance".
- **Backends.** Native.
- **Target.** Distance test in `libs/blackbox-gfx/src/reflection/` (generic numbers) and applied in
  `crates/nfsmw/src/scenes/world/reflections.rs` and `rear_view.rs`.

## 8. Restored content in reflections

The port currently draws in reflections: placed scenery and props (`residency.placed`, `residency.props`) and
the original shadow filter (`original_shadows`). It does not draw traffic in the car capture, though the rear view
does (`crates/nfsmw/src/scenes/world/rear_view.rs` calls `traffic.instances`). The car capture also omits the
player car by design.

Items the mods restore (all **[community]** unless noted). One proposed setting covers all of them:
`reflection_content`, values `original` (default, no change) or `restored`.

| Item | Mod option (default) | What it restores | Port status | Target |
|---|---|---|---|---|
| Traffic | not named by either mod | traffic cars in the cube | missing in car capture | `reflections.rs`: add `traffic.instances` as in `rear_view.rs` |
| Traffic lights, sun flares, car shadows | RestoreDetails (1, Aero and sniten) | light, flare and car shadow objects in reflections | `original_shadows` flag exists; lights and flares not checked | `reflections.rs` (visibility flags) |
| Particles | RestoreDetails level 2 (sniten); Aero level 2 | particle effects in the mirror | not checked | `reflections.rs` |
| Water | RestoreWaterReflections (0, Aero) | water reflections | separate contract, not in port | later |
| Visual treatment passes | RestoreVisualTreatment (1, Aero) | two extra passes in game only (state 6) | the port has `visual_treatment`; check the pass runs in captures | `libs/blackbox-render/src/gpu/post/` |
| Cubemap cars | RestoreCars (0, Aero) | other cars drawn in the cube | depends on traffic row | `reflections.rs` |
| Geometry fix | GeometryFix (1, sniten) | missing geometry in front-end and world reflections | not checked | `reflections.rs` |
| Shadow mirror | RestoreShadows (1, Aero) | shadow pass in reflections | not checked | renderer shadows |

Rules:

- The state 6 check for visual treatment is the in-game flag [decomp]. Captures in the port should follow the
  same rule: extra passes only in game.
- Each item is decided per target. Traffic appears in cube and mirror. Particles appear in the mirror only.
- Verify each item in game by comparison, not from the mod text (section 15).

## 9. Vehicle reflection brightness

- **Mods.** Aero: VehicleReflectionBrightness (default 1.0). It scales two float constants in the vehicle
  reflection path [community]. sniten: scales ambient, diffuse and specular colour of vehicle reflections, default
  1.0 [community]. A third mod, not in scope, uses 0.8.
- **Original.** No setting. Vehicle reflection strength comes from the light material constants (environment
  minimum, range and power) [docs/specs/car-reflections.md].
- **Proposed setting.** `vehicle_reflection_brightness`, percent, 0 to 300, default 100. Multiplies the environment
  term (`envmap_power`) in `MaterialUniform::new` (`libs/blackbox-render/src/gpu/glossy/uniforms.rs`). Diffuse and
  specular are unchanged unless a test shows the sniten-style scaling is needed.
- **Console and menu.** `vehicle_reflection_brightness`; row "Vehicle Reflection Brightness".
- **Backends.** Native (the glossy path). Bevy's material is separate (`blackbox.wesl`), not changed here.
- **Target.** `libs/blackbox-render/src/gpu/glossy/uniforms.rs` (the uniform packing) and the setting in
  `libs/blackbox-gfx/src/settings/`.

## 10. Mirror tint

- **Mods.** sniten: MirrorTint on (default 1), with RGB 150, 150, 150 (0 to 255 per channel) [community]. Aero:
  none stated.
- **Original.** No tint is applied to the mirror in the port's data. Original tint behaviour **[unconfirmed]**.
- **Proposed setting.** `mirror_tint`, three integers 0 to 255, default `[255, 255, 255]` (no tint). Multiplies the
  captured picture colour in `crates/nfsmw/src/app/mirror.rs` before the bezel clip, in the same place as the
  premultiply step (`clip_to_bezel`). Premultiplied alpha is kept.
- **Console and menu.** `mirror_tint` accepts `r g b` or `#rrggbb`; row "Mirror Tint" with three numeric rows.
- **Backends.** Native. Bevy only if its mirror path is checked (section 3).
- **Target.** `crates/nfsmw/src/app/mirror.rs`.

## 11. Memory and slot pools

- **Mods.** sniten: ExpandSlotPool (default 1) raises an object slot limit to 1,024,000 so that objects stop
  disappearing in busy scenes [community]. Aero: ExpandMemoryPools (default 1) raises several engine pool sizes,
  one of them to twice that value [community].
- **Original.** The stock limits are the reason objects vanish in the original. The stock pool sizes are not
  recovered in this pass [unconfirmed].
- **Port.** The port's slots (`libs/blackbox-render/src/gpu/slots.rs`) and instance buffers
  (`instances.rs`) grow at run time. A grep for fixed-capacity constants in those files found none.
- **Proposed.** No setting. Check instead: add a counter for slots and instances in the stats, and only add a cap
  if a busy test scene shows a limit.
- **Backends.** Native.

## 12. Backface culling

- **Mods.** Aero: RestoreBackfaceCulling (default 1) and DisableBackFaceCulling (default 0). The second turns
  culling off for all draws. A front-end car fix flips the culling state around the front-end car draw
  [community]. sniten: GeometryFix and "barrier animation" only, no culling option [community].
- **Original.** Mapped in [backface-culling.md](backface-culling.md): culling per texture and effect, a global
  enable byte and a per-view cull mode [exe].
- **Port now.** The colour, prepass, normal-map, window and motion pipelines cull single-sided draws by that rule;
  shadow maps and the sky keep both faces ([backface-culling.md](backface-culling.md) section 3).
- **Proposed setting.** `backface_culling`: `original` (default) or `off`. Applies to the car and front-end
  pipelines only. The port keeps its current behaviour when `original`.
- **Backends.** Native. Culling state is wgpu, so it is backend specific.
- **Target.** `libs/blackbox-render/src/gpu/pipelines.rs`, and the front-end car draw in `crates/nfsmw/src/frontend/`.

## 13. Front-end car reflection cube

- **Original.** Aero sets a front-end cube size (default 256) separately from the car cube [community]. In
  `speed.exe`, the six-face cube routine that uses 256 px when the car option is on is a candidate for the front
  end too [decomp, unconfirmed].
- **Port now.** No front-end car reflection. `crates/nfsmw/src/frontend` has no reflection code (grep).
- **Proposed setting.** `reflection_frontend`: `off` (default) or `on`. Size `frontend_size` = car face size (section 2).
  Only when the garage/car view exists in the port. Until then the row is hidden.
- **Backends.** Native.
- **Target.** `crates/nfsmw/src/frontend/` (new capture on the car view) and `libs/blackbox-render/src/gpu/glossy/env.rs`
  for the environment face upload.

## 14. Settings summary

All keys are top-level, like the existing `reflections` key in `crates/nfsmw/src/settings/file.rs`. Each needs a
`file.rs` field, an `env.rs` entry only if an environment override is wanted, a `write.rs` entry, a console entry,
a menu row and an availability rule.

| Key | Type | Default | Section | Console | Menu row |
|---|---|---|---|---|---|
| `reflection_car_scale` | percent 25-400 | 100 | 2 | yes | Car Reflection Size |
| `reflection_mirror_scale` | percent 25-400 | 100 | 3 | yes | Mirror Reflection Size |
| `reflection_road_scale` | percent 25-400 | 100 | 4 | yes | Road Reflection Size (hidden until pass) |
| `road_reflections` | off / on | off | 4 | yes | Road Reflections (hidden until pass) |
| `reflection_auto_size` | bool | false | 5 | yes | Auto Reflection Size |
| `reflection_pow2` | bool | false | 5 | yes | Power-of-Two Cube Size |
| `reflection_lod` | original / high | original | 6 | yes | Reflection Detail |
| `reflection_distance` | original / unlimited | original | 7 | yes | Reflection Distance |
| `reflection_content` | original / restored | original | 8 | yes | Reflection Content |
| `reflection_frontend` | off / on | off | 13 | yes | Front-End Reflections (hidden) |
| `vehicle_reflection_brightness` | percent 0-300 | 100 | 9 | yes | Vehicle Reflection Brightness |
| `mirror_tint` | 3 x int 0-255 | 255 255 255 | 10 | yes | Mirror Tint |
| `backface_culling` | original / off | original | 12 | yes | Backface Culling |

Each value is clamped on load with a log line. An unknown value falls back to the default, as the existing
`reflections` key does. Availability rules use the `Capabilities` struct: `local_reflections` gates every
reflection key; road and front-end keys stay hidden until their pass exists.

## 15. Verification plan

- Synthetic scene tests for size rules (scale, auto, pow2, clamp), distance caps and tint maths. They live with
  the pure functions in `libs/` and in `crates/nfsmw` tests.
- Installed-game comparison, by the user, not by this pass: same spot, heading and time of day, compare the mirror
  picture, the car reflection and the road reflection with and without each change. Record the result in the
  provenance file.
- Mod facts are compared, not copied. Where Aero and sniten disagree (mirror distance 384 or 192; vehicle
  brightness 1.0 or 0.8; LOD default 1 or 2; mirror scale rule), the original value wins, or the setting stays
  off until observed.

## 16. Open questions and next leads

1. Stock option values: the initial data values are 0 for car environment and road reflection. The value the
   options screen sets on a fresh install is not known. Next lead: the options-screen code that writes these
   globals (the registry writer's callers).
2. The meaning of the car environment option value. It selects the cube size when non-zero (256). Whether
   the value is also a level (for example, 1 or 2) is not confirmed. Next lead: the readers of the same global
   (`0x006bd0d0`, `0x006bd4b0`).
3. The front-end cube size: whether the 256 px routine is used for the front end too. Next lead: callers of the
   cube routine during the front end.
4. The 384 value of Aero's mirror distance is not found in the executable. Next lead: find float constants near
   the cube camera update (a byte search would be needed; none was run).
5. The `DO*` option names (`DOCarReflectRate`, `DOCarReflectDetail`, `DORoadReflectDetail`) have no readers other
   than unwind code. Next lead: the detail option table in the frontend graphics code.
6. Road reflection contents: which objects set the active flag per frame. Next lead: the flag at the object
   list in the road state machine.
7. Restored items in section 8 are mod claims. Each needs an in-game comparison.
8. The port's reflection mirror picture: the RVM mask and mirror shape are not part of this spec (see
   docs/specs/rear-view-mirror.md).
9. Bevy mirror support: unchecked. Next lead: whether `request_view_capture` runs the mirror path in the Bevy harness.

## 17. Port status

Part 1 (sections 2, 3, 5, 10) and part 2 (sections 6, 7, 8 traffic, 9; 12 documented) of the implementation.
Road, front-end cube (sections 4, 13) and the rest of section 8 are still open.

| Item | Status | Where |
|---|---|---|
| Car cube face size (section 2, 5) | Done, native only | `reflection_cube_size`; `ReflectionResolution::cube_face` in `libs/blackbox-gfx/src/reflection.rs`, used by `libs/blackbox-render/src/gpu/reflections.rs` |
| Rear mirror size (section 3, 5) | Done, native and bevy | `reflection_mirror_size`; `ReflectionResolution::mirror_size`, used by `crates/nfsmw/src/app/mirror.rs` |
| Power-of-two cube faces (section 5) | Done, cube faces only | `reflection_pow2` |
| Mirror tint (section 10) | Done, applied to every renderer's picture | `mirror_tint`; `tint_rgba` in `libs/blackbox-gfx/src/reflection.rs`, called in `crates/nfsmw/src/app/mirror.rs` before the bezel clip |
| Reflection LOD threshold follows the face (section 6, partial) | Done | `crates/nfsmw/src/scenes/world/reflections.rs` uses the resolved face size |
| Reflection LOD option (section 6) | Done, native | `reflection_lod`; the car cube takes the main view's 480 px rule with `visibility::collect_with_shadows` |
| Distance caps (section 7) | Done, native and bevy mirror | `reflection_distance`; `within` and `cull_instances` in `libs/blackbox-gfx/src/reflection/distance.rs` |
| Restored content: traffic in the car cube (section 8, traffic only) | Done, native | `reflection_content`; `scenes/world/reflections.rs` |
| Vehicle reflection brightness (section 9) | Done, native | `vehicle_reflection_brightness`; `MaterialUniform::new` and `Glossy::sync_brightness` |
| Backface culling (section 12) | Culling by the original rule; no setting yet ([backface-culling.md](backface-culling.md)) | 17.8 |
| Road, front-end cube (sections 4, 13); lights, flares, particles (section 8) | Not started | 17.10 |

### 17.1 Settings as built

Each size is one value: `auto`, a percentage (`150%`, 25% to 400%, out of range is an error as for `render_scale`), or
pixels (`256`). This replaces the proposed `reflection_car_scale`, `reflection_mirror_scale` and `reflection_auto_size`.

| Key | Default | Values | Env | CLI | Console | Menu row (Video options) |
|---|---|---|---|---|---|---|
| `reflection_cube_size` | `100%` | `auto`, `N%`, `N` px | `NFSMW_REFLECTION_CUBE_SIZE` | `--reflection-cube-size` | yes | Car Reflection Size |
| `reflection_mirror_size` | `auto` | `auto`, `N%`, `N` px (height) | `NFSMW_REFLECTION_MIRROR_SIZE` | `--reflection-mirror-size` | yes | Mirror Reflection Size |
| `reflection_pow2` | off | on, off | `NFSMW_REFLECTION_POW2` | `--reflection-pow2` / `--no-reflection-pow2` | yes | Power-of-Two Cube Size |
| `mirror_tint` | off | `off`, `on`, `#rrggbb`, `r g b` | `NFSMW_MIRROR_TINT` | `--mirror-tint` | yes | Mirror Tint (off or stock) |

The defaults keep today's picture. A 100% cube is the quality preset's face (128 at medium, 256 at high). A 100% mirror is
the bezel's 384 x 131. The tint default is off, which is the port's current look.

### 17.2 Rules

- Cube face: `auto` is the output height; `N%` multiplies the quality preset's face; `N` is pixels. Clamped to 16 to 1024,
  then rounded down to a power of two when `reflection_pow2` is on.
- Mirror: `auto` is the output height over 6; `N%` multiplies the base height 131; `N` is the height in pixels. The width
  always follows the bezel's aspect (384 : 131). The height is clamped to at least 64 and the width to at most 2048.
- Tint: `on` is 150 per channel (sniten's value). Its multiply keeps alpha. It is a CPU step on the captured picture, not a
  change to the HUD shader.
- Device limit: the rules take the largest texture edge. The native renderer and the mirror pass 2048 (`PLANAR_MAX`), the
  wgpu downlevel minimum, so no device query is needed.

### 17.3 Choices and deviations

- The auto mirror uses the output (surface) height. Section 5 says "after render scale"; the choice here is the output
  height, not the internal render height. Using the internal size would only change the `output_height` argument.
- The car cube and the LOD threshold read the same resolved face size, so the detail threshold follows the capture
  (section 6).
- Menu rows show only where the renderer has car reflections (`caps.local_reflections`). That gates the mirror rows too,
  as a proxy: the mirror is HUD-fed and its bevy path is not checked (section 3, lead 9). Cube and power-of-two rows are
  also greyed out when car reflections are off or car shading is simple.
- The menu steps size rows through 50%, 100%, 150% and 200% (plus `auto`). A value outside that list steps from `auto`.
  The tint row toggles off and stock.

### 17.4 Bevy

`libs/blackbox-bevy-render/src/apply/capture.rs` was not changed. Its `begin` takes the size from the capture request, so
the mirror size reaches the bevy capture path as it is. Bevy reports `local_reflections: false`, so there is no car cube
capture there: the cube size is native only. The mirror tint is applied before the picture reaches the renderer, so it
applies on bevy too if the mirror runs there (still unconfirmed).

### 17.5 Verification

- Unit tests (pure rules): `libs/blackbox-gfx/src/reflection.rs` covers size text, cube and mirror resolution, auto, scale,
  power-of-two, device clamps, sanitizing, tint text and tint maths.
- Settings tests: `crates/nfsmw/src/settings/reflection_res_tests.rs` covers defaults, menu cycle and labels, config round
  trip through `write::merge` and `file::parse`, whole-pixel and channel-array forms, and environment layers.
- Console and menu: `settings_cmd.rs` covers get, set and validation; `graphics_options.rs` covers the new rows.
- Results: `cargo test -p blackbox-gfx --lib` 103 passed; `cargo test -p blackbox-render --lib` 38 passed;
  `cargo test -p nfsmw -- settings::` 241 passed; `saves::` and the reflection and console filters pass.
  `cargo clippy -p blackbox-gfx -p blackbox-render -p nfsmw -- -D warnings` is clean.
- Not verified: the picture in game. The user compares the mirror, car reflection and road reflection on the installed
  game with each setting (section 15). No window was launched.

### 17.6 Open after part 1

- `crates/nfsmw/src/devtools/console/parse.rs` has a fixed list of completion names. The eight part-1 and part-2 keys are not in it.
  That file was outside this task, and its completion test asserts the `re` prefix set.
- The menu has no rows for the custom tint colour or fixed pixel sizes. Those are set through the config file or console.

### 17.7 Part 2 settings as built

Values follow part 1's text rules. Every key is native-only in effect; the menu rows use `caps.local_reflections`.

| Key | Default | Values | Env | CLI | Console | Menu row (Video options) |
|---|---|---|---|---|---|---|
| `reflection_distance` | `unlimited` | `unlimited`, `original`, `25m`–`2000m` | `NFSMW_REFLECTION_DISTANCE` | `--reflection-distance` | yes | Reflection Distance |
| `reflection_lod` | `capture` | `capture`, `view` | `NFSMW_REFLECTION_LOD` | `--reflection-lod` | yes | Reflection Detail |
| `reflection_content` | `original` | `original`, `restored` | `NFSMW_REFLECTION_CONTENT` | `--reflection-content` | yes | Reflection Content |
| `vehicle_reflection_brightness` | `100%` | `N%` or `N`, 0 to 300 | `NFSMW_VEHICLE_REFLECTION_BRIGHTNESS` | `--vehicle-reflection-brightness` | yes | Vehicle Reflection Brightness |

The four rows sit after Power-of-Two Cube Size and before Mirror Reflection Size. Brightness needs glossy cars; detail and
content also need the car cube on.

### 17.8 Part 2 choices and deviations

- Defaults keep today's picture (unlimited, capture, original, 100%). The proposed defaults in sections 7 and 6 would change it.
- LOD values are `capture` and `view`, not the proposed `original` and `high`: the request asked for the main view's rule.
- Distance is a cull, not a far plane. `Projection` (`libs/blackbox-gfx/src/frame.rs`) has no far variant, and that file is
  outside part 2. The limit tests each object's origin: 100 m from the cube origin for the car cube, 192 m from the mirror
  eye for the mirror (`app/mirror.rs`, so bevy's mirror path gets it too). Objects straddling the limit are kept or dropped whole.
  The cube's near plane stays 0.25 m; the original's 0.5 m is not applied, to keep the default.
- Content: `restored` adds the traffic cars to the car cube. Each face keeps the cars inside its frustum, using a 4 m half-box.
  The mirror already drew traffic. Lights, flares and particles are not added: the cube capture takes only placed scenery,
  props and traffic, and no list of lights, flares or particles reaches it. That is a new input, not a cheap flag.
- Brightness multiplies `envmap_power`. Each material keeps its source value, and `Glossy::sync_brightness` packs the
  block again when the value changes. It runs from `set_lighting_rig`, which the world calls every frame.
- Backface culling: `libs/blackbox-render/src/gpu/pipelines.rs` line 186 is in `Pipelines::make`, which builds every scene
  pipeline (blend modes by Lit, Prelit, Sky and Glossy). So `cull_mode: None` covers all scene draws, not only the car, and
  the car cube and mirror captures use the same pipelines (`encode_scene_mode`). A car-only setting needs a second pipeline
  variant, and the original's culling state is still unconfirmed, so `off` would equal today's state. Nothing changed.

### 17.9 Part 2 verification

- `cargo test -p blackbox-gfx --lib` 117 passed; `cargo test -p blackbox-render --lib` 39 passed; `cargo test -p nfsmw`
  1690 passed, 113 ignored (game or GPU needed). Clippy with `-D warnings` is clean for the three crates.
- Pure rules and settings are tested in the files named in 17.7 (text, clamps, cycles, labels, config round trip, environment
  over file, console get and set, Video row counts and placement).
- Not verified in game: the picture. Compare on the installed game (section 15).

### 17.10 Open after part 2

- A true far clip needs a `Projection` far variant in `libs/blackbox-gfx/src/frame.rs`. Origin culling stands in for it.
- Lights, flares and particles in the car cube (section 8) need a scene list passed to the capture.
- Car and front-end backface culling (section 12) needs a second pipeline variant and the original's culling state.
- Road reflection and front-end cube (sections 4, 13) are not started.
- `crates/nfsmw/src/settings/mod.rs` is at 499 lines. The next key should go in a sub-struct, as `ReflectionExtra` does.
