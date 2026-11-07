# Race play-path integration

This follows the user's second remake playtest. Source observations and the
existing native contracts are separate from a new visual parity claim.
Sources and limits: [provenance](../provenance/race-play-path-parity.md).

## Audio ownership

The front end wraps the world in `Pausable`. Its `Scene` implementation must
forward the NIS audio request, soundtrack ownership and voice feedback. Pausing
keeps the one-shot request; the audio mixer pauses playback. Concealing a world
suppresses its request. Testing `WorldScene` alone does not cover this boundary.

A race that bars police must not produce a free-roam 911 feed. Discard its
collision counters while barred so they cannot become delayed calls afterward.
This policy is independent of whether a police car has spawned.
Temporary spawn delays in free roam do not bar police and retain their 911 feed.

## Stock-car tuning

Stock performance equipment does not unlock the tuning sliders. It must still
be possible to open the tuning page, inspect its locked categories, and return
to pause. The entry is enabled in supported modes; category availability belongs
inside the page. Opening it snapshots the driven car without running purchase
eligibility checks or replacing it with a loaded career's car. See
[tuning lifecycle](tuning-ui-lifecycle.md). Back and navigation remain usable.

## Custom options and grid

Ordinary custom races allow zero opponents, as reported by the user for the OG.
Keep the existing extended maximum. Racer ID zero remains the player even when
the physical grid slot varies. Authored forced positions and scripted career
starts take priority. A solo race has one valid grid slot.

The remake draws custom starting slots from a separate random stream, seeded
from the clock in single player or from the world's explicit session seed.
The precise OG distribution is not established. Keep racer identity separate
from physical slot. The primary intro channel follows the human; the primary
physical slot's actor swaps into the human's former channel. Unlinked race cars
use visible fallback staging. See [presentation follow-up](race-presentation-followup.md).
The race grid itself is restored at countdown. Player revs follow the linked
channel, with the primary-curve fallback retained for other race-start scenes.

## Result controls

The win photo and race-statistics screens need contextual host prompts, like
the pursuit statistics screen. Use the actual input bindings and controller
glyphs. The actions must agree with the screen handlers: Continue, Restart,
Race Stats and Back where available. Preserve the package's entrance/fade and
keep legacy keyboard art from obscuring replacement controller prompts.

The first winning reference opens `InGamePhotoMaster.fng` about 0.050 seconds
after the human finish callback, with simulation time already frozen. Open a
win photo on the next update instead of first advancing three seconds of
gameplay under the generic result banner. Retain the existing loss interval.
Advance the finish camera on presentation time while vehicle physics stays
paused. Invalidate frozen rendering while that camera moves.

## Pace evidence

Course completion alone is not a competitive-pace test. The user reports easy
wins with a stock Carrera GT on Hard, both catch-up settings, on Rockridge &
Union (8.3 km). Investigate the production settings conversion, opponent car
performance, planner limits, live driving and distant simulation separately.
Do not invent a speed multiplier to meet an unsupported finish-time target.

The PC race-vehicle creation path supplies normalized performance `[1, 1, 1]`
to vehicle construction. Its upgrade matcher maps each target from that car's
stock/max rating interval, then selects ordinary package levels **[exe]**.
For installed endpoints at or below one this selects maximum ordinary hardware.
This does not grant Junkman parts. The action planner still caps performance
against the player's car; quick-race matching bias defaults to zero **[decomp]**.

Apply maximum ordinary hardware to computer opponents in ordinary quick races.
Keep human cars and scripted career/preset setup outside this follow-up change.
An installed probe must inspect both actual vehicle hardware and action limits.
Its external player trajectory controls catch-up and residency; it is a diagnostic
replay, not evidence of identical human driving or a finished OG parity pass.
Installed maximum packages must also be checked for usable wheel support and
drive force on ordinary ground. A full throttle request with zero supported
wheels does not establish a working physical launch.
Spawned racers must receive the racer collision classification. The existing
[terrain contract](vehicle-ground-reference.md) assigns mask zero to racers;
the traffic mask can discard road faces beneath an otherwise grounded body.
Held rivals also enter physical chassis staging, independently of whether they
have a player launch-band configuration. Release staging at GO. The shared
rest placement seats race cars on their springs; ordinary traffic retains its
separate unloaded placement. Do not substitute a speed boost or freeze pitch
and roll to keep the starting grid upright.

The PC behavior selector overrides race opponents to Simple suspension. Their
catch-up therefore affects positive engine torque, drag and downforce using
the existing [speed/skill contract](ai-driver-speed-skill.md), section 8.3.
Pass those per-car force scales into simulation; leave human, traffic and police
scales at one. Do not change installed specs or use a fixed race-speed boost.
