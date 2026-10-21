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

## Cop zones (`EBreakerStopCops`)

The event carries radius and duration at 4/8; the emitting model's position is the centre. **[decomp]** The cop
manager keeps a list of zones `{centre, radius, end = now + duration}`. On every manager run, before it updates
patrols and pursuits, each police vehicle that is not already destroyed and has pursuit AI is tested: when its
horizontal distance to any zone's centre is below the radius, its damage behaviour **destroys** it (total damage to 1,
every damage zone to the top level, the vehicle-destroyed notification). Then the zones whose end time has passed are
dropped. Nothing brakes the cops. No pursuit has to be active, and the test covers every police car (patrol,
pursuing, roadblock) and anyone who enters while the zone lives. The helicopter is in the same list in the original;
the rewrite destroys it too (horizontal test, height ignored).

The first rewrite braked the cops inside the zone for its duration. That contradicted this behaviour (and
`ai-pursuit-roadblocks.md` section 7) and left the cops alive, so the breakers produced no visible result in a chase.
Zones now destroy: a car wrecked this way is counted once as a wreck of the chase (rep points, cost to state, the
destroyed counter), keeps its wreck and stops sensing, like one wrecked by damage.

## Which models carry breakers, and how one fires

The track holds 16 event engines that open a zone (radius 5 to 24 m, always 5 s) **[verified]**: gas station roof,
donut-shop roof, donut sign, drive-in, gazebo, greenhouse, scaffold support, wooden scaffold, heavy concrete, hill base
sign, radio tower and its supports, sailboat, seafood patio, water tower support. The roofs and towers are triggered
models themselves (`no_trigger`): the supports and pillars next to them send the area stimulus, and a roof counts
several before it collapses. The track places 116 such models in 48 of its streaming sections **[verified]**: 9 gas
stations, 3 donut/tyre shop roofs, 4 gazebos, 57 scaffold supports, 17 radio-tower pieces, 8 heavy-concrete pieces and
the rest (the installed census test counts them; `pursuit breakers` in the console lists the resident ones with their
distance, zone, supports and scene). The structure falls under any vehicle, pursuit or not, and the cops need not be
near for it to open its zone.

What triggers a breaker differs by structure, and is read from each engine's responses **[verified]**. The gas
station's pillars have a permanent attachment (`DETACH_FORCE` -1): ramming one never detaches it and nothing happens
(installed test). Its two **pumps** detach at an impulse of 10 000 N s; the detach (`WORLD_DETACHED`) and the
`EXPLOSION` it causes run the pump's sequence, which sends an area stimulus (30 m) to the roof; the roof's first such
stimulus runs its collapse, with the zone and the moment at 0.067 s, and the pillars hide on the explosion. The player
driving an Elise into a pump at 14 m/s fires it (installed test). Roofs (`no_trigger`) never react to a hit
themselves. Other structures react to `IMPACT_1` on their supports (the wooden scaffold) or to the supports'
detachment (donut/tyre shop posts, scaffolds).

## Cinematic moment (`EPlayRaceNIS`)

Every breaker engine except the gazebo issues, at the same time as the zone, an `EPlayRaceNIS` event (`0x66666DC8`)
whose scene type is "Moment" and whose scene names the structure (`WMGasStation`, `WMBoat`, `WMWaterTower`...)
**[verified]**. The original does not play an animation for it: the NIS activity treats a Moment apart and only runs an
ICE camera track while the world goes on, gated by a player-touch window and a close chasing cop **[decomp]**. The
rewrite plays that cut: [pursuit-breaker-moment](pursuit-breaker-moment.md) (gate, marker, camera track, what the world
does during it, what is not reproduced). The NIS animation codec is not needed for it.

## Map icons

The loading tip says the breaker objects are marked with icons on the map during a pursuit and that hitting one uses
it **[text: `TIP_PURSUIT_BREAKERS`, English.bin]**. The HUD package holds the pool `PURSUIT_BREAKER_0..7` (8 objects,
`Minimap_Icon_Pursuit_Breaker`) **[verified]**. When `EBreakerStopCops` fires, the event also hides every map icon of
kind 12 within 20 m of the breaker (it clears the icon's shown bit) **[decomp]**; the original's icon records are
separate gameplay records placed near the structures, whose source table was not located, and kind 12 was not
identified beyond that.

The rewrite marks each resident breaker that has not fired, while a chase runs, with the pool's icons (drawn only
where the minimap view covers them, at most eight), and a fired breaker takes every icon within 20 m of it with it
(`scenes/world/breaker_icons.rs`). The icon places are the structures' own, not the original's icon records
**[approximation]**. The full-screen map is a front-end screen outside the chase and shows none.

## `EPursuitBreaker`

No installed event engine carries `EPursuitBreaker` (`0x47C1D36D`) **[verified]**. The original posts it from the
local player when the speedbreaker ability toggles and uses it for the speedbreaker music and sound modes. It has no
link to scenery breakers; the scenery handler for it never receives an event from installed data.

## Not reproduced

Authored visual effects of the collapse beyond fragments, the original's own map icon records, and the player
autopilot, traffic stop and camera noise of the moment (see its spec). **No pursuit music, speech or statistic is
raised by a breaker itself**: the kind of `MBreakerStopCops` is used at four places in `speed.exe`: the event's two
constructors, its make-callback and one function (`0x41CD10`, not decompiled) that is the cop manager's handler
registration by the CC0 source; no other listener (speech, sound, HUD) exists in that source **[decomp, inferred]**. The destroyed cops reach the rest the way any wreck does: the speech machine says "unit disabled"
for a wrecked cop, and the chase's destroyed counter, rep points, cost to state and the career's cops-destroyed
statistics count it once (`pursuit-system.md`). There is no breaker statistic in the rap sheet or the milestones. The
loading tip exists (`ui/tips.rs`, `PURSUIT_BREAKERS`, `HIDING_SPOTS`).

## Checks

Installed, run with `NFSMW_GAME_DIR`: the profiles of all 75 engines (`nfsmw-data`), the census of placements per
section, the streamed gas station (resident site with its zone, supports and scene; a pump hit opens the zone, wrecks
the patrol car inside and not the one outside, counts one wreck in the chase; pillars alone do nothing; the player's
car fires it; its moment is answered and its map icon is gone), the moment data and the framing of the gas station's cut
(`moment_tests`), the pool of eight breaker icons in the HUD package, and the console listing. The console command `pursuit breakers` prints the resident breakers nearest
the player, the open zones and the last events.

Native retirement of a destroyed police car coasts or brakes lightly according to shock and speed; the roadblock hold
brake is not kept on a destroyed car.

Checks: parse every installed hierarchy; exercise real support-to-roof chains,
fragment meshes/bounds and repeated gas-station counts; compare complete event,
body and cleanup traces across low/high/uneven render rates; verify parent unload
and offscreen release. A playable smoke run does not prove original-game parity.
