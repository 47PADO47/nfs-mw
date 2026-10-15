# Original gameplay camera follow

## Evidence and scope

**[PC binary checked]** The matched PC Cubic mover, POV target producer,
speed-tweak sampler, direction producer and scalar cubic updater establish the
following contract. See the companion provenance record. This describes the
seven original views; custom Chase and Dynamic FOV retain their own controller.
It does not establish equivalence for unobserved original-game playback.

## Coordinate and state contract

Local camera coordinates are forward, left and up. Eye and target offsets use
the camera anchor position supplied by the native geometry/centre-of-gravity
producer, not an added front-bumper offset.
There is no extra half-length offset for positive `LAG`.

Retain separate cubic state for local eye, local target, world follow direction,
world up and field of view. Each scalar retains value `x`, derivative `v`,
desired value `y`, desired derivative `w`, normalized time, duration and state.
Desired derivatives are zero for ordinary follow. Changing the desired value
arms an update when it differs from the current value. A snap copies desired
value/derivative and clears the active state.

For an armed scalar, reset normalized time and build these Hermite coefficients:

- `a = v + w - 2*(y-x)`
- `b = 3*(y-x) - w - 2*v`
- `c = v`, `d = x`

Advance `t` by `dt/duration`, clamped to one. For duration at most 0.00001
seconds, use one immediately. Evaluate `x = ((a*t+b)*t+c)*t+d` and
`v = (2*b+3*a*t)*t+c`. The derivative is with respect to normalized time:
do not divide it by duration or treat it as metres per second. The camera's
rolling flag rebuilds the coefficients on every active update, preserving the
previous derivative. This is different from an exponential position filter.

## Frame ordering and mode transitions

At a camera update, sample the current view's speed tweaks, set durations,
advance the previous targets, construct the current camera basis, then publish
new direction/eye/target/up/FOV targets. The output uses the advanced state.
Thus continuously changing targets have a deliberate one-update delay.

Close through Pursuit retain their state when switching among those views.
Crossing to or from Bumper/Hood requests a snap after publishing new targets.
Bumper/Hood snap eye, target, FOV and up every update. A reset snaps all state.
Pause does not advance the mover, except its explicit first-update request.

## Speed and direction

Interpolate tweak endpoints by `clamp(speed/71,0,1)` with speed in m/s. These
endpoints are unrelated to the fullscreen/split-screen camera attribute pairs.

| Views | Eye seconds | Target seconds | FOV seconds | Direction seconds | Up seconds |
|---|---|---|---|---|---|
| Bumper | 0 to 0 | 0 to 0 | 1 to 0.5 | 0 to 0 | 0 to 0 |
| Hood | 0.7 to 0.5 | 0.7 to 0.5 | 1 to 0.5 | 0.4 to 0.2 | 0.7 to 0.5 |
| Close | 1.25 to 0.75 | 0.4 to 0.3 | 1 to 0.5 | 0.4 to 0.2 | 0.8 to 0.4 |
| Far | 1.25 to 0.75 | 0.4 to 0.3 | 1 to 0.5 | 0.4 to 0.2 | 0.8 to 0.4 |
| Super far, Drift, Pursuit | 1.5 to 0.5 | 0.5 to 0.2 | 1 to 0.5 | 0.5 to 0.1 | 1 to 0.5 |

Add the anchor's collision damping and drift values to direction duration.
For outside views, reflect a negative forward velocity component so reversing
does not turn the camera around. Normalize that velocity and blend it with body
forward using `clamp((speed-8)/5,0,1)`. Attenuate its vertical component by
`clamp(1-time_since_ground*0.5,0,1)*(1-body_forward.z^2)`.
Normalize the filtered direction for the outside camera's forward axis. Build
left as `world_up.cross(forward)` and up as `forward.cross(left)` without
normalizing either cross product. Their lengths shrink when looking uphill or
downhill, affecting both placement and acceleration response. Bumper and Hood
use the full body basis instead. When body up points
below the world horizon, their placement basis flips its lateral and up axes;
the separately supplied body-up orientation remains unchanged.

## Acceleration and targets

The camera anchor's acceleration producer uses the change in velocity magnitude
per camera second, rotated along the body's forward axis. It does not publish
the chassis's full physical acceleration vector. When elapsed time is zero or
anchor displacement per second is at least 300 m/s, it publishes zero instead.
This prevents teleport displacement from creating camera kicks.

Each world acceleration component is sampled against knots
`[-30,-15,0,15,30]` mapping to `[-30,-3,0,3,30]`, saturating at the ends.
Keep a five-update running average of these mapped vectors. Transform the
average into the current camera basis before applying view-specific scales and
clamps. Startup averages use only samples actually present.

Eye and target acceleration modifiers are component-wise scaled and clamped by
the independently sampled tweak rows. Up modifiers have their own scales and
clamps. These are three separate responses, not a single camera shake value.

The following tuples are forward/left/up, with arrows showing the low-speed to
high-speed endpoint; unqualified tuples remain constant. Interpolate each
component by the same speed fraction as the durations.

| View | Eye scale | Eye minimum | Eye maximum |
|---|---|---|---|
| Bumper | (0,0,0) | (0,0,0) | (0,0,0) |
| Hood | (-0.01,-0.02,0.1) to (0,0,0) | (-0.1,-0.1,-0.1) | (0.3,0.1,0.1) |
| Close | (-1,0,0) to (-0.75,0,0) | (-1.25,-1,-1) to (-1,-1,-1) | (1.5,1,1) to (1,1,1) |
| Far | (-1,0,0) to (-0.5,0,0) | (-1,-1,-1) | (2,1,1) |
| Super far, Drift, Pursuit | (-0.25,0,0) to (-0.05,0,0) | (-2.5,-1,-1) to (-0.5,-1,-1) | (2,1,1) to (0.5,1,1) |

| View | Target scale | Target minimum | Target maximum |
|---|---|---|---|
| Bumper | (0,0,0) | (0,0,0) | (0,0,0) |
| Hood | (-0.1,-0.02,0) to (0,0,0) | (-0.05,-0.1,0) | (0.2,0.1,0) |
| Close | (-0.5,0,0) to (-0.2,0,0) | (-2,0,-1) to (-0.5,0,-1) | (2,0,1) to (0.5,0,1) |
| Far | (-0.1,0,0) to (-0.05,0,0) | (-2,0,-1) to (-0.5,0,-1) | (2,0,1) to (0.5,0,1) |
| Super far, Drift, Pursuit | (-0.1,0,0) to (-0.02,0,0) | (-2,0,-1) to (-0.5,0,-1) | (2,0,1) to (0.5,0,1) |

Outside up response uses world horizontal acceleration with equal forward/left
scales: Close 0.004 to 0.002, Far 0.005 to 0.003, other outside views 0.006 to
0.002. Clamp horizontal components to [-1,1], keep vertical one and normalize.
Inside views use their body up directly, so their unused up-response state must
not change the rendered pose.

Read fullscreen `LAG`, lateral offset, `HEIGHT`, `ANGLE` and `FOV`; divide lag by
Speedbreaker zoom and multiply FOV by the same zoom. Quantize the angles through
the original unsigned 16-bit full-turn representation. Before eye construction,
scale lag by a five-point top-speed curve at speed fractions 0,0.25,0.5,0.75,1,
with values 1,1,1.02,1.125,1.2. Height's matching curve is constant one.

Nitrous adds 1638 full-turn angle units (approximately nine degrees) to the
already quantized FOV target for outside views when the car is moving. Inside
views and stationary vehicles receive no such boost. It enters the ordinary
FOV spline; activation must not snap directly to a different lens. `STIFFNESS`
is copied into the native POV record, but has no read in the checked Cubic
target/update path; no extra host smoothing coefficient is inferred from it.

The base local eye is `(lag,lateral,height)`. For outside views the local target
is `(0,0,height+tan(angle)*lag)`. For Bumper/Hood it is
`(lag+1,0,height-tan(angle))`. Add the separately clamped acceleration modifiers
to eye and target. Local `ANGLE` is therefore not applied by subtracting a host
pitch from vehicle heading.

When `tilting` is enabled, the up target starts with one plus the clamped scaled
world acceleration average; otherwise it starts at world up. Add the original
nearby-car influence only above 5 m/s, normalize and publish it. Outside views
use filtered up with its vertical component forced to one; Bumper/Hood use the
body up vector. Renderer orthogonalization remains a distinct final operation.

## Clock and high refresh rates

### Anchor damping and drift

Grounded camera drift is `clamp(abs(body_slip_radians)/(pi/2),0,1)` multiplied
by `clamp((absolute_speed-10)/10,0,1)`. It is zero while airborne. The physics
steering-assist drift strength is a different quantity and must not be reused.

The camera action retains three impact timers with a two-second normalization
duration. Each update first subtracts real camera elapsed seconds and clamps at
zero. A collision callback uses the length of its closing-velocity vector:

| Collision | Threshold m/s | Normalized strength above threshold | Timer |
|---|---|---|---|
| World | 5 | `min((speed-5)*0.025,1)` | Direction damping |
| Ground | 3 | `min((speed-3)*0.05,1)` | Ground impact |
| Object | 20 | `min((speed-20)*0.01,1)` | Object impact |

At or below the threshold, retain the existing timer. Otherwise raise the timer
to the maximum of its current value and two seconds times the strength. Divide
each retained timer by two for the anchor's normalized outputs. Do not use
impulse, damage, an audio severity, or a screen-shake amplitude as this input.

The matched PC main loop obtains wall-clock scheduler elapsed time, passes it
unchanged to the camera action/mover dispatcher, and shares the same accumulated
value between the two updates. Speedbreaker world-time scaling is not applied
to that dispatcher argument. The original caller supplies real camera elapsed
seconds. The spline's rolling
rebuild and five-sample average make direct per-display stepping rate dependent.
To preserve the reference 60 Hz trajectory at higher display rates, retain a
60 Hz camera clock, feed timestamp-aligned body inputs and interpolate its
presented output. Do not replace the native algorithm with a smoothing factor
scaled by display FPS. Paused time and teleport/mode snaps must not interpolate
through stale states. This host clock policy is an intentional adaptation.

## Remaining surrounding effects

The base follow equations do not alone reproduce collision-triggered offset
transients, gear-change pulses, camera noise, nearby-car and world clearance,
collision impulses, or cinematic/ICE transition admission. The separate
original-camera-transients and original-camera-noise specs describe implemented
transient/noise stages; clearance has its own evidence. Cinematic/ICE mover
admission and external camera impulse producers remain separate coverage gaps.
