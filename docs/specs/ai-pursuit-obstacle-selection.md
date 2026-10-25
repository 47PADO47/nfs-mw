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
**[reference]** The vehicle's spatial sweep uses the larger of its avoidance radius
and twice its speed. Race driving supplies its road look-ahead as avoidance radius.

The reconstruction receives an unfiltered world-body iterator. **[ours]** Form a
local neighbourhood before its 32-body cap: use cursor distance or twice speed,
whichever is larger, include body extents, exclude the existing unrelated-height
cases, and retain nearest bodies when the neighbourhood exceeds capacity. Keep
stable input order among selected bodies. Exact original spatial-list ordering
and formation exclusions remain separate parity boundaries.

Distant or vertically excluded bodies must not hide a nearby stationary vehicle
or oncoming obstacle. This applies to ordinary traffic and pursuit road steering.
It does not add arbitrary braking, remove physical contacts, or change speed caps.

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
