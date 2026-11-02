# The police helicopter: when it is asked for, when it is taken out of play, and how it leans

A follow-up to [ai-helicopter.md](ai-helicopter.md) (sections 2 and 5), from a pass over the PC executable on
2026-11-02. Behaviour only; provenance: [provenance/ai-helicopter-tuning.md](../provenance/ai-helicopter-tuning.md).

- **Sources:** `speed.exe` v1.3, the cop manager's update, the pursuit's cop request and the out-of-play pass, read in Ghidra
  and the disassembly for behaviour only; the CC0 decomp ([dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)) for the
  names. No code was copied.
- **Tags:** **[binary]** read in the executable; **[inferred]** a reading not yet confirmed by a capture; **[ours]** a
  choice of the rewrite.

## 1. The request

The cop manager asks a pursuit for its next cop type only while fewer ground cop cars are active than the manager's
maximum and no roadblock request holds the dispatcher. The answer is "none" while the pursuit's own spawn timer runs.
Otherwise the helicopter is allowed when its timer has run out and the pursuit has lasted longer than the support row's
minimum support delay; the search-mode roll may force it ([ai-helicopter.md](ai-helicopter.md) section 2.1). Without a
forced helicopter the type is drawn by weight among the wave's types that still lack members: **a type whose chance is 0
weighs 100**, and the helicopter weighs 0 unless it is allowed. **[binary]**

So the helicopter competes for the same opportunity as a ground car: a full fleet or a standing roadblock request keeps it
away too.

The rewrite (`ai/pursuit.rs` `update_pursuit`, `ai/air.rs`, `blackbox_pursuit::wave::weight`) asks for the helicopter only
after the fleet check, and maps a chance of 0 to a weight of 100 in both the helicopter's and the ground cars' pick.

## 2. Taken out of play by distance

The cop manager's out-of-play pass looks at every active cop. A vehicle that is out of the game camera's view and farther
than **600 m** (a ground car: 375 m) from the point **75 m ahead of its target** along the target's direction is removed
from the world at once: no exit flight, no turning back. **[binary]** The removal counts as the helicopter leaving the
pursuit, so the next one waits the level's time between helicopters **[inferred]**. Running out of fuel and the end of the
pursuit still start the exit flight (section 2.3 of the main spec).

The rewrite (`Helicopter::out_of_play`, used in `police/heli.rs`) measures the 3D distance from the point the respawn rules
use (`respawn::ahead_point`: 75 m along the target's velocity above 1 m/s, otherwise its facing). The view test is the
rewrite's own **[ours]**: in view while the helicopter lies within 60 degrees of the camera's forward direction, wider than
the game's view so that a helicopter on screen is never removed; with no camera nothing is in view. The old rule (600 m
from the player, horizontal, then an exit flight) sent a far searching helicopter back over the player before it left.

## 3. The lean

The attitude rule of [ai-helicopter.md](ai-helicopter.md) section 5 reads the acceleration in the body frame: the pitch from
the forward-back part, the roll from the side part. The rewrite's body frame is nose +X, right -Z, up +Y:

- forward acceleration `a_f` is the local X component and side acceleration to the right `a_r` is minus the local Z;
- pitch `= -(a_f * 9/80) * PITCH_ANG` (plus the climb term), so speeding up dips the nose and braking lifts it; it turns the
  body about its **right** axis;
- roll `= -(a_r * 12/80) * ROLL_ANG`, so a push to the right drops the right side; it turns the body about its **nose**;
- yaw turns it about the vertical;
- the three rates are composed from those body axes into one world angular velocity, then smoothed `(new + 7 old) / 8`.

The acceleration is the velocity change over the last step, as the original reads it back from the body after the physics
step. **[ours]**: the axes follow the physical meaning of the rule; the original's own frame conventions were not re-read.
The rewrite had read the pitch and roll from swapped components and lost the read-back (the velocity was saved after the
step's change, so the change read was zero): the body never leaned. Tests: `helicopter::flight` in `blackbox-pursuit`.

## 4. Not settled

- The original's camera view test (its cone or frustum) and the exit flight's unnormalised fly-out direction.
- Which radius the search reads (the main spec reads it as `SearchModeRoadblockRadius`, up to 10 km); a capture decides.
- The helicopter's one extra cop to evade when it first engages (pursuit side): the meaning and the reset are inferred.
