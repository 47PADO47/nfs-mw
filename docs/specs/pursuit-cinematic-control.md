# Pursuit cinematic control and arrest placement

Sources read on 2026-10-23: the PC 1.3 executable (SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`),
Ghidra functions at `0x0044da10`, `0x007862d0`, `0x00785a00`,
`0x00777d30` and `0x005cb5d0`; and the CC0-1.0 reference reconstruction
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
`AIVehicle.cpp`, `AIGoal.cpp`, `AnimChooseArrest.cpp`,
`AICopManager.cpp` and `AITrafficManager.cpp`. No original code is copied.

## Moment control

The existing [moment specification](pursuit-breaker-moment.md) records the
PC activity's AI/player ownership events. The reconstruction additionally
identifies the human-vehicle switch: changing AI ownership clears the old goal,
creates the racer goal and sends the current speed to the AI physics switch.
While AI owns it, the human vehicle updates through the race-car driver.
The racer goal includes race driving and unstuck recovery. [decomp]

For a world moment, acquire temporary road navigation and the existing racer
PID driver on the same physical player car. Ignore player/script pedals,
steering, reset, gear changes and Speedbreaker while acquired. The car continues
through the road rather than freezing or holding the user's turn during the
camera cut. The transition does not respawn the car or replace its damage.
Release temporary driver state and pending one-shot inputs when the moment
ends; held physical inputs become effective again after release.

The rewrite can reuse its current race driver and obstacle trail. Its fallback
cruise target is the car's speed on entry because the exact non-race goal's
speed selection remains unresolved. This is an explicit implementation limit,
not an established original speed rule. Failure to locate a road uses a neutral
coast while ownership remains acquired; it never re-enables live input mid-cut.

## Population during a moment

Traffic density is zero while a NIS exists. The cop manager refuses acquisition
and spawning while it exists. A world moment permits retirement of existing
cops; a full NIS does not. [decomp]

A moment therefore holds new traffic, patrol, wave, support, queued cop and
roadblock placements. It must not invoke the session lockout, remove existing
cars, clear pending requests or consume the active-wave budget as if a car had
spawned. Timers and existing car physics continue normally.

## Arrest placement

PC `ChooseArrestLocation` queries the road navigator at the player's physical
position, using cop path rules, direction navigation and a forced centre lane.
The lane profile remains Racing, initialized by the navigator's reset; cop path
rules do not select the Cop lane profile. The PC constructor and reset helpers
at `0x0077b9e0` and `0x00774260` corroborate the reconstructed reset behavior.
There is no direction weight in the segment search. Failure takes the existing
no-scene outcome. A successful query uses the midpoint of the road navigator's
left and right boundary positions, including their height. It does not use the
player's offset lane position or the midpoint between road nodes. [decomp]

The supporting PC helper resolves the road segment, computes nearest progress,
resolves its lane profile and records the road bounds. The former description
of the heading value as distance was incorrect: its helper computes a signed
16-bit angle and divides by 65536. The arrest scene faces from the player's
transform. The exact transform-axis convention still needs native corroboration;
the runtime retains the NIS player's existing facing conversion.

Anchor an arrest NIS at the queried road cross-section midpoint. Do not move
physics to fabricate a staging area. The NIS car tracks provide authored poses
and the existing hand-back path restores the player's physical car. If the
road query fails, acknowledge the unavailable scene immediately so free-roam
payment or the race result can proceed. No procedural animation is added.

The herd's road query uses zero direction weight, matching the original
`Update` query (`AIPursuit.cpp`, reference reconstruction). Temporary player
AI also snapshots and restores `ControlConfig`, so AI steering is not passed
through the person's dead zone, automatic reverse or H-shifter mapping. The
player collision role and physical chassis are retained.
