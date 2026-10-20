# Race barriers: the walls a race puts up

During a race the original closes the side streets of the course with chevron walls, so neither the player nor the
racers can take a wrong turn. The walls are scenery that is always in the data, hidden and non-colliding; a race's
`Barriers` list switches named **scenery groups** on for the run and the end of the race switches them off again. The
same mechanism closes the career's region gates ([career-regions.md](career-regions.md)) and the junkyard door. This
spec covers what a race does; the gate rules (which stage closes what) stay in the career spec. Addresses and method
are in the [provenance file](../provenance/race-barriers.md); the chunk layouts are in
[maps.md](../formats/maps.md#scenery-overrides-and-groups) and [road-network.md](../formats/road-network.md).

- **Evidence tags:** **[decomp]** read in `speed.exe` (Black Edition PC v1.3) through Ghidra, not tested in game.
  **[verified]** checked against the install's data (`GLOBAL/gameplay.bin`, `TRACKS/L2RA.BUN`, `TRACKS/STREAML2RA.BUN`)
  by the repo's real-install tests. **[ours]** a rewrite choice with no original behaviour behind it.
- **Terms:** a *group* is a scenery group (chunk `0x34109`): a name hash, a group number and a list of override
  indexes. An *override record* (chunk `0x34108`) names one scenery instance and the flags the engine writes into it.

## 1. The data **[verified]**

| What | Count | Notes |
|---|---|---|
| Gameplay collections with a `Barriers` list | 238 | the 8 career bins with gate lists and 230 races |
| Distinct names in the race lists | 736 | all of them name a group of `L2RA.BUN`; 611 are listed plain by some races and starred by others |
| Scenery groups | 754 | numbers 1 to 754; group 1 (15,381 entries) is world-wide and not a barrier |
| Override records | 22,340 | one per switchable instance, in global override index order |
| Wall pieces in the barrier groups | 6,922 | 5,771 `XO_TrackBarrierPlayer_1`, 1,151 `XO_TrackBarrierTraffic_*` (the scenery name is cut at 23 characters), all with flags `0x02808070` |
| Gate pieces | 32 | `XO_TrackBarrierLocked_1`, flags `0x02008070` |
| Door pieces (`SCENERY_GROUP_DOOR`, group 71) | 5 | four `XO_Junkpile_1b_LR_00` and one `XV_Digger_1b_00` in sections 135 and 902, at about render `(-385, 3010)` in front of Hickley Field |
| Drive-through groups (byte `+0x11` set) | 126 | exactly the groups of the 1,151 `XO_TrackBarrierTraffic_*` pieces |
| Track path barrier lines (`0x3414D`) | 1,376 | each names a group by hash; 23 belong to the door |

Entries look like `BARRIER_SPLINE_516`, `*BARRIER_SPLINE_506` (star) and `SCENERY_GROUP_DOOR`. Race 10.2.1
(`race_bin_10/10_2_1_sprint`) lists 36 entries, 24 starred; the tollbooth challenge 01 lists 16.

Every wall piece is hidden in every view on disk (instance bit `0x10`, see
[scenery-visibility.md](scenery-visibility.md)), so free roam shows none. Every group except groups 1 and 641 has
world-collision instances whose group number field is the group's number; their barrier records look the same for
drive-through and full groups.

## 2. Switching a list on **[decomp]**

For each entry, in list order:

1. A leading `*` is removed and remembered as **turned**.
2. The name must start with `BARRIER_`, `BARRIERS_`, `PLAYER_BARRIERS_`, `SCENERY_GROUP_` or `FREE_ROAM` (compared
   ignoring case); any other entry is skipped without a message.
3. The hash is `bStringHash` of the name (case sensitive). The group with that hash is looked up; a name without a group
   does nothing.
4. For each override record of the group: the flags lose bits `0x10` (hidden) and `0x400` (turned), and get `0x400`
   back when the entry was starred. The new low 16 bits are written into the instance's flag word (the high 16 bits are
   kept).
5. The group's state byte becomes `1`, plus `2` when starred, plus `4` when the group's drive-through byte is set. A
   collision query accepts an instance of group `g` only while `g` is 0 or its state byte is non-zero.
6. Every track path barrier line with the group's hash is enabled and copies the drive-through byte.

A group listed twice takes the later entry's turn. After the whole list the road barrier flags are recomputed
(section 5).

A race's list is added **on top** of what is on: the region gates of the stage stay closed. The end of the race
(`basicrace` graph, state `done`) clears every group (each switched-on group gets bit `0x10` back on its instances and
state 0; every barrier line is disabled; the segment barrier and race-route flags are cleared), switches
`SCENERY_GROUP_DOOR` on again without the star, then switches the stage's bin list on again. **[decomp]**

## 3. The turn **[decomp + verified]**

When an override changes bit `0x400` of an instance whose flag word has bit `0x800000` (**turnable**), the engine
rebuilds the instance matrix as `diag(-1, 1, 1) * M` in its row-vector convention, keeping the position: the piece is
**mirrored along its local x axis**. A later switch that clears `0x400` mirrors it back; switching a group off keeps
`0x400` (the piece is hidden anyway). Instances without `0x800000` keep their matrix whatever the star says.

All 6,922 race wall pieces are turnable; the gate and door pieces are not. A wall piece's local x axis runs along the
wall (consecutive pieces of a group step about 8 m along it), so the turn reverses the chevrons: the star makes the
arrows point the other way for a race that runs the street in the other direction. The collision walls are not turned.
So `*SCENERY_GROUP_DOOR` does not open the door: the door pieces are not turnable, and the star only sets state bit 2,
whose reader was not found.

## 4. What the rewrite does **[ours]**, following sections 2 and 3

```
at world creation:            switch SCENERY_GROUP_DOOR on (not turned)            # the engine's clear does it
race start (race or challenge):
    for entry in race.Barriers:
        switch = parse(entry) (star, prefix filter); group = group with bStringHash(name); skip if none or group 0
        remember the group's state before its first switch in this race
        set group on, turned = star
race end, stop or quit:       put every remembered group back to its state before, last switched first
draw:                         an instance of group g is drawn only while g is on; a turnable instance of a turned
                              group is drawn with its x column negated (bounds widened to the cube around it)
collide:                      a collision instance of group g counts only while g is on (the existing query rule)
```

Putting the previous states back gives the same set as the engine's clear-then-re-enable: groups that were off go off,
the gates and the door stay as they were. The career's gate code switches the gate groups by number through the same
set.

## 5. Road barrier flags

The engine recomputes, after every enable pass **[decomp]**:

```
clear CrossesBarrier and CrossesDriveThroughBarrier on every segment
exempt roads: in a race, the road of each race start point (up to four); outside a race, the road at physics
              (-2511, 147.8, 1783), direction (-0.7, 0, 0.7)
for each enabled barrier line (2D, physics x = -y2d, z = x2d):
    for each segment listed in the collision-grid cells along the line:
        cut the segment curve into n pieces (n from its length, at least 4) and test each piece against the line
        if one crosses and the segment's road is not exempt:
            set CrossesBarrier, or CrossesDriveThroughBarrier when the line's drive-through byte is set
```

Racers, the player, the GPS and race routes avoid segments with either flag; cops and the helicopter ignore them
([ai-road-network.md](ai-road-network.md) section 5.3).

The rewrite (`blackbox_roads::mark_barrier_crossings`) cuts each segment's centre line into `max(ceil(length / 10), 4)`
pieces and counts touching lines as crossing. **[ours]** Measured on the install with each race's list plus the door
and the start road exempt: the 215 race courses with a list close 29,495 segment flags in all, and **15 courses lose a
racer route** between two of their points, mostly where a checkpoint snaps to a junction connector that a line crosses.
Since the engine's piece count, its line test and the curves of its junction connectors are not pinned down, the
rewrite **keeps the road flags clear during a race** and applies them only in free roam, where the door's lines (2
segments) and the career's closed gates close roads to the GPS. **[ours]**

## 6. Checks

- `crates/nfsmw-data/tests/real_install/race_barriers.rs`: every listed name has a group; every override record
  names the instance its section hook names (the record stores the drivable section's number, the instance lies in
  that section or its far tile, number + 40) and stores the instance's low flag bits; the piece counts and flags above.
- `crates/nfsmw-data/tests/real_install/race_barrier_routes.rs`: the route numbers of section 5.
- Unit tests: the list reading, the turn (`visibility.rs`), the restore at the end of a race, the crossing test.
- Offscreen screenshots (`view-world --drive --exec "race start 10.2.1 ..." --exec "goto 985 4430 12" --screenshot`):
  with the race running a row of yellow chevron walls stands along the road edge near render (1000, 4460), the
  arrows pointing west, the way the race runs there; after `race stop` the same view has none. The free-roam view of
  the junkyard shows the junk piles and the digger across the gap in front of Hickley Field.

## 7. Open questions

1. The engine's exact segment test (`n` and its rounding, the touch rule) and why its racers do not lose routes;
   until then race barriers do not reach the road graph.
2. Who reads group state bit 2 (starred) and bit 4 (drive-through), and the group byte at `+0x10`.
3. The meaning of the barrier line's `left handed` byte and of the override flag bit `0x8000` (set on every record).
4. Whether the original shows the walls only on the race route's streets or also the turned state on the collision
   walls (only the scenery matrix is rebuilt).
5. Who else switches groups: a scripted event object enables or disables a named group (provenance file); it was
   not traced (the final pursuit probably opens the door this way).
