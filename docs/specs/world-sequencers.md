# World event sequencing and vehicle fragments

Sources: [event format](../formats/event-sequences.md), the original PC 1.3
executable, and the CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
reference: `EventSequencer.cpp`, `DamageVehicle.cpp`, `DrawVehicle.cpp`,
`DrawCar.cpp`, `SimModel.cpp`, `CarPartNames.cpp` and generated event layouts.
This spec precedes the implementation; unresolved behavior is listed explicitly.

## Sequencer

An engine has independent named systems. Each system starts in its initial
state (falling back to its default state if missing). A matching stimulus picks
an end state and an action. Fire its `start` tag, then timed events at time zero.
Advance action time on the shared physics clock; each event fires once when its
time is crossed. On reaching a nonnegative duration, fire `end`, clear the
action, enter its end state and process the oldest queued stimulus. Negative
duration means no automatic end. Render interpolation must not advance events.

The PC function at `0x645A40` resolves an error in the unfinished reference:
an **idle** system invokes a stimulus immediately; a busy system may queue it.
Queue modes are disable (0), shallow (1), allow (2), flush (3), abort (4).
Shallow permits one waiting item; allow permits four. When queuing, check the
response against the projected state at the end of the queued actions, using
the default state if needed. Unknown stimuli do not consume queue capacity.
Flush clears queued items; abort stops the active action before invoking.
An explicit event dispatch queue preserves list order and avoids recursively
borrowing systems when a process-stimulus event starts a different system.

Filters require expression evaluation. Until implemented, filtered states must
return an explicit unsupported result. Unknown event IDs remain observable to
the host, not treated as successfully implemented gameplay effects.

## Damage triggers

An impact's closing speed must be at least 1 m/s. Ignore a hit from a smackable
whose NO_CAR_EFFECT flag is set. Resolve the impacted vehicle's `damagespecs`.
Its FORCE times 1000 is the material strength. Object collisions use twice the
body's collision delta-speed times its mass. Classify the contact using its
local arm, local normal and half dimensions:

- Above/below half the longitudinal extent: front/rear. A corner is selected
  when the arm is beyond half the width and the normal points inward with a
  lateral component exceeding 0.8.
- Otherwise top/bottom when the vertical arm exceeds half height and the normal
  points inward with vertical magnitude exceeding 0.8.
- Otherwise right/left according to the sign of local arm.x.

Compute `floor(impulse / strength * zone.VisualScale)`, capped at six. Send
`COLLISION`, then vehicle/cop/player stimuli when applicable, then `IMPACT_1`
through the computed level to that zone's system. Keep the pre-impact velocity
as the event's dynamic velocity. The original also tracks hit points, visual
damage, shock, suppression distance and repair; those remain separate work.

## Fragments

Resolve part hash to the car-part slot and collision hash to the bounds subtree.
For `ATTACHMENT0..9` the model slots are 52..61. A fragment needs both its mesh
and its collision subtree; missing references produce no fabricated part.
Use the event's smackable attributes and its inherited-velocity factor. Spawn
and draw from the subtree's frame, preserving the mesh's original world pose.
Hide only the parent part requested by the event, retaining it on failed spawn.
Keep detached bodies collidable, renderable and owned after their truck moves.

Original DrawVehicle caps vehicle fragments at 61 and all models at 434. Its
offscreen lifetime comes from KILL_OFF_SCREEN, with the timer cleared on seeing
the fragment again. The exact fragment initialization, explosion response,
constraint limits, child cleanup and event ordering need PC routine validation
before claiming complete parity. Parsing a sequence does not prove its effects.

## Bounds and hitch

Named female/male joint bounds identify the tractor and trailer anchors. Convert
their pivots from model space to body space by subtracting the root pivot.
Match joint names; do not infer anchors from vehicle length or a constant height.
Conical/prismatic child bounds carry constraint data; the current ball-joint
solver remains an approximation until those constraints are implemented.

Disabled hierarchy nodes suppress their children for the intact parent. A
spawned fragment explicitly selects its own subtree, so a disabled car-carrier
attachment can become active independently. The precise primitive transforms
and disabled inheritance must be confirmed against construction before use.
