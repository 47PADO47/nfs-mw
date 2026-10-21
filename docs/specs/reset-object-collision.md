# Reset object collision grace

The original state distinguishes none, automatic reset and manual reset; both reset flavors use
the same admission and countdown rules below.

A successful reset-to-road placement starts vehicle object-collision invulnerability with a
2-second remaining timer. Initial model placement does not start it. Failed placement does not
start it. Another ordinary manual reset is refused while the state is active; the original guided
reset-zone exception is outside this adapter's current reset implementation.

When an actual primitive overlap exists with another vehicle or resident prop body, query the
invulnerability state before separation, response or event emission. An active vehicle refuses
the penetration and records the current simulation time. Query both vehicles even if the first
refuses, so each active endpoint refreshes its own timestamp. Refused contacts generate no push-out,
impulses, damage, prop activation, collision sounds or collision effects.

At each simulation tick, advance time. If invulnerable and current time is strictly greater than
the last refused-penetration time plus 1 second, subtract this tick's `dT` from the remaining timer.
Clear the state when that decrement reaches zero. The timer pauses during the contact cooldown;
it does not run independently and then merely wait for the cooldown at its end. Starting a reset
does not replace the existing last-contact timestamp. Constructor values are zero for both the
timestamp and timer, and the state starts inactive.

This state does not disable wheel support, body-versus-ground or world/barrier collision. It does
not alter authored mass, friction, collision reactions or prop lifetime. Reset visibility/blinking
and pursuit reset penalties have their existing independent consumers.

The runtime uses the fixed CarSim simulation clock. Ordinary streamed initial traffic placement
does not gain reset invulnerability. Complete native reset breadcrumbs, guided zones, drawing blink
timing, and tractor-to-trailer propagation remain separate work.

Tests must witness a real approaching overlap before checking its refusal; verify both-body order,
unchanged velocities/positions and empty callbacks, actual-contact cooldown refresh, timer recovery,
resident prop activation refusal, retained ground support and successful reset placement wiring.
