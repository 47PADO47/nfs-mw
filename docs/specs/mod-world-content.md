# Mod world content: scenery packs and interiors

- **Status:** proposal (task B14, 2026-10-16). Not implemented. It makes [ADR 0005](../decisions/0005-native-mod-plugins.md)
  and [api-content.md](../modding/api-content.md) concrete for two things: map packs (new scenery placed in the city or
  outside it) and interiors (an entry zone that leads to a place the car or player can go).
- **Evidence tags** as in [docs/README.md](../README.md#evidence-tags). **[verified]** = matched against the install's
  data or the game. **[decomp]** = read from `speed.exe` in Ghidra, behaviour not yet tested in game. **[design]** = a
  choice of this rewrite with no original precedent. **[unconfirmed]** = open.
- **Addresses** are `speed.exe` virtual addresses (image base `0x00400000`). Decompiled code and Ghidra names stay out of
  this file. Method, addresses found and dead ends: [provenance/mod-world-content.md](../provenance/mod-world-content.md).
- **Scope:** v1 as ADR 0005 section 5 says. Mods replace section files and add scenery instances and interiors. A new
  road network or new districts are not in v1 (section 5 gives the reasons).

## 1. What the original does

The design rests on these facts. Each is the reason for a choice in sections 2 and 3.

### 1.1 Track load and the streaming index

- **Load order** **[decomp]** (`0x00666aa0`, the region loader; its debug name string is `RegionLoader::LoadHandler`):
  `Global\InGameA.bun` (shared sky and models), `Global\InGameSplitScreen.bun` only in two-player split screen,
  `GLOBAL\INGAMEB.LZC` (HUD packages), then `TRACKS\L2RA.BUN` (metadata, format `TRACKS\%s.BUN`). Then `0x00659530`
  builds `TRACKS\STREAML2RA.BUN` (format `TRACKS\STREAM%s.BUN`) and starts the streamer at `0x0072c070` with a split-screen
  flag. The handler then sets the game-flow step `BeginGameFlowLoadTrack`.
- **The index** is chunk `0x00034110`, read by `0x0072bd60` **[decomp]**. Its payload is an array of 92-byte records
  (`0x5C`); the count is payload size / 92 and the loader sets no cap. The field layout is in
  [formats/maps.md](../formats/maps.md) (name, `SectionNumber` at `0x08`, `FileOffset` at `0x14`, `Size` at `0x18`,
  centre, radius). Finding a section by its number is a linear scan that compares the `i16` number at `+0x08`
  **[decomp]**.
- **Read-only check on the install** **[verified]**: `TRACKS\L2RA.BUN` holds 720 records, all with unique numbers, from
  126 (`A26`) to 2600 (`Z0`). This was a read-only parse, not the Rust reader.
- The chunks `0x00034111` (infos) and `0x00034112` (barriers) are stored as pointers only. `0x00034113` (disc bundle) is
  walked by the same loader **[decomp]**; the format doc says it is mostly empty on PC.

### 1.2 Loading one section

- **Read** **[decomp]**: `0x00658dd0` reads `Size` bytes at `FileOffset` (sections start on 0x800 boundaries), runs a
  pointer fix-up pass (`0x006589a0`), then dispatches the section's top-level chunks (`0x00658a00`).
- **Dispatch** **[decomp]**: each top-level chunk goes to the loader registered for its id. The table is at `0x008f7ef8`,
  with a fallback table at `0x008f7e40`. The dispatcher works on windows of 64 chunks and runs each window from the last
  chunk to the first. A loader must not rely on another chunk of the same section being loaded already. The port handles
  this for textures and solids with the shared sets (`shared_pending` in `residency.rs`).
- **Completion** **[decomp]**: the record's `Status` (`+0x0C`) is set to 4 by `0x00727de0`.
- **Record to object** **[decomp]**: `0x0072c070` binds each record to a per-number pool (pointer at record `+0x58`). The
  key is the section number after a remap: when `n % 100` is in `[D, 2D)`, `D` is subtracted. `D` is stored at `0x00903164`;
  it is probably 40, the `LODOffset` of the visible-section manager **[unconfirmed]**. So a far tile (number `n + 40`)
  shares the object of its drivable tile `n`.
- **Triggers** **[decomp]**: chunk `0x80036000` (`EmTriggerPack`) is loaded by `0x0072b130`. Its children are `0x36001`
  (header; a word at payload `+0x08` must be 2; the 32-bit word at payload `+0x0C` keys the trigger pool at `0x009a3b50`,
  probably the section number), `0x36002` (tree) and `0x36003` (trigger records). The pack joins the ready queue only when
  its tree and trigger pointers are set and a third word is non-null (not identified). Unload is `0x0072b220`. What the trigger records mean is open (section 7).
- **Other chunk parsers** located, not decompiled: `ScenerySection` (`0x80034100`) in `0x007307e0`;
  `VisibleSectionManager` (`0x80034150`) in `0x00729a80` and `0x00731e70`.

### 1.3 Section numbers

- **Scheme** **[verified]**: a number is `letter × 100 + n`, with `A = 1` to `Z = 26`. `X0` is 2400. Numbers 1 to 39 are
  drivable close-up tiles, 41 to 79 far tiles, 80 to 89 non-drivable scenery, 90 to 99 panoramas (format doc).
- **Letters in the install** **[verified]**: `A B C D E F G H I J L M O P Q R S T V X Y Z`. Letters `K`, `N`, `U` and `W`
  have no section.
- **Free bands** **[verified for K, N, U, W; design for the rule]**: `K` 1100 to 1199, `N` 1400 to 1499, `U` 2100 to 2199,
  `W` 2300 to 2399, and 2700 and above (the largest install number is 2600). Inside a band, use `n % 100` in `0..39` or
  `80..99`, so the remap of 1.2 never folds a mod tile onto another number.

### 1.4 Visibility and LOD (existing specs)

- Each scenery instance has 64 bytes, a world-space bounding box, an exclude-flags word, a position, a 3 × 3 rotation
  (`i16 / 8192`, rows are the object's axes) and an info index. Details: [formats/maps.md](../formats/maps.md).
- **Visibility** [verified]: `visible = ((flags & 0xFF) ^ 0x60) & view == 0`. The player view uses `0x12`. So a flag word
  of `0x60` draws in every main view. This is the default for mod objects. See [scenery-visibility.md](scenery-visibility.md).
- **LOD** [verified in the spec]: the radius `R` is the info radius + 6 m. The projected size is `R·H / (d − R)`, and
  `size <= 17` draws nothing. `H` is the view's projection scale. See [scenery-lod.md](scenery-lod.md).

### 1.5 Collision and props

- **Static world collision** is one `CarpWCollisionPack` per tile, found through the grid in `0x0003B800` **[verified,
  docs]**. The grid is built from the 390 install packs, with a 64 m cell edge and 2,779 nodes. A pack that the grid does
  not list is not found by a grid query.
- **Props** have bounds keyed by a name hash in `0x8003B900`. The catalogue takes the first set for each hash and tries a
  few name endings **[decomp + code, `nfsmw-data/src/world/props.rs`]**. A mod gets prop collision by supplying bounds
  under its own prefixed names.

### 1.6 Entry zones and triggers in the data

- **Zones** **[verified, gameplay-races.md section 7]**: a `zonetrigger` collection with `Position`, `Radius` (6 or 7 m),
  `ZoneType` (`customshop`, `safehouse`, `carlot`) and a `respawn` child marker. Groups with a `ZoneList` hold them
  (`menu_gate_customshop`, `menu_gate_safehouse`, `menu_gate_carlot`). The reader is `nfsmw-data/src/gameplay/markers.rs`.
- **Races** **[verified]**: a `racestart` (start grid, class `startgrid`) gives the race entry. The `boxtrigger` class
  holds checkpoints and finish lines. They are not entries.
- **Grid triggers**: each grid node also lists trigger references **[code, `libs/blackbox-collision/src/grid.rs`]**.

### 1.7 Interiors in the original

- **No interior is streamed with a track.** The safe house, car lot and auto shop are front-end rooms. The five
  platform files are `FRONTEND\PLATFORMS\CAREER_SAFEHOUSE.BIN`, `CAR_LOT.BIN`, `CUSTOMIZATION_SHOP.BIN`,
  `CUSTOMIZATION_SHOP_BACKROOM.BIN` and `PLATFORMCRIB.BIN` **[verified, format doc; strings at `0x0089b4d8`–`0x0089b580`]**.
  A small function at `0x00516e40` returns the file name for a platform index **[decomp]**.
- **Entry** opens front-end screens: the safe house opens `MainMenu_Sub.fng`, the car lot `Car_Select.fng` and the shop
  `CustomizeMain.fng` **[exe, provenance of the zone handler at `0x0058c640`]**. Script commands named `JumpToSafeHouse`,
  `JumpToCarLot`, `DoSafeHouseIntro` and `AllowEngageSafehouse` are referenced by `0x0061e750` **[xref]**. The table's
  meaning is not read yet.
- **The name `%s_INTERIOR`** is a car part (cockpit model), not a building. The function at `0x00751860` builds all
  car part names (`_SKIN`, `_WHEEL`, `HEADLIGHT_`, `_DRIVER`, `_ENGINE`, `_LICENSE_PLATE` and others) **[decomp]**.
- **Tunnels** are track-zone type 3 (`TUNNEL`, with an elevation) in the track path zones **[verified, road-network.md,
  pursuit-data.md]**. They are polygons, not sections, and the audio and AI read them.

## 2. Mod scenery pack

### 2.1 Files a mod supplies

| File | Format | Read by (reused) | Notes |
|---|---|---|---|
| `files/SCENERY/<pack>.BUN` | bChunk section: `TexturePack` (`B3300000`), `GeometryPack` (`80134000`) with solids, optional `BoundsPack` (`8003B900`), optional `CarpWCollisionPack` (`0003B801`) | `nfsmw-data::world::parse_section(index, &[u8])` and the `blackbox-tpk`, `-solid`, `-collision`, `-scenery` readers | A pack is a section file. No new binary format. |
| `content/scenery/<pack>.toml` | TOML manifest: anchor, placements, LOD, collision, visibility | new reader in `blackbox-scenery` (a `manifest` module), no game names | Parsed at Load. Built into memory (2.4). |

The overlay (`GameDir`, [api-content.md](../modding/api-content.md#vfs-overlay)) gives the pack its path. A mod that
replaces `TRACKS/STREAML2RA.BUN` is out of scope; that is file-level replacement and is the install's data.

### 2.2 Manifest

```toml
[pack]
key = "monument"                 # registry key: example.pack:monument
file = "SCENERY/MONUMENT.BUN"    # in files/
section = 1100                   # number in a free band (1.3); must be unique
anchor = [1200.0, -900.0, 40.0, 180.0]   # render space: x, y, z of the centre, then radius (metres)
source_game = "nfsmw"

[[place]]
model = "MON_ARCH_A"             # a solid name in this pack's GeometryPack
pos = [1200.0, -900.0, 0.0]      # render space, metres (x forward, y left, z up)
yaw = 90.0                       # degrees about up; or rotation = [[...], [...], [...]]
scale = 1.0                      # row length; quantised, so at most about 4.0
coarse = "MON_ARCH_C"           # optional slot 2, the low-detail model of the player view
mirror = "MON_ARCH_B"            # optional slot 1, used by the mirror and reflections; default: model
draw_m = 900.0                   # cull distance (2.3)
lod_m = 300.0                    # distance where slot 2 starts (2.3)
collision = "prop"               # "none" | "prop" | "static"
exclude = 0x60                   # optional raw exclude flags; default 0x60 (all main views)
boost = true                     # sets 0x02000000: +10 px in the LOD choice
```

### 2.3 Field rules

- **Position and rotation.** Positions are render space, as the rest of the API (`api.md`, conventions). A `yaw` builds
  the rows of the rotation matrix in the row-vector convention of 1.4. `scale` multiplies the rows. The builder checks
  that every component fits `i16 / 8192`.
- **Model.** `model` names a solid. The info's slot 0 key is `bStringHash` of that name. The player view picks slot 0 or
  slot 2 (`scenery-lod.md`), so `coarse` fills slot 2. Slot 1 (mirror and reflections) takes `mirror`, or slot 0 when it is
  absent; slot 3 is left empty in v1. A model that is not in the pack is a load error, not a silent skip.
- **Radius and bounds.** The info radius is the solid's bounding radius. The instance box is the solid's local box
  transformed by the rotation, as the format doc describes. The anchor radius defaults to the union of the boxes.
- **Cull distance.** `draw_m` becomes a size rule. With `R = radius + 6` and the view's `H`, a distance `d` gives size
  `R·H / (d − R)`, and the object is cut at size 17. So `d_cull = R + R·H / 17`. `H` depends on the player's field of view.
  The manifest converts with the reference `H` of [scenery-lod.md](scenery-lod.md), 343 px (70° field of view, 480 lines)
  **[design; the reference is a choice]**.
- **LOD distance.** Only solids with more than 39 polygons use the density test. For them the coarse slot starts where
  `size < 8.7 · max(Density, 6)`, so `d_lod = R + R·H / (8.7 · max(Density, 6))`. `Density` is the slot-0 solid's
  density **[verified, scenery-lod.md]**. For smaller solids `lod_m` is ignored and the builder logs a warning.
- **Collision.** `none`: no collision. `prop`: the pack must hold a `BoundsSet` whose name hash is the info name (plus
  the catalogue's endings). `static`: the pack must hold its own `CarpWCollisionPack`. v1 does not generate static
  collision from solids; that is a tool step (section 5).
- **Visibility.** `exclude` is the raw word. `0x60` draws in all main views, `0x00` hides in the mirror and reflections,
  and bit `0x10` or `0x02` hides in the player view. The rules are in [scenery-visibility.md](scenery-visibility.md).

### 2.4 Build

- At Load the host reads the manifest and the pack file, then builds an in-memory `ScenerySection` (infos from the
  solids, instances from the placements) with `blackbox-scenery`'s new `build` module. Nothing is written to disk.
- Any error rejects the whole pack with a one-line reason, shown by the `mods` command (ADR 0005 section 3). A bad
  placement never drops the others silently.
- The build is deterministic: the same manifest gives the same bytes in memory and the same instance order.

### 2.5 Injection into streaming

Three routes. The manifest says which one (`[pack] route = "tile"` is the default).

- **Route A, mod tile (default).** The builder makes a `StreamingSection` record in memory: name from `key` (8 bytes),
  number from `section`, centre and radius from `anchor`, file = the pack. It is spatial (`is_spatial()` is true) but it is
  in no visible list. `Residency` loads it when the camera is inside the anchor plus a margin of 150 m **[design]** and
  unloads it outside that. Drawing and unloading use the existing `SectionResources` path.
- **Route B, append to an install tile.** After `parse_section` returns a tile, and before its textures and solids are
  uploaded, the builder appends instances to that tile's `ScenerySection`. Only models that resolve in that tile or in the
  shared sets can be used. Appended instances go at the end of the list. Never insert in the middle, because
  `SceneryOverrideHooks` refer to instances by index **[verified, maps.md]**.
- **Route C, mod shared set.** A pack with no anchor is a non-spatial section (radius 0). `Residency` requests all
  non-spatial sections at start, so the set is resident for the whole session. It holds models and textures that several
  mod tiles use. Its cost is permanent memory.

Route A is the only one that places objects where no install tile exists. Route B is for decoration on an existing street.

### 2.6 Ids and clashes

| Id | Rule | Checked at |
|---|---|---|
| Section number | Unique across the index and all loaded mods. Inside a free band (1.3). Not inside the remap ranges. | Load: `Exists` or `Arg`, rejected with reason |
| Record name (8 bytes) | Unique; the manifest key prefix (`modid_`) is added by the host if missing | Load |
| Solid and texture names | `bStringHash`. Mod names must start with the mod's prefix. A name equal to an install name is rejected. The install wins anyway (`upload_solid_list` keeps the first upload). | Load: scans the install's name hashes |
| Instance key | Tile number and list position (`keys.rs`). Route B appends, so install keys do not change. | By construction |
| Prop id | From the prop world's own counter only. A mod never picks one. | Runtime |
| Pack, zone and interior keys | `modid:name`. A collision is `Exists`. | Registry |

### 2.7 Collision integration

- **Props** go through `PropCatalog`. Its first-set-wins rule is safe, because mod names carry the mod prefix.
- **Static packs** outside the grid need `CollisionWorld` to test a second list of packs that the grid does not index.
  The change is one method, `insert_ungridded(pack)`, in `libs/blackbox-collision/src/world.rs` (generic, no game names).
  Every ray cast and ground query tests the install's grid candidates and then this list. `resident_near` ignores the list,
  so the grid's coverage check is unchanged **[design]**.
- **Cost**: the list is small (a few packs), so a linear test is enough for v1.

### 2.8 Visibility and sky

- Mod instances use the existing exclude rules and LOD choice (2.3). No new visibility code.
- The sky domes are scenery instances at the origin with flag `0x200`. They are 19.4 km across and do not follow the camera
  **[verified, world.md]**. A mod tile placed under a dome shows the sky above it. An interior needs its own ceiling.

### 2.9 Residency rules for mod tiles

- Mod tiles have their own load and draw budget **[design]**. They never displace install tiles from the visible lists.
- Uploads run on the render thread and are limited per frame, as install tiles are (`UPLOADS_PER_FRAME`, a port constant,
  not an original value).
- Unloading releases the meshes and textures of the tile (`SectionResources::release`). A tile that is still in a
  mod's interior is not unloaded (5.2 and 3.7).

## 3. Interiors v1

An interior is an entry zone, a prompt, and a place to go. The place is a mod area (a route A tile far from the city) or,
for the reuse variant, an existing front-end room (3.6).

### 3.1 Definition

```toml
[interior]
key = "hideout"
label = "Hideout"
prompt = "Press Enter to go in"          # the card's line (optional)
blip_style = "example.pack:hideout"      # from bb.map
zone = { shape = "sphere", pos = [1520.0, -840.0, 12.0], radius = 6.0 }   # render space
kind = "area"                            # "area" (v1, teleport) | "page" | "shop" | "safehouse" (reuse, 3.6)
pack = "hideout"                         # content/scenery/hideout.toml, the place
spawn = { pos = [3010.0, 4020.0, 0.0], heading = 90.0 }   # inside the area
exit = { shape = "sphere", pos = [3016.0, 4020.0, 0.0], radius = 3.0, prompt = "Press Enter to leave" }
time = { hour = 22.0, lights = true }    # optional, restored on exit
suspend = ["traffic", "cops", "pursuit"] # what stops inside
```

### 3.2 Zones

- **Sphere** is the existing shape (`EntryZones`, `entry_zones.rs`, the card and the drive-over test). The entry card and
  the Interact rule (`enter_wanted`) are reused unchanged.
- **Box** is new: `shape = "box"` with `half` and `yaw`. It needs a new `world.add_box_trigger` (api-world section
  "World state"). The box test is the rule the format doc gives for route boxes **[unconfirmed for the entry use]**.
- Exit zones use the same two shapes. The exit is tested only while the car is inside the area.

### 3.3 Flow

| Step | State | What happens | Event |
|---|---|---|---|
| 1 | Outside | Car in the entry zone, chase view, card up (existing rules). | `ZoneEntered` |
| 2 | Requested | Interact pressed. The host saves the return pose (car position, heading), the traffic setting, the time and the pursuit state. | `InteriorEntering` (cancelable) |
| 3 | Fading | Fade out 0.4 s **[design]**. The area pack is requested. | none |
| 4 | Waiting | Wait until the area pack is resident, at most 10 s **[design]**. A card shows "Loading". A timeout goes back to step 1 with a message. | `PackResident` (new) |
| 5 | Inside | Car teleported to `spawn` with zero velocity (`vehicles.teleport`). Traffic count 0, cops and pursuit ended. Time override applied. Fade in. | `InteriorEntered` |
| 6 | Exiting | Exit zone and Interact pressed. Fade out. | `InteriorExiting` (cancelable) |
| 7 | Outside | Car teleported to the return pose. Traffic, time and pursuit restored from the saved state. Fade in. | `InteriorExited` |

- Step 2 is cancelable: a mod may veto the entry (`InteriorEntering` returns "no").
- The car is put on the road only by the host's own teleport path: the area's floor comes from its static pack, not the road.

### 3.4 The placement area

- The area is a route A pack (2.5) placed where the install has no tile. Pick a point outside every visible-section
  boundary, outside the collision grid (its extent comes from `fMin` and the 64 m cell edge) and outside the ocean tiles.
  The host should offer a check command (`interior check <key>`) **[design]**.
- The area's collision is static (a `CarpWCollisionPack` in the pack, tested through the ungridded list of 2.7). A floor
  without static collision would let the car fall; the check command must refuse an area with no floor under `spawn`.
- The sky dome (2.8) is above the area. The area's own ceiling is part of its geometry.

### 3.5 Time, traffic and pursuit

- **Time.** The override sets the hour and the lights through `bb.time`, and freezes the clock while inside
  **[design; the API has the calls, not a scoped override]**. The original's safe house changes nothing at the sky, and
  the sky does not render a full night yet ([api.md](../modding/api.md#time-of-day)). The override is undone at step 7, even
  after a crash (3.7).
- **Traffic and cops.** Traffic count goes to 0 and cop share to 0 (`vehicles.set_traffic`), and the saved value returns
  at step 7. Existing cops are despawned on entry, as the safe house resets heat on entry (api-nfsmw, "Safe houses").
- **Pursuit.** Ended on entry (`nfsmw.pursuit`). A pursuit cannot start inside the area: the area has no cop routes
  **[design]**.

### 3.6 Reuse variant (no 3D pack)

- `kind = "page"` opens a mod menu page on entry (api-content). `kind = "shop"` or `"safehouse"` reuses the front-end room
  of that name (1.7). These are the original's own interiors and do not need a route A pack.
- v1 offers this for the owner's decision 7 in ADR 0005 ("reuse first"). The `area` kind is the teleport variant for
  places that have no room.

### 3.7 Failure and restore

- **Restore is the host's job.** The saved return pose, traffic, time and pursuit state are restored on every way out:
  normal exit, the pack unloading, a mod panic inside the interior (the SDK's trampolines catch it, ADR 0005 section 3),
  or the car being despawned. The host logs which path it took.
- **Quarantine.** A quarantined mod owning the interior forces the exit at once (step 7).
- **Online.** `interiors.enter` and the teleport are writes (**W**): denied in an online session unless every peer enables
  mods (ADR 0005 section 6). An online session never runs an interior it did not agree to.

## 4. API mapping

Names follow [api-content.md](../modding/api-content.md) and [api-world.md](../modding/api-world.md). **new** = not in the
catalogue yet and proposed here. Existing names are as written there.

| Spec step | Call | Status |
|---|---|---|
| Register a pack from its manifest | `content.scenery().register(PackDef { key, file, section, anchor, route, source_game })` | existing, with `anchor` and `route` added (**new** fields) |
| Place an instance | `content.scenery().add_instances(PackId, &[InstanceDef { model, pos, rotation, scale, coarse, mirror, draw, lod, collision, exclude, boost }])` | existing, fields **new** |
| Remove an install instance | `content.scenery().hide(model_or_id)` | existing |
| Is the pack resident | `content.scenery().is_resident(PackId) -> bool` | **new** |
| Pack resident event | `PackResident { pack }` | **new** event |
| Register an interior | `content.interiors().register(InteriorDef { key, label, prompt, pos, radius, shape, kind, pack, spawn, exit, time, suspend, blip, on_enter, on_exit })` | existing, with `kind = Area` (**new**), `shape`, `pack`, `spawn`, `exit`, `time`, `suspend` (**new**) |
| Enter or leave from code | `content.interiors().enter(InteriorId)`, `exit()` | existing (`W`) |
| Is inside, and which one | `content.interiors().current() -> Option<InteriorId>` | existing |
| Entry or exit trigger | `world.add_trigger(ZoneDef { centre, radius })` | existing (sphere) |
| Box trigger | `world.add_box_trigger(BoxDef { centre, half, yaw })` | **new** |
| Interior events | `InteriorEntering` (cancelable), `InteriorEntered`, `InteriorExiting` (cancelable), `InteriorExited` | `InteriorEntered` and `InteriorExited` existing; the two cancelable ones **new** |
| Teleport the car | `vehicles.teleport(VehicleId, pos, heading)` | existing (`W`) |
| Traffic and cops | `vehicles.set_traffic(count, cop_share)`; `vehicles.despawn` | existing (`W`) |
| End pursuit | `nfsmw.pursuit` end call | existing in `nfsmw-modding` (`W`) |
| Time override, scoped | `time.push_override(TimeOverride { hour, lights, freeze }) -> OverrideId`, `time.pop_override(OverrideId)` | **new**; `set_hour`, `freeze`, `set_lights` existing |
| Suspend AI, scoped | `world.suspend(Suspend { traffic, cops, pursuit }) -> SuspendId`, `world.resume(SuspendId)` | **new** (the host uses it for the interior and restores on every exit) |
| Area check | `interior check <key>` (console) | **new** |

Per-view and LOD fields of `InstanceDef` use the metre names of 2.2. The engine converts them with the rules of 2.3.

## 5. Not in v1, and why

- **A new road network.** The road network is one chunk (`0x0003B800`, `RNgp`) read as a unit with the grid. AI driving,
  traffic and cop routing all read it ([world.md](../formats/world.md)). A mod road would need its nodes and lanes merged
  into the install's graph, and nothing in the format says how. Wrong nodes break AI for the whole city.
- **New districts.** A district is a drivable zone: its boundary, its visible list and its loading group. The zone
  tables (chunk `0x00034150` and its children) come from one file, and the index plus the zones would both need merging
  (api-content "Open items"). Route A places a pack outside the city, which covers the need without touching the zones.
- **Static collision generated from solids.** The static pack holds triangle strips and barriers with surface data
  ([collision.md](../formats/collision.md)). A tool must produce them from a mesh. In v1 a mod supplies its own pack.
- **Custom interior scenes (`kind = "scene"`).** Reserved (ADR 0005, api-content). The area kind (3.1) covers places
  that need geometry, without a room scene.
- **Overlaying install `.BUN` chunks.** The overlay is file-level. A mod cannot merge into `TRACKS/STREAML2RA.BUN` or
  `L2RA.BUN` (api-content, "Chunk-level patches").
- **Editing the visible lists of install zones.** The original can edit them while an overlay is active (`VisibleSectionOverlays`,
  `0x00034158`), but the mechanism is not read yet (section 8). Route A does not need it.
- **Night sky and interior-specific sky.** The sky does not render a full night (api.md). Interiors use their own ceiling.

## 6. Test list

Unit (no install needed):

1. `blackbox-scenery` build: a placement with `yaw = 90` gives the rotation rows of the row-vector convention, and
   `matrix_columns()` places the origin at `pos`. A scale of 2 doubles the rows. A component beyond about 4.0 is rejected.
2. Build: the instance bounding box equals the solid's local box transformed by the rotation (within 0.14 m of padding, as
   the format doc says).
3. Visibility: flag `0x60` is visible in the player view and the mirror; `0x00` is hidden in the mirror; `0x10` is hidden
   in the player view.
4. Cull distance: the `draw_m` conversion gives size 17 at the computed distance, for `H` of 343 px and `R` of 12 m.
5. LOD: a solid with 39 polygons ignores `lod_m`; one with 40 polygons switches at `d_lod` from 2.3.
6. Section numbers: a number in `K`, `N`, `U` or `W` bands is accepted; an install number, a number in a remap range, or a
   number already used by another mod is rejected with the reason.
7. Name clash: a solid named like an install solid is rejected; a prefixed name is accepted.
8. Fuzz: a truncated or malformed pack file, and a manifest with bad field types, return an error and never panic.
9. Ungridded packs: `insert_ungridded` packs are tested by ray casts when the grid has no candidates; the grid's
   `resident_near` result does not change.

Integration (fake host, the `libs/blackbox-modding` test style of ADR 0005 section 9):

10. A content-only mod with a route A pack: the pack registers, its tile loads when the fake camera enters the anchor and
    unloads outside anchor plus margin. The meshes are released (count of destroyed meshes equals count created).
11. Route B: appended instances come after the install's; the override hook indices of the install tile still point at the
    same instances.
12. An interior: enter, wait for `PackResident`, teleport, traffic 0, time override applied; exit restores the saved pose,
    traffic, time and pursuit.
13. Restore paths: a panic inside a mod's `InteriorEntered` handler, a quarantine during the interior, and an unload of the
    pack all end in the exit path, with the log line.
14. A vetoed `InteriorEntering` leaves the car, traffic and time untouched.
15. Online session: `interiors.enter` and the teleport return `Denied` unless mods are enabled for it.

Game-side (install present, offscreen):

16. `--screenshot` run with an example map pack: the placed object appears from the player camera and not in the mirror
    when its flags are `0x00`.
17. `interior check` on a point inside the install footprint is refused; on a point outside every boundary and tile it
    passes.
18. Regression: the installed index, the visible-section tables and the install tiles read the same as before (the existing
    `installed_tests` in `residency`), with no mod loaded.

Manual, against the original (to record in the provenance file):

19. At a safe house, time the fade and note whether traffic and cops stop. The original's timing becomes the default for
    `fade` and the wait.

## 7. Open questions

- The value at `0x00903164` (the remap constant `D`). If it is not 40, the free-band advice of 1.3 changes.
- Whether the pool keys of 1.2 are section numbers. The loader stores the word at payload `+0x0C` as the key.
- What the trigger records (`0x36003`) mean, and whether they drive scripted events or the entry zones. The docs give
  `EventManager.cpp` as the decomp reference, not read here.
- Whether `VisibleSectionOverlays` (`0x00034158`) can add a mod tile to the visible list of an install zone while
  the overlay is active. That would allow route A tiles inside the city without a separate anchor rule.
- Whether `U` and `W` have sections in the other installs on this machine (the Desktop copy and the Redux install). The
  Black Edition install was read; the others were not.
- The reference field of view for `draw_m` and `lod_m`. The player's setting changes `H`, so the conversion holds
  only for one field of view.

## 8. Next leads

1. `0x00903164` and the remap function `0x00724360`: read the constant and the mapping, then close the remap question.
2. `0x0072bd60`, offset `+0x6C4` and the pending list at `+0x6C8`: find the writer. If something fills it at run time, the
   original has a second route for records, which would change section 2.5.
3. The chunk parsers `0x007307e0` (scenery), `0x00731e70` and `0x00729a80` (visible sections): read the record fields,
   check them against the format doc, and find the unresolved hit at `0x0073124b`.
4. `0x00658dd0`: the second read through `0x0064a9c0` (a status check or a size probe), and the buffer step through
   `0x00465a70`. This decides whether a section is copied before its pointers are fixed.
5. The trigger tree (`0x36002`) and records (`0x36003`): the reader around `0x00651030`, and the consumer of the pool at
   `0x009a3b50`.
6. `0x0061e750`: the script command table that holds `JumpToSafeHouse` and `JumpToCarLot`. Its entry format and whether the
   jumps load a platform file or switch the mode.
7. The `TUNNEL` string table at `0x009020f4`: it has no code cross-reference, so search for the table's base address
   and the reader of the track-zone type names.
8. The visible-section overlay reader (`0x00034158`) and the code that applies `AddRemove` entries while a flyby runs.
9. The streamer's budget. No cap was found in the index loader. Find the memory or count limit in the section queue
   before the port's `MAX_IN_FLIGHT` is judged against the original.
