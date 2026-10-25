# Provenance: the Chameleon formats and the 2012 car import

Covers [`libs/chameleon-bundle`](../../libs/chameleon-bundle), [`libs/chameleon-gfx`](../../libs/chameleon-gfx),
[`libs/chameleon-vehicle`](../../libs/chameleon-vehicle) and [`crates/nfsmw-import2012`](../../crates/nfsmw-import2012).
Specs: [formats/chameleon-bundle.md](../formats/chameleon-bundle.md), [formats/chameleon-gfx.md](../formats/chameleon-gfx.md),
[formats/chameleon-vehicle.md](../formats/chameleon-vehicle.md). Process: [licensing.md](../licensing.md).

These crates are **format readers**: they follow the format docs and contain no behaviour taken from decompiled code.

## Sources

| Source | License | What was used |
|---|---|---|
| The user's own NFS: Most Wanted (2012) install (v1.4.0.0) | n/a (never copied into the repo) | Every layout, id and number in the format docs was measured on its files with scratch scripts and with the Rust readers |
| `NFS13.exe` (2012), read in Ghidra | proprietary, read only | The table of resource type ids and names: the executable registers each type with one call that takes the id and a name string; the 59 pairs were read from the call sites (no code was read or reproduced beyond that table). The names `VertexDescriptor`, `Renderable`, `VehicleGraphicsSpec` and the like in the docs come from it |
| Community knowledge of the `bnd2` container (Burnout Paradise and NFS: Hot Pursuit modding) | not used as text or code | Pointed at the right place to start: a `bnd2` header, a resource table of typed entries with zlib blocks, and a pointer-patching import table. No text, code or table was copied; each field was re-measured and the doc says what the 2012 files show |

No decompiled code, no game data and no dump of either is in the repository. The 2012 game was never run or patched.

## Method

1. The bundle header and table were read with a Python scratch reader and checked on all 2,021 `.BNDL` files:
   zlib blocks inflate to the stated sizes, every entry's offsets fall inside the file, and the import offsets fall
   inside block 0. The Rust reader repeats the check on the 203 vehicle bundles (`CHAMELEON_GAME_DIR`, `--ignored`).
2. Resource types were named from the executable's registry (above), then each type's block layout was found by
   comparing instances: a renderable's mesh table, index and vertex buffer records, the material's pointer slots through
   the import table, the shader's sampler-name table.
3. The vertex layout was confirmed three ways. The position scale of 10 comes from matching the decoded extent of
   renderables against their stored bounding spheres (ratio 9.9965 to 10.0001 on four parts). The winding comes from
   the signed volume of closed parts. The texture roles come from looking at the textures (a normal map, a grey
   underbody map and the shared effects map are the first three textures of a textured body material).
4. The assembled cars were drawn: wireframes of the 2012 data first, then the converted cars in the Rust game
   (`nfsmw view-car`, offscreen screenshots). A 911, a Gallardo, a Charger, a Veyron, a Corvette ZR1, a Ford GT, an LFA,
   a Cobra 427, the SWAT van, a Venom GT, an F-150 Raptor and a Tesla Roadster all looked right.

## Known limits

The Genesys objects (gameplay data), damage and crumple data, collision hulls, skeletons, lamp locators and the
diffuse-colour slot rule for materials are only partly understood; the docs say which parts are **[unconfirmed]**.
The import is a visual port: the converted cars drive with the physics, sound and damage of the stock car they clone.
