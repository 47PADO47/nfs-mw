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

## 4. Checked against a stock-game capture

A recording of the original game (stock install, one pursuit from heat 1 to 5, about 14.5 minutes; the helicopter's
place, goal and fuel, its requests and the player's place sampled 20 times a second) shows **[capture]**:

- **Request and placement.** The first request came on the update the chase reached heat 4, 391 s into it (the
  helicopter enters the wave at heat 4; the timer and the 45 s support delay had long run out). Four placements failed
  within 0.1 s; the fifth, 5.2 s later, succeeded. Both helicopters appeared 242 to 249 m from the player, 12 to 21 m
  above it, at rest: the 250 m along the road and the 20 m lift of the main spec's section 2.2.
- **Fuel.** 60 s at heat 4 and 90 s at heat 5 (`HeliFuelTime`), counted from the placement: the exit goal began 60.2 s
  and 90.1 s after it.
- **Leaving.** Both exits set the "seek car" point (85 m ahead of the player, 25 m up) at once, the helicopter being
  more than 15 m above the player already. The first helicopter hovered at that point while the player drove away and
  was removed 7.7 s later about 540 m from the player, by the out-of-play pass of section 2. The second switched to the
  fly-out after 8.9 s: its seek point lay **1811 m** from the player, which is `200 * |forward + 9 * right|`, so the
  fly-out direction is not normalised (the rewrite's `exit` already uses it so). It was removed 0.5 s later, more than
  25 m up, behind the player and over 150 m away.
- **The next one.** The second helicopter was requested 180.3 s after the first was removed: `TimeBetweenHeliActive`
  (180 s at heats 4 and 5). After the second left, none came in the remaining 126 s.
- **Distance while chasing** (3134 samples): 5th percentile 26 m, quartiles 38 / 71 / 197 m, 95th percentile 307 m,
  most 403 m. While leaving, median 102 m.
- **No search.** The player was never out of the helicopters' sight long enough for a cool-down while one flew, so the
  search and its radius were not seen.
- **No extra cop to evade.** Neither helicopter moved the pursuit's counter for it (see
  [ai-pursuit-cops.md](ai-pursuit-cops.md) section 4).

These agree with the rewrite's rules and tables (`police/heli.rs`, `blackbox_pursuit::helicopter`); nothing changed.

## 5. Not settled

- The original's camera view test (its cone or frustum).
- Which radius the search reads (the main spec reads it as `SearchModeRoadblockRadius`, up to 10 km); a capture with a
  helicopter searching decides.
- Why the four first placements failed (the rewrite places at the first opportunity once the model is loaded).
- The meaning of the helicopter-related counter that adds one cop to evade; the rewrite does not add it.
