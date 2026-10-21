# Police vehicle damage rendering

## Sources and scope

**[decomp, PC verified]** `CarRenderInfo`, `VehiclePartDamageBehaviour`,
`VehicleDamagePart`, `CarPartDatabase::GetNextCarPart` and
`CarRenderConn::UpdateEffects` define a separate police appearance pipeline.
See [provenance](../provenance/cop-visible-damage.md). Health and the installed
collision sequencer remain governed by [cop-destruction-contacts](cop-destruction-contacts.md).

## Authored replacement parts

The supported types are COPMIDSIZE, COPSUV, COPSPORT, COPGHOST, COPGTO,
COPSUVL, COPGTOGHOST, COPSPORTHENCH and COPSPORTGHOST. Each of model slots
0–22 remembers its initial part and the next database part of the same slot
and type search order, without an upgrade-level filter. The next part is the
damaged variant; its upgrade number must not be guessed from its name.
A missing successor clears the slot, rather than retaining the pristine part.
Load the successor's authored geometry and textures for each driving LOD.
Supported police bodies occupy `DAMAGE_BODY`, rather than the ordinary `BODY`
slot. Their lower-LOD body guard must accept this slot; otherwise valid B-D
assemblies are discarded as though they contained only wheels. Other car types
retain the ordinary body guard.

**[PC verified]** The normal ride range can select E. The part-range function
does not clamp police damage-body slots to D, and the draw loop skips a missing
model at the selected level. The installed heat-one pristine body has A-E,
whereas its damaged successors have A-D. Preserve this authored E absence;
do not invent a D cap or substitute pristine parts when the damaged E model
is absent. This can remove damaged body parts at distant E detail.

The appearance uses the original per-zone damage levels, not the rewrite's
paint/panel/glass counters. Any positive zone level selects damaged part level
1; levels only rise until repair. Later contacts, sequencer time, lower-detail
round trips and shared material rebuilding must preserve the actor's selected
damage state. An unavailable authored LOD can omit a part; returning to the
same LOD must return its damaged replacement. Fragment expiry must not clear
the owning actor's hidden-slot selection. The slot membership is:

| Zones | Slots damaged |
|---|---|
| all except bottom | body, both headlights, both brake lights, all five damage windows |
| front | hood, front bumper, bushguard, police lights |
| rear | trunk, rear bumper |
| left | both left doors, police lights |
| right | both right doors, police lights |
| either front corner | hood |
| either rear corner | trunk |
| top | hood, trunk, bushguard, police lights, all doors |

The five damage windows are front, front left/right and rear left/right. Their
positive part levels select WINDOW_DAMAGE0, including the rear defroster
replacement. Each actor carries its own selection even when its uploaded rig
is shared. Hidden sequencer slots still take precedence. Ordinary cars retain
the corner-overlay pipeline and do not acquire these replacement rules.

## Health effects

**[decomp, PC verified]** Engine emitters are independent of the collision
sequencer. The car's `ecar` references emitter groups directly:

1. Nonzero DeathEffect and remaining health <= 0: use DeathEffect.
2. Otherwise nonzero DamageEffect and health <= 0.5: use DamageEffect.
3. Otherwise an engine blown flag: use EngineBlownEffect.
4. Otherwise disable emission; existing particles may finish.

Intensity is 1, emission is continuous, the frame follows the car, and velocity
inheritance uses the car velocity. Authored particle parameters and textures
come from the installation. A health crossing does not fabricate a new
sequencer stimulus or promote every visible zone to maximum damage.

For the PC engine source, its generated local frame uses the mean of the two
front `TireOffsets` positions, with height increased by 0.2 times their lateral
separation. This is independent of the physical hood's damage state. The port's
health-driven cop emitters are independent of tire smoke quality/on-off settings.

## Remaining limits

Animated hood/trunk/door pivots and articulated fragments remain separate
work. Exact emitter visibility culling and all engine-blown player paths need
their own validation. Synthetic and installed integration tests can establish
mesh/emitter selection and renderer delivery, but do not establish visual
equivalence to an original-game playthrough.

## Delayed panel detachment

The installed `CopCar` front-zone `DAMAGE_1` response triggers the hood's
separate action. Its 20-second action emits the hood fragment on the `end`
tag, then hides the parent slot after successful spawning. A destroyed actor
can still complete that already-running action; this is detachment, not repair.

The native fragment connector finds the parent's current slot model, including
its damaged replacement. The fragment must not substitute pristine geometry
for a bent hood. Hidden-slot and duplicate-spawn gates still apply; successful
cleanup of the detached body must not reveal the parent slot again. The
remake's fragment path currently selects the correct actor damage variant at
the canonical A detail. Original current-LOD and animated-pivot inheritance
remain explicit gaps; a shared rig cannot hold one current LOD for all actors.
