# World scenery visibility

Modules: `blackbox-scenery` (`SceneryInstance::visible_in`, `layout::VisibilityRules`) and the world scene
in `crates/nfsmw/src/scenes/world/resident.rs`.

- **Spec:** [docs/specs/scenery-visibility.md](../specs/scenery-visibility.md)
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled): `World/Scenery.cpp` (instance culling,
  `SceneryOverrideInfo::AssignOverrides`) and `Ecstasy/Ecstasy.cpp` (`SetupSceneryCullInfo`). Read for
  understanding only.
- **Implemented:** 2026-10-07, from the spec (a one-line rule plus two constants in the MW layout table).
- **Checked against the game by:** the flag statistics of all 77,783 instances (race barriers and animated
  props carry bit 0x10), and renders of the city without chevron barriers.
- **Known differences from the original:** scenery overrides (race barriers switched on per event) and the
  LOD bits are not applied yet.
