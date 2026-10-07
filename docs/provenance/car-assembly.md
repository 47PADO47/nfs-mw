# Car assembly

Modules: `blackbox-carparts` (`PartsDb::find`, `PartsDb::model_hash`, `SlotTypes::search_types`),
`blackbox-solid` (`PositionMarker`) and `crates/nfsmw-data/src/car/` (`stock.rs`, `assemble.rs`,
`wheels.rs`, `ecar.rs`, `paint.rs`, `swaps.rs`).

- **Spec:** [docs/specs/car-assembly.md](../specs/car-assembly.md); layouts in
  [docs/formats/cardata.md](../formats/cardata.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled): `World/CarInfo.{hpp,cpp}`,
  `CarPartID.h`, `CarRender.cpp`, `CarRenderConn.cpp`, `CarSkin.cpp`, `FEPkg_GarageMain.cpp`,
  `Generated/AttribSys/Classes/ecar.h`. NFSTools/VaultLib (MIT) and SpeedReflect/Nikki (MIT) to cross-check
  layouts. Read for understanding only; the slot names and the slot-to-part-id table are facts (enum
  values), listed in `blackbox-carparts/src/layout/most_wanted/slots.rs`.
- **Implemented:** 2026-10-08, from the spec. While implementing, three points were re-checked in the
  decompiled source and the spec was corrected: name-valued part attributes (`BRAND_NAME`,
  `LIGHT_MATERIAL_NAME`) hold hashes; a model table's selector 1 starts from the part's own type name; and
  undamaged windows all show `WINDOW_FRONT`.
- **Checked against the game by:**
  - the stock rule reproduces the `CE_GTRSTREET` preset slot for slot (81 parts);
  - the M3 GTR's wheel transforms reproduce the spec's worked example (tyre spans, brake depth, 0.095 m
    ride height), and all 164 aftermarket rim models resolve;
  - every car type with an `ecar` record gets four wheels; renders of the M3 GTR and 911 Turbo.
- **Known differences from the original:**
  - the car shader (§8) is not implemented: cars use the generic lit shading with the paint as a flat
    texture;
  - no vinyls, decals, damage, spin, steering or suspension travel; trucks' `ExtraRearTireOffset` is
    ignored;
  - cars without an `ecar` record (`BMWM3`) get no wheels;
  - the front-end LOD range is not applied: the viewer draws the LOD it is asked for.
