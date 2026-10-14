# World entry zones: the enter icon search

- **Spec:** [docs/specs/world-markers.md](../specs/world-markers.md) (the ground disc, the drive-over test, the card).
- **Modules:** `crates/nfsmw/src/scenes/world/entry.rs`, `entry_zones.rs`, `effects/entry_marks.rs`, `hud/interact_prompt.rs`.
- **Restricted sources read:** `speed.exe` v1.3 from the user's own install, through the Ghidra project `/speed.exe`
  (image base `0x400000`, decompiled and disassembled). Read for behaviour only; no code copied. The FEng packages
  dumped with `nfsmw dump-screen` (all 197 screens of the install).
- **Status:** the in-game enter icon (the animated circle with cars and an up arrow) is **not located**. Its FEng object
  name, hash and package are unknown. The card and the disc stay as they are; no enter-icon flag is wired.

## What was checked

**The package search (not found).**

- `HUD_SingleRace.fng` (`GLOBAL/InGameB.bun`, 372 objects) has no enter, drive-in or entry texture. Its textures
  include `Safehouse_Icon.tga`, `Minimap_Icon_Chop_Shop.tga`, `Minimap_Icon_Car_Lot.tga` (the map blips) and
  `Hide_Car_Icon.tga`, none of which is the drive-in icon.
- All 197 FEng screens were dumped and their texture names read. Candidate textures that look like a car or an arrow
  are used by other screens: `Generic_AddCar.tga` by `SafehouseMarkers.fng` (the Kamikaze marker chooser),
  `ARROW_MAIN.tga` by the tutorial and the date entry screens, `Arrow_Glow.tga` by the pause, bounty, milestone and
  world map screens, `Marker_Icon_Visual.tga` by a shop backroom. None is a zone prompt.
- About 40 likely object names were hashed with `fe_hash_upper` (the Black Box string hash of the upper-cased name,
  checked against `SpeedometerGroup` = `0x941fff09` and `FONT_MW_BODY` = `0x545570c6`): `DriveIn*`, `DRIVE_IN*`,
  `Enter*`, `ENTRY*`, `Engage*`, `Interact*`, `ZONE_*`, `SAFEHOUSEINDICATOR`, `CARLOT_ICON`, `SHOP_ICON` and others.
  None matches an object name in any dumped screen.
- The language table has the game's copy for the shop prompt: `0xab1032fa` = `Enter Shop / Engage Event`. This is a lead
  for the card's line (the spec has `Press Enter to go in` as a guess).

**The zone code in `speed.exe`.**

- `0x0058c640` is the enter handler. It switches on the zone's type string (`safehouse`, `carlot`, `customshop`) and
  opens a front-end screen: `safehouse` opens `MainMenu_Sub.fng` (state 2), `carlot` opens `Car_Select.fng` (state 5),
  `customshop` opens `CustomizeMain.fng` (state 3, only with a car selected through `FUN_0056ecc0`). This is where a
  real entry would go; the stub in `entry.rs` does not yet do it.
- `0x0056e100` maps a zone type string to a hash: `carlot` `0x4eaee18b`, `safehouse` `0x0ed39f69`, `customshop`
  `0x0cf07089`, `0` otherwise. Called from `0x0057be83`.
- The routine around `0x0057be40` (its entry is not defined in the Ghidra project, so the containing function is not
  decompiled) works on a per-zone state record. It writes the zone kind at `[state+0x14]` and sets `[state+0x1d] = 1`,
  then uses the object at `[state+0x0c]`. For that object it starts script `0x5079c8f8` unless the object is already
  running it, and checks script `0x0280164f` first. It sends the hash message from `0x0056e100` to the same object.
  **[unconfirmed]** The exact meaning of the two checks and the run flag.
- Script `0x5079c8f8` is a generic colour and position script that almost every screen carries
  (`SafehouseMarkers.fng`, `HUD_SingleRace.fng`, the customization screens, the menus). So it does not identify the icon.
  Its `3700`-tick colour track is the only hint of a pulse, and it is shared.
- The string `DriveInMoment` is at `0x00893270`. No code refers to it by address; it may be a class or debug name, or a
  hashed name. Not resolved.

## Second pass: uncatalogued packages, the marker lead, the routine

**Uncatalogued packages.** The loader (`ui/catalog.rs`, `SCREEN_FILES`) reads 6 files. A throwaway test (not committed)
loaded the other install packages through the same loader and listed their resources:

- `GLOBAL/Ingameb.lzc` (lower case, 2019): 59 FEng packages in all, including a second copy of the HUD set
  (`HUD_SingleRace.fng`, 372 objects, same textures), `Pause_*`, `InGameBounty`, `WorldMapMain` and the in-game screens.
  No enter texture.
- `GLOBAL/InGameSplitScreen.bun`: `HUD_Player1.fng` (95 objects), `HUD_Player2.fng` (57), the split-screen drag HUDs.
  No enter texture.
- `GLOBAL/THINSCREEN_GLOBAL.BUN`, `GLOBAL/DEMO_GLOBAL.BUN`, `FRONTEND/DEMO_FRONTEND.BUN`: loading screens, the demo
  car select (`Cshop_Load.tga`), marketing and demo overlays. No enter texture.
- `GLOBAL/InGameA.bun` (5.9 MB) yields no packages through the loader; its contents were not decoded.
- Across the 198 unique texture names of these packages, the car or arrow names are the minimap cars, `Shift_Up_Icon`,
  `Radar_DirectionArrow`, `Arrow` and `Arrow_Glow` (generic arrows of the pause and bounty screens).

**The SafehouseMarkers lead.** `Generic_AddCar.tga` (75 x 40) is used once, in `SafehouseMarkers.fng` (the Kamikaze
marker chooser, with `Choose Markers`). It is a car with an add mark, not an enter prompt. Not wired.

**The routine, decompiled.** The Ghidra project had no function at `0x0057bde0`; this pass defined one there
(`FUN_0057bde0`, `create_function`, in the shared project). Its body:

- `this` is the zone state. It sets `[this+0x1d] = 1` and `[this+0x14] = param` (the zone type hash, see above).
- If `[this+0x0c]` (an FEng object) is set, it runs script `0x5079c8f8` on it, unless the object runs it already and
  unless it runs `0x0280164f` (then it does nothing).
- It looks up the object named `0x0a729b1b` through `FUN_00524850` (the manager at `[this-0x20]`). If found and not
  running `0x5079c8f8`, it runs that script on it and calls `FUN_004ae8f0(DAT_00911fa8, 0x13)`.
- It sends the zone type hash (`FUN_0056e100`) to the object at `[this+0x0c]` (`FUN_00515b90`).

**The object `0x0a729b1b`.** It is `Image button_ring.tga` in `HUD_SingleRace.fng` (guid `0xe43a0`, size 32 x 43, alpha
80, rotated 90 degrees). Three more `button_ring` images sit at (+23, 0), (-23, 0) and (0, +23) in group `0x35236dbe`,
with `outter_ring.tga` (74 x 74) at the centre. That group is under `0x3345911d` at (3, -189, 50), the same spot as
`Mode_Icon_Circuit.tga` and `Generic_Infraction.tga`: the race-mode badge. So the ring object looks like the race-mode
frame, not the zone icon. **[unconfirmed]** The routine may be shared by more than one prompt.

**Still open.** The object at `[this+0x0c]` (the one that gets the zone hash message) is set by the code that creates the
zone state. The vtable table around `0x008a24f0` has no cross-references, so the constructor was not found. The icon's
cars and up arrow are not in any dumped group. The remaining steps:

1. Find the creator of the zone state (a `new` of a class whose vtable starts at `0x008a24f0`, or a call that stores
   an object at `+0x0c`). The class name may be `DriveInMoment` (the string at `0x00893270`, no direct xrefs).
2. Read the car and arrow pieces of the icon in the game's own screenshot and match them to a group in the packages
   above. The screenshot was not available to this pass.
3. `InGameA.bun` (5.9 MB, not decoded): decode it with `ea_compress::unwrap` and read its packages.
