# Police acceleration and the physical vehicle handoff

Sources read: the CC0-1.0 [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
`AI/Common/AIVehicle.cpp`, `Physics/Behaviors/EngineRacer.cpp` and
`Physics/Behaviors/SuspensionSimple.cpp`, for facts only. No source is copied.
Existing specifications: [driver speed](ai-driver-speed-skill.md),
[simulation modes](ai-simulation-lod.md), and
[NPC chassis state](npc-chassis-contact-state.md).

## Separate the requested speed from the achieved speed

An action supplies a direction and requested speed. The speed governor limits the
request using a performance estimate and previous physical speed samples. The
driver turns that request into pedals, steering and gear commands. The physical
engine, transmission, tire forces and road support determine actual acceleration.
Changing an action's acceleration multiplier does not directly multiply physical
engine torque. [decomp]

The ordinary cop has no racer catch-up cheat. The racer's engine torque cheat is
therefore not a justified way to repair police acceleration. [decomp]

## Distinct physical chassis

Installed ordinary police select the racer engine and simple suspension. The
simple suspension has its own tire response and direct axle torque distribution;
it is not merely the racer chassis with different steering range and centre of
gravity. Powered wheel slip, braking, grip and surface response participate in
the result. [decomp; installed mechanic selection described in simulation modes]

The current remake's simple chassis adapter covers centre of gravity, steering
range and landing damping. Its live wheel force path still uses the racer tire
solver. Destroyed police have a separate no-powered-slip coasting response. This
is a known model gap; this document does not infer its precise effect on pursuit
acceleration without a production-path measurement. [ours: source audit]

## Physical handoff diagnostic

A renderer-free installed test loads the stock police midsize vehicle and drives
it on an unlimited flat road. It records actual speed, requested speed, pedals,
gear, RPM and wheel support at one, three and five seconds in four modes:

- Full physical throttle: isolates the body, engine, gearbox and tires.
- Direct PID speed request: adds the production driver to the same physical car.
- Measured-curve governed PID request: adds the existing governor and installed
  acceleration multiplier at the ordinary cop task interval.
- Analytical-curve governed PID request: uses the production action estimate
  with the same governor, driver and physical body.

The same parameters and starting state are used in all modes. The test checks
finite forward movement, persistent support, and that the governor/driver path
does not stall far below both its request and available full-throttle speed.
These are integration checks, not fitted original-game acceleration thresholds.

A stock installed diagnostic passed before changing action estimation. At one,
three and five seconds, physical full throttle reached approximately
20.6/48.6/69.8 m/s; the direct PID request reached 20.5/48.4/69.6 m/s. The existing
measured-curve governor requested 18.7/41.8/55.5 m/s and achieved
15.2/39.4/53.7 m/s. This isolated car has ample physical acceleration; these
results do not justify a torque boost or prove the live simple tire model.
[verified installed test; flat-road configuration only]

## Runtime comparison limits

A measured original pursuit can establish that requests rise before physical
speed catches up. Aggregates across heat levels, different vehicle models, road
grades, maneuvers and simulation modes cannot define a stock midsize car's flat
acceleration curve. Initialization-entry fields may precede reset or the first
new action command. [verified observation; limited capture scope]

Exact requested/actual velocity equality during cooldown does not establish the
physics mode used by an active chasing car. Do not add a universal kinematic
speed handoff or an arbitrary torque boost from that observation.

## Analytical action performance estimate

Fresh inspection of the hashed PC 1.3 executable confirms that AI construction
uses an analytical estimate rather than running the physical car at full throttle.
The estimator and its inducted-torque helper were inspected separately. [verified]

Inputs are installed mass, engine torque samples and RPM range, induction data,
rear tire dimensions, forward gear ratios and efficiencies, final drive, and
chassis drag. The estimator uses the rear tire rolling radius. It averages the
fully inducted torque samples from peak-torque RPM up to the red line, excluding
sample positions at or above the red line. No nitrous or physical launch delay
participates in this average. [verified]

For each forward gear with total ratio `q`, efficiency `e`, rear radius `r`,
average inducted torque `T` in newton metres and red-line RPM `R`, form:

```text
speed = R * (2*pi/60) * r / q
drive_force = T * q * e / r
acceleration = (drive_force - drag * speed^2) / mass
```

The speed/acceleration points follow increasing forward-gear speeds. If a negative
acceleration follows a positive point, append a zero-acceleration point between
the two speeds. The shipped interpolation uses the complementary weight
`1 - previous_acceleration / (previous_acceleration - current_acceleration)`.
This is an observed arithmetic quirk, not the textbook zero-crossing formula;
preserve it for the action estimate. If no earlier positive point exists,
estimation fails. Top speed is the greatest accepted point speed.
Linearly sample the resulting curve at ten evenly spaced speeds from zero to top
speed; values below the first gear point use that first point's acceleration.
The implementation's existing grip calculation supplies the accompanying grip
values. [verified; grip reuse is an independent adapter choice]

The native estimator's arithmetic does not contain clutch settling, gear-change
delays, wheel-slip history or the driver's pedal controller. Replacing this
estimate with a physical acceleration trial can therefore reduce requested speed
before the driver has a chance to use the car's available force. [verified
algorithm; inferred integration effect]

This action estimate must remain distinct from physical measurement. It does not
change the engine, body velocity or the tire solver. Invalid or unsupported
parameter sets may retain the measured fallback as an explicit remake policy.
