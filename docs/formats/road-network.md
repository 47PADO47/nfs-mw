# Road network and track paths (`RNgp`, `TrackPathZones`, `TrackPathBarriers`)

Where the original stores the lanes, roads and junctions that traffic, racers and cops drive on, and the
zone and barrier tables next to them. One file holds it all: `TRACKS/L2RA.BUN`. How the engine uses the
data (lane geometry, navigation, path finding) is in [specs/ai-road-network.md](../specs/ai-road-network.md).
For the tag meanings, see [evidence tags](../README.md#evidence-tags). Little-endian, metres, **physics
space** (x right, y up, z forward; see [collision.md](collision.md#coordinate-space-verified)).

All numbers below were measured on this install (PC v1.3) with throwaway scripts that walk the `UGroup` tree
of [collision.md](collision.md#the-carp-blob-ugroup-tree-decomp--verified) **[verified]**; field names and
record sizes are the decomp's `WRoadElem.h`, `WRoadNetwork.cpp`, `TrackPath.hpp` **[decomp]**.

## Where it lives **[verified]**

| Data | Chunk | File | Size |
|---|---|---|---|
| Road network | `RNgp` group inside the `0x3B800 CarpWGrid` blob | `TRACKS/L2RA.BUN` | 5 records, 340 kB |
| Spatial index of road segments | `CDat/CGcn` grid cells (element type 3) | same blob | 17,718 references |
| Track path zones | `0x3414A TrackPathZones` inside `0x80034147 TrackPathManager` | `L2RA.BUN` | 705 zones, 74,628 B |
| Track path barriers | `0x3414D TrackPathBarriers` | `L2RA.BUN` | 1,376 barriers, 33,024 B |
| Position markers (not AI) | `0x34146 TrackPositionMarkers` | `L2RA.BUN` | 15,748 B |

There is **no** road data in `STREAML2RA.BUN` (the stream holds scenery, collision, triggers, emitters,
smackables and `HeliSheet` only) and the road network is not split by section: one network covers the whole
city **[verified]**. These decomp chunks do not occur in the install: `TrackPaths`, `TrackPathPoints`,
`TrackRoutes`, `TrafficIntersections`, `TrackCops` ([world.md](world.md)). There are no speed limits,
traffic-light timings or stop-line records anywhere in this data (see [Not in the data](#not-in-the-data)).

## The `RNgp` group **[decomp + verified]**

The `0x3B800` payload is a `CARP` blob (a 16-byte-aligned `UGroup` tree). The payload of the chunk starts
with 8 bytes of `0x11` filler, then the blob begins at the 8-byte mark (the reader in
`libs/blackbox-collision` handles this). The root has three child groups: `CDat` (collision grid), `Map `
(empty) and `RNgp` (road network). `RNgp` holds five data records, tags `RN` + two characters:

| Tag | Count (this install) | Element | Meaning |
|---|---|---|---|
| `RNhd` | 1 | 14 B | header: six `u16` counts + 2 pad bytes |
| `RNnd` | 4,385 | 32 B | road nodes |
| `RNpf` | 710 | 64 B | road profiles (lane layouts) |
| `RNrd` | 1,308 | 8 B | roads (named groups of segments) |
| `RNsg` | 6,538 | 22 B | road segments |

Each record's byte size equals `count × element size` exactly **[verified]**. An intersection record
(`WRoadIntersection`, 64 B) exists in the decomp, but this install has none: `RNhd` says 0 intersections and
0 junctions, and there is no record for them. Intersections are encoded in the segment graph instead
([below](#junctions-intersections-without-records)).

### `RNhd` (`WRoadNetworkInfo`, 14 B)

| Offset | Type | Field | Value here |
|---|---|---|---|
| 0x00 | u16 | profiles | 710 |
| 0x02 | u16 | nodes | 4,385 |
| 0x04 | u16 | segments | 6,538 |
| 0x06 | u16 | intersections | 0 |
| 0x08 | u16 | roads | 1,308 |
| 0x0A | u16 | junctions | 0 |

### `RNnd` road node (`WRoadNode`, 32 B)

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32[3] | position (physics space) |
| 0x0C | i16 | own index (equals the array index in all 4,385 nodes) |
| 0x0E | i16 | profile index into `RNpf` (valid for all nodes: 0…709) |
| 0x10 | u8 | number of segments attached (1…7) |
| 0x12 | u16[7] | indices of the attached segments (first `n` used) |

Degree histogram: 5 nodes with 1 segment (dead ends), 1,766 with 2, 1,280 with 3, 1,061 with 4, 194 with 5,
68 with 6, 11 with 7 **[verified]**. Node coordinates span x −4,569…987, y −13…265, z −1,404…4,857. The
decomp hard-codes a debug position (−2511, 147.8, 1783); a dead-end node sits at (−2528.4, 147.8, 1798.2),
which confirms that node positions are in the same space as the collision data **[verified]**.

### `RNsg` road segment (`WRoadSegment`, 22 B)

| Offset | Type | Field |
|---|---|---|
| 0x00 | u16[2] | node indices: `[0]` is the start, `[1]` the end of the stored direction |
| 0x04 | u16 | length, `length_m = value × 1000 / 65535` (max 240.5 m here) |
| 0x06 | i16 | road index into `RNrd`, or −1 (none) |
| 0x08 | i16 | own index (equals the array index) |
| 0x0A | u16 | flags ([below](#segment-flags)) |
| 0x0C | u16 | end handle length, `metres = value × 500 / 65535` |
| 0x0E | u16 | start handle length (same scale) |
| 0x10 | i8[3] | end handle direction (x, y, z), scale 1/127; points **back into** the segment |
| 0x13 | i8[3] | start handle direction (x, y, z), scale 1/127; points **along** the segment |

(The decomp has an extra `EA_BUILD_A124` field set for the prototype; the retail layout has 22 bytes.)

**Curve.** Flag `Curved` marks segments drawn as a cubic curve with four control points: start node, start
node + start handle, end node + end handle, end node. The handle vector is `direction/127 × length`. With
these four points as a cubic **Bézier**, the polyline length of every curved segment matches the stored
length with a mean ratio of 1.0006 (min 0.929, max 1.126, σ 0.0036), so the curve type is confirmed
**[verified]**; uncurved segments are straight lines (stored length / chord = 0.9997 on average). The
decomp builds the curve with `USpline::BuildSplineEx`, whose body is not in the decomp (the type enum names
Bézier and Catmull-Rom).

**Handles.** Handle length / chord length averages 0.44 for curved segments (range 0.05…1.9), handle
direction vectors have length 126 or 127 (a unit vector scaled by 127) **[verified]**.

**Segment flags** (`fFlags`, counts in this install **[verified]**):

| Bit | Name | Count | Meaning |
|---|---|---|---|
| 0 | `Decision` | 3,463 | connector inside a junction; always together with bit 3 |
| 1 | `NoTraffic` | 1,356 | traffic must not use the segment |
| 2 | `RaceRouteForward` | 0 | runtime only: race direction (0 in the file) |
| 3 | `Intersection` | 3,463 | same set as `Decision` |
| 4 | `Entrance` | 1,809 | plain road end that enters a junction; never set on decision segments |
| 5 | `CopsXorTraffic` | 21 | flips the NoTraffic test for cops (cops may use it exactly when traffic may not) |
| 6 | `OneWay` | 104 | one-way in the stored direction (start → end) |
| 7 | `Shortcut` | 0 | runtime only: set by race setup |
| 8 | `Curved` | 5,892 | Bézier segment (3,432 of the 3,463 decision segments, 2,460 plain) |
| 9 | `EndInverted` | 1,220 | the profile at the end node is stored mirrored for this segment |
| 10 | `StartInverted` | 1,222 | same for the start node |
| 11 | `ChopperStayLow` | 7 | helicopters keep low over this segment |
| 12 | `CrossesBarrier` | 0 | runtime only (computed from the barriers) |
| 13 | `CrossesDriveThroughBarrier` | 0 | runtime only |
| 14 | `LaneMap` | 239 | no reader in the decomp; meaning unknown |
| 15 | `InRace` | 0 | runtime only: segment is on the current race route |

The runtime-only bits are 0 in the file (bits 2, 7, 12, 13, 15) **[verified]**; the game sets and clears them
when a race starts or barriers change.

### `RNrd` road (`WRoad`, 8 B)

| Offset | Type | Field | Notes **[verified]** |
|---|---|---|---|
| 0x00 | u16 | scale | 256 for all roads; `scale = (value << 8) / 65536`, so 1.0. Multiplies the length of shortcut segments when path distance is summed |
| 0x02 | u16 | length | ≈ 16.37 × (sum of the road's segment lengths in metres) → `metres ≈ value / 16.38` (4000 m = 65535, the decomp's commented `ROAD_LENGTH_*` pair) **[unconfirmed]** scale |
| 0x04 | u8 | shortcut number | 0xFF for all roads in the file; filled at race setup |
| 0x05 | u8 | minimum width | ≈ 8.5 × the narrowest total profile width in metres, saturating at 255 (30 m) **[unconfirmed]** scale |
| 0x06 | u16 | speech id | street-name id for cop dispatch (0 for 549 roads, ids up to 97) |

A road is a chain of plain segments (1…23, mean 2.3). Junction connectors have no road (index −1), as do 2
plain segments. Every road has at least one segment.

### `RNpf` profile (`WRoadProfile`, 64 B)

| Offset | Type | Field |
|---|---|---|
| 0x00 | u8 | number of zones (lanes), 1…15 |
| 0x01 | u8 | middle zone (index of the first zone right of the road's centre line), 0…7 |
| 0x02 | u8[2] | padding |
| 0x04 | u32[15] | zones, only the first `zones` are used |

A **zone** is one packed `u32`: bits 0…3 type, bits 4…17 width (signed 14-bit), bits 18…31 offset (signed
14-bit); both distances are `value × 100 / 8191` metres. Zone 0 is the left-most zone in the stored segment
direction; the offset is the distance of the zone's **centre** from the road's centre line and is stored as a
positive number on both sides (the side comes from the zone index: zones below the middle are left, others
right).

Zone types seen **[verified]** (names follow the decomp's order of `kRoadProfile*` constants, whose numeric
values the decomp does not print; they were derived from the drivable/selectable bit masks the nav uses and
agree with the data, e.g. type 4 is a wide strip at offset 0 and type 7 is never drivable) **[decomp +
inferred]**:

| Type | Name | Zones here | Notes |
|---|---|---|---|
| 1 | Traffic | 1,634 | travel lane; width ≈ 4.0 m |
| 2 | Sidewalk | 251 | |
| 3 | Shoulder | 732 | outermost zone on most roads |
| 4 | Median | 160 | wide centre strip (15.9 m on 9-zone profiles) |
| 7 | Barrier | 75 | undrivable for everyone but the "any" lane type |
| 10 | Road | 471 | plain drivable lane that is not a traffic lane (profile 0 is two of them) |

Types 0, 5, 6, 8, 9, 11, 12, 13 are named in the decomp but absent from this data. Profile zone counts: 2
(194 profiles), 3 (76), 4 (114), 5 (87), 6 (93), 7 (57), 8 (11), 9 (42), 10 (3), 11 (23), 13 (6), 15 (2), 1
(2). Example (profile 1): shoulder, traffic, traffic | traffic, traffic, shoulder, middle 3, centres at
10.0, 6.0, 2.0 | 2.0, 6.0, 10.0 m, all 4.0 m wide **[verified]**. Zones right of the middle carry traffic in
the stored direction (right-hand traffic).

**Profiles belong to nodes.** Every node points at one profile; a segment interpolates between the profiles
of its two nodes. The "inverted" flags say whether a node's profile must be mirrored for this segment (the
profile is shared by every segment at the node, and segments are stored with arbitrary orientation).

## Junctions: intersections without records

A junction is a small group of nodes joined by `Decision` segments, one node per approaching road
**[verified]**:

- A node is one of three kinds: a chain node (2 plain segments, no decision segment: 1,765 nodes), a dead
  end (1 plain segment: 5 nodes) or a junction node (exactly 1 plain segment plus `k` decision segments).
- Junction nodes by `k`: 1 (1 node), 2 (1,280), 3 (1,061), 4 (194), 5 (68), 6 (11); 2,615 in total. An approach node usually connects to every other approach node of the same junction, so a
  4-way junction is 4 nodes × 3 decision segments (6 curved connectors, each used in both directions). The
  decision segments form 731 connected groups: 420 of 3 nodes, 256 of 4, 33 of 5, 12 of 6, 3 of 7, 4 of 8, 2
  of 9 and one of 23; 711 groups are complete (every pair joined), 20 are not (missing turns). No pair is
  joined twice.
- Decision segments are short (4…174 m, mean 37.5 m), curved (99 %) and carry no road index.
- Dead ends (5 nodes) have no junction around them.

## Spatial index **[decomp + verified]**

The collision grid's cells (`CDat/CGcn`, [collision.md](collision.md#the-collision-grid-0x3b800-decomp--verified))
list road segments as element type 3: 17,718 references in 2,779 non-empty cells (64 m cells), covering all
6,538 segments, indices up to 6,537. The game finds segments near a point by asking the grid for the cells in
a radius and reading their type-3 lists; a rewrite may build its own index from the node positions instead.

## Track path zones (`0x3414A`) **[decomp + verified]**

Zone records are stored back to back with variable size; the size of each is its own `MemoryImageSize`
field. Chunk payload starts at the chunk data offset (no filler). 705 zones, the whole chunk is consumed
exactly.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | type ([below](#zone-types)) |
| 0x04 | f32[2] | position (the 2D frame below) |
| 0x0C | f32[2] | direction (unit vector; (1, 0) when unused) |
| 0x14 | f32 | elevation (m); heights for tunnels, overpasses, garages |
| 0x18 | i8 | source (1 for elevation zones, 0 for the others here) |
| 0x19 | i8 | cached index (runtime) |
| 0x1A | i16 | visit info (runtime) |
| 0x1C | u32 | user-data pointer (runtime) |
| 0x20 | f32[2] ×2 | bounding box min, max |
| 0x30 | i32[4] | data |
| 0x40 | i16 | number of polygon points |
| 0x42 | i16 | record size in bytes = `0x44 + 8 × points` (100…, exact for all 705) |
| 0x44 | f32[2][n] | polygon points |

**2D frame.** Zone and barrier coordinates are in the track's 2D frame, the pair the engine calls
`bVector2`: physics `x = −y2d`, `z = x2d` (the road code builds physics-space vectors from barrier points as
`(−y, 0, x)`) **[decomp]**. Zone positions span x2d 6…4,767 and y2d −1,585…5,589, which maps into the node
range above **[verified]**.

### Zone types

| Id | Name | Zones | Data / use |
|---|---|---|---|
| 0 | Reset | 0 | |
| 1 | ResetToPoint | 0 | |
| 2 | GuidedReset | 0 | |
| 3 | Tunnel | 70 | elevation = tunnel height |
| 4 | Overpass | 143 | |
| 5 | OverpassSmall | 178 | |
| 6 | StreamerPrediction | 211 | `Data[0..2]` look like streaming section numbers (e.g. 810, 804) **[unconfirmed]** |
| 7 | Garage | 8 | |
| 8 | Hidden | 34 | |
| 9 | TrafficPattern | 11 | `Data[0]` = bStringHash of the `trafficpattern` collection name (1837492109 = `collegesouth`, 1398340799 = `downtown`, 1831559237 = `collegenorth`) **[verified]**; polygon 6…14 points |
| 10 | Dynamic | 14 | |
| 11 | Neighbourhood | 8 | `Data[0]` = a name hash (not resolved here) |
| 12 | JumpCam | 22 | |
| 13 | NoCopSpawn | 3 | cops may not spawn inside |
| 14 | PursuitStart | 3 | |

Polygon point counts: 4 (480 zones), 5 (79), 6 (73), 7 (28), 8 (20), more for the rest. The point-in-zone
test is a plain polygon test on the 2D point; `FindZone` caches the zones overlapping a 64 m box around the
last query **[decomp]**. Which gameplay code reads which zone type is in the AI specs
([ai-traffic](../specs/ai-traffic.md) for pattern zones, the pursuit specs for the cop zones).

## Track path barriers (`0x3414D`) **[decomp + verified]**

24-byte records: two 2D points (`f32[4]`), `i8 enabled`, `i8 pad`, `i8 player barrier` ("drive-through"),
`i8 left handed`, `u32 group hash` (bStringHash of a scenery group name). In the file `enabled` and `player
barrier` are 0 for all 1,376 barriers; the game enables the barriers of a scenery group when an event turns
that group on, and sets `player barrier` from the group's drive-through flag. The hashes repeat: 23 barriers
share `2279506743`, others come in runs of 4…5. The road code marks every segment whose curve crosses an
enabled barrier (segment flags 12 and 13, [spec](../specs/ai-road-network.md#barriers)).

## Not in the data

- **Speed limits:** `WRoad` and `WRoadSegment` carry none. Target speeds come from AI tuning
  ([specs/ai-driver-control.md](../specs/ai-driver-control.md), [specs/ai-traffic.md](../specs/ai-traffic.md)).
- **Traffic lights and stop signs:** no records. The lights in the world are scenery/props
  ([world.md](world.md)); see the traffic spec for whether the AI reacts to them.
- **Race routes:** not stored in the track. They are computed from gameplay markers by A*
  ([specs/ai-pathfinder.md](../specs/ai-pathfinder.md)).
- **Lane connectivity at junctions:** not stored; the nav keeps the lane index and picks the matching lane
  on the next segment.

## Reading it in Rust

`libs/blackbox-collision` already walks the `CARP` tree and exposes the grid with the road-segment lists.
The record layouts above are enough for a `RoadNetwork` reader (a new engine-generic lib or a module next to
the grid; no MW names are needed because the counts come from `RNhd`).

## References

| Source | What | License |
|---|---|---|
| dbalatoni13/nfsmw `World/WRoadElem.h`, `WRoadNetwork.h`, `Common/WRoadNetwork.cpp`, `TrackPath.hpp`, `TrackPath.cpp`, `Common/WGrid.h`, `Common/WGridNode.h` | struct layouts and field meanings | CC0-1.0 |
