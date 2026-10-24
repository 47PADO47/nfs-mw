# Physical player city pursuit acceptance

This is an engineering scenario, not an original-game replay or measured parity
threshold. It complements the unchanged recorded-target stress test, whose
35-second same-unit requirement remains a failed engineering stress.

## Independent physical target

Load the installed stock BMW's complete player physics, tire integration and
normal chassis/control configuration. Supply the pursuing fleet with the
analytical performance and nitrous capability derived from that same physics.
Do not configure the BMW as an AI car or substitute a simpler police chassis.

An explicit private target trace supplies an ordered road route and initial
position/direction only. Recorded timestamps and speeds must not set the
physical target's later pose or velocity. A native-style PID driver receives a normal
bounded look-ahead point along that fixed route. Its requested speed is limited
by the BMW's own analytical top speed and upcoming route curvature. Engine,
tires, gears, gravity, walls and pair contacts determine actual movement.
The BMW does not use the pursuit's new target-road tracker to follow its route.

The steering aim uses the longest preview within the normal 30–100 m range
whose straight chord stays within one BMW full width of the intervening route.
Measure this deviation in horizontal XZ only. Search the finite preview interval
in one-metre steps, including its exact upper endpoint. If even the minimum
preview exceeds the budget, retain that minimum and report the excess; do not
hide it by changing the route-validity gate. This geometric bound shortens
steering preview through bends without tuning controller gains. Speed and
curvature planning retain their original speed-based preview independently.
Log both preview distances, chord deviation, actual route error and the initial
cop's detailed action telemetry once per simulated second.

Begin the BMW and initial cop at the same rolling speed, 50 m/s, with the cop
approximately 90 m behind on an accepted road cursor. Use real heat-five rows
and their loaded vehicle variants. Do not substitute an easier cop model,
increase torque, inject sight answers, teleport either vehicle during the run,
or change police tuning to make the scenario pass.

Prepare that cursor with the selected cop's actual half-width and enabled trail
before forming its spawn descriptor. Its width-inset centre is the accepted
placement point. Assert unchanged segment, lane, travel direction and parameter;
keep the existing physical handoff tolerance. Comparing the placed body with an
unprepared raw lane centre would reject the intended native centre clamp.

## Simulation and measurements

Run thirty-five seconds of physical steps. Stream collision around the physical player
and all police. Advance the BMW through ordinary player controls and the police
through the production world. Resolve actual player/cop pair contacts and feed
post-contact body motion, real nitrous telemetry and real sight to the pursuit.

Validate the target independently before interpreting police results. Require
finite state, supported physics on at least 90% of ticks, at least 1,500 m of route progress,
mean physical speed of 50 m/s, at least 30 degrees of traversed route bends,
and at least 10 m of traversed height range. The requested speed uses the existing
potential-speed calculation with the same BMW limits and upcoming route curvature.
A target that leaves the route,
stalls against a wall or fails these conditions invalidates the scenario;
it does not establish police success.
For this bounded fixture, a continuous second more than 8 m from the geometric
route or below 5 m/s is a material route-following failure.

Record the initial cop's actual/requested speed, gap, nitrous, sight, action and
lifetime separately from later replacements. The initial unit should close at
least 20 m of its initial deficit and retain moving engagement during at least
one third of the final ten seconds. Require actual pursuit contact during at
least half of that period. These are declared paired-regression criteria, not
an original-game guarantee that every initial unit survives a long pursuit.

## Limits

The automatic route follower supplies repeatable player inputs, not a model of
human skill. Passing this scenario cannot change the result of the recorded
fast-target stress or prove original-game equivalence. Keep target validity,
initial-unit performance, replacement behavior and overall pursuit contact
separate in diagnostics. Authored race routes, co-op targets and modded-player
performance remain independent acceptance work.
The current fixture has no populated dynamic props. Bend and height coverage
describe the projected route; they do not measure actual chassis rotation.
