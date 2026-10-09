# The HUD minimap

- **Spec:** [docs/specs/hud-minimap.md](../specs/hud-minimap.md) (behaviour), [docs/formats/minimap.md](../formats/minimap.md) (data).
- **Modules:** `libs/blackbox-minimap` (tiles, projection, view placement, blip placement), the minimap reader in
  `crates/nfsmw-data/src/minimap.rs`, `crates/nfsmw/src/hud/minimap.rs` (the binding to the HUD package), and the
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
  - Tile numbers outside 0..63 draw nothing; in-range row-overlap tiles at the left/right edges still load.
  - The car model's origin stands in for the rigid body's position.
  - The [HUD viewport](hud-viewport.md) reproduces the native 16:9 shift and offers
    centered and Xbox-scaled presets; other wide aspects use a documented host extension.

## Local gameplay polish

- Non-finite/zero-width calibration, incomplete minimap objects or missing/unreadable masks keep the map hidden
  instead of sending invalid transforms or drawing unmasked square tiles. These are host validation rules.
- Landmark tests cross tile and half-tile selection boundaries in fixed and rotating mode, checking the actual
  HUD piece transforms against the world-to-map projection. A real-install test exercises all tiles and checks
  that stationary frames do not upload pixels and movement reuses a bounded set of texture slots.
- The optional hidden `--screenshot-size WIDTHxHEIGHT` creates a real off-screen render target for display-size
  checks. The default remains 1280x720; it does not change saved window preferences.
- A modded install's map failed at the first HUFF tile because the initial reader assumed every tile was JDLZ.
  The block reader now detects wrappers individually with the existing `ea-compress` codecs; measured counts
  are in the format document. No compression implementation or replacement texture data was copied.

## CPU mask optimisation and checks (2026-10-09)

Sliding unrotated masks factor bilinear sample indices and weights by axis, so they are computed once per
column/row and empty rows skip sampling. Rotated masks reuse the horizontal coordinate for every row. Texture
resolution, channel arithmetic and rounding are unchanged. An exact RGBA regression compares the previous
compositor with the optimised path at source sizes 2, 4, 31, 128 and 512, with a non-square mask, different
pivots, zero and nonzero rotations, and partial/flipped sampling windows. A stitched four-tile raster check
also verifies that scrolling the mask leaves no tile seam. Empty masks produce transparent pixels.

The ignored real-install presenter check runs 512 frames spanning all 64 map tiles, moving scroll windows and
both orientations. On the local machine, an optimised release build measured:

| Install assets | Previous CPU time/frame | Optimised CPU time/frame | Texture bytes/frame |
|---|---|---|---|
| Original 128-pixel tiles | 0.542 ms | 0.230 ms | 243,712 |
| Replacement 512-pixel road tiles and small blank tiles | 7.556 ms | 3.078 ms | 3,686,403 |

These measure the HUD presenter and its generated texture payloads, not game FPS or GPU time. Both runs keep
five masked texture slots and 67 decoded images after visiting the full map; stationary frames upload no
repeated pixels. Native 1920x1080, 2560x1440 and 3840x2160 render targets and both DX12/Vulkan were inspected.
The 4K and 1080p original-map captures keep the same logical geometry (after normalising only for comparison,
mean RGB difference below 0.36 on the map region). Replacement maps now render too; the modded install's
separate font-atlas/HUD-digit corruption remains outside this work.
