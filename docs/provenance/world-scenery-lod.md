# World scenery LOD

Modules: `blackbox-scenery` (`LodView`, `layout::LodRules`, `LodRules::choose`), `blackbox-solid`
(`Solid::num_polys`, `Solid::density`) and `crates/nfsmw/src/scenes/world/visibility.rs`.

- **Spec:** [docs/specs/scenery-lod.md](../specs/scenery-lod.md)
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled): `World/Scenery.cpp` (model choice),
  `World/Scenery.hpp` (`InlinedViewGetPixelSize`), `Ecstasy/eView.cpp` (`GetPixelSize`),
  `Ecstasy/Ecstasy.hpp` (`eSolid` fields). Read for understanding only.
- **Implemented:** 2026-10-07, from the spec; the constants are data in `layout::MOST_WANTED.lod`.
- **Checked against the game by:**
  - `Density` values read from all 20,377 world solids (median 0.01, up to 4,478);
  - the origin-centred radius of the 10,911 world-space instances (ratio 1.00);
  - renders: the aerial benchmark view drops from 5,134 to 1,701 drawn instances with no visible loss.
- **Known differences from the original:** the view's pixel scale `H` comes from PC code that is not
  decompiled yet. We assume a 480-line reference screen.
