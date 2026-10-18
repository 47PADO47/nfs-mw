# Vehicle damage

Visible, persistent damage for the cars in the world: dented panels, scraped paint, cracked glass,
punctured tires. What the original does and what the port does differently is marked below.

## What drives it

Impacts already reach the sequencer ([world-sequencers.md](world-sequencers.md)). Each impact is
classified into one of the ten zones `DZ_FRONT`, `DZ_REAR`, `DZ_LEFT`, `DZ_RIGHT`, `DZ_LFRONT`,
`DZ_RFRONT`, `DZ_LREAR`, `DZ_RREAR`, `DZ_TOP`, `DZ_BOTTOM` and gets a severity from 0 to 6:

    severity = clamp(force / FORCE * VISUAL_SCALE[zone], 0, 6)      (FORCE in N, from damagespecs)

Hits below a closing speed of 1 m/s, hits flagged `no_car_effect`, and hits past `SUPPRESS_DIST` do
not count, as before.

## Levels

[`blackbox-damage`](../../libs/blackbox-damage) keeps three levels per zone, each 0 to 3:

| Level | Rises when | Shows as |
|---|---|---|
| `paint` (scrape) | severity 1 → 1, 3 → 2, 5 → 3 (max of all impacts) | a dark scuff on the zone (procedural) |
| `panel` (deformation) | every impact of severity 2 or more, one step each | the zone's corner overlay (`DAMAGE0_*`), where it has one |
| `glass` (crack) | a glazed zone, severity 2 or more, one step each; severity 5 or more sets 3 at once | two pale crack lines per level (procedural) |

Glazed zones are every side and corner zone; the roof and underbody have none
(`nfsmw-data/src/car/damage.rs`, `ZONE_HAS_GLASS`). Levels only rise. The only reset is
`Damage::repair`, called by the `repair` console command.

## Corner overlays

The body kit's corner overlays `DAMAGE0_FRONT`, `DAMAGE0_FRONTLEFT`, `DAMAGE0_FRONTRIGHT`,
`DAMAGE0_REAR`, `DAMAGE0_REARLEFT` and `DAMAGE0_REARRIGHT` (slots 46–51) are placed on every car. The
rig draws one only while its zone's panel level is at least 1 (`revealed_overlays`). The car viewer
skips overlays, so the garage shows the undamaged car.

## Scuffs and cracks

The procedural bounding-box squares and crack-line quads have been removed from
normal rendering. They did not follow the body/window geometry and could float
near the wheels. Installed corner overlays and cracked-window texture swaps
remain; original paint/deformation parity is still incomplete.
## Punctured tires

Two parts. The **tire state machine** is the spike-strip rule of the original
([ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md) section 6.3), implemented in
[`libs/blackbox-vehicle/src/tires/damage.rs`](../../libs/blackbox-vehicle/src/tires/damage.rs):

- a sound tire that is punctured stays inert for 0.5 s (180 s on run-flat tires), then is **blown**;
- a blown tire takes the full brake command (`brake = 1`, handbrake released, `brakes::wheel_command`) and
  keeps 30 % of its traction (`BLOWN_TRACTION`).

The **trigger** is contact with a roadblock spike strip. Ordinary impacts,
including the severity-6 damage command, never puncture a tire. The world's
strip test sweeps consecutive grounded patches across the inflated strip box,
including elevation. A reset or large teleport discards the previous patch.
The visible strip and its dimensions remain procedural; exact installed-prop
collision and original swept-box parity are unverified. See
[pursuit-free-roam](pursuit-free-roam.md) and
[pursuit-system](../pursuit-system.md). Run-flat data remains unread. `repair`
resets tire state.
## Not done

- **Scraped paint texture.** The procedural overlay is removed; original paint scuff rendering remains missing.
- **Cracked windows.** The cracked-window texture is `WINDOW_DAMAGE0` in `GLOBAL/GLOBALB.BUN` (key
  `0x0A155545`). Each glazed zone with a crack swaps its window placeholders to it (`zone_windows`,
  `TextureSwaps::with_window_hashes`), re-uploading the car when the set changes. Confirmed in a screenshot after a
  glazed rear hit: the rear window shows the shattered-glass texture. The swap keys on the window solid's own
  diffuse texture (`zone_window_solid`), because the rear glass does not draw `WINDOW_REAR`.
  (Earlier note, now superseded:) a screenshot
  after a rear hit shows the rear panels lighter, but the crack pattern on the window is not yet confirmed.
- **Original spike prop presentation** and the complete original `blown_tires` integration.
- **Repair in the garage or between races.** The `repair` world console command clears the damage and
  the punctures; no in-game repair exists yet.
- **Original damage visual parity.** Body damage has no generic handling penalty; spike blowouts affect the tires.
- **Persistence** across sessions. Damage is per run.

## Code

- `libs/blackbox-damage`: the pure model and its tests.
- `libs/blackbox-vehicle/src/tires/damage.rs`: the tire state machine (`Vehicle::tire_damage`); `vehicle/assists.rs` applies it.
- `crates/nfsmw-data/src/car/damage.rs`: zone → overlay slot, glazing, zone → wheel, `revealed_overlays`.
- `crates/nfsmw/src/scenes/world/events.rs`: `VehicleEvents.damage`, fed by `advance()`.
- `crates/nfsmw/src/scenes/world/damage_marks.rs`: the procedural scuffs and cracks.
- `crates/nfsmw/src/scenes/world/drive/rig.rs`: `CarRig::instances` takes the revealed set.

## Testing it in the game

- `damage <zone> <severity 0-6> [glass]` (world console, zone `0`–`9` or `DZ_FRONT` … `DZ_BOTTOM`) hits the
  car as an impact would; `repair` clears it. A hit before the car exists is queued and applied when it is
  placed.
- The car's damage specs come from the nearest ancestor in its `pvehicle` lineage that has them. The
  sequencer is optional: a car without a sequencer still takes damage (the model, the overlays and the
  punctures), but its timed sequencer effects do not play. For the BMW M3 GTR the install's `pvehicle`
  records carry damage specs but no `EventSequencer` key, so its sequencer effects are not played. The key
  for ordinary cars is not located yet.
- A screenshot run (`view-world --drive BMWM3GTR --screenshot … --exec "damage DZ_REAR 6 glass"`) shows
  the impact sparks but not the overlay or the cracks at the rear. Whether the overlay models are placed for
  this car is not checked. Checked: the six corner overlays are placed on the BMW (`DAMAGE0_*`, at the body frame's
  identity transform, the same frame as the body), and the rear overlay is revealed (slot 49) and drawn after a
  rear hit. The overlays are body-frame parts, not corner-sized patches, so whether they show in front of the
  body depends on the original's assets, which are not confirmed.

## Original glass damage (Ghidra, speed.exe)

- The string `_GLASS_DAMAGE0` (`0x008b4eac`) is used only by `FUN_00751860`, the car-part assembly. It builds
  the part names of the car's body parts from `<CAR>_KIT<nn>` and a suffix.
- The `_GLASS_OFF`, `_GLASS_ON` and `_GLASS_DAMAGE0` suffixes are attached to the **headlight and brake-light
  parts** (the lamp glass), not to the windows. So the original's glass damage is a damaged lamp-glass model
  swapped in for the intact one, found by part name.
- The windscreen and side-window cracks are not identified in the binary yet. The earlier
  `WINDOW_DAMAGE0` note (`docs/specs/car-assembly.md`) is not confirmed by this function.
- Not implemented: the lamp-glass swap. It needs the exact `<CAR>_KIT<nn>_HEADLIGHT*` names from the parts
  database and a per-lamp zone map, which `car/assemble.rs` does not have yet.

## Searching the install for crack and window damage (checked)

- **Method.** Texture keys are the engine's name hash, little-endian in the archive
  (`blackbox-hash`; the `BMWM3GTR_BADGING` key `0x33CCB33D` appears in `CARS/BMWM3GTR/TEXTURES.BIN` as
  `3d b3 cc 33`). A candidate name is hashed and its bytes are searched for in the archive.
- **Geometry.** `CARS/BMWM3GTR/GEOMETRY.BIN` has no window damage parts; only the lamp glass
  (`*_HEADLIGHT_GLASS_A`–`D`) and the `DAMAGE0_*` corner parts. The midsize cop car
  (`CARS/COPMIDSIZEINT/GEOMETRY.BIN`) has `_DAMAGE0_FRONT_WINDOW_A`, so window damage *geometry* exists for at
  least one car.
- **Textures, negative so far.** No crack, scrape, scratch, dirt, damage or window-damage name hashed to a key in
  the BMW's `TEXTURES.BIN` (`BMWM3GTR_` and bare forms of `WINDOW_DAMAGE0`, `GLASS_DAMAGE0`, `WINDOW_CRACK`,
  `CRACKED_GLASS`, `SCRAPE`, ...). This does not prove absence: the control `WINDOW_FRONT` also did not hash to a
  key in that file (its key is only in `GEOMETRY.BIN`), so the names the window textures use are not yet known.
- **Next step.** Find how `TEXTURES.BIN` (or the global texture packs) name the window texture keys, using a
  texture that is known to be a window, then repeat the hash search with that naming.

## Window texture (found by the hash search)

- `WINDOW_DAMAGE0`, `WINDOW_FRONT` and `WINDOW_MASK` are in `GLOBAL/GLOBALB.BUN` (the global pack, read with
  `blackbox_tpk::read_texture_packs`). Their keys are the name hashes. The earlier negative search (the car's own
  `TEXTURES.BIN`) was the wrong file; the window textures live in the global pack.
- Resolved: the BMW's rear glass (`KIT00_REAR_WINDOW_A`) draws its own diffuse texture, not `WINDOW_REAR`; a swap on
  the placeholder changed nothing on the glass. The lighter panels seen earlier were from that same wrong swap.
- Still open: the side windows (zones 2 and 3) have no window solid in the BMW, so they keep the placeholder
  swap only; and the glass is checked on the rear window only. The front and side-window cracks are not yet
  seen in a screenshot.

## Lamp-glass swap: no asset in the install

- The original's damaged lamp glass is named by `_GLASS_DAMAGE0` in `FUN_00751860`. A search of every
  `CARS/*/GEOMETRY.BIN` for `GLASS_DAMAGE0` finds nothing: the install has the intact lamp glass
  (`*_HEADLIGHT_GLASS_A`–`D`) but no damaged variant. So the lamp-glass swap has no model to swap to, and it is
  not implemented.

## Window cracks: status per zone

- **Rear (zone 1).** Confirmed in a screenshot: the rear window shows the shattered-glass texture. Swapped by the
  rear window solid's diffuse texture.
- **Front (zone 0).** Confirmed in a screenshot: after a glazed front hit the windshield shows the shattered-glass
  texture. The camera for that check is a free camera at `goto 2143 1391 2` (it frames the car from the front-left;
  the chase camera faces the rear). The windshield solid `BMWM3GTR_KIT00_FRONT_WINDOW_A` uses the same swap path as
  the rear.
- **Side (zones 2 and 3).** The BMW draws its side windows from the body solid (`BASE_A`) through the
  `WINDOW_LEFT_*` / `WINDOW_RIGHT_*` placeholders, so those zones swap those placeholders. **Not confirmed:** a
  glazed left-side hit at the same camera left the driver's-side glass without a visible crack texture. The swap is
  configured, and checked in the rig: the left-window hash reaches the swap map, and the damaged texture is in the
  car's loaded textures. The placeholder `WINDOW_LEFT_FRONT` itself is not a loaded texture. So the swap should
  apply; why the driver's-side glass still does not show the crack is unexplained.
- **Finding (probe).** Every window group on the BMW (windscreen, rear, and the side glass on `BASE_A` and
  `KIT00_*_WINDOW_A`) has effect `0x4`, flags `0x4180`, and an **opacity** texture of `WINDOW_FRONT` (key
  `0x7B220DDF`). The swap changes only the diffuse texture, so the side glass keeps the windscreen's opacity
  mask. **Refuted as a cause:** the port's material lookup (`scenes/car/materials.rs`, `Lookup::material`) resolves
  only the diffuse hash and never reads the opacity slot, so the windscreen and the side glass take the same path
  (same damaged texture, same blend mode).
- Earlier reading, kept for the record: the mask might hide the crack on the side glass. The windscreen crack shows with the same
  mask, so the mask alone does not explain it. Next check: which solid draws the driver's-side glass in the
  drawn car (the `BASE_A` group 4, 13 vertices) and whether it is placed where the camera sees it.
- **Side view, checked.** A free camera at `goto 2132 1399 1.5` shows the car from the side after a left hit: the
  side glass still looks dark and intact, with no crack. The side-glass groups are small (11 to 18 triangles, against
  56 for the windscreen), so the likely reason is that the crack texture covers too little of the glass to read at
  this distance.
- **UV check, done: the texture-scale explanation is refuted.** Every window group spans nearly the full texture
  (U 0.00–0.99, V 0.23–0.92 on the windscreen; the driver's-side group U 0.00–0.99, V 0.29–0.71). So the side glass
  maps the whole crack texture onto a small surface, and the crack should still read. The cause is open: it is not
  the UVs, the opacity slot, or the placeholder names. Next: check whether the side glass draws at all with the
  damaged texture (for example with a debug colour on that group).

## Side windows: confirmed at close range

- A free camera at `goto 2146 1399 1.0` (about 5 m from the car, side on) after a left hit shows the **driver's-side
  front window cracked** (the shattered pattern). The earlier misses were distance: at 13 m the car is about 40 px
  across, too small for the crack lines to read. The undamaged and damaged side views also differ in the glass.
- The **rear quarter window** on the same side still looks intact. Zone 2 swaps `WINDOW_LEFT_FRONT` and
  `WINDOW_LEFT_REAR`, and the quarter glass is the `BASE_A` group 0 (`WINDOW_LEFT_REAR`, 18 vertices). Why it does
  not show is open: check whether that group is drawn with the swapped texture at this range, as for the front.

- **Checked: the quarter window does not take its own zone's swap.** After a left-*rear* hit (`DZ_LREAR`, zone 6,
  which swaps `WINDOW_LEFT_REAR` and the rear-left solid's diffuse), the driver's-side front window shows a small
  crack patch and the rear quarter window is still intact. So zone 6's swap reaches neither the quarter window nor
  its solid in a way that shows. The mapping and the texture load are correct, so the draw path for that group
  differs (candidate: a group with its own material or placement). Open.

## Quarter window: tint test (done)

- A temporary swap of `WINDOW_LEFT_REAR` to `WINDOW_MASK` (reverted) left the driver's-side **quarter window**
  unchanged, so that window is **not drawn with `WINDOW_LEFT_REAR`**: its texture is another hash.
- The same test removed the crack patch that had appeared on the **front-door glass** after a left-rear hit. So the
  placeholder names do not map one-to-one to the physical windows: `WINDOW_LEFT_REAR` also covers part of the
  front-door glass. The placeholder-to-window mapping in `zone_windows` needs a per-solid check.
- Next: list the diffuse hashes of every group on `BMWM3GTR_BASE_A` and of the `KIT00_*_WINDOW_A` solids, match them
  to the window placeholders and to the damaged texture, and find the quarter window's hash.

## Correction: the tint test was inconclusive

- The tint test above swapped `WINDOW_LEFT_REAR` to `WINDOW_MASK`, an 8×8 texture that may itself look like dark
  glass. So "the quarter window did not change" does not show the group is off the swap path. The conclusion that
  the quarter group does not use `WINDOW_LEFT_REAR` is withdrawn.
- Data check (done): four window groups draw from placeholder hashes with no loaded texture of their own
  (`4cdebfca` = `WINDOW_LEFT_REAR`, `e7e4ef49`, `0ab88f5d`, `60f8b13c`). Their only texture is the swap, so the damaged
  texture reaches them through `Lookup::material`. `CarMaterials::upload` uploads every loaded car texture, so
  `WINDOW_DAMAGE0` is on the GPU.
- Open: a swap test with a texture that is obviously different from glass (for example `REAR_DEFROSTER` or a bright
  diffuse) would show whether the quarter group takes the swap.

## Quarter window: REAR_DEFROSTER test (done)

- With `WINDOW_LEFT_REAR` swapped to `REAR_DEFROSTER` (a loaded, clearly non-glass texture; reverted after the shot),
  the driver's-side quarter window still looks like glass. So under the damage swap the quarter window is not drawn
  through `WINDOW_LEFT_REAR` at that view. The path that draws it is still unidentified; this supersedes the
  inconclusive `WINDOW_MASK` test above.

## Quarter window: four-texture test (done)

- The four unnamed window-group texture hashes (`4cdebfca`, `0ab88f5d`, `e7e4ef49`, `60f8b13c`) were all swapped to
  `REAR_DEFROSTER` at once (reverted). The driver's-side front door glass changed slightly, but the quarter window
  still looks like glass. So the quarter glass is drawn from a texture outside those four hashes and outside the
  window placeholder set. Candidates: a body texture on `BMWM3GTR_BASE_A` (groups 3 and 6 use `WINDOW_MASK` and
  `REAR_DEFROSTER`) or a texture that is not loaded. Open.

## Quarter window: solid and mask tests (done, inconclusive)

- Every solid on the BMW was listed with its group textures. `BMWM3GTR_KIT00_BODY_A` has a group with diffuse
  `WINDOW_MASK` (and one unnamed hash `3617dcfc`). Tinting `WINDOW_MASK` to `REAR_DEFROSTER` (reverted) did not
  visibly change the quarter window.
- Method caveat: tinted textures barely show on the side glass, so these tints cannot prove which texture draws it.
  A source test on this glass needs a different probe (for example a solid colour the glass cannot blend away, or
  hiding one group at a time).
- Open: the quarter window's source. Candidates left: `3617dcfc` on `KIT00_BODY_A`, and the glass blend itself.

## Quarter window: per-group hide test (done)

- A temporary change dropped the indices of `BMWM3GTR_KIT00_BODY_A` group 1 (the `WINDOW_MASK` group) at upload
  (reverted). The driver's-side quarter window is unchanged, so that group is not its source. The front-door
  crack patch also remains, so that patch is drawn by another group.
- Open: the quarter source is one of the other body groups or the glass blend. Next test: hide each remaining group
  of `KIT00_BODY_A` and `BASE_A` in turn with the same method.

## Quarter window: hide-all tests (done)

- Hiding every group of one solid at a time (reverted after each shot), the driver's-side quarter window stayed in
  place for: `KIT00_BODY_A` (the body panels above the windows vanished, the quarter area did not), `BASE_A` (the
  front-door crack patch stayed, so that patch is not from `BASE_A`), and `KIT00_REAR_LEFT_WINDOW_A`.
- So the quarter-window area is not drawn by any of these solids. Hypothesis, not yet tested: the quarter area has
  no textured glass group in the model at all, and the interior is visible through it. Test: find the solid that
  draws the front-door crack patch (`KIT00_FRONT_LEFT_WINDOW_A` is the candidate), and check whether any solid covers
  the quarter area when everything else is hidden.

## Front-door crack source (attempted, blocked)

- Next test: hide `BMWM3GTR_KIT00_FRONT_LEFT_WINDOW_A` (all groups) after a left hit and check whether the front-door
  crack patch disappears. The attempt was blocked: the build could not replace `target/debug/nfsmw.exe` (locked by a
  running process), so no screenshot was taken. The probe edit was reverted.

## Front-left window solid: hide test (done)

- With `BMWM3GTR_KIT00_FRONT_LEFT_WINDOW_A` fully hidden after a left hit (reverted), the driver's-side front-door
  crack pattern stays, and so does the quarter area. So that solid draws neither the front-door crack nor the quarter
  glass. Solids tested so far, with all groups hidden: `KIT00_BODY_A`, `BASE_A`, `KIT00_REAR_LEFT_WINDOW_A`,
  `KIT00_FRONT_LEFT_WINDOW_A`. None of them removes the quarter area. The quarter-area hypothesis (no textured glass
  group there) is still the leading one, and the front-door crack source is still unidentified.

## Full solid list and the interior test (done)

- The car has 24 solids (listed unfiltered). There is no door solid. The windows are `BASE_A` (placeholder groups),
  `KIT00_FRONT_WINDOW_A`, `KIT00_FRONT_LEFT_WINDOW_A`, `KIT00_FRONT_RIGHT_WINDOW_A`, `KIT00_REAR_WINDOW_A`,
  `KIT00_REAR_LEFT_WINDOW_A` and `KIT00_REAR_RIGHT_WINDOW_A`.
- Hiding `KIT00_INTERIOR_A` (all groups, after a left hit, reverted) removes the seats but leaves both the front-door
  crack and the quarter area. The solids hidden so far, with all groups: `KIT00_BODY_A`, `BASE_A`,
  `KIT00_REAR_LEFT_WINDOW_A`, `KIT00_FRONT_LEFT_WINDOW_A`, `KIT00_INTERIOR_A`. None removes either.
- Open: the front-door crack and the quarter area are not drawn by any single solid of the car in this set. The
  remaining explanations are a draw path outside the car solids (for example a window decal or a reflection pass) or
  a group inside a solid that is drawn by a different mesh. Settling it needs a draw-call trace, not more solid hides.

## Draw-call trace: scope (not started)

- The quarter-window and front-door source questions need a per-draw ID pass: each draw writes its solid and group ID
  into an ID target, and a readback returns the ID at the pixel under the window. `blackbox-render` has no such path
  (its readback is the screenshot only). This is a new wgpu pass in the renderer, not a probe, so it is not started.

## Draw-ID pass: narrowed scope

- `blackbox-gfx` already gives every drawn instance an identity: `InstanceKey` (a group and an index, for a car part
  `keys::car_part(car, placement)`). So the trace needs no new identity scheme.
- What it needs: a pick target in `blackbox-render` (`gpu/frame.rs` draws instances in `encode_scene_mode`) that
  writes each instance's key per pixel, and a one-pixel readback at the quarter-window screen position. That is the
  remaining work, a wgpu change in the shared renderer, and it is not built yet.

## CPU ray pick of the car's left side (done, no renderer change)

- A CPU ray cast (Moller-Trumbore) over every car triangle in car space, with rays from outside the left side
  (`y = +6`, travelling `-y`) across a band of x and z, reports the first surface each ray meets. Probe removed.
- First hits: `BASE_A` group 4 (the windscreen-side window, `WINDOW_LEFT_FRONT`) 130 rays; `BASE_A` group 0
  (`WINDOW_LEFT_REAR`) 55 rays; `BASE_A` group 3 (`WINDOW_MASK`) 39; `KIT00_BODY_A` group 8 237 rays (body); the
  damage overlays `KIT00_DAMAGE0_REARLEFT_A` 170 rays and `KIT00_DAMAGE0_FRONTLEFT_A` 102 rays.
- Overlay test: after a rear-left hit, `KIT00_DAMAGE0_REARLEFT_A` hidden (reverted). The quarter area looked the same,
  so the overlay is not what covers the quarter glass in the view used.
- So the first-hit map does place `BASE_A` group 0 (the rear-left window placeholder) along the side band, which is the
  quarter window's surface in the car's frame. Why its cracked texture does not show on screen is still open.

## Quarter window: isolated test (done)

- With every solid that could cover the quarter area hidden (body, interior, both damage overlays, both left window
  solids), only the base solid's glass shows. The front-door glass (`BASE_A` group 4) shows the shattered pattern there.
  The quarter glass (`BASE_A` group 0) shows as a plain grey slab.
- Swapping `WINDOW_LEFT_REAR` to `REAR_DEFROSTER` in the same isolated view changes the quarter slab's look. So the
  quarter group does take the placeholder swap: the routing works.
- So the open question is narrower: with the swap applied, the quarter group renders the damaged texture as plain grey,
  not cracked. The likely reason is its UV range (about U 0.09 to 0.93, V 0.26 to 0.75), which may sample a low-detail
  part of the damaged texture, or the damaged texture's alpha on that blend. Not yet tested: sample the damaged texture
  with the quarter UV range and see whether the crack pattern lies outside it.

## Quarter window: UV, alpha and material checks (done)

- The damaged texture's crack pattern is present in the quarter's UV rectangle (standard deviation 57, against 64 for
  the whole texture), and its alpha there matches the front window's (mean about 223, minimum 136). So neither the UV
  range nor the alpha explains the plain-grey quarter glass.
- The quarter group (`BASE_A` group 0) and the front-door group (`BASE_A` group 4) have the same light material
  (`471a1dca`) and the same opacity texture. Their texture slots differ: the quarter group uses its one diffuse hash
  for all four texture slots (diffuse, normal, height, specular), while the front-door group uses a separate texture
  per slot. So the quarter group's material differs in its normal and specular inputs, not in its light material.
- Open: whether the renderer samples those extra slots for the quarter group and washes the crack out. Testing it
  needs a render with the normal and specular slots of that group also swapped to the damaged texture.

## Quarter window: slot hypothesis refuted by code

- `blackbox-scene` `solid_mesh` (`libs/blackbox-scene/src/meshes.rs`) takes only `g.diffuse_texture(solid)`
  (`texture_slots[0]`) and looks up its material. The normal, height and specular slots are never read by the port.
- So the planned render with those slots swapped would show nothing, and the "unswapped normal and specular" explanation
  is refuted. The quarter group's diffuse path, texture region, alpha and light material all match the front window's,
  and that front window does crack. The reason the quarter glass renders grey is still unidentified.

## Correction: the renderer readback is not the blocking step

- The CPU ray pick (above) already identifies the surface under the quarter window: `BASE_A` group 0. A GPU per-draw
  readback would only confirm which draw covers those pixels, which the pick already established.
- The open question is the render path for that surface: why its damaged texture renders as plain grey while the
  front window's, with the same texture region, alpha and light material, cracks. That needs a trace of how the
  material is bound and shaded for that draw, not a readback. Earlier plans that named the readback as the next step
  are superseded by this.
