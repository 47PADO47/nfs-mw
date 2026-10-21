# Cop destruction contact semantics

Sources and evidence boundaries are recorded in
[the provenance note](../provenance/cop-destruction-contacts.md).
This refines [cop damage](ai-pursuit-cop-cars.md#6-damage-and-destruction).

## Health and visual destruction

When accumulated health damage reaches one, every lamp query reports broken,
including an impact that reaches exactly one without emitting the strict
greater-than-one destruction event. Propagate that health predicate to the
lamp consumer independently of event emission. An ordinary lethal impact still
updates only its selected visual zone through the normal impact/damage stimuli.
Do not substitute explicit Destroy, which sets all visual zones to six, merely
to turn off a health-destroyed car's lamps. Later contacts cannot add health
damage, but may still issue collision/impact stimuli for a retired body.

## Incoming collision data

Damage eligibility uses the magnitude of the incoming velocity component along
the contact normal, excluding tangential sliding. It does not use total contact
speed or the struck body's resolved change in velocity. A magnitude below 1 m/s rejects
the impact. Preserve each body's own local contact arm, signed contact normal,
surface and incoming linear velocity. The second body's normal is the first
body's normal negated.

For object contacts, keep the original collider contact point for callback
position and arms. Compute each local arm relative to that body's position
**after** penetration separation. The solver plane point can move with the
penetrating body; its incoming relative moment velocity, including the temporary
contact centres, supplies the normal closing speed. The separate sliding vector
does not contribute to this gate. Preserve incoming linear velocities independently
of these post-separation arms and solver input velocities.

World and ground callbacks retain this same normal-only quantity from the
successful reaction. A fast scrape along a wall or road can remain below the
damage gate even while its tangential speed is large. Ground callbacks also
retain incoming linear velocity at that reaction, before its response, rather
than substituting the vehicle's velocity at the beginning of the physics tick.
Their local arm remains the original local ground vertex: the later penetration
push must not move that arm into a different damage zone.

The source contact impulse for each body is a change in speed. Multiply that
body's value by its own mass to obtain the impact quantity used by damage.
Object contacts double this quantity; world/ground contacts do not. The contact's
shock-force value is separate from that body's impulse and is passed to shock
handling for object contacts.

After the incoming-speed gate, object callbacks apply shock before virtual
`OnImpact` eligibility and camera-distance filters. A light or no-car-effect
smackable can therefore leave shock without health or visual damage. Do not put
shock behind the cop-specific smackable filter.

The callback's change-in-speed value is the magnitude of the solver's accumulated
normal impulse divided by that body's effective collision mass. It excludes
friction and restitution. It is not the magnitude of the final linear-velocity
change, which includes both. An immobile body's callback instead uses the other
body's reciprocal effective mass. The subsequent damage conversion uses the
struck body's stored physical mass, preserving the distinction between stored
mass and a contact-specific mass scale.

A cop ignores a smackable unless it exposes a dynamics body that is immobile or
has mass strictly greater than 2000 kg. The ordinary vehicle damage path also
rejects a smackable with `NO_CAR_EFFECT`. Retain the source camera-distance
suppression and linked `damagespecs`; these filters apply independently.

For this eligibility test, immobile includes attachment to the world as well as
the source immobile-object-collisions flag. A light prop that stays attached
after the response can therefore damage a cop. A light prop released by the
response is rejected unless its separate source immobility flag still applies.
The original queues collision callbacks; a later contact can change another
body's attachment before dispatch. Capturing post-contact attachment implements
the immediate case without establishing complete deferred-state lookup parity.

The native prop adapter retains the normal accumulator as `shock_force` even
when the other prop is light or has `NO_CAR_EFFECT`. Passing only the final
damage force would lose object shock before those damage filters run.

## Zones and health

Front/rear zones become corners only when both the arm lies past the side's half
extent threshold and the normal points into that side: right arm requires local
normal x below -0.8, left arm requires x above 0.8. Absolute normal magnitude
alone is insufficient. A centre-side fallback at arm x equal to zero selects
left. Top and bottom retain the signed vertical-normal tests.

Use every zone's source `HitPointScale` and `VisualScale`, the source `FORCE`,
`HIT_POINTS` and `HP_THRESHOLD`. Health accumulation and visual damage are
parallel outputs. Damage uses the strict threshold comparison described by the
cop-car spec. Impact stimuli can reach level 6; ordinary stored visual damage
targets at most level 2 per impact. Preserve the incoming velocity in sequencer
dynamic data.

### Sequencer damage tiers

Maintain an original visual level for each of the ten damage zones, initially
zero and distinct from generic paint wear, window state or hit-point health.
Let `v = trunc(ratio * zone.VisualScale)`, `impact = min(6,v)` and
`target = min(2,v)`. On every accepted hit, send to that zone's system, in order:

1. `COLLISION`.
2. For another vehicle, `COLLISION_VEHICLE`, then `COLLISION_COP` when applicable,
   then `COLLISION_PLAYER` when applicable.
3. `IMPACT_1` through `IMPACT_impact`, including tiers already sent by earlier hits.
4. If visual damage is permitted, `target > 0`, the old stored level is at most
   5, and `target > old`, send `DAMAGE_(old+1)` through `DAMAGE_target`, then store
   `target` as that zone's new original visual level.

All stimuli use the same simulation time, allow queuing, and retain the damage
callback's incoming velocity as dynamic event context with its fourth component
set to 1. Missing sequencer/system prevents dispatch, not the stored zone-level
update. At old level 0 and target 2 the first hit sends `DAMAGE_1` and `DAMAGE_2`;
another equal hit repeats impact stimuli but sends no new damage-tier stimuli.
An explicit-destruction level 6 does not regress to ordinary target level 2.
Neither health crossing nor accumulated generic visual wear substitutes for
these per-hit target and per-zone transition rules.

The health threshold is applied after zone scaling: accept only when
`ratio * HitPointScale > HP_THRESHOLD`, then add
`ratio * HitPointScale / HIT_POINTS`. The original compares both sides divided
by positive `HIT_POINTS`; it does not test the raw ratio before choosing a zone.

`IsDestroyed` is true when total damage is at least 1. The ordinary impact path
only accumulates while previous total damage is below 1 and emits the destroyed
event when the new total is strictly greater than 1. These event and predicate
boundaries are distinct. An explicit destruction sets total damage to 1, sets
all visual zones to level 6 when allowed, and emits the destroyed event.

The health-total guard protects only further health accumulation. It does not
return from the entire impact callback: eligible collision and impact stimuli,
and the ordinary visual-level logic, still run on subsequent contacts. An impact
that crosses the health threshold does not automatically set every visual zone
to level 6; that is the separate explicit-destruction operation. Do not discard
queued presentation contacts solely because the AI health flag became destroyed.

## Flips and presentation

The upright task checks approximately every 0.1 simulated seconds, while the cop
is not destroyed, animating or paused and no cinematic is active. Its modeled
body must have fewer than all wheels on the ground, up-dot-ground-normal below
0.5 and up height below 0.5. Unattributed flips destroy after strictly more than
6 seconds. A flipped body whose model causality resolves to the player destroys
immediately; a fixed recent-hit timeout is an approximation of that causality.

Update damage flags, sequencer/model presentation and police retirement from the
same destruction transition. Merely stopping AI does not implement destruction.
Ordinary damage does not imply generic fire or smoke: use established source
effects, and label additional wreck effects as enhancements until verified.

The inspected destroyed-event handler notifies gameplay/race systems; it does
not directly broadcast a destruction stimulus to the vehicle sequencer. Do not
invent a `DESTROYED` or `VEHICLE_DESTROYED` stimulus to replace missing callbacks.
Ordinary damage stimuli and explicit zone-state changes remain separate from
that gameplay notification.

Some installed cop fragment events request named joints. Those pieces require
the parent and child dynamics, their matching authored female/male anchors and
constraint nodes, and collision exclusion for the articulated pair. Treating a
jointed hood or door as a freely detached prop changes its behavior. A backend
without this integration must keep the unsupported case visible in its coverage
report rather than claiming complete cop-part destruction.

The original retirement action ends pursuit, then requests zero gas, brake,
steering, vertical steering and handbrake. When the vehicle is not in shock and
its horizontal speed is at least 2.5 m/s, it instead requests brake 0.25. It coasts
while shocked and below that speed. This action does not establish an engine-off
or full-brake rule; retain the ordinary vehicle dynamics after disabling pursuit
driving.

## Native integration

Native car, world, ground and eligible smackable contacts feed health and vehicle
sequencers from one captured body-specific callback; do not construct a second
visual impact. Preserve source physical mass and contact-specific reaction mass,
inertia, centre and elasticity independently. Goal-selected reaction profiles
must follow the currently active pit, ram, pull-over or static-roadblock goal.
Installed vehicle damage links provide all health and shock fields. A model-name
fallback is only for a missing resolved record, not a replacement for inheritance.
The simulation camera supplies the distance used by damage suppression.

Destroyed cops continue ordinary native rigid-body and suspension integration,
using the retirement controls above. Pursuit-breaker destruction invokes explicit
destruction rather than setting a temporary AI brake. The native constraint
pipeline for jointed doors/hoods, full model causality and matched live original
impact captures remain open; this change does not establish complete parity.


## Remaining source rendering distinction

The original renderer consumes the stored zone levels, not an independent count
of repeated impacts. Ordinary cars enable each installed corner overlay when
its corresponding zone level exceeds zero. Windscreen, rear and corner window
replacement states become damaged when the corresponding source zone level
exceeds one; side-zone levels alone do not drive this ordinary-car replacement
path. Rear window state also controls its defroster replacement.

Supported police models instead own a vehicle-part damage behavior. It maps source
zone levels to installed damage slots, retains the largest damage level affecting
each part, replaces that part's model, and derives window replacements from the
mapped window part state. Cop part-slot damage, window state and articulated
constraints are coupled; generic per-zone glass or panel counters are not the
original police rendering contract.

The police part replacement and health-driven engine effects are implemented
separately as specified in [police visible damage](cop-visible-damage.md).
Supported police appearances consume source zone levels instead of generic
aspect counters. Articulated hood/trunk/door motion remains incomplete.

The generic `blackbox-damage` paint/panel/glass mapping retained for ordinary
cars remains a presentation approximation. Its repeat accumulation, severity
thresholds and uniform glazing table do not establish original visual parity.
