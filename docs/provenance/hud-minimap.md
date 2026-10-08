# The HUD minimap

- **Spec:** [docs/specs/hud-minimap.md](../specs/hud-minimap.md) (behaviour), [docs/formats/minimap.md](../formats/minimap.md) (data).
- **Modules:** `libs/blackbox-minimap` (tiles, projection, view placement, blip placement), the minimap reader in
  `crates/nfsmw-data/src/track_info.rs`, `crates/nfsmw/src/hud/minimap.rs` (the binding to the HUD package), and the
  multi image mask rectangles in `libs/blackbox-feng` and `crates/nfsmw/src/ui/present/`.
- **Sources read for the spec:**
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), `src/Speed/Indep/Src/`:
    `Frontend/HUD/{FeMinimap.cpp, feMinimap.hpp, FeMinimapStreamer.cpp, FeMinimapStreamer.hpp, FeHudElement.cpp,
    FEPkg_Hud.cpp}`, `World/TrackInfo.hpp`, `Frontend/FEngInterfaces/FEngInterfaceFEObjects.cpp`,
    `bWare/Src/bMath.cpp`, `Frontend/Database/FEDatabase.cpp`, `FEng/FEMultiImage.h`, `config/SPEED_EXE_1_3/symbols.json`
    (function addresses). Read for understanding; no code copied.
  - `speed.exe` v1.3 (the user's own install), disassembled locally with Capstone (not in the repository):
    `Minimap::Update`, `SetupMinimap`, `UpdateTrackMapArt`, `UpdateElementArt`, `ConvertPos`, `GetVehicleVectors`,
    `bATan`, to confirm the formulas (the decompiled `Update` and `SetupMinimap` are marked unsolved) and to read
    the values of `MinimapMaxSpeed` (100.0) and `MinimapShowPursuitCops` from the data section. The zero
    initialised globals (`MinimapPivotX`, `MinimapPivotY`, `MinimapDispX`) are only written by the constructor and
    `AdjustForWidescreen`.
  - Probes of the install: the three whole-city map files, the `TrackInfos` chunk, the HUD package (objects, UV
    rectangles, the mask), the mask and icon textures, and the streaming sections laid over the stitched picture.
    The probe programs are not part of the repository; the facts they measured are in the format document.
- **Implemented:** 2026-10-09, from the spec only (the decompiled code was not open while writing the Rust).
- **Checked against the game by:** unit tests of the projection, the tile choice, the scroll and the blip rule
  (constructed numbers, hand-checked against the formulas); `#[ignore]` tests that read the tiles and the
  calibration from an install; screenshots of the drive scene (`view-world --drive --screenshot`) compared with the
  stitched map (the arrow, the section positions and the road under the car agree). Not compared with a capture
  of the original game.
- **Known differences from the original:**
  - Always the full `MINI_MAP` file, not the career-dependent `Unlock_1` / `Unlock_2` pictures.
  - No blips; the placement rule is implemented in the library and not called.
  - Tile numbers outside 0..63 draw nothing (the original reads a wrapped or out-of-range tile).
  - The car model's origin stands in for the rigid body's position.
  - No widescreen shift (the HUD as a whole is not widescreen aware yet).
