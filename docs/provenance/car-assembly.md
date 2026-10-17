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
  - Historical assembly baseline: car shading was initially generic. Selectable original PC material
    families and rigs are now specified in [car material shading](car-material-shading.md) and
    [car lighting](car-lighting.md); remaining skin composition and reflection/shadow gaps are explicit;
  - no vinyls, decals, damage, spin, steering or suspension travel; trucks' `ExtraRearTireOffset` is
    ignored;
  - cars without an `ecar` record (`BMWM3`) get no wheels;
  - the front-end LOD range is not applied: the viewer draws the LOD it is asked for.

## PC traffic tires, 2026-10-12

The selected original PC executable (SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`)
was inspected read-only in Ghidra. Its car-render constructor (`0x0075f5d0`)
checks the car type's usage field at offset `0x94` and redirects the tire texture
only for usage 2. The used-texture helper (`0x00751860`) forms the original tire
name from the base model name plus `_TIRE`. This confirms the existing §6 rule
for PC, independently of the community decompilation. No decompiler output is
included in this repository.

An installed-data probe found missing `TRAILERLOG_TIRE` and `TRAILERB_TIRE`
references on all four assembled wheel placements, while the global texture
pack supplies `TRAFFIC_TIRE`. The `SEMI` wheel already references that shared
texture directly. The regression test uses actual loaded stock meshes and the
same material lookup as both render backends. This verifies texture resolution;
the original/remake visual comparison is a separate runtime check.

The independent missing `TRAILERLOG_SKIN1` group is four triangles of thin rear
trim, not a collision volume. Both its car-type skin flag and `ecar.IsSkinned`
are false in the install. No additional skin substitution was introduced without
evidence of the original fallback. The uploaded-cargo handover regression checks
seven log parts and three carrier cars: their existing textures and world poses
are preserved at release, and the parent slots hide once. This does not establish
the complete dynamic motion or shading parity of those fragments.
