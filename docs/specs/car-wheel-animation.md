# Drawn wheel steering and roll

How the original draws a car's front-wheel steering angle and each wheel's roll. Both are presentation values: they
are kept apart from the physical wheel angle and wheel speed that the tire model uses, and they do not affect handling.

- **Sources read:** the original PC executable (read for behaviour only), with the CC0
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reconstruction as a map of the car renderer's update order.
  No code copied.
- **Data inputs:** `ecar` attributes `SteerSpeed` (degrees per second), `MaxTireSteer` (degrees) and `WheelSpokeCount`
  ([../formats/cardata.md](../formats/cardata.md)).

## 1. Why it matters

Computer-driven cars apply no filtering to their steering input: the pursuit and racer controllers write a new steering
value every simulation step, and their drive targets move in steps at the think rate, so the physical wheel angle can
change sharply from one step to the next ([ai-driver-control.md](ai-driver-control.md)). The original keeps their wheels
visually steady in the renderer, not in the controller. **The steering input must not be filtered** to get this look: that
would change how the cars drive.

## 2. Drawn steering

Each update, for each front wheel:

```
step    = SteerSpeed * dT            (degrees, converted to radians)
drawn   = move_towards(drawn, physical_angle, step)
drawn   = clamp(drawn, -MaxTireSteer, +MaxTireSteer)
```

`physical_angle` is that wheel's current suspension steering angle (left and right separately, Ackermann included). The
tire, rim and brake are posed from `drawn`. When the car is not visible, or is farther than 100 m (times the renderer's
distance modifier, 1.0 by default) from the camera, both drawn angles are set to zero at once and the update stops there.
It applies to every car, the player's included. [PC binary]

A car whose record lacks one of the two attributes has no limit for that term **[ours]**.

**Installed values [verified].** Every cop car with an `ecar` record (`copmidsize`, `copsport`, `copsporthench`, `copsuv`,
`copsuvl`, `copghost`, `copgto`, `copgtoghost`) and the BMW M3 GTR read `SteerSpeed` 180 degrees per second and
`MaxTireSteer` 90 degrees. The clamp therefore lies beyond the physical range of 45 to 60 degrees and only the slew
shapes the drawn angle: a full-lock flip is drawn over about a third of a second.

## 3. Drawn roll

Each update, for each wheel:

```
delta = clamp(wheel_speed / render_radius * dT, -max_delta, +max_delta)
roll  = wrap(roll + delta)           (kept within one turn either way)
```

`max_delta` is 17 degrees. For a wheel with a non-zero spoke count *n*, it is `180 / |n| - 1` degrees, so the spoke
pattern never moves more than half a spoke pitch per update and does not appear to turn backwards. `wheel_speed` is the
wheel's angular speed times its physical radius; the renderer divides by its own tire radius. [PC binary]

Suspension travel is drawn directly from the suspension state, without filtering. [reference]

## 4. Rewrite notes

- The original updates both values once per rendered frame. The rewrite updates them once per fixed 60 Hz simulation step
  and interpolates between steps, which matches the original at 60 frames per second and keeps the values deterministic.
  Above 60 frames per second a per-step cap is not the same as a per-frame cap; this is a deliberate rewrite choice.
- The rewrite's drawn and physical tire radii are the same, so the roll advance is the wheel's angular speed times the step.
- Interpolating the roll between steps takes the short way round the wrap point: a step never advances more than half a
  turn, so the short way is the direction of travel.
- The distance gate uses the distance the cop's damage gate already measures from the nearest view each step.
- The drawn values are not part of the simulation state hash or of any network snapshot.

## Open questions

- The renderer's distance modifier under the PC graphics settings, and whether the global override that skips the gate is
  set outside cinematic and photo modes, were not read.
