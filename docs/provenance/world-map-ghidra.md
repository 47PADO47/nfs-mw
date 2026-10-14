# World map: how the executable drives it (Ghidra)

Source: `D:\Need For Speed Most Wanted Black Edition\speed.exe` (Ghidra program name `speed.exe`, image base
0x400000, 26079 functions). Read-only analysis. No names, comments or bytes were changed in the program.
Companion to `docs/provenance/world-map-screen.md` (object names, from the FEng dump) and
`docs/specs/world-map.md` (the port's current behaviour).

Tags: **[verified]** = read from the decompilation or disassembly, or matched by hash against the dump.
**[assumption]** = inferred from context, not proven. Function names are Ghidra's `FUN_<addr>` unless noted;
"label" means a role name I gave in this note (not renamed in Ghidra).

## Method and checks

- Name hashes: `fe_hash_upper` (the repo's `libs/blackbox-feng/src/hash.rs`, the `bstring_hash` of the
  upper-cased name) was re-implemented in Python. **[verified]** it reproduces the dumped GUIDs:
  `TRACK_MAP` = 0x0F365871, `IconSelection_Glow` = 0x5FBBA846, `MMICON_SPRINT_%d` 0..5 = 0x5D8CA830..35,
  `MMICON_CIRCUIT_%d` = 0x44075303.., `SAFEHOUSEINDICATOR_%d` = 0x3750C9EE.., `SHOP_%d` = 0x79E58668..,
  `CAR_LOT_%d` = 0x008BA2D2.., `SPEED_TRAP_CAM_%d` = 0xD4662C05.., `MINIMAP_ICON_EVENT_RIVAL` = 0x975D7590.
- Formatted names: `FUN_00573140(fmt, arg)` is a printf-style formatter followed by the hash (`FUN_004643D0`,
  `FUN_00460BF0`). **[verified]** the caller pushes the running item index (see Q2).
- Pool name strings in the exe: `MMICON_SPRINT_%d`, `MMICON_CIRCUIT_%d`, `MMICON_DRAG_%d`,
  `MMICON_KNOCKOUT_%d`, `MMICON_TOLLBOOTH_%d`, `MMICON_SPEEDTRAP_%d`, `MINIMAP_ICON_EVENT_RIVAL`,
  `SAFEHOUSEINDICATOR_%d`, `CAR_LOT_%d`, `SHOP_%d`, `HIDING_SPOT_%d`, `PURSUIT_BREAKER_%d`,
  `SPEED_TRAP_CAM_%d`, `SPEED_TRAP_RACE_CAM_%d`, `MMICON_CHECKPOINT`, `MMICON_ROADBLOCK_%d`. **[verified]**
- Not in the exe as plain text (so they live in the language or FEng tables, not in code): "LOCATIONS",
  "RACES", "Activate GPS?", "ZOOM x 1.5". The code only uses their hashed keys. **[verified]**
- Helper scripts were run through `run_script_inline`. They were written to `C:\Users\PADO\ghidra_scripts\`
  (outside the repo, not committed). The shared folder prints unrelated compile errors on each run.

## Function map

| Address | Role | Status |
|---|---|---|
| 0x0057C660 | HUD per-frame command dispatcher. Command 0x2F opens the world map (creates the state and calls 0x00625AD0). | **[verified]** call; M = 0x2F **[assumption]** |
| 0x00625AD0 | World-map screen state ctor (vtable 0x008A7730). Loads `WorldMapMain.fng` through 0x00525940. | **[verified]** |
| 0x00625A50 | State close. Unloads the screen through 0x005258B0. | **[verified]** |
| 0x00525940 / 0x005258B0 | FEng screen push / unload by file name (not traced inside). | **[verified]** targets |
| 0x00570D80 | Caller of the controller ctor 0x005614D0 (not traced). | **[verified]** caller |
| 0x005614D0 | Map controller ctor. Sets fields, attach radius `this+0x174 = 20.0`, calls init and refresh. | **[verified]** |
| 0x0055B8C0 | Controller init: binds WorldMapMain objects by hash, sets picture helper, reads saved mode and zoom, fills initial blips. | **[verified]** |
| 0x0055B0C0 | Controller message handler (size 2044). Dispatches on message hash. Called through the screen. | **[verified]** |
| 0x00552CB0 | Zoom index setter (0..3). Sets the picture UV scale and recentres. | **[verified]** |
| 0x00552DA0 | Per-frame hover: map hit rect, nearest-blip pick, cursor follows mouse, sends click 0x0C407210. | **[verified]** |
| 0x00552F40 | Per-frame drag: pan, drag messages, detach at 20 px, recentre easing. | **[verified]** |
| 0x00530A60 | Nearest-blip pick (cursor vs blip list). Returns true when the attachment changed. | **[verified]** |
| 0x00530660 | Pan: moves cursor and clamps the picture UV. | **[verified]** |
| 0x00530C10 | Zoom centre: new UV centre and clamp for a zoom factor. | **[verified]** |
| 0x00524EE0 | FEng object query: centre position of an object, with scale. | **[verified]** |
| 0x00525050 | FEng object move (set position). | **[verified]** |
| 0x00511D90 | World to picture pixel transform (city calibration at `controller+0x13C`). | **[verified]** |
| 0x005486A0 | Per-type item loop: looks up the pool name for a type, creates one blip per item. | **[verified]** |
| 0x005315A0 | Per-item: finds the named FEng object, builds the blip node (0x38 bytes, `FUN_0052F500`). | **[verified]** |
| 0x0052F500 | Blip node ctor: map position, item pointer, visibility (profile mask). | **[verified]** |
| 0x00530DF0 | Style node (0x4C bytes) created after each type's loop. | **[verified]** |
| 0x005487A0 | Races set: types 1..7, 13, 14. | **[verified]** |
| 0x00548770 | Locations set: types 10, 8, 9. | **[verified]** |
| 0x00548810 | Pursuit set (mode 3): types 8, 12, 11, then 0x00531060 and 0x00531350. | **[verified]** |
| 0x005488B0 | Refresh: filter label text, zoom hint, card (or blank), GPS button text. | **[verified]** |
| 0x005316A0 / 0x00531760 | Card fill: Item Title, Race Length. | **[verified]** |
| 0x00598C00 | Opens `InGameDialog.fng` with title, body and Yes/No messages. | **[verified]** |
| 0x0051FA10 / 0x0051FA30 | GPS target set / clear, global `DAT_0091CA4C`. | **[verified]** (names are labels) |
| 0x0051FA50 | Per-item enable or disable (show/hide of a blip). | **[verified]** |
| 0x00515A40 / 0x005158B0 | Hit test of a point against a rect (the map area). | **[verified]** |
| 0x0059D500 | HUD minimap ctor (not the world map). Same pool names, 8 slots per `%d` style, `TRACK_MAP1..4`. | **[verified]**, related only |

Note: vtable 0x008A7730 has real entries only in slots 0 and 1 (0x00632340, 0x00625B60). The rest of the
table overlaps the strings "EWorldMapOn" and "EWorldMapOff". Those two functions were not decompiled.

## Q1. Which functions open, load and drive WorldMapMain.fng

- The string `WorldMapMain.fng` (0x0089FC00) is referenced by FUN_00625A50 and FUN_00625AD0 (code), and by one
  data entry at 0x008F4390 (a table, not traced). **[verified]**
- FUN_00625AD0 writes the state vtable, calls `FUN_00542670(0, 1, 0)` (a "can open" check), then
  `FUN_00525940(screen, "WorldMapMain.fng", 0, FUN_00573020(save+0x0C), 0)`. **[verified]**
- FUN_00625AD0 has three callers: FUN_0057C660 (command 0x2F), and two others (FUN_00632370, FUN_006323C0),
  which are pool or push helpers. **[verified]**
- The controller is created by FUN_005614D0, called by FUN_00570D80. The link from the state to that call was not
  traced. **[assumption]** that FUN_00570D80 is the screen's object factory.
- The message handler is FUN_0055B0C0. Its only callers are indirect (through the screen), so the handler is
  registered by the FEng screen code. **[verified]** no direct caller.
- The screen objects the controller binds (by hash, in FUN_0055B8C0): cursor `0xF156F6C5` (controller
  `+0x124`), `TRACK_MAP` MultiImage `0x0F365871` (`+0x140`), and the screen name at `+0x10`. **[verified]**
  The name of 0xF156F6C5 was not resolved against the dump (the dump names the cursor group `0xF93EF`).
- The `WORLD MAP` title is not a controller concern. **[assumption]** it is static in the package.

## Q2. Blip placement, pool choice and pool counts

### Type ids and pools (verified from the record table at 0x008F3AC0, 20 bytes per record)

Each record holds the name pointer, two hashes, a type id and a visibility mask. The type id of a record is the
index the item carries (`*item`). The name is used with `%d` for the pool slot.

| Type | Name (exe) | Pool base hash | Race or Locations use | Objects in dump (provenance note) |
|---|---|---|---|---|
| 1 | MMICON_SPRINT_%d | 0x5D8CA830 | Races: Sprint | 6 (x 157) |
| 2 | MMICON_CIRCUIT_%d | 0x44075303 | Races: Circuit | 6 (x 97) |
| 3 | MMICON_DRAG_%d | 0x979E2E2E | Races: Drag | 3 (x 217) |
| 4 | MMICON_KNOCKOUT_%d | 0x5008535E | Races: Knockout | 6 (x 127) |
| 5 | MMICON_TOLLBOOTH_%d | 0x4101C707 | Races: Tollbooth | 3 (x 187) |
| 6 | MMICON_SPEEDTRAP_%d | 0xC045FEB8 | Races: Speedtrap | 6 (x 65) |
| 7 | MINIMAP_ICON_EVENT_RIVAL | 0x975D7590 | Races (Rival) | 1 (x 277) |
| 8 | SAFEHOUSEINDICATOR_%d | 0x3750C9EE | Locations: Safe house; pursuit | 4 |
| 9 | CAR_LOT_%d | 0x008BA2D2 | Locations: Car lot | 4 |
| 10 | SHOP_%d | 0x79E58668 | Locations: Auto shop | 8 |
| 11 | HIDING_SPOT_%d | 0xFCFD3786 | Pursuit (mode 3) | not in note |
| 12 | PURSUIT_BREAKER_%d | 0x2A567665 | Pursuit (mode 3) | not in note |
| 13 | SPEED_TRAP_CAM_%d | 0xD4662C05 | Races (speed trap cam) | 6 (x 247) |
| 14 | SPEED_TRAP_RACE_CAM_%d | 0xD5F1539F | Races (speed trap race cam) | not in note |
| 15 | MMICON_CHECKPOINT | 0x3A4570E9 | (not in either set) | not in note |
| - | MMICON_ROADBLOCK_%d | 0xCBD81AE1 | Pursuit (0x00531350) | not in note |

Event_Cell (0x69585E84, x 307 in the dump) does not match any name above. Its type is **unknown**.

The six race styles of the spec map to types 1..6. **[verified]**: Sprint 1, Circuit 2, Drag 3, Knockout 4,
Tollbooth 5, Speedtrap 6. The Races set also has Rival (7) and the speed trap cams (13, 14). The Locations
set is types 10 (auto shop), 8 (safe house) and 9 (car lot). **[verified]**.

### How an item gets its object

- FUN_005486A0 loops over the world's item list (`FUN_005E4940`). For each item whose type `T`
  is in the set and whose record gate is non-zero, it calls `FUN_005315A0(this, gate, hash(fmt, j), item)`.
  `j` is the running count of matched items of that type (starts at 0 each call). **[verified]** from the
  disassembly (`PUSH EBX` = `j`, then the format pointer, then `CALL 0x00573140`).
- The gate for type `T` is the mask word of the previous record (the 5th word of record `T-1`). Types with a zero
  gate are skipped. **[verified]** from the disassembly (`MOV EDX,[ESI*4+0x8F3ABC]`).
- `FUN_005315A0` looks up the FEng child whose name is `hash(fmt, j)`. If it is not found, the item is **dropped
  silently**. There is no cap in the code: the pool size is the number of named objects in the package.
  **[verified]**.
- The per-type style node (`FUN_00530DF0`) is created once, after the loop, only if at least one item was placed.
  **[verified]**.
- Per-type visibility: the mask is checked against the profile block at `DAT_0091CF90+0x10+0x38` through
  `FUN_0056D540`. A hidden node has `+0x34 = 1` (not pickable, object hidden). This is the Hide/Show Icon feature.
  **[verified]** for the check; **[assumption]** for the meaning of the profile block.

### Picture transform

1. World to picture pixels (`FUN_00511D90`): with calibration `C = controller+0x13C` (from
   `FUN_00722840(DAT_00904FB8)`), `u = (wx - C[+0xAC]) / C[+0xB4]`, `v = (C[+0xB0] - wz) / C[+0xB4] + 1.0`, and
   `px = X0 + W*u`, `py = Y0 + H*v`. `X0, Y0` = picture origin (`+0x144,+0x148`); `W, H` = size
   (`+0x14C,+0x150`). **[verified]** formula. The calibration constants are **data** (not in code); their values
   were not read. The repo's `open_city_calibration` should be compared against that object's fields.
2. Picture to screen each frame (`FUN_0055B0C0`, refresh branch): with zoom `Z`, crop origin `(U, V)` (from
   `FUN_00510CE0`), blip map point `(xm, ym)`:
   `screen_x = X0 + Z*(xm - X0) - Z*W*U`, `screen_y = Y0 + Z*(ym - Y0) - Z*H*V`. **[verified]** formula.
   **[assumption]** that `(U, V)` is the crop origin, not the centre (consistent with Z = 1 giving U = V = 0).
3. Origin: `X0, Y0` come from `FUN_00514FD0` on the picture object. **[assumption]** that this is the top-left.
   The provenance note assumed the centre; this is the open question for the port.

### Pool counts used

- The world-map controller has no fixed pool size. The pool is whatever the screen package holds (table above).
  Items past the last object are not drawn and not pickable. **[verified]** (drop path). The counts above are
  from the dump. **[verified]** by hash.
- The HUD minimap (`FUN_0059D500`) loops `0..7` (8 slots per `%d` style). That is the HUD, not the map.
  **[verified]**.

## Q3. Zoom and pan

- Zoom levels: index 0..3, multiplier `Z = 1.0, 2.0, 3.3, 5.0`. Step is one index, clamped to 0..3.
  **[verified]** (`FUN_00552CB0`, and the same table in `FUN_0055B8C0`). **This contradicts the port**, which
  uses x1.25 steps up to x8 (`camera.rs`).
- Input (all in `FUN_0055B0C0`, only when the option list is closed, `+0x170 == 0`):
  - `0x9803F6E2` (mouse wheel): the sign of `DAT_00987F6C` picks the step. Negative gives +1 (zoom in),
    positive gives -1 (zoom out). **[verified]** code; the wheel sign convention **[assumption]**.
  - `0xD9FEEC59`: +1 (zoom in). `0x5073EF13`: -1 (zoom out). **[verified]**.
  - No dedicated Zoom-All message was found. Index 0 (x1.0) is reached by zoom-out steps. The key that the spec
    calls "Z" was not identified. **[assumption]** / open.
- Zoom index stored in the profile block: `+0x44` (Locations and Races), `+0x45` (pursuit). Filter mode at
  `+0x46`. **[verified]**.
- UV crop: the picture helper (`controller+0x160`) holds the UV scale at `+0x1C` and the UV centre at `+0x74`.
  Scale = `1/Z` in both axes (`FUN_00552CB0`, `FUN_00651550`). **[verified]**.
- Pan clamp (`FUN_00530660`, `FUN_00530C10`): half crop `h = 0.5 / Z`; the centre is clamped to `[h, 1 - h]`
  in UV. Cursor clamp box: x from `X0+8` to `X0+W-8`, y from `Y0+26` to `Y0+H-32`. **[verified]**.
- Pointer hit rect (`FUN_00552DA0`): x `X0+8`, y `Y0+26`, width `W-16`, height `H-58`. **[verified]**.
- Pan input: drag deltas in `controller+0x128` (x) and `+0x12C` (y), set elsewhere (source not traced).
  **[assumption]** the mouse drag and the right stick feed these.
- Drag messages: `0x9F710838` when a drag starts, `0x7E6687DA` when it ends (sent by `FUN_00552F40` on flag
  change). **[verified]**.
- Zoom recentre: after a zoom, `FUN_00552F40` (flag path) moves the cursor toward the target
  `controller+0x134/+0x138` by `1/Z` per frame, the target set by `FUN_00530C10`. **[verified]** code;
  **[assumption]** this is the ease-out after zoom.
- `ZOOM x 1.5` (`0xCB76CE5B`): **not** a static label. `FUN_005488B0` sets it from the zoom index to one of four
  text keys: `0x213587BF` (index 0), `0x0A9BE7D7` (1), `0x0A9BE7D8` (2), `0x0A9BE7DA` (3). **[verified]** keys.
  The text is in the language table (not in the exe). **[assumption]** it reads "ZOOM x1", "x2", "x3.3", "x5".

## Q4. Filter tabs and list

- Mode byte: 0 = Locations, 1 = Races, 3 = pursuit (no tabs). Mode 2 is only a text key. **[verified]**.
- Tab switch: message `0xB5971BF1`, only when the option list is closed and mode != 3. It flips 0 <-> 1: from 0
  it calls `FUN_005487A0` (Races), from 1 it calls `FUN_00548770` (Locations). **[verified]**.
- Label text: `LOCATIONS` object `0xD259525F` is set (`FUN_005488B0`) to a text key by mode:
  `0xBF55E8B2` (0), `0xDFD23484` (1), `0xF74B357D` (2), `0xFEA872D4` (3). **[verified]** keys. The strings are
  in the language table.
- Pool visibility per mode: Locations shows only types 10, 8, 9. Races shows types 1..7, 13, 14. Pursuit shows
  8, 12, 11 and the roadblock and cop functions. Switching calls the set builder, which creates new blips
  (the old ones are dropped by `FUN_005305B0` / `FUN_00530F40`, not traced). **[verified]** sets;
  **[assumption]** the clear.
- Mode at open: from the profile `+0x46` (remembered between sessions). Pursuit (3) is forced when the game is
  in a pursuit (`FUN_005D59F0` check at init). **[verified]** check, **[assumption]** meaning.
- Option list (the 6 `option_name` rows, `0xB2980BA8..AE`): the controller holds a separate object at `+0x34`
  with show (`vtbl+0x24`), hide (`vtbl+0x28`) and refresh (`INIT_COMPLETE 0x35F8620B`). Toggle message
  `0xC519BFC3` (sends `0x5C28136D` on open, `0x947E6205` on close). **[verified]** calls. The row count,
  how the list is filled, and whether it scrolls: **not traced** (open gap). The rows' code is near 0x00533A15
  (unnamed by Ghidra).
- The card rows (Item Title, Race Length) are at the bottom of the screen, not in the list.

## Q5. Cursor and attachment

- Cursor object: `controller+0x124` (hash `0xF156F6C5`). **[verified]**.
- Pick: `FUN_00530A60` scans the blip list (`controller+0x154`) and takes the nearest node with `+0x34 == 0` whose
  distance to the mouse (canvas coords, mouse minus (320, 240)) is below `controller+0x174` (= **20.0**).
  **[verified]**. Radius units = canvas pixels **[assumption]**; the port uses 0.06 of the
  picture width (about 20 px at 340), a close approximation.
- Attach: the node is stored in `controller+0x15C`; the cursor is moved to its point (`FUN_00525050`).
  **[verified]**.
- Detach: during a drag with an attachment, if the cursor moves 20 px or more from the attached blip, the
  attachment is cleared and the ring is removed (`FUN_00552F40`). **[verified]**. The port keeps the blip when
  the cursor moves away (`pick.rs`), which differs from the exe in the drag case.
- No attachment: the cursor follows the mouse while it is in the hit rect (`FUN_00552DA0` calls
  `FUN_00525050(cursor, mouse)`). **[verified]**.
- Card fill (`FUN_005316A0`, `FUN_00531760`, from `FUN_005488B0`):
  - Item Title `0x9331FD4F` gets the record's first hash for the item's mask (a text key). **[verified]**.
  - Race Length `0xFEEEB39B` uses the format `"%$.1f %s"`; the unit key is `0x8569A26A` or `0x867DCFD9` by
    the save byte `+0x3B` (metric or imperial). The record's second hash (name + 8) is used as a text key on
    an object at `0x9A5AB124`, and hidden when it is zero. **[verified]** format and keys.
  - With no attachment both fields are set to the empty string `pszFaceName_00890978`. **[verified]**.

## Q6. GPS prompt and confirm

- Activate is `0xC407210`. `FUN_00552DA0` sends it when `FUN_00515A40` is true: the mouse is inside the map
  rect and `DAT_00987F72` is set. **[verified]** the send and the test. **[assumption]** that `DAT_00987F72` is a
  button state (a click), since a per-frame send would otherwise fire constantly. Its source was not traced.
  Enter is assumed to map to the same message. **[assumption]**.
- With the option list closed, `0xC407210` opens `InGameDialog.fng` (`FUN_00598C00`):
  - Blip attached and it has an item: body key `0x96AC0A32`, title `0x70E01038`, button text `0x417B25E4`.
    This is the "Activate GPS?" prompt. **[verified]** keys.
  - No blip attached but a GPS target is already set (`DAT_0091CA4C != 0`): body `0xA6BE2EBB`, `0x1A294DAD`,
    `0x417B2601` (the clear variant). Otherwise nothing opens. **[verified]**.
  - Yes = `0xA16CA7BD`; No = `0xB4EDEB6D`. The controller does not handle No (the dialog closes itself).
    **[assumption]**.
- Yes handler (`0xA16CA7BD`, `FUN_0055B0C0`):
  1. If `FUN_0041AD00()` is true, clear the GPS target (`FUN_0051FA30`).
  2. If a blip with an item is attached and `FUN_0042C830(pos, 0)` is true: set the GPS target
     (`FUN_0051FA10(item)`, sets `DAT_0091CA4C` and the item's flag `|= 0x80`), then post `0x911AB364` (close).
  3. If `FUN_0042C830` is false: `FUN_00598A60` (a message box). **[verified]** calls.
  - `FUN_0042C830` is a position check (`FUN_0042C6C0`); its exact meaning was not traced. **[assumption]**:
    a reachability or range test.
- Close (`0x911AB364`): with the option list closed it sends `0x587C018B` (LEAVE_SCREEN). **[verified]**.
- What the port should stub: the confirm call is `FUN_0051FA10(item)` (set `DAT_0091CA4C`), then the close.
  The minimap route reads `DAT_0091CA4C` (not traced). **[assumption]**.
- Enter/Activate hint text: `Activate` key `0x9ACD367A` changes between `0x43512519` (no target) and
  `0xF1D0D8A5` (target set) in `FUN_005488B0`. **[verified]** keys.

## Q7. Messages used by the screen

Handled by `FUN_0055B0C0` (the controller). Hashes are the ones in `frontend/ids.rs` where they match.

| Hash | Name (as the repo writes it) | What the controller does |
|---|---|---|
| 0x9803F6E2 | mouse wheel | zoom step from the sign of `DAT_00987F6C` |
| 0xD9FEEC59 | zoom in | zoom index +1 (list closed only) |
| 0x5073EF13 | zoom out | zoom index -1 (list closed only) |
| 0xB5971BF1 | tab | toggle Locations <-> Races (list closed, mode != 3) |
| 0xC407210 | activate (Enter, click on map) | list open: forward to the list; else GPS prompt |
| 0xC519BFC3 | option list toggle | sends 0x5C28136D (open) or 0x947E6205 (close) |
| 0xA16CA7BD | dialog Yes | clears GPS, sets GPS if attached, closes (see Q6) |
| 0xB4EDEB6D | dialog No | not handled here (dialog closes) **[assumption]** |
| 0xB5AF2461 | back | close list, or leave (0x587C018B) |
| 0x911AB364 | close | leave (0x587C018B) when the list is closed |
| 0x72619778, 0x911C0A4B | refresh | redraw labels, card, hints only |
| 0x35F8620B | INIT_COMPLETE | list object refresh (`vtbl+0x28`) |
| 0x9F710838 / 0x7E6687DA | drag begin / end | sent by `FUN_00552F40` (not received) |
| 0xE1FDE1D1 | (unknown) | creates a small object (`FUN_00625A40`), purpose unknown |
| 0x9120409E | (generic) | not handled by the controller; passed to the base |
| 0x7EABCA56 | MOUSE_LEFT_RELEASED | not used by the controller; the click uses the hit test instead |
| 0x97BDB465, 0x696851AD, 0xC5DB4288, 0x3806C4C1 | (unknown) | not found as immediates in the exe |
| 0xB30793C1 | (generic) | only in FUN_005B0940 (a base FEng object) |
| 0x587C018B | LEAVE_SCREEN | sent to the screen; not received by the controller |

The `0xC407210` value in `FUN_00552DA0` is a send, not a receive. **[verified]**.

## Verified vs uncertain

**Verified (from code or hash match):**
- Map opened by HUD command 0x2F; state loads `WorldMapMain.fng`; controller handler `FUN_0055B0C0`.
- Pool names, type ids 1..15, the Races and Locations and pursuit sets, the per-item naming `hash(fmt, j)`,
  and the drop of items beyond the screen's objects.
- Zoom table 1.0 / 2.0 / 3.3 / 5.0, step and clamp; UV scale `1/Z`; pan clamp margins.
- Blip screen formula (Q2, picture transform).
- Attach radius 20 px; detach at 20 px in a drag; nearest-node pick.
- Tab toggle, mode persistence (profile `+0x44..+0x46`), label text keys.
- GPS prompt (InGameDialog), Yes, GPS set call `FUN_0051FA10`, close, Activate hint keys.

**Uncertain:**
- Calibration constants (values not read); whether `U, V` is the crop origin; origin `X0, Y0` is top-left.
- The wheel sign convention; the source of `DAT_00987F72`; the source of `0x128/0x12C` drag deltas.
- The Zoom-All key and its code path.
- What `FUN_0042C830` checks; what `FUN_0041AD00` checks.
- The option list object (`controller+0x34`) and its name; the meaning of `+0x170` (list open) and `+0x171`.
- The pursuit detection (`FUN_005D59F0`) and what sets mode 3.
- The Hide/Show mask (profile `+0x38`) and what it stores.

## Open gaps

1. List rows: count, fill, scroll, and whether they are moved on screen. Entry points: `controller+0x34`,
   the code near 0x00533A15, and the `option_name` hashes `0xB2980BA8..AE`.
2. Event_Cell (0x69585E84) and the roadblock pool type (0xCBD81AE1) are not mapped to a type id.
3. The link from the state (`FUN_00625AD0`) to the controller ctor (`FUN_00570D80` -> `FUN_005614D0`).
4. `FUN_00525940` (FEng push) internals, for how the screen's own messages are fed to the handler.
5. Where the drag deltas and `DAT_00987F72` / `DAT_00987F6C` are written (input layer).
6. The calibration object from `FUN_00722840(DAT_00904FB8)`: fields `+0xAC`, `+0xB0`, `+0xB4` and their values
   for each track, to compare with `nfsmw_data::minimap::open_city_calibration`.
7. The route itself: what reads `DAT_0091CA4C` (the GPS target) and how it draws the route.
8. The messages 0x97BDB465, 0x696851AD, 0xC5DB4288, 0x3806C4C1 (not in the code; probably in the FEng layer).

## Differences from the port (for the next step, not changed here)

- Zoom: exe is 4 levels (1, 2, 3.3, 5), port is x1.25 steps up to x8
  (`C:\Users\PADO\github\nfs-mw\crates\nfsmw\src\frontend\world_map\camera.rs`).
- Pursuit (mode 3) and its types (8, 11, 12 plus roadblocks) have no filter in the port's `filter.rs`.
- The port's Races filter is the six race styles. The exe's Races set also includes Rival (7) and the speed trap
  cams (13, 14).
- The GPS prompt is a separate `InGameDialog.fng` popup; the port's prompt is in egui.
- Detach at 20 px in a drag (exe); the port keeps the attachment.
