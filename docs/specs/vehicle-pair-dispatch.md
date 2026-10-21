# Vehicle compound-pair dispatch

Run the original body radius and both body-admission checks before narrow phase.
Visit eligible object primitives in runtime list order, preserving disabled/object
channel filters. Prepare A once at the start of each outer iteration. Prepare B
from its current body pose before each inner pair. Keep previous primitive centres
from the fixed tick. Geometry centres are copied values: do not silently refresh
A after a prior pair separates the bodies.

Immediately resolve each successful intersection, even if its overlap is shallower
than an earlier pair. Separation may occur without an approaching impulse. Continue
the remaining pairs regardless of that response result. A successful reaction sends
its own damage, material sound/effect and gameplay callbacks in that order; never
replace all reactions with a single deepest-overlap or strongest-impulse event.
Temporary mass, inertia and COG modifications remain per contact. Stored body mass
and authored primitive geometry remain unchanged.

The host may expose a strongest-hit summary for diagnostics, but production traffic
must receive every successful reaction for health, shock, strikes and pursuit state.
A body-pair near-miss must only be reported when no successful reaction occurred;
property/assault identity remains deduplicated by the existing gameplay consumer.

Tests cover an earlier separating contact followed by a shallower approaching
contact, refreshed B geometry and retained A geometry, inactive primitives, callback
order/material identity and rotated physical hit locations across installed cop
families. Native dispatch-order and contact-math oracles are independent evidence;
installed tests do not imply matched original driving trajectories.
