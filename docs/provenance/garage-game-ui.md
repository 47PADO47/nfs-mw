# Original garage UI integration

- **Specification:** [garage-game-ui](../specs/garage-game-ui.md).
- **Date:** 2026-10-24.
- **Reference:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0.
  Read `MyCarsManager.cpp`, `CarCustomize.cpp`, `CustomizeManager.cpp`,
  `FEPkg_GarageMain.cpp`, `CustomTuning.cpp`, `uiPause.cpp` and
  `FEPackageData.cpp`, `VehicleDB.cpp` and `uiQRCarSelect.cpp` for behavioral
  facts, including the native model-badge texture fallback and the Add selector's
  stock-car filter, confirmation-only creation and unchanged record on Cancel.
  `UIQRCarSelect::ScrollCars` describes circular car navigation; its source is
  marked unresolved. `ScrollLists` separately returns in customization mode.
  Installed manufacturer-arrow ancestors respond to mouse release with the
  corresponding directional game message; gauge-color vertical arrows use a
  different authored click message and need explicit host pointer binding.
  Its constructor and header refresh identify the maker/model image bindings
  and hidden name placeholders. An installed package dump also confirms two
  name strings share a hash, including a `Debug:` prefix; the host must suppress
  both rather than rely on a first-match object lookup.
  `uiShowcase.cpp` and the installed Showcase hierarchy establish the shared
  maker/model image bindings and the separate controller-hints group replaced
  by the host's binding-aware orbit footer.
  No reference code is copied.
- **Installed assets:** actual package names, image/string/group names and
  hierarchy were measured through the existing FEng catalog and parser. The
  original assets remain in the user's installation and are read at runtime.
  The two door-position selectors explicitly reference the same room overview;
  the actual left/right decal-choice collections reference separate side poses.
  Camera regressions compare those authored references rather than assuming
  each location must have a unique pose in every selection phase.
  Vinyl remap and gauge palettes are read independently from their native
  compatible car-part slots, RGB attributes and the original vinyl brand filter.
  A bounded tuning geometry check also matched the named fill/base images,
  signed dimensions and UV rectangles against `TuningSlider` initialization and
  the common slider draw contract. Texture-coordinate conversion was checked
  against the frontend image interface; these are static facts, not a live
  animation comparison.
  The numeric `OPTION_DATA_1` string is absent from both installed PC tuning
  packages. The generic native widget tolerates that missing data object. The
  geometry regression therefore checks the signed bars and retained tuning
  value without assuming a numeric widget exists.
  Remake preview QA found a transparent gauge sample despite valid textures;
  the installed parent group's INIT-to-HIDE chain and separate SHOW script
  explain that failure. The correction uses that authored animation at page
  entry/resume and is checked through presented-tree visibility.
- **Static executable check:** Ghidra program identity matched the unmodified
  PC 1.3 executable with SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Category constructors independently establish category membership/order,
  package routing and backroom omissions. Platform and camera selectors
  independently establish the shop/backroom names and collection fallbacks.
  Bounded array-scroller notification and directional routines establish
  horizontal row wrapping, bounded vertical movement and the short-final-row
  clamp. The native vertical branch tests the selectable flag; the reference
  source marks this routine unresolved and names that condition differently.
  Paint constructs a selectable twenty-column array; My Cars uses five columns.
- **Private evidence:** retained full package/object dumps, executable identity
  and bounded Ghidra query results. These do not belong in public commits.
- **Live reference:** launched the installed modified game and loaded an
  existing profile without purchases, deletions or save actions. Recorded the
  My Cars manager, customization main, all three subcategory groups, body choices,
  engine packages, paint and empty quick-race cart for one saved Audi TT. The
  session ended through the game's normal Quit command. Those captures establish
  these layouts only; other pages and full economic behavior require separate
  verification.
- **Later live scrollbar check:** read-only navigation in the modified PC copy
  established one-row backing-click selection changes and ordinary acceptance
  when thumb-drag attempts released. No continuous drag was observed. The
  session made no modifications and the recorded profile hashes were unchanged.
  This evidence applies to that installation, not every original platform.
- **Recorded Add-tile action check (27 October 2026):** the user's continuous
  modified-PC capture shows Back and Add New Car while the manager's Add tile
  is selected, then Customize Car, Delete Car and Showcase on a saved car.
  This establishes contextual prompt visibility in that installation. The
  capture does not exercise a Showcase press on the Add tile. Rejecting that
  unavailable preview also follows the existing actual-car-only contract;
  installed-package regressions check the host input path and prompt switching.
- **Implementation boundary:** implement from the written behavior and installed
  data. Reuse the shared FEng runtime, input, animation, presenter and sound
  handling. Keep the temporary F6 testing interface outside normal garage flow.
- **Rim-paint controls:** the CC0 `CustomizePaint::SetupRimPaint` hides the two
  filter groups; `ScrollFilters` returns immediately for rim paint. The native
  fixed palette therefore has no body-finish selector. The installed package
  supplies the retained swatches and their original geometry.
- **Validation boundary:** these references establish static behavior. They do
  not establish live original animation timing, camera fit, controller operation,
  economy execution or matched original/remake rendering. Those comparisons
  require separately recorded runtime evidence.

## Retained carousel return correction (2026-10-27)

The user reported lost garage chrome after Back. The installed regression returned from a part choice to the
retained category screen with no drawn text, instead of its prior item/title/footer. Replaying the retained
package's own INIT scripts and then restoring host strip/model presentation repairs this observed return. No
translated executable code was used. Exact original return animation timing remains unverified; other garage
presentation issues remain separate playtest reports.
