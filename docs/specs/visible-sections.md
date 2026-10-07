# Visible sections (which tiles are loaded and drawn)

- **Sources read:** dbalatoni13/nfsmw (CC0, decompiled): `src/Speed/Indep/Src/World/VisibleSection.hpp`
  and `VisibleSection.cpp` (section numbering, boundaries, zone lookup, loading sections, overlays),
  `TrackStreamer.cpp` (`DetermineStreamingSections`, `DetermineCurrentZones`, `SwitchZones`,
  `GetPredictedZone`), `Scenery.cpp` (`GrandSceneryCullInfo::WhatSectionsShouldWeDraw`). Read for
  understanding; no code copied.
- **Data inputs:** the `VisibleSectionManager` chunk (`0x80034150`) and `TrackStreamingSections`
  (`0x00034110`) in `TRACKS/L2RA.BUN`; layouts in [../formats/maps.md](../formats/maps.md#visible-sections-zones-decomp-layout-verified-on-all-records).

## Behaviour

### Section numbers

A section number is `letter × 100 + n`, with `A` = 1 … `Z` = 26 (`A41` = 141, `X0` = 2400). The
`LODOffset` from `VisibleSectionManagerInfo` (40 in MW) splits `n`:

- **regular** sections have a letter from `A` to `T`;
- a regular section is **drivable** when `1 ≤ n < LODOffset`;
- the **far** counterpart of drivable section `s` is `s + LODOffset`; a section is a far section when
  `LODOffset ≤ n < 2 × LODOffset`;
- `Y`/`W` are texture sections, `X`/`U` library (model) sections.

A number in a list may have no entry in the streaming index; it is skipped wherever sections are looked up.

### Boundaries and the current zone

Every drivable section has a **boundary**: a closed 2D polygon in world x, y. A point is inside when it is
inside the bounding rectangle and the even-odd crossing test says so: walking the edges (i, j = previous
vertex), an edge counts when `y` lies in the half-open range between the two vertices' y
(`yi ≤ y < yj` or `yj ≤ y < yi`) and `x` is left of the edge's x at that height.

The **drivable section at a point** is the one whose boundary contains it. If none does, the engine takes
the boundary with the smallest distance from the point to any of its edges (point-to-segment distance;
ties go to the lower section number) and accepts it only if that distance is below **0.1 m**. Otherwise
there is no drivable section there. Only boundaries of drivable sections take part. The drivable
boundaries of MW do not overlap, so the answer does not depend on search order.

The streamer's **current zone** is the drivable section at the player's position (the engine may look up
to 1.5 s or 100 m ahead along the velocity when a prediction zone allows it; that only changes *when*
loading starts). With no drivable section there, the zone is 0.

### What is loaded

Whenever the zone changes, the streamer rebuilds the set of needed sections:

1. Always: `Y0`, `X0`, `Z0`, plus up to four sections the game pins explicitly ("keep" sections; none in
   free roam).
2. For the current zone, if a **loading section** lists the zone among its drivable sections (the first
   such loading section): the union of the visible lists of all its drivable sections, then its extra
   sections; an extra that is drivable also brings its far counterpart.
3. Otherwise: the zone's own visible list.

Needed sections that are not loaded are loaded. Loaded sections that are no longer needed are unloaded,
except texture (`Y`, `W`) and library (`X`, `U`) sections, which stay until memory is needed.

### What is drawn

Each view finds the drivable section at its camera's x, y (the same lookup as above). Then it draws the
scenery of:

- every loaded `Z` section (`Z0` … `Z99`), and
- every loaded section in that drivable section's visible list, in list order.

If the camera is in no drivable section, the view instead draws every loaded section whose `n` is below 10.
Nothing else is drawn, even when it is loaded: a section loaded as an extra of a loading section is drawn
only from zones that list it. Within a drawn section, the exclude flags
([scenery-visibility.md](scenery-visibility.md)) and the LOD rule ([scenery-lod.md](scenery-lod.md)) apply.

Consequences in MW:

- panoramas (`n` = 90–99, `R86`–`R89`) appear only from the zones that look at them; `A91`, `A94`, `C99`
  and `O93` are in no list and are never drawn;
- zones list both a nearby drivable section `s` and its far counterpart `s + 40`, and far zones only list
  `s + 40`.

### Overlays

A named **overlay** (`FlyBy`, `E3Demo`) is a list of edits "add / remove section S in the visible list of
drivable section D". Activating one applies the edits (keeping each list sorted) and records the inverse
edits; deactivating applies those. Not used in normal play.

### Regions

`DrivableSectionsInRegion` lists the drivable sections of the region (373 numbers, 369 unique). No code
we read uses it at runtime. In MW it leaves out exactly the 66 drivable sections whose visible lists hold
only a handful of entries, typically just the zone and its far counterpart: places a car cannot reach.

### For a free camera (not in the game)

The rules assume a car on the road network. A free-flying camera also crosses the 66 unreachable zones and
leaves the map, where the game would draw almost nothing. The viewer therefore uses the drivable section
at the camera only if it is in `DrivableSectionsInRegion`, keeps the previous zone otherwise, and starts in
the region zone whose boundary is closest to the start point.

## Constants

| Value | Where | Meaning |
|---|---|---|
| 40 | data (`LODOffset`) | split between drivable and far section numbers |
| 0.1 m | `VisibleSectionManager::FindBoundary` / `FindDrivableSection` | tolerance outside a boundary |
| `Y0`, `X0`, `Z0` | `TrackStreamer::DetermineStreamingSections` | always loaded |
| `Z0`…`Z99` | `WhatSectionsShouldWeDraw` | always drawn when loaded |
| `n < 10` | `WhatSectionsShouldWeDraw` | sections drawn when the camera is outside every zone |
| 72, 16, 8 | record capacities | visible list entries, drivable and extra sections per loading section |

## How to check it

- In `nfsmw view-world`, `Panorama_TC_City` (`C99`) and the city panorama cards `Panorama_TC_NWestView`
  (`C94`) and `Panorama_TC_EastView` (`C95`) must not appear in the streets around x 1300–1800,
  y −250…400; in the game they are never seen there.
- Standing in zone `D14` (x 2000, y 0), the game draws 35 tiles; distance-based loading at 700 m would
  load 139, including the panoramas above.
- In the original game, drive across a zone border and watch distant panoramas appear and disappear; they
  should change at the same borders in the viewer.
