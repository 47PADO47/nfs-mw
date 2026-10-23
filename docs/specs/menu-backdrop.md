# Main menu backdrop (the car behind the menus)

Status: `frontend/scene.rs` (`MenuScene`) shows the career-start M3 GTR (`BMWM3GTRE46`, preset `M3GTRCAREERSTART`) standing still
in the crib room (`PlatformCrib.BIN`, drawn since section 1b), with the menu packages on top. The car's own placement and the camera of
the crib are still the car viewer's. The other looks (a profile's car, the safe house room with the original's car stand and camera) are
in [menu-backdrop-looks.md](menu-backdrop-looks.md).
Tags follow [frontend-menus.md](frontend-menus.md): **[decomp]** read from decompiled code, **[verified]** checked
against the install data, **[ours]** this rewrite's choice.

## 1. Where the original's stage comes from [decomp]

`FUN_00516e40` maps a stage number, stored at `+0x18` of the front-end state, to a file under `FRONTEND\PLATFORMS\`:

| stage | file |
|---|---|
| 1 | `PLATFORMCRIB.BIN` |
| 2 | `CAREER_SAFEHOUSE.BIN` |
| 3 | `CUSTOMIZATION_SHOP.BIN` |
| 4 | `CUSTOMIZATION_SHOP_BACKROOM.BIN` |
| 5 | `CAR_LOT.BIN` |
| other | empty |

- `FUN_00662b80` loads the stage's file as a scene (`FUN_0065fd30(path, 2, …)`), records the result in the
  front-end state, and hands it to the completion callback `FUN_006616f0`.
- `FUN_007b40e0` checks that the loaded file name matches the stage and then runs the same load again.
- The install docs ([formats/frontend.md](../formats/frontend.md)) name `PLATFORMCRIB.BIN` a texture pack, not a
  geometry pack; the decompile loads it through the same path as the others, so which of the two it is needs a
  look at the file (open question).
- The strings `FRONTEND\PLATFORMS\*.BIN` are at `0x0089B4D8`..`0x0089B580`.

Which stage the main menu uses, and whether the car is part of the stage or drawn separately, is not read yet.
Candidates: stage 1 (`PLATFORMCRIB.BIN`, the name suggests the menu's own room) and the car from the showroom
model in `FRONTEND/FrontB.lzc` (a `GeometryPack`, see formats/frontend.md).

## 1a. What `PlatformCrib.BIN` contains

The counts below describe the earlier inspected asset snapshot, not a universal contract. Other supported
installations contain 15 crib solids, including emitter and light helpers. The room loader follows the selected
stage file and `Stage::draws`; regression tests compare those drawable names and order rather than a fixed count.

Historical snapshot [verified, `python tools/chunkdump.py`]:

- Root: a `TexturePack` (19,924,984 bytes, its `TexturePackDataArray` is 19,923,064 bytes) and a `GeometryPack`
  (406,080 bytes, 12 children).
- The `GeometryPack` has `MeshContainerInfo` and five `SolidPack`s (5,128 to 181,244 bytes each). Each has a
  `SolidInfo`, `SolidTextures`, `SolidShaders` and `MeshInfoContainer` (vertices, faces, shading groups).
- No car is in the file. It is the room the menus stand in: the walls and floor and their textures. Its size
  (406 KB of geometry, 20 MB of textures) fits a room; a car model is far larger.
- So the car on the main menu is drawn separately from this stage (the player's car or a showroom car).
  Which car, and where it stands, is still open.

This corrects [formats/frontend.md](../formats/frontend.md), which lists `PlatformCrib` as a texture pack only.

## 1a'. The `FrontB.lzc` geometry pack is not the car [verified, `python tools/chunkdump.py -d 4`]

- `FRONTEND/FrontB.lzc` has one `GeometryPack` (72,368 bytes, 19 children) with a mesh container and 17 solids.
- Their names are world markers: `CHECKPOINT_GEOMETRY_*` (speed trap, circuit, rival, chop shop, safe house,
  sprint, drag, tollbooth, finish line, hide car), `CHECKPOINT_BEAM*`, `BMARKER_DIRECTION_AID`, `BMARKER_BOUNDARY`,
  and `DEBUG_CUBE`. No car.
- So the showroom candidate in section 3 is ruled out. The car on the main menu is still unidentified. The likely
  source is the player's current car, loaded the way the car viewer loads one (not confirmed).

## 1b. The room in the rewrite [ours, not yet built or seen]

- The solids of `PlatformCrib.BIN` parse with a new `SolidInfo` layout, version `0x18` (`libs/blackbox-solid/src/layout/v18.rs`,
  documented in [models.md](../formats/models.md)). The probe `platform_crib_solids_parse_with_the_0x18_layout`
  (ignored, needs the install) checks five solids with geometry.
- `frontend/room.rs` uploads the room's textures (`blackbox_tpk::read_texture_packs`) and solids (`upload_solid`), and
  `MenuScene` draws them with the car. Rendered offscreen (`nfsmw view-screen MainMenu.fng --screenshot out.png`): the room shows behind
  the car. [verified] (This install's `PlatformCrib.BIN` is a modified room, dated 2022, not the original's.) `room.rs` loads
  the stage of the look (`Stage::file`), see [menu-backdrop-looks.md](menu-backdrop-looks.md) section 3.

## 2. The stand-in [ours]

- `MenuScene::new(Some(dir))` loads `BMWM3GTRE46` with the `M3GTRCAREERSTART` preset at LOD A with `nfsmw_data::car::load` and shows it in the crib room (on the car
  viewer's floor only when the room cannot be loaded). If the car cannot be loaded, the menus fall back to the flat dark backdrop.
- The car stands still. The camera settles once, when the menus come up, from 30° round and 1.5× further out onto
  its rest pose over 0.5 s (`INTRO_SECONDS`, `INTRO_YAW_DEGREES`, `INTRO_DISTANCE_SCALE`). The 0.5 s is the original's
  camera move length (the float at `DAT_00905E28`, read with Ghidra); the start pose is ours.
- The original's camera is not a turntable. `FUN_007B9EA0` and `FUN_007A2310` run a timeline of camera moves (the
  `CubicCameraMover` path): each shot holds for 300 ticks (`DAT_00905E24`), and a move eases the camera from its
  old pose to the new one over the move length. The shots themselves are authored data, not read yet.
- The room's solids are drawn pre-lit (`Shading::Prelit`), as the world's scenery is; the stage's vertex colours
  carry the light.
- The radio plays on the menus (`Scene::radio_on_menus`), from the front-end list, when the `radio` setting is on.
- The picture was checked offscreen (`nfsmw view-screen MainMenu.fng --screenshot …`): the car sits behind the
  title, Career and the footer hints.

## 3. Still to do

1. ~~Render the crib room~~ Done (section 1b).
2. ~~Find which car the original shows, and where.~~ The car is the player's current car (section 7), stood by code in the
   executable, not by the stage file ([menu-backdrop-looks.md](menu-backdrop-looks.md) section 4).
3. The car's turn in the crib, the crib's camera (`angle3` is the lead) and the original's turn rate are not applied to the crib
   (`Look::Main` stays as it is on purpose). The safe house has its own car stand and camera.
4. The other platforms (customization shop, back room, car lot) are not drawn.

## 4. Lead: the showcase [strings, not yet decompiled]

- The binary has `CM_SHOWCASE`, `CDActionShowcase`, `ShowcaseCameraMover` and a `Showcase.fng` package, next to
  `eMM_PLAYERCAR` (the player's car event) and `CARLOT`. The showcase is the likely owner of the car the menus show,
  and its camera mover is likely the turntable.
- Not confirmed. Next: find the code that creates `CDActionShowcase` and the car it places, from its string
  references (`get_xrefs_to` on `0x00894C70` and `0x008949A0`).
- Checked: `CDActionShowcase` (`0x00894C70`) is referenced only from RTTI and unwind data (`Unwind@0086ACE0`,
  `0x0047396A`..`0x00473999`, `0x00884DC0`). `CM_SHOWCASE` and `ShowcaseCameraMover` have no code xrefs. So the
  creator is reached through the class factory or a vtable, not by a direct reference. Not yet followed.
- Decompiled (`FUN_007b7080`, the `Showcase.fng` screen logic): it handles the showcase's own messages (`0x35F8620B`
  init, `0x9120409E`/`0xB5971BF1` left and right, `0x406415E3` accept, `0xC519BFBF` open, `0xB5AF2461` open the
  shopping cart). It shows a car taken from the collection at `DAT_009B9EC0`, and on accept it opens
  `ShoppingCart.fng`. So the showcase browses the car lot's cars; it is not the main-menu scene itself.
- Open: whether the main menu's car comes from that same collection (the player's current car). Not confirmed.

## 5. The collection at `DAT_009B9EC0` [Ghidra xrefs]

- `DAT_009B9EC0` is referenced from about 200 code locations across the garage, showcase, car-lot and career
  functions (for example `FUN_007AED20`, `FUN_007B7080` (the showcase), `FUN_007B8F10`, `FUN_007BB560`, `FUN_007C3A50`).
  So it is the global player-car manager, not a menu-only collection.
- Helpers used with it: `FUN_007AEB90` (find an entry from a car id), `FUN_007AEB60` and `FUN_007B5870` (remove and
  set the current entry). The menu's car is probably the current entry, but the main-menu path into this manager
  is not traced yet.
- Not done: tracing the main-menu path (`MainMenu.fng` logic, `factory.rs`'s `IconMenu` for `Kind::Main`) into this manager.

## 6. The main-menu path [Ghidra, traced]

- `MainMenu.fng` is referenced by `FUN_005538E0`, which allocates a 0x164-byte screen object and calls
  `FUN_005496A0(this, …, "MainMenu.fng", "OL_MAIN.fng")`.
- `FUN_005496A0`'s callees are `FUN_00460CA0` (string copy), `FUN_00520260`, `FUN_00525940` (open a screen by
  name, the same call the rewrite's `Command::Switch` mirrors), `FUN_0056E320`, `FUN_0056E360` and `malloc`. It does
  not reference the car manager (`DAT_009B9EC0`).
- So the menu class does not place the car. The car must come from the stage or platform loader: the platform
  loader is `FUN_00662B80` / `FUN_007B40E0` (section 1), which calls `FUN_0065FD30` to load the stage file. The
  next step is to read where `FUN_0065FD30`'s scene places the car, and whether that code reads the car manager.
- The stage load chain, as far as traced: `FUN_007BF180` (caller) → `FUN_007BEED0` (references the `BACKDROP`
  string; builds the scene objects; calls `FUN_007B9EA0`) → `FUN_007B9EA0` (calls `FUN_007B40E0`, which loads the
  stage with `FUN_00662B80`; also calls `FUN_007A8C70`, `FUN_007A8FE0` and `FUN_007A96C0`). Whether any of these
  places the current car (the manager at `DAT_009B9EC0` is not read by them directly) is not yet read. Next: read
  `FUN_007A8C70`, `FUN_007A8FE0`, `FUN_007A96C0` and `FUN_007BF180`'s caller.
- `FUN_007A8C70` is string handling; `FUN_007A8FE0` is a small helper (string, memory and one call into
  `FUN_0056D0D0`). Neither places a car.
- **`FUN_007A96C0`: the likely car placement [decomp, not yet confirmed].** Its argument is the car display object.
  It checks the object's id at `+0x8` (absent or `-1` means nothing to show, and it sets `+0x574` to 0). Otherwise it
  reads four per-corner values with `FUN_00737AD0` (position) and `FUN_00737B60` (a size, defaulting to 1.5), sums them
  into a centre and a height, and writes the placement to `+0x320`..`+0x330`. It then builds the car's transform
  (three `FUN_006BD*`/`FUN_006BDB*`/`FUN_006BDC*`/`FUN_006BDD*` operations, each scaled by `FUN_007C4B80() / 0x168`)
  and stores the result. The `FUN_00516E10`/`FUN_00516E20` calls use the same platform singleton as the stage loader.
  So this is where the menu car gets its position on the platform, and the id at `+0x8` is the link to the car
  manager (`DAT_009B9EC0`). Still to read: `FUN_00737AD0`/`FUN_00737B60` (to confirm they read wheel or corner data),
  and where the id at `+0x8` is set from the manager's current entry.

## 7. The menu car, traced [decomp; the chain is read, the picture is not compared]

- **Source:** `FUN_00516AC0(DAT_0091CADC)` returns `*(frontend + 0xE0)`, the frontend's current car record (0 if the
  frontend has none). `FUN_007B9EA0` (the scene's per-frame update) calls it, and so does the quit handler
  `FUN_005A4820`. So the menu car is the frontend's current car: the player's selected car.
- **Name:** the record's name at `+0xC` goes through `FUN_007A8C70` into a hash. The scene keeps that hash at
  `+0x74`. When the hash changes, `FUN_007B9EA0` reloads the car's appearance (`FUN_004770B0` with values from the
  record at `+0x20`..`+0x54`) and resets an animation timer (`DAT_009B9E58`).
- **Placement:** `FUN_007A96C0(scene, car)` reads the car model's corner table at the model's `+0x318` (through
  `FUN_00737AD0` and `FUN_00737B60`). Each entry is 16 bytes: the corner position (x, y, z) and a fourth value that
  is the wheel size (`FUN_00737B60` returns it; 1.5 when absent). `FUN_00737AD0` adds a height from a per-wheel
  table keyed by hash `0x8D5BEB72`. So the placement is set from the car's own wheels.
- **Platform:** `FUN_007B9EA0` ends with `FUN_007B40E0()`, which loads the stage (section 1). The car stands on the
  stage's platform.
- **Rewrite:** `MenuScene` shows the career-start preset `M3GTRCAREERSTART` (car type `BMWM3GTRE46`), found in `PresetRides`; the preset must be loaded under its own car type, or its parts do not resolve. The original shows the frontend's current car. A rewrite
  equivalent needs that car selection to exist in the rewrite (none yet), and the wheel placement from the car
  model, which `nfsmw-data`'s `CarModel::corners` already provides.
- **Not checked:** the frontend's `+0xE0` is set on career and quick-race selection (not traced here). The picture
  is not compared with the original.

## 8. Setting the frontend's current car (`+0xE0`): dead ends

- The player-car events (`EBecomePlayerCar` `0x008A687C`, `ESetPlayerCarReset` `0x008A7438`) and the name
  `eMM_PLAYERCAR` (`0x00897A64`) have no code references. They are dispatched through a hashed table; the only
  reference to `eMM_PLAYERCAR`'s entry is at `0x008F03A8`, which nothing references.
- So the handler is reached by hash lookup at run time, not by a direct call. Next: find the code that calls the
  event dispatcher with the `BECOME_PLAYER_CAR` hash, or the career and quick-race screens that write `+0xE0`.

## 9. Hash search for the player-car event [Ghidra byte search]

- The rewrite's name hash for event names is AttribSys `lookup2` with initval `0xABCDEF00` (`libs/blackbox-attrib/src/hash.rs`).
  Ported to Python, `vlt("EBecomePlayerCar") = 0xBBE323C4`, `vlt("BECOME_PLAYER_CAR") = 0x32C8F83A`,
  `vlt("eMM_PLAYERCAR") = 0xA58E8495`.
- Byte search (`search_byte_patterns`, little-endian): `c423e3bb` (EBecomePlayerCar) occurs once, at `0x008A55CC`,
  in a data table, with no code references. `3af8c832` and `95848ea5` occur nowhere.
- So the event is a table entry, and the code reads it by index or by walking the table. The next step is the
  table's layout at `0x008A55CC` (what sits beside the hash) and the code that walks it.
- Correction: the bytes at `0x008A55B0` (96 bytes, read with `read_memory`) look random: no structure, pointers or
  names. A random 4-byte value would match about once in the 6.7 MB image by chance, so the hit at `0x008A55CC` is most
  likely a coincidence in a constant table, not the event. The byte search does not locate the event; drop the lead.

## 10. Render check (the build works now)

- The working tree builds (`cargo build -p nfsmw`, exit 0). The main menu renders with the room
  (`nfsmw view-screen MainMenu.fng --screenshot …`): the room's textures and solids show behind the car and the menu.
  The render is recorded, not compared with the original picture.
- Frontend tests: 86 pass, 3 fail. Two are the pre-existing video-option tests. The third,
  `controls_options_show_every_response_row_with_visible_values` (`invert_camera_y`), is in the input-settings code
  from another contributor's work, not in this change. Not investigated further.

## 11. Menu car: what the rewrite can show today [code check]

- The rewrite has no car-selection state (career, quick race or the car lot). Its only player car is
  `DEFAULT_CAR = "BMWM3GTR"` (`crates/nfsmw/src/scenes/world/mod.rs`), the car free roam starts with. The menu already shows
  that same car, so within the rewrite's current state the menu's car is its current car.
- Matching the original's menu car (the frontend's current car, `+0xE0`, section 7) needs a selection feature the rewrite
  does not have yet. Once that exists, `MenuScene::init` should load the selected car instead of `MENU_CAR`. That is a
  product decision (which screens set the selection), not a reversing step.
