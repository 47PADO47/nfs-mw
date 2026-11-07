# Game Moment Camera

The original's gameplay option **Game Moment Camera** (label `0xF26A5CBF`) switches the short cinematic camera shots
that a chase plays on its own: the cut of a pursuit breaker's collapse, a look at the cop that starts a pursuit, a
camera beside a big jump and a camera on a crash through a roadblock. Method and addresses:
[provenance](../provenance/game-moment-camera.md). Evidence tags as in [docs/README.md](../README.md#evidence-tags).

## 1. The option

The row edits one byte of the profile's gameplay settings, `JumpCam`, on by default; the row's value reads On or Off
**[decomp]**. The camera code reads it through a getter that also answers no in the split-screen and online modes
**[decomp]**. Every shot below checks it first.

The rewrite keeps the row under its own label on the Gameplay page (`[game_moments] camera`, on by default). **[ours]**
Below it, one row per kind of shot (`breakers`, `pursuit`, `jumps`, `crashes`, all on by default) lets a kind be left
out; the rows are dimmed while the main switch is off. The defaults play what the original plays. Moments play only in
local single player, as the original's getter allows.

## 2. Which shots it covers

| Shot | Camera | Started by | Lasts |
|---|---|---|---|
| Pursuit breaker | the breaker scene's ICE camera track ([pursuit-breaker-moment](pursuit-breaker-moment.md)) | the breaker's `EPlayRaceNIS` "Moment" | the track's length |
| Pursuit start | the cop-tracking camera (section 4) | a player's pursuit starting | 2 s, from 0.5 s after the start |
| Jump | the car-tracking camera (section 3) | the player's car leaving the ground | twice the predicted airtime, 1 to 4 s, times 4/3 |
| Crash | the car-tracking camera | a roadblock about to be hit at speed | 2 s |

The totalled camera (the car-tracking camera for 2 s when the player's car is wrecked) is not covered by the option
**[decomp]**. A shot plays only while no scripted scene plays and the gameplay camera is in its ordinary state; a
pursuit-start or jump shot also needs the car-tracking camera not to be running already **[decomp]**. Each shot posts
its moment sound (`spot_cop_camera`, `jump_cam`, `crash_cam` of the moment streams) when it starts **[decomp]**.

## 2.1 Pursuit start

When a pursuit of the player begins (the pursuit takes its target), the camera director's cop clock is started at
2.5 s if it is not running **[decomp]**. The camera follows the cop while the clock is between 0 and 2 s, so the shot
starts half a second after the pursuit and lasts two seconds; when the clock runs out the gameplay camera returns
**[decomp]**. In a race the shot is skipped in a pursuit race and in the race's first second **[decomp]**.

The original places a forced start's first cop as the pursuit starts, so the cop is there when the shot looks for it.
The rewrite places that cop a little later (after a jump to a bounty place or a milestone, once its model is in), so a
started pursuit waits up to 3 s for a cop that sees the player before its clock starts, counting only while no screen
fade shows **[ours]**. A cop that is there at the start starts it at once, as in the original.

## 2.2 Jump

The game posts `EPlayerAirborne` on the step a player-driven car's wheels-on-ground count drops from some to none,
that is at the takeoff **[decomp]**. The jump camera then predicts the flight from the car's position and velocity for
up to 3 s against the ground's height and gets three numbers: the mean height of the path over the ground, the peak
height and the time to land **[decomp]**. It plays when the car is faster than 10 m/s, the mean height is between 1 and
20.1 m, the peak is under 20.1 m and the landing within 3.1 s, and the flight is big enough: a peak above 2.5 m and
more than 25 m of ground covered (airtime times speed), or above 1.8 m and more than 15 m when the car takes off inside
a `JumpCam` track zone (type 12, 22 of them in the city) **[decomp]**.

## 2.3 Crash

Each step the gameplay camera checks for a roadblock about to be hit **[decomp]**. A crash shot starts when the pursuit's
nearest roadblock car is between 3 m and a quarter second's travel ahead, an active car (any car: the roadblock's, a
cop's, traffic) is in the same reach, within 0.7 (cosine) of the heading and less than 2.75 m off the line of travel, and
the player's car is faster than 20 m/s, or faster than 10 m/s with a wheel on the ground **[decomp]**. It starts only
from the gameplay camera and lasts 2 s. The rewrite takes the roadblock cars of the running chase **[ours]**: the
original's "nearest cop in a roadblock" of the pursuit.

## 2.4 Slow motion

Both car shots play the world at a fifth of real time **[decomp]**: the car camera writes the camera's simulation time
multiplier, 0.2, every frame. The pursuit shot's half second before the cut ramps the world down to a quarter of real
time within an eighth of a second, and the cop camera keeps that speed **[decomp, the keeping inferred]**. Whenever a
shot's clock runs out, the gameplay camera comes back at a quarter of real time and recovers to full speed over two
seconds (normal once above 90 %) **[decomp]**. The reader of the multiplier was not located; the rewrite slows the world
by it, as it does for the breaker cut's track speed **[inferred]**. The shots' clocks run on real time **[decomp]**. The
return also pulses the visual treatment and zooms the gameplay camera **[decomp]**; the rewrite leaves both out.

## 3. The car-tracking camera

The jump and crash shots look at the player's car from a fixed eye **[decomp]**. The framing depends on the shot: a
roadblock crash (lead 1.0 s, 3.5 m up, looking 1.2 m ahead of the car and 1 m up), a chase (1.3 s, 3 m, looking 4 m
behind the car's centre, 0.2 m down), or any other shot (0.5 s, 0.1 m, 1.2 m ahead, 0.2 m down) **[decomp]**.

The eye is chosen once, from four candidates **[decomp]**:

1. On each side of the road, the lead time ahead along the road (no farther than 150 m): a point at 90 % of the way from
   the road's centre to its edge, at most 9 m out, raised to the ground and the framing's height above it.
2. Beside the car's path 1.5 s ahead (no farther than 150 m), 4 m to each side and 2 m above the ground.

A candidate counts when it sees the car (or, except for a roadblock crash, the car a tenth of a second ahead) and stands
more than 3 m from the last car shot's eye. The road points score their distance from the car's place at the lead time,
the side points a tenth of theirs. The side with the higher total wins; on it the road point unless the side point
scores more. With no usable eye the shot looks from 2 m above the gameplay camera, or from the camera itself when that is
hidden. The eye is raised 0.5 m and never moves **[decomp]**.

The lens keeps 6.6 m of the car in frame: `2 * atan(3.3 / d)` with `d` the eye's distance to the car, between 4.4 and 72
degrees **[decomp]**. The camera looks with the world's up. In the shot's last second it ends early when the car goes out
of the eye's sight **[decomp]**. A random focus pull and a depth of field of 7.5 belong to the shot **[decomp]**; the
rewrite has no depth of field there.

## 4. The cop-tracking camera

The pursuit shot looks at the cop nearest the player, within 300 m, that chases the player, sees the player now and is
in the gameplay camera's sight **[decomp]**. With no such cop it ends at once. It hides the HUD **[decomp]**.

From the gameplay camera's eye and look point, three cubic Bézier paths run **[decomp]**: the eye's over 0.78 s, the look
point's and the lens's over 1.31 s. The look point goes from the old one to the cop's place when the shot started (it
does not follow the cop). The lens goes from the current one to one that keeps 6 m of the cop in frame (between 5.5 and
72 degrees). The eye stays where it is, unless the cop is on the camera's side of the old look point: then it swings
round to 5 m beyond the old look point, away from the cop, at most 2.25 m higher plus 0.25 m, along an arc whose
control points stand off to the side; when either control point cannot see the old look point it stays **[decomp]**.

## 5. What the rewrite does

`scenes/world/moment_cams.rs` (the clocks, the slow motion and the choice of shot), `moment_cams/` (the rules, the jump
prediction and the two cameras) and `moment_cams_world.rs` (the world's facts and queries). The shots post their moment
sounds; whether the installed moment streams sound them is the audio's business. During the speedbreaker, its fade, a
pursuit breaker's cut and any scripted scene, no shot starts; a scripted scene stops a running one.

## 6. Not verified

- No live comparison with the original; the rules are read from `speed.exe`.
- The jump and crash shots have unit tests only. The pursuit shot is checked in the installed city: a chase started
  beside two cops slows the world, looks at the cop for two seconds and recovers (`installed_breaker.rs`).
- The slow motion's reader (above) and the cop camera keeping the slowed speed.
