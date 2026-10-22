# Traffic brake-lamp admission

Sources and boundaries: [provenance](../provenance/traffic-brake-lamps.md).
This state is presentation only; it must not change brake torque or AI controls.

The vehicle FX task reads whether the regular brake control is strictly positive.
For a player-owned vehicle, brake lamps follow that predicate immediately. For a
non-player vehicle, accumulate simulation seconds while the predicate stays true;
reset the accumulation to zero on every non-braking tick. Illuminate the brake
lamp group only when the accumulated value is strictly greater than 0.5 seconds.
Use the same admitted state for texture swaps, side brake flares and the centre
brake flare. A single released tick clears the lamps immediately. Handbrake alone
does not satisfy the regular-brake predicate.

Advance this state once per shared physical tick, never once per render frame.
Read the current NPC input even when its rigid body sleeps; an old physical
control snapshot must not keep the lamps lit after the input releases.
Trailers are independent non-player vehicles: their own cleared brake control
also clears their lamp timer after uncoupling. New placement resets the timer.

This prevents short AI brake corrections from flashing the brake lamps. It does
not add arbitrary smoothing to the driver's physical pedals or invent a minimum
time for which released brake lamps remain illuminated. Headlights, emergency
patterns, turn signals and damaged-lamp admission are separate state.
