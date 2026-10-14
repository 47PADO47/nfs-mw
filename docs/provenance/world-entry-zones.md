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

## Where the object might be (next steps)

1. The state record's object at `[state+0x0c]` is created by the caller of the routine at `0x0057bde0` (the entry of the
   handler table at `0x008a2508`). Defining the function there in the Ghidra project would show who creates it and with
   which name. Not done: this pass did not change the shared Ghidra project.
2. Ghidra's string search on `ENTER`, `Enter`, `DRIVE` (upper and lower case) across `speed.exe` and `server.dll` for
   FEng object names that the code hashes at run time, rather than as literal strings in the FEng data.
3. The catalogue (`nfsmw list-screens`) reads 197 packages from 6 files, and all of them were dumped here. If the icon is
   not in any of them, it is either in a package the catalogue does not read (for example `InGameSplitScreen.bun`, not
   checked) or it is not a FEng object at all (a drawn image in code). Checking those is the next step.
