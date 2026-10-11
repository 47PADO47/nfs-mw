# Scenery sequences and pursuit breakers

Sources: installed event packages and model/bounds hierarchies; CC0 generated
event/attribute layouts and scenery rendering; isolated PC event dispatch,
detachment and explosion callbacks. This spec precedes implementation.

Every physics placement keeps its original `EventSequencer` attribute. Its
model can receive area stimuli while still placed and inactive. Preserve each
system's state and queue policy. Advance action times on the shared 60 Hz clock,
dispatch immediate lists in order, and resolve cross-model events through a
bounded queue. No event may advance on the render clock.

Supports and roofs are distinct models. `EProcessAreaStimulus` has system,
stimulus, queue mode and radius at bytes 4/8/12/16. Its dynamic centre is the
emitting model's position. Send the specified stimulus to eligible models in
that sphere; it must not be replaced by a generic collapse command. In particular
gas-station roofs count repeated support events before their collapse sequence.
An attached body's actual release emits `WORLD_DETACHED`. Impact and explosion
stimuli retain the original hashes (`IMPACT_1`, `EXPLOSION`, `EXPLOSION_DAMAGE`).
The rewrite's initial trigger-to-model adapter sends `IMPACT_1` on first
substantial contact; its thresholds and original static-trigger behavior still
need a live comparison. Unconfirmed trigger rules must remain documented.

`ESpawnFragment` selects named render and collision children of the current
model. Match both; missing art or bounds must not hide an intact parent. Use the
event's attributes, inherit linear and angular velocity by the same factor,
preserve its mesh pose, and hide the parent or requested part only after a
successful spawn. The selected collision node establishes the child's body
frame. A child can own another sequence and spawn further descendants. Keep
children associated with their original parent until model cleanup. Releasing
a detached child does not restore a parent part. Placed-model restoration resets
its sequence and removes its children. Streaming out releases tile-owned art
and children together. Vehicle fragments retain their separate 121-piece limit;
scenery models use the PC model capacity of 2297.

`EHideObject` hides drawing; `EKillObject` releases the current model/simulation;
`EWakeObject` wakes it. `EHidePart` changes the named node's visibility. Respect
the distinction between model visibility and collision. Full rigid-body motion
for events requesting simple physics is a documented adapter until that original
solver is measured. No-physics fragments have no collision or integration.

An explosion uses the event's named bounds marker, expansion speed, initial and
maximum radius, and target flags. It advances with fixed ticks. A target receives
the wave once; include newly spawned pieces still inside the wave. Exclude source,
players, vehicles or smackables according to the event. Nonpositive expansion
speed or an invalid marker must not invent an explosion. The PC response opposes
insufficient outward point velocity using mass and simulation delta; positive
`ExplosionEffect` enables the smackable response. It sends explosion stimuli even
when an attached body cannot move. Exact broad-phase shell selection, damage,
causality, world occlusion and cinematic slowdown still need validation.

`EBreakerStopCops` carries radius and duration at 4/8. Track an expiring area on
the fixed clock and brake pursuing cops inside it, including later entrants.
Cinematic events, authored visual effects, dynamic regions, music/speech and
pursuit score notifications remain separate integrations; report their absence.

Checks: parse every installed hierarchy; exercise real support-to-roof chains,
fragment meshes/bounds and repeated gas-station counts; compare complete event,
body and cleanup traces across low/high/uneven render rates; verify parent unload
and offscreen release. A playable smoke run does not prove original-game parity.
