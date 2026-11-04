# Pursuit obstacle selection and roadblock road walks

## Roadblock site

**[native verified]** The supported PC roadblock builder initializes its navigator
at the target with zero heading weight, enables the trail, selects cop path type,
and walks 250 metres with a zero junction direction hint. Decision segments are
allowed during traversal. Only the resulting site is rejected when it is a
decision segment, disallows traffic, is shorter than 40 metres or lacks a profile.
It does not exclude junctions from the starting road lookup or intermediate walk.
Heading still selects travel direction on the chosen nearest road.

Preserve the selected target's continuous lateral offset before walking. Retain
the reconstruction's minimum target distance and physical placement checks.
This does not guarantee that a site is ahead in screen space: a road can bend
behind the target. Do not reject legitimate curved routes with a global dot test.

## Avoidable neighbourhood

**[native verified]** Road hole punching fetches at most 32 bodies from the AI
vehicle's avoidable neighbour list, not from the world's vehicle insertion order.
**[PC binary]** A shared task rebuilds every list four times a second. Each active
AI vehicle enters with a sweep, the larger of its avoidance radius and twice its
speed; the radius defaults to 20 metres and race driving sets it to its road
look-ahead (traffic sets its own). Two vehicles are neighbours when the squares of
half-size equal to their sweeps, centred on their positions, overlap on both
horizontal axes: `|dx|` and `|dz|` at most the sum of the two sweeps. Both
vehicles' sweeps count, so a fast oncoming car or a parked one is found before it
enters the cop's own square.

The reconstruction receives an unfiltered world-body iterator. **[ours]** Form a
local neighbourhood before its 32-body cap: the cop's sweep uses its cursor
distance or twice its speed, whichever is larger; the other bodies carry no
radius, so each uses the 20-metre default or twice its speed. Exclude the existing
unrelated-height cases and retain nearest bodies when the neighbourhood exceeds
capacity. Keep stable input order among selected bodies. Exact original
spatial-list ordering and its quarter-second refresh remain a separate parity
boundary; formation exclusions follow the next section.

Distant or vertically excluded bodies must not hide a nearby stationary vehicle
or oncoming obstacle. This applies to ordinary traffic and pursuit road steering.
It does not add arbitrary braking, remove physical contacts, or change speed caps.

## Avoidable sets for pursuit driving

**[PC binary]** A pursuing cop uses two sets of its neighbours. The corridor set
cuts its road corridor (road chase and the close-seek lane cursor) and contains
every neighbouring vehicle, the pursued player included. When the cop holds a
formation slot, the corridor set omits the pursuit target and every cop of the
same pursuit that also holds a slot; a cop without a slot keeps all of them. The
separation set, used by close-pursuit capsule separation, contains every
neighbouring vehicle, the target and formation members included, unfiltered.
Props, cones, spike strips and barriers are not vehicles and are not avoided this
way. **[ours]** The rewrite has one pursuit: "same pursuit" is every slotted cop.

**Corridor cuts: who is ignored.** Besides a vehicle more than 5 m above or below
the cop's road section and one entirely behind the cop, a vehicle whose front is
behind the cop's position while it overlaps the cop's width is ignored: it follows
the cop. **[PC binary]** The width is the cop's half width plus the other car's
half extents projected on the cop's right axis, without margins; a car in the next
lane is still judged. Timing, side and width of a cut follow the trail spec
([hole punching](ai-road-nav-trail.md#5-hole-punching-around-other-cars)).

## Path refresh

**[PC binary]** In pursuit the road action asks for a new path search on every
think on which its navigator has no search pending. It aims at the target's
seek-ahead point (the target itself for head-on support) and remembers that aim
as its last find position. The search is queued: one shared path finder runs every
frame with a 1 ms budget, working through queued searches in order (at most 16
searches and 3,072 search nodes). A new request for the same navigator replaces
its queued search. There is no fixed replan period.
**[ours]** The rewrite's search is synchronous and completes in the think that
asks for it, so the road action searches on every think.

## Stuck recovery

**[reference]** When a cop has pressed the throttle for 3 s and moved less than
3 m, it reverses for 2 s at a requested 15 m/s. Its lane cursor is re-initialised
at the car on the centre lane, with any pending path search dropped, so it does not
drive back into what it hit. When the reverse ends, the cursor is re-initialised at
the car again and snapped to the nearest selectable lane. The stuck test does not
run while reversing. **[ours]** A cop that slides along a wall without headway
also counts as stuck (see `ai-driver-control.md` section 7.1).

## Support checks

The original suppresses heavy support while a roadblock remains attached. Keep
that rule. **[native verified]** Selecting any heavy support option, including ram
SUVs, also resets the ordinary roadblock lottery timer to 15 seconds. The delay
must reach the dispatcher for an accepted ram request as well as for a heavy
roadblock request. A rejected request must not repeatedly reset that timer.
This postpones new lottery rolls; it does not cancel an already latched request.
Synthetic/installed automatic-selection tests must distinguish a
permitted support roll from model loading, placement admission and active driving.
Missing SUVs or helicopters in a playtest remain unresolved until the relevant
runtime deployment is observed; a forced deployment test is insufficient.
