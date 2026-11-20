# Provenance: the Fairhaven world and the RIDE 6 track survey

Covers [formats/chameleon-world.md](../formats/chameleon-world.md), [formats/ride6-tracks.md](../formats/ride6-tracks.md) and the
scripts `tools/chameleon_bundle.py`, `chameleon_world.py`, `chameleon_export.py`, `chameleon_census.py`, `chameleon_overhead.py`,
`chameleon_collision.py`, `chameleon_collision_view.py`, `glb_preview.py`, `glb_merge.py`, `glb_merge_preview.py`. Process:
[licensing.md](../licensing.md). Written for [plans/tracks-and-fairhaven-feasibility.md](../plans/tracks-and-fairhaven-feasibility.md).

## Sources

| Source | License | What was used |
|---|---|---|
| The user's NFS: Most Wanted (2012) install, v1.4.0.0 | never copied | Every layout and number in the world doc was measured on its files with the scripts above |
| The repository's own readers (`chameleon-bundle`, `chameleon-gfx`) and their docs | MIT or Apache-2.0 | The bundle, mesh, material, shader and texture layouts; the scripts re-implement them in Python |
| `NFS13.exe` (2012), read in Ghidra (`mcp__ghidra-mcp`, program `NFS13.exe`, not auto-analysed) | proprietary, read only | Only strings: the resource type names (`InstanceListResourceType` 0x00FA0DA0, `PolygonSoupListResourceType` 0x00FA0ED0, `NavigationMeshResourceType` 0x00FA0EEC, `AIDataResourceType` 0x00FA0F20, `RoadDataResourceType` 0x00FA0F5C, `DynamicInstanceListResourceType` 0x00FA0F8C, `LightInstanceListResourceType` 0x00FA0FAC, `GroundcoverResourceType` 0x00FA0FCC, `GroundcoverInstancesResourceType` 0x00FA0FE4, `CompoundInstanceListResourceType` 0x00FA1054, `PropInstanceListResourceType` 0x00FA1078, `ZoneListResourceTypeV2` 0x00FA0D6C), the file names `PVS.BNDL` (0x00FAAFFC), `AIDATA.BNDL` (0x00FA98C8), `GAMELOGIC\ROADDATA.BNDL` (0x00FB7E24) and `TRK_UNIT` (0x00FAB170, 0x00FAB200, 0x00FAB184 lower case). No function was read: the region around the references has no function defined (the program was not analysed) |
| RIDE 6 install (`D:\RIDE 6`, UE 5.6.1) read with CUE4Parse (FabianFG, Apache-2.0, cloned outside the repository) and the game's `.usmap` | n/a | The level, mesh, material and spline structure of the track survey |
| Community knowledge of Burnout Paradise's polygon soups | not used as text or code | None: the soup layout was taken from this repository's car-collision doc and then measured on the world |

No decompiled code and no game data is in the repository; the pictures and exports are in `C:\r6work\` (outside it).

## Method and results

1. **Units.** `tools/chameleon_bundle.py` reads the 169 `TRK_UNIT` tables (resource counts by type, sizes) and the 28,738
   distinct resource ids of `HAWAII/*.BNDL`; the model ids of unit 1 were looked up across bundles (all resolve to `Model`
   resources in the unit, the districts or `GLOBALRESOURCES`).
2. **Placement.** The first `InstanceList` was hexdumped: 240 imports 96 bytes apart gave the record size; the matrix at +0x20
   with the translation in the last row was confirmed by the position -1,148, 22, 1,759 of the first instance and by the
   whole-world extent matching the zone rectangles. An early run read the matrix at +0x30 and gave nonsense extents (all
   zero); the bug was the offset, not the format.
3. **The zone list.** `PVS.BNDL` was read as 169 entries of 0x50 bytes; every entry's unit number matches a file and the
   rectangles cover the instance dots of the census picture.
4. **Meshes.** World renderables use the car renderable layout with float positions; checked by the exporter drawing
   recognisable districts (roads, quays, buildings, a roundabout) in a software preview.
5. **Duplicates.** Resource ids repeated between units were compared by md5: 293 of 293 sampled were identical; the
   deduplicated size (1.07 GB) is the sum over unique ids.
6. **Collision.** The soup header showed `i8` bytes where a car has 0 or 0xFF. Solving for the cells that put the decoded
   chunk box on the stored tree box (x -3, y -1, z +2 for soup 0 of unit 1, i.e. offsets -1,500, -500 and +1,000 m) gave the rule
   `q * 500 / 32768 + 500 * cell`. All 169 units decode with no error and every unit's faces lie within its zone rectangle.
   The quad order was found by drawing: file order gives bow-ties, order 0, 1, 3, 2 gives clean faces.
7. **Not decoded**, with what was seen: `RoadData`, `AiData`, `TrafficLaneData`, `NavigationMesh`, `TriggerData`
   ([formats/chameleon-world.md](../formats/chameleon-world.md#navigation-and-road-data-layouts-partly-seen-not-decoded)).
8. **RIDE 6.** `R6ToolParts list` gave the 1,075 `.umap` files; `jsonmany` dumped the 26 Okayama levels; `export` wrote the
   ground meshes as glb; `raw` measured the cooked size of the track's own assets (622 MB: 468 MB textures, 99 MB meshes, 15 MB
   levels, 69 MB other). Material instances of the ground gave the physical-material mapping.

## Known limits

- Python reads only; no Rust reader exists for the world yet, so the claims are measured with scratch tools, not with unit tests
  in the repository.
- Whether a unit's resources are all that the engine loads with it (the `ZoneList` offsets, `PVS` visibility lists) is not known.
- The flag word of a collision face is not mapped to surface kinds.
