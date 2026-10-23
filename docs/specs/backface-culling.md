# Back-face culling

Which faces of a triangle the original draws, and how the rewrite reproduces it.

- **Sources read:** the install's `speed.exe` (PC build), Ghidra MCP, program `/speed.exe`, image base `0x400000`.
  Functions are named by address; the method and the address list are in
  [docs/provenance/backface-culling.md](../provenance/backface-culling.md). No decompiled code is reproduced here.
- **Data inputs:** the texture info record of each texture pack (`AlphaUsageType` 0x55, `AlphaBlendType` 0x56,
  `Flags` 0x57; [../formats/textures.md](../formats/textures.md)), the shading group's effect id
  ([../formats/models.md](../formats/models.md), [world-material-routing.md](world-material-routing.md)) and the
  scenery instance's flag word ([../formats/maps.md](../formats/maps.md)).

Evidence tags as in the [docs README](../README.md#evidence-tags). **[exe]** means read from `speed.exe` in Ghidra.
**[verified]** means measured on the install with the Rust readers or a short script; the numbers are from those runs.

## 1. The rule in the original

The original has no sidedness flag on solids, shading groups or materials. It decides culling every time it binds
the diffuse texture of a draw, and hands the result to the effect as its `CULL_MODE` parameter (the effects' render
states read it) [exe]:

1. **Copy-mode texture** (`AlphaBlendType` 0): culled, unless `AlphaUsageType` is 1 (punch-through) or bit 1
   (`0x02`) of the info record's `Flags` byte is set. Either of those draws both faces.
2. **Blended texture** (`AlphaBlendType` not 0): both faces, except with the car effect (effect id 4, `CarShader`),
   which culls blended surfaces too (car glass). For blended textures the punch-through value and `Flags` are not
   consulted.
3. **Global switch:** a byte in the executable's data, 1 in the shipped file, turns culling off for every draw when
   it is 0. Ghidra finds no write to it [exe]. (Aero's `DisableBackFaceCulling` option, see
   [hd-reflections.md](hd-reflections.md) section 12, presumably clears it [unconfirmed].)
4. **Which side:** a global cull mode chosen per view: 1 draws both faces (its value at start-up), 2 keeps the faces
   that run counter-clockwise on screen (`D3DCULL_CW`), 3 keeps the clockwise ones (`D3DCULL_CCW`) [exe]. The world
   render sets 3 for the road reflection and the six environment-map faces, then 2 for the player view and what
   follows it; the other render paths set 2 [exe].
5. **Per-draw reversal:** a draw flag (`0x40000` in the render call's flag word) swaps the kept side once more [exe].
   - Scenery sets it for every instance whose flag word has `0x400000` [exe].
   - The car renderer sets it on four of its draw calls [exe]; which parts they are (presumably the mirrored left
     wheels and brakes, [car-assembly.md](car-assembly.md) sections 3 and 4) is not established [unconfirmed].

`Flags` bit 0 and the other bits do not enter the rule.

## 2. What the data says

**Texture flags [verified].** Over the 4,273 textures of the car packs and the streamed world (`STREAML2RA.BUN`):
`Flags` 0 on 1,712, 1 on 2,316, 2 on 21 and 3 on 224, so bit 1 is set on 245 textures. Among them every car's
`*_MISC`, the cop cars' `*_BADGING` and `*_TIRE`, and world textures used by both copy-mode and blended groups.

**Winding of the car models [verified].** For every triangle the right-hand-rule normal `(b - a) × (c - a)` was
compared with the sum of its three stored vertex normals. Over the 100 car geometry files the Rust reader opens,
6,748,296 triangles point the same way and 4,986 (0.07 %) the other way (test
`car_models_are_wound_counter_clockwise_around_their_normals` in `libs/blackbox-solid`). Cars are closed models drawn
with culling, so a triangle is front-facing when its corners run counter-clockwise (right-hand rule) as seen from
the camera. This matches mode 2 keeping the counter-clockwise faces.

**Winding of the world [verified].** The same comparison on the streamed world is not a usable reference: with
effect 1 99.99 % of the triangles agree with their normals, with effect 0 90.5 %, with effect 3 76 %, and the
foliage cards of effect 6 are half and half. World normals are lighting data, not facing; the original culls by
winding alone, and so does the rewrite.

**Mirrored scenery [verified].** Of 77,783 scenery instances in the stream, 5,797 have a rotation with a negative
determinant (a mirror), and all of them have flag `0x400000`. 321 more have the flag with an identity rotation: all
are world-space copies (flag `0x200`, names `*_DEINST*`) whose vertices already hold the mirror, so their triangles
are wound the other way round. The flag reverses the kept side for both kinds.

## 3. The rule in the rewrite

The renderer keeps the counter-clockwise faces of a single-sided draw (wgpu `FrontFace::Ccw`, back faces culled).

- `blackbox_gfx::DrawRange::two_sided`: the draw keeps both faces. Every hand-built mesh and every mesh whose
  lookup does not know its textures sets it.
- `blackbox_gfx::Instance::flip_winding`: the mesh is wound the other way round in this placement.
  `Instance::mirrored()` is true when the transform mirrors (negative determinant) or `flip_winding` is set, but not
  both. A mirrored instance keeps the clockwise faces.
- `FrameParams::mirrored_view()`: the view mirrors (negative determinant). It swaps the kept side once more. The
  environment cube's faces are drawn with a vertically flipped view, so they are mirrored views; this replaces the
  original's mode 3 for cube views. The projected road view shares the main camera,
  so its capture flag explicitly reverses the kept side once more. Per-instance
  winding and two-sided texture rules still apply; sky and particles stay two-sided.
- The texture rule is `blackbox_scene::Sidedness` (rules 1 and 2 without the effect); `MaterialLookup::two_sided`
  asks the game which effects cull blended surfaces. In the game crate that is `scenes/culling.rs`: effect 4.
- Scenery: `flip_winding` is flag `0x400000` exclusive-or a mirroring transform, so it is set only for the
  world-space copies with the mirror baked in. Mirrored instances are already reversed by their transform.
- Car parts: left wheels and brakes are drawn with a mirroring transform ([car-assembly.md](car-assembly.md)), so
  their transform reverses them; this stands in for rule 5's car flag.

**Passes.** The colour pass (scene and glossy pipelines), the normal-map and window families, the camera depth
prepass (the colour pass tests `Equal` against it, so both keep the same faces), the motion-vector redraw and the
screen-space glossy pass cull. The pipelines get a variant per kept side, built once the first mesh with a
single-sided draw exists. These keep both faces:

- the sun's shadow maps: the original's cull mode while it draws them was not read, and culling would let open
  single-sided geometry (walls, fences, the `*_DEINST` copies) stop casting from behind;
- the sky layers, effects, particles and the UI;
- the Bevy backend, which ignores `two_sided`.

The global switch of rule 3 has no setting yet; the rewrite always culls where the original does by default.

## 4. Checks

- `libs/blackbox-render`, `cull_tests.rs` (GPU): a single-sided quad keeps its counter-clockwise face, a mirroring
  instance, `flip_winding` and a mirroring view each swap the kept side and two of them cancel; the culling
  pipelines appear only after the first single-sided mesh.
- `libs/blackbox-render`, `draw_list_tests.rs`: runs of one mesh split where the mirroring changes, only for meshes
  with a single-sided draw.
- `libs/blackbox-scene`, `sides.rs`; `crates/nfsmw`, `scenes/culling.rs` and the world material tests: the texture
  and effect rules.
- In the running game: drive past a mirrored `*_DEINST` fence and a mirrored street prop and check both sides look
  as in the original; look through car glass from inside the car (front end) and at a car's mirrored left wheels.

## 5. Open questions

- The cull mode while the shadow maps and the front-end car are drawn.
- Which car draws set the reversal flag, and whether anything else does.
- Whether the rear-view mirror view is drawn mirrored in the original.
