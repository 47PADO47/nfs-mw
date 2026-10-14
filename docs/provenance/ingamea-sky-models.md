# InGameA shared models and the sky

- **Spec:** docs/formats/global-databases.md (InGameA section), docs/formats/world.md (sky section),
  docs/specs/time-of-day.md (sky colour)
- **Sources read:** the retail PC executable `speed.exe` (Ghidra, base 0x400000, read only), the installed
  `GLOBAL/InGameA.bun`. No decompiled source was read.
- **Found in Ghidra:**
  - The string `Global\InGameA.bun` (0x008A91C4) is pushed in `FUN_00666AA0` (the region loader handler,
    `RegionLoader::LoadHandler` by its log string), called from `FUN_00666DE0` during world entry.
  - `FUN_00666AA0` opens it with `FUN_0065FD30(name, 3, 0, 0, 0)`, stores the handle, then calls
    `FUN_006616F0` on it. Then it opens `GLOBAL\INGAMEB.LZC` with the callback `0x0064B390`.
  - `FUN_006D6000` builds 31 effect objects (0x50 bytes each, vtables `PTR_FUN_008AF204`..`PTR_FUN_008AF2A0`),
    each set up by `FUN_006D5C10` from the name table at 0x008F9B60 (stride 0x24).
  - The skybox effect name `skyshader` (0x008AE3E8) is referenced from the effect-record table at 0x008FA5F0.
  - `FFSKYCOLOR` (0x008AF054) and `SUNCOLOR` (0x008AEC4C) are effect constant names. The constant-name
    registration `FUN_006C3D20` (called once per effect from `FUN_006D6000`) is the only code that uses
    `FFSKYCOLOR`'s address directly.
- **Not pinned:** which function resolves the `SKYDOME` and `SKYDOME_XENON` solids from the InGameA set, and
  which function writes `FFSKYCOLOR` / `SUNCOLOR` from the lighting object. `SkyRender.cpp` was not found as
  a string. Those are open.
- **Implemented:** the InGameA solids are read with `blackbox_solid::read_solids` in
  `crates/nfsmw-data/src/world/globals.rs` (`GLOBAL_MODEL_FILES`) and uploaded into the shared set in
  `crates/nfsmw/src/scenes/world/residency.rs`. Solids in the stream win over InGameA solids of the same name.
  That precedence is our choice, not verified in the game.
