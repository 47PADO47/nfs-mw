# NIS events, part 3: what the PC game does with them

What each scene event does in the PC game, the cars a scene makes from its special car list, and the camera key bytes
the camera mover reads. Part 1 ([nis-events.md](nis-events.md)) has the data layout and the census; part 2
([nis-events-2.md](nis-events-2.md)) the per-scene timelines. Where this part and part 1 disagree, this part holds: it was
read in the PC executable, and checked on the running game.

Evidence tags as in part 1, plus: **[pc]** read in the PC executable (`speed.exe` 1.3, the unmodded install's), not
from the decomp; **[live]** observed on the running unmodded game (the number of runs is given); **[pictures]** from
pictures of the running game. Addresses are in [provenance](../provenance/nis-events-pc.md), not here.

## 1. How an event acts

- A scene event is built by its type's make-event callback from the event's static data. The callback copies the fields
  into an event object taken from the event pool, and records the handle of the car whose channel system fired it
  (the sequencer's current target). **[pc]**
- Most types act **when they are built**, in their constructor: the car events call the car's NIS control interface
  (section 2.1), the world events set a global or call a system. **[pc]**
- Five types act when the event object is **destroyed**, at the end of the frame's event processing: `ENISDetach`,
  `ENISReattach`, `ECameraPhotoFinish`, `ECellCall` and `EDDaySpeech`. **[pc]**
- Five types do nothing on the PC: their callback stores the fields and nothing reads them, no constructor or
  destructor acts, and no code refers to their objects. They are `ENISTimeOfDay`, `ENISFakeFar`, `ENISPlayEffect`,
  `ENISNukeSmack` and `ENISDetail`. **[pc]** So the sunset light of `IntroNisDD` does not come from its six `ENISTimeOfDay` events, and the debris of
  its pile-up does not come from its `ENISPlayEffect`. (The third-party `ENISTimeOfDay` mod note in part 1 describes a mod
  that adds a handler.)
- A channel with no car fires nothing: in the original, the car events of `car3`/`car4` of the two-racer starts
  (`IntroNis04`) never appear, while the events of the bound channels do. **[live, 1 run]** The rule that stops them
  (the sequencer system without a target, or the scene's handling of a missing car) was not read.
- Event times match the scene data: across the six prologue scenes, 192 events paired by type and channel were within
  0 to 0.067 s of the data's time on the scene's animation clock (that clock runs one frame behind the activity clock,
  section 5). **[live, 1 run]**

## 2. The handlers

### 2.1 Car events: the NIS control interface

The car of the firing channel is looked up by its handle; the event then calls one method of the car's NIS control
interface (cars made for a scene have the spline suspension, the spline engine, the NIS input and the NIS draw
behaviours). Methods and fields below are those of the CC0 decomp's `SuspensionSpline`, `EngineSpline`, `DrawCar` and
`CarRenderConn`, which match the PC code read. **[pc + decomp]**

| Event | Call | Effect |
|---|---|---|
| `ENISBrakelock` (front, rear) | `SetBrakeLock(front != 0, rear != 0)` | Locked tyres do not turn (angular velocity 0, traction 0). Both locked: brake 1; rear only: handbrake 1; else neither. Held until the next brake-lock event or the car's reset. |
| `ENISBurnout` (speed) | `SetBurnout(speed)` | The driven wheels turn at road speed plus `speed`; the NIS input's throttle is 1 while a burnout is set, else 0.5. |
| `ENISCarPitch` (dip, time) | `SetAnimPitch` | A triangle over `time`: 0, up to half the angle at mid-time, back to 0. The angle is `atan2(dip, wheelbase / 2)` divided by pi, then used as radians. Added to the drawn body's pitch. |
| `ENISCarRoll` (dip, time) | `SetAnimRoll` | The same with the car's width; added to the body's roll. |
| `ENISCarShake` (dip, rate, ramp, time) | `SetAnimShake` | A body **roll** vibration, not a camera shake: `angle * cos(2 pi * phase) * envelope`, with the angle from `dip` and the width as for the roll, one cycle every `rate` seconds, `phase` the fraction of the current cycle. The ramp is `trunc(ramp / rate + 0.99)` cycles: cycle `n` below it has envelope `n / ramp`, a cycle after `total - ramp` (total = `trunc(time / rate)`) has `(total - n) / ramp`. Zero after `time`. |
| `ENISSteering` (deg, weight) | `SetSteering(deg / 360, weight)` | Front-wheel angle clamped to the car's maximum, weight to 0..1. Not in the prologue. |
| `ENISNitro` (on) | engine `SetNitro(on)` | The car's nitrous counts as engaged (its flames and sound follow). |
| `ENISNeutralRev` (on, throttle, speed) | engine `SetNeutralRev` | The RPM target is `idle + (red line - idle) * throttle`, reached at `speed` per second of the difference (at once when `speed` is 0). |
| `ENISLights` (cop, head) | vehicle glare on/off | Police lights (`0x7000` glare bits) on if `cop`, else off; head lights (bits `7`) on if `head`, else off. Every event sets both. |
| `ENISCarDamageReset` | damage `ResetDamage` | The car is repaired. |
| `ENISDetach` | vehicle `SetAnimating(false)` (destructor) | The car leaves its channel. Its `fSpeed` field is not read. |
| `ENISReattach` (repair) | vehicle `SetAnimating(true)`, then `ResetDamage` if `repair` (destructor) | The car follows its channel again. |

The pitch, roll and shake pulses advance only while the car follows its channel (they run inside the per-frame
position update). **[pc + decomp]**

### 2.2 World events

| Event | Effect |
|---|---|
| `ENISMotionBlur` (on) | Sets the global motion blur flag to `on != 0`. The renderer's motion blur amount fades to 0 while it is off. The flag is set back to on when the scene is released and when a camera track reaches its end. **[pc]** |
| `ENISAeroDynamics` (factor) | Sets the global NIS aerodynamics factor (the callback writes it). While it is above 0, every NIS car's suspension adds downforce with it. It is not reset when the scene ends. **[pc + decomp]** |
| `ENISRain` (intensity) | Turns the rain override on and sets its intensity. At the release the override is cleared and the earlier rain flag restored. The only prologue use sends -1 (`IntroNis10`); what -1 does in the rain code was not read. **[pc]** |
| `ENISRoadNoise` (off) | Sets the global the car road-noise sound reads. Reset at the release. **[pc]** |
| `ENISStopEffects` | Kills every particle emitter group that is not marked persistent, then restarts the world's scenery effects. **[pc]** |
| `ENISOverlayMessage` (package, message) | When a front-end package is up, sends the message's hash to the package. **[pc]** |
| `ECameraPhotoFinish` | Only in a race with exactly two racers: a visual-treatment pulse, a sound pause, and the photo-finish sound state. **[pc]** Fired three times in `IntroNis04` **[live, 1 run]**; its look was not compared. |
| `EDDaySpeech` | Plays the speech event `D_Day`. **[pc]** |
| `ENISTimeOfDay`, `ENISFakeFar`, `ENISPlayEffect`, `ENISNukeSmack`, `ENISDetail` | Nothing (section 1). **[pc]** |

The release of a scene also hides the camera overlay (section 4). **[pc]**

## 3. What a detached car does

- A car following its channel takes only its **x/z position, x/z velocity and yaw rate** from the channel each frame; its
  height, pitch and roll come from its suspension on the ground, clamped to plus or minus half the constraint angle
  (60 degrees by default). **[pc + decomp]**
- **Remake approximation.** For channel-driven cars, sample road height at the four displayed treads. Use grade and bank
  across the footprint for the body and the remaining height difference for each wheel. When a clip lies below the
  street and misses collision, recheck at the nearest mapped deck height, at the actor's own coordinates. Rigs without separate wheel
  geometry use their chassis footprint. Released opening traffic takes its rotation and suspension pose from physics.
  The opening crash solver places the animated collision box with its physical tyre clearance before release.
  For upright released actors the display corrects tyre penetration at the road surface. Airborne wheels retain droop.
- A detached car is no longer placed. It keeps the velocity the channel gave it last and runs on its physics with the
  NIS input (throttle 0.5, or 1 with a burnout; brakes from the brake lock) and collides with what it meets. **[pc + decomp]**
- The channel goes on without it; at the reattach the car is placed on the channel again. In `IntroNisDD` the channel
  is often cut to another place during the detach (40 to 270 m away), while the car stays where its own motion takes it.
  **[live, 1 run]**
- Against a straight run at the channel's velocity at the detach, the original's detached cars stayed within 0 to 5 m for
  the detaches up to about 1 s, while the channel itself was 6 to 230 m away; over the long detaches of the pile-up
  (36.8 to 42.9 s) they braked or crashed to a stop. **[live, 1 run; private comparison script]**

## 4. Camera keys: the mover's bytes

Read in the PC camera mover. **[pc]**

- **Spaces** (bytes 4 and 5): car (0) uses the player car's matrix; world (1) the point as it is; **hybrid (2) adds the
  player car's position with no rotation** (a world-axis offset that follows the car); scene (3) the NIS scene's matrix.
  "The player car" is the human racer's car (the car-space keys follow it, not channel `car1` as such).
- **Byte 6** (constrain to cars): the eye is pushed out of cars. **Byte 7** (constrain to world): the eye is kept out of
  the world (a ray from the look point to the eye, and a minimum height above the ground). Both apply only in some
  camera contexts; in the NIS context both are applied always.
- **Byte 8** (ignore orientation): the car frame keeps a stored orientation and follows only the car's position.
- **Byte 9** (lag): in one camera context the car frame is smoothed: its position and forward axis move each frame a
  share `0.9 t + 0.1` of the way to the car's (with `t` the track's progress, 0 to 1), and the points are offset against
  the car's velocity. Which context is the NIS one was not established.
- **Overlay** (byte 0x0A): on a key whose overlay differs from the last key's, the old package is hidden and the new one
  shown, in camera contexts 0, 5 and 7. The values: 1 `FLASHERS.fng`, 2 `BLACK_FADE_OUT.fng`, 3 `BUSTED_OVERLAY.fng`,
  4 `BLACK_BACKGROUND.fng`, 0 none. The `IntroNisDD` key with 4 (27.26 s) showed no black frame in pictures taken every
  0.5 s **[pictures, 1 run]**, so what that package draws is not settled.

## 5. Clocks

- The activity clock adds the frame time while the scene plays (0 while paused). The scene's animation clock adds the
  **previous** frame's time, so it runs one frame behind. **[pc]**
- In the reference run the animation clock started 0.55 s after the sim clock's zero in `IntroNisDD`. **[live, 1 run]**

## 6. The special car lists

- The scene candidate table's list (count at 0x18, records of scene hash and code from 0xE9C, see
  [nis-scenes.md](nis-scenes.md) section 6) gives a scene a code. **[verified]** `IntroNisDD` has code 8 (`TRAFFIC_MIX7`),
  `EndingNis03` code 9 (`TRAFFIC_MIX8`); the race starts have none.
- The scene makes its police-channel cars from the code: one fixed list of eight vehicles, given to `cop1`..`cop8` in
  order. **No random choice is made.** `ONE_COP` and `ONE_SPORT_COP` make only `cop1`; `LOTS_OF_COPS` makes
  `copmidsize_nis` and seven `copmidsize_nis_ld`. **[pc]** The lists are in
  `crates/nfsmw-data/src/world/scene_cars.rs`; two vehicles of codes 4 to 6 have names not recovered (only their vault
  hashes are known).
- On the running game `IntroNisDD`'s cop channels were three `copmidsize_nis_ld`, then `trafstwag`, (`cop5`), `traf4dsedc`,
  `trafminivan` and `trafpizza`, as the list says. **[live, 1 run]**

## 7. What the rewrite does

- Car events act on the car of their channel (`crates/nfsmw/src/scenes/world/nis/car_events.rs`): pitch, roll and the
  shake tilt the drawn body; the brake lock holds the wheels still; the lights set the car's light bar and head lights; the
  repairs repair the car. The events of a channel with no car are dropped.
- A detached car keeps following its channel until the channel jumps more than 5 m in a frame (a cut); from there it
  coasts in a straight line at the channel's last velocity, slowed at 8 m/s² while a brake is locked, on the ground under
  it, until the reattach (`coast.rs`). The world here does not step its cars during a scene, so collisions, the
  suspension and the NIS input are not modelled **[ours]**. Against the original's detached cars of `IntroNisDD` (812
  samples, 1 run) this rule drew the car a median 8.7 m from the original's, following the channel throughout 13.1 m,
  coasting from the detach 44.6 m.
- `ENISMotionBlur` 0 holds the radial blur off until the scene ends; `ENISStopEffects` ends every object effect at once.
- The scene's police channels take the vehicles of the scene's list, made for the scene once their model has loaded;
  without a list, or when a model fails, the earlier police-car rule holds.
- Not acted on, and logged: `ENISNitro` and `ENISNeutralRev` (no NIS car flames or revs), `ENISAeroDynamics`,
  `ENISRain`, `ENISRoadNoise`, `ECameraPhotoFinish`, `EDDaySpeech`, the camera key bytes 6 to 9, hybrid space and the
  overlay values 2 and 4.
- `ENISPlayEffect` still plays its named effect, as a stand-in for the crash effects the original's detached cars make
  by colliding **[ours]**; the original does nothing with the event.

### Opening crash cars

In IntroNisDD, the scene-spawned traffic and patrol vehicles become physical actors on Detach. Carry the
motion from their last animated poses into suspension, ground and car contacts. Reattach restores channel
placement, including across a shot cut. The race grid and the player's handoff keep their own ownership.
During an in-game scene, advance placed particle emitters and requested object effects with the scene clock.
A paused scene or a frame held for capture contributes no particle time.
