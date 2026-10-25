# Target offsets and SUV action entry

Sources: bounded inspection of the supported original PC executable, with
program identity checked before queries; the CC0 reconstruction's road navigator,
target road walker and head-on ram action. See the matching provenance record.

## Target road offsets

**[native verified]** Selectable-lane snapping for cop/racing road cursors preserves
continuous position within the nearest selectable lane's width. It does not
replace every input by the lane centre. Choose using the travel-frame profile
at the heading endpoint. Clamp the requested offset within half the chosen lane
width, then within its connected drivable corridor inset by vehicle half-width
plus 1.5 metres. Degenerate input must remain finite in the reconstruction.

Retained road spots store a floating lateral offset. Reconstruct a spot with
that offset; when given a new physical point, project that point onto the
selected road and apply selectable-offset snapping. Preserve route identity,
increment/rethink periods, prediction horizons and cache ownership.
Close pursuit applies the same selectable-offset rule. Its four-metre snap gate
compares the requested and constrained offsets, not distance to a lane centre.
An immediate offset change rebuilds the line without deleting the existing trail.

## Junction filters at action handoff

**[native verified]** A cop's own drive navigator permits decision segments.
A temporary placement search may exclude junctions when choosing a spawn point;
that restriction must not stop the admitted vehicle following a connected path
through the next junction. Preserve the accepted pose and all other filters.
Promoting an existing patrol initializes its new cop navigator without a
placement-only decision filter, including when it is already inside a junction.

## Head-on SUV action

**[native verified]** Action eligibility requires the vehicle/AI/physics interfaces,
drivability to the target, drivability to its drive navigator, and a false chicken
flag. It has no target-speed threshold. The 60 mph constant belongs to the
one-time launch speed cap. The active action requests 27.78 m/s and steers toward
the velocity intercept; it continues while its target remains valid.

The reconstruction must pass the existing two drivability tests into action
entry. Do not use target speed as a substitute. Preserve the launch-speed cap
and active drive speed. Once active, later failed entry checks do not by
themselves finish the action. Missing target state ends it.

**[native verified]** The road-action fallback for the head-on support goal
disables seek-ahead. Before the ram can enter, it routes toward the current target,
not the normal chase's distant predicted road point. This keeps an oncoming SUV
approaching instead of turning away to join ordinary pursuit traffic. Removing
the support goal restores normal prediction and starts fresh road-action state.
A zero-length path-to-target direction remains zero; it must not invent target
forward as a substitute.

Heat-three light SUVs and heat-four/five heavy SUVs are support requests for
pairs, separate from ordinary chase cars and heavy roadblocks. Correct action
entry does not establish complete support placement, attachment or lifetime
parity. The missing chicken-state adapter remains an explicit boundary.

## Support-pair placement

**[native verified]** Each support search advances its distance counter by ten
metres on every trial, including success. Each car gets up to eight trials;
seven is not a cap shared by the entire request. Thus an unobstructed heavy
pair uses the 340 m and 330 m road positions, rather than requesting one point
twice. The native helper walks the target's road, then reverses the heading for
deployment. A leader's negative distance is walked backward before reversal.

Use an accepted road cursor through admission. Projecting a straight-line point
back onto nearby roads can select another street or deck on bends. Ordinary
reinforcement spacing must not prevent the authored support pair. **[ours]**
Support admission uses non-overlapping physical bounding spheres plus 2 m;
this is a safety adapter, not a claim of exact native spawn-query thresholds.
Existing ordinary reinforcement spacing remains separate.

The current executor can retain partial acquisition/placement. Until complete
native batch activation is implemented, a failed eight-trial car attempt restores
its starting index before a later pending retry. This bounds search distances
without inventing an ever-growing counter or moving an admitted support car.

## Verification boundaries

Synthetic fixtures cover continuous offsets, an admitted cop crossing a junction,
head-on entry against a fast target, paired vehicle choice at heats 3-5, and road
placement around a bend. The installed pair test uses real player/SUV physics and
support rows on a synthetic road/floor, with stand-in rendering geometry. It
measures initial spacing, physical approach and ram requests against a 60 m/s
scripted target. It does not establish original visual or whole-city behavior.
The independent physical BMW city gate remains unchanged and separate.
