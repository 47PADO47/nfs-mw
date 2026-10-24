# Installed garage decals

- **Date:** 2026-10-23.
- **Code:** `nfsmw-data/src/car/decals.rs`, customization validation and decal
  model assembly.
- **Specification:** [garage-decals.md](../specs/garage-decals.md).
- **Inputs:** The user's installed parts table, texture packs and authored
  decal meshes, read at runtime. No assets or extracted records are included.
- **Behavioral reference:** CC0
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), `CustomizeNumbers`
  and `FECustomize.hpp`, through the separate reference investigation's plain
  behavior summary. Used to identify number slots and paired number editing.
  The same reference's `CarRenderInfo::UpdateDecalTextures` establishes shape
  suffix hashing. Installed texture names verify the `_RECT`/`_WIDE` identities;
  neither the raw base hash nor a different texture pack supplies that bitmap.
- **Implementation:** Independent Rust code from the written contract. Native
  texture hashes bind existing authored geometry; no external code was copied.
- **Navigation review, 2026-10-24:** the same CC0 reference's
  `CustomizeSub::SetupDecalLocations`, `SetupDecalPositions`,
  `CustomizeDecals::BuildDecalList`, `CustomizationScreen::AddPartOption` and
  `CustomizePartOption` establish the six location icons, twelve door-position
  icons, direct non-door routing, and shared manufacturer-choice icon.
  Public code stores those bitmap identities, not extracted bitmap data.
- **Color filter review, 2026-10-24:** `CustomizeDecals` constructor,
  `Setup`, `NotificationMessage`, `BuildDecalList` and `RefreshHeader`
  establish exact `BRAND_NAME`/`NAME` comparisons, `_WHITE` continuation,
  cart-before-installed initialization, trigger toggling, preserved brand,
  removal fallback, Showcase restoration and the two caption language hashes.
  Installed metadata is read from the user's car-part attributes at runtime.
- **Checks:** Synthetic slot/compatibility tests and ignored installed binding
  tests accompany the implementation. Execution results belong in the change's
  validation report. No matched original-game visual comparison is claimed.
