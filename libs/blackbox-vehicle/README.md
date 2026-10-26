# blackbox-vehicle

Deterministic, fixed-step vehicle physics for EA Black Box style cars: one rigid body on four spring and
damper corners, a slip-based tire model, engine, clutch, automatic gearbox, differentials, nitrous and
forced induction, brakes, speed-sensitive steering with Ackermann geometry, aerodynamics, and driver-input
shaping.

- **Pure.** `f32` state, no I/O, no allocation in the step, no game-install access, no dependency on the
  collision library. The world is a `Ground` trait the caller implements. The same inputs always give the
  same state, bit for bit, on one machine.
- **Plain parameters.** A `VehicleSpec` is a tree of plain structs in the units of the attribute data
  (inches, ft*lb, lb/in, mph, rpm). The caller fills them from AttribSys; this library knows no game, no
  file format and no car.
- **Written from the specs, not from the decompilation.** `docs/specs/vehicle-*.md` are the only source;
  see [`docs/provenance/`](../../docs/provenance) and [`docs/licensing.md`](../../docs/licensing.md).

```rust
use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};

let ground = FlatGround::new(0.0);
let mut car = Vehicle::new(VehicleSpec::example());
car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0);
let input = InputState { throttle: 1.0, ..Default::default() };
for _ in 0..600 {
    car.step(FIXED_STEP, &input, &ground);
}
println!("{:.1} m/s in gear {}", car.forward_speed(), car.gear());
```

## Conventions

- **Space:** x right, y up, z forward, right-handed, metres, kilograms, seconds, radians. A rotation maps
  body to world (`glam::Quat`, columns of the matrix are the world right, up and forward axes). Car-solid
  space (+x forward, +y left, +z up) is a render concern: `phys = (-asm.y, asm.z, asm.x)`.
- **Wheel order:** 0 front left, 1 front right, 2 rear **left**, 3 rear right (the arms of wheels 2 and 3 are
  at -x and +x). Car assembly lists the rear as right then left; the caller maps. Open question 1 of
  `docs/specs/vehicle-suspension-tires.md`.
- **Gear ids:** 0 reverse, 1 neutral, 2 first, ... `GEAR_RATIO` and `GEAR_EFFICIENCY` are indexed by gear id.
  Ratios are used as magnitudes; reverse flips the sign itself.
- **Step:** `Vehicle::step(dt, &InputState, &dyn Ground)`, `dt = FIXED_STEP = 1/60`. The tuning (springs,
  damper blow-off, tire relaxation, clutch gains) assumes 60 Hz; other steps are accepted but untested.
  A zero, negative or NaN `dt` is ignored. Forces computed in a step move the body in the next one, so
  positions lag forces by one step.
- **Angles inside the tire code** are turns (1.0 = 360 degrees) as in the spec; `WheelState` reports radians.

## Modules

| Module | What it holds |
|---|---|
| `math` | `ramp`, `lerp`, evenly spaced `table`, piecewise `graph`, unit factors (`FT_LB_TO_NM`, `MPH_TO_MS`, ...) |
| `rigid_body` | `RigidBody`: integration, box inertia, drag, sleep, iterative plane/object response and ordered ground queries |
| `engine` | `EngineSpec` (torque curve clamped at the red line, engine braking, inertia), `Clutch` |
| `drivetrain` | `TransmissionSpec`, `ShiftPoints` and the automatic box, `calc_split` (differential), `split_drive_torque`, `Powertrain` (clutch, torque loop, limiter, speedometer) |
| `induction`, `nos` | turbo or supercharger spool and boost; nitrous tank, burn and recharge |
| `brakes` | `BrakeSpec` in ft*lb and the per-wheel brake and handbrake commands |
| `tires` | `Tire` state, `update_loaded` and `update_free`, the lateral force table, per-step grip and traction scales |
| `suspension` | `ChassisSpec`, wheel `Geometry`, centre of gravity, the ground probe, compression, spring, damper and anti-roll force |
| `steering` | speed-sensitive angle and rate, counter-steer, post-collision limit, `ackermann` |
| `aero` | drag (doubled off the throttle) and linear downforce |
| `input` | `InputState` to `Controls`: dead zone, automatic reverse, idle auto-brake, handbrake priority, shift requests (`ControlConfig::automatic = false` is manual shifting: nothing shifts by itself, see `docs/specs/vehicle-manual-shifting.md`) |
| `ground` | the `Ground` trait, `GroundHit`, `SurfaceGrip`, and `FlatGround`, `SlopedGround`, `NoGround` |
| `vehicle` | `Vehicle`, `VehicleSpec`, `Tunings`, `WheelState`; `VehicleSpec::example()` is a plausible rear-drive saloon for tests, not any game car |
| `stats` | `CarStats`: a car in absolute SI units (mass, torque curve, gear ratios, tyre grip, brake torque, nitrous ...) and `VehicleSpec::from_stats`, which builds the whole spec from it with documented generic values for what the stats do not give; `upgrade` (shop levels as shares of the car's own numbers), `solve_drag` (the air drag that gives a top speed) and `handling_index` |

`RollingChassisConfig` optionally selects direct brake/axle controls, fixed steering,
simple drag, a force-arm multiplier and an optional fixed centre of gravity.
`RollingTireResponse` supplies linear or caller-configured curve grip independently
of the racing slip solver. Its integer-turn angle lookup generates mathematical
octant samples once with `f64`; normal simulation state and forces remain `f32`.
Game-specific selection and coefficients belong to callers; see the
[rolling chassis contract](../../docs/specs/traffic-suspension-runtime.md).

`Vehicle::set_retirement_sleep_delay(Some(seconds))` optionally allows supported-body
sleep after a caller-selected duration with more than half the wheels supported.
The default is `None`. Call `finish_retirement_contacts` after external contacts;
`mark_object_contact` records an admitted overlap without waking the body, while
`notify_object_collision` also retains ordinary collision notification. The body's
authored speed/angular/contact thresholds still decide sleep. Game-specific destroyed
state and delay selection stay in the adapter; see the [contract](../../docs/specs/wreck-motion.md)
and [provenance](../../docs/provenance/wreck-motion.md).

Each file stays under 500 lines; modules with many tests keep them in a `tests` folder or file.

`rigid_body::sphere_contact` and `sphere_box_contact` retain spherical geometry;
the latter takes relative primitive travel for face and corner sweeps. Measured
behavior and the coincident-centre exception are in `docs/specs/sphere-contacts.md`.

## The ground

```rust
pub trait Ground {
    fn body_friction(&self, surface: u32) -> f32 { 1.0 }
    fn world_friction(&self, surface: u32) -> f32 { 1.0 }
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit>;
}
pub struct GroundHit { pub distance: f32, pub normal: Vec3, pub surface: SurfaceGrip }
pub struct SurfaceGrip { pub lateral: f32, pub drive: f32, pub rolling: f32 }
```

`hit` casts a ray from `origin` along the unit vector `dir` for at most `max_distance` metres and returns
the nearest ground surface. The vehicle only casts straight down: from 0.5 m above each wheel contact point
(so a wheel pressed into the road still finds it). Body support uses motion-dependent tolerance. The
surface is `simsurface`'s `LATERAL_GRIP`, `DRIVE_GRIP` and `ROLLING_RESISTANCE`. `max_distance` is the one
addition to the interface sketched in the task (`hit(point, dir)`): it lets a ray cast over a collision
world bound its work, as `blackbox-collision`'s segment query does.

## One step, in order

1. `RigidBody::begin_frame`: integrate the previous step's force and torque, add gravity.
2. Shape the input (`input::shape`) and act on gear requests.
3. Put the centre of gravity where the weight bias and ride height say; update the steering and Ackermann.
4. Tires: step scales, then `Powertrain::tick` (nitrous, limiter, induction, shifting, clutch, torque
   loop) reads last step's wheel speeds and gives the drive torque, which `split_drive_torque` shares out.
5. Brake commands, the rear stability boost and the handling tuning; aerodynamic forces.
6. Per wheel: ground probe, compression, spring, damper and sway force, tire update, force at the contact
   patch. The summed force and torque go to the body once; the body is lifted if a spring is past its travel.
7. Landing and creep damping; drag; the body box against the ground; sleep.

## Mapping from the data

`VehicleSpec` fields follow the attribute classes: `engine`, `transmission`, `induction`, `nos`, `brakes`,
`tires`, `chassis` (split into `ChassisSpec` and `AeroSpec`), `rigidbodyspecs` (`RigidBodySpec`) and
`pvehicle` (`mass`, `tensor_scale`). `dimension` is the half extent box of the car (the root collision
bound). Every field documents its unit. The player sliders are `Tunings`; the pedals are `InputState`.

## Deviations from the specs

Each is a deliberate choice where the spec was ambiguous, or where a literal reading was unstable at
60 Hz. All are recorded in `docs/provenance/vehicle-*.md`.

Items 1–7 describe the generic `TireIntegration::Regularized` default. The optional
`Reference` profile follows the original force branch, wheel histories, sign-dependent force bound,
reported wheel speeds, net-force sleep and asymmetric burnout instead. The game can select that
profile; see [tire profiles](../../docs/specs/vehicle-tire-profiles.md) and
[validation/provenance](../../docs/provenance/vehicle-driving-corrections.md).

1. **Tire branches are blended.** The spec picks the gripping or the sliding force by last step's
   `traction` being below 1. A tire at its limit then alternates between the two every step and loses half
   its force. Here the two are mixed by that traction, which settles on the friction limit.
2. **A gripping wheel sheds its slip.** Its spin acceleration follows the ground and removes the slip left
   over, so it tracks the extrapolated ground speed. Holding a constant offset (the literal reading) made the
   sliding branch push the wrong way under braking.
3. **Sliding friction is regularised** below 0.5 m/s of skid speed (it ramps up linearly instead of jumping
   to its full value in the direction of a tiny slip).
4. **The low-speed viscous brake is capped** so it cannot push a quarter of the car through zero speed in
   one step (the spec says it "cannot reverse the car" but gives no cap).
5. **The `|torque| / r` cap on sliding longitudinal force** applies only to wheels that are not
   brake-locked; otherwise a locked wheel would give no braking force.
6. **A wheel that was in the air** forgets its road speed and spin acceleration when it touches down.
7. **Creep damping** (the "all asleep" case of `DoSleep`) damps velocities but does not scale the pending
   force and torque, which froze a freshly landed car off its resting ride height.
8. **The speed limiter** tapers the throttle from the unclamped speedometer; the spec clamps the speedometer
   to the limiter, which would make the taper unreachable.
9. **Conflicting tables in the spec:** the steering range coefficient uses the `[1, 1.05, 1.1, 1.2, 1.3, 1.4]`
   table of the steering section (the input section lists `[1, 1, 1.1, 1.2, 1.25, 1.35]`), and the input
   remap uses the "medium" set of the input section (the steering section lists a milder one).
   `max_slip` uses 71 m/s as the top of its ramp in every gear.
10. **Hard-turn hold beats the post-collision limit.** In the spec's order the "held full lock keeps
    `last_max`" rule runs after the collision scaling and undoes it; kept as written.
11. **Pedal dead zone** defaults to 0.05 (the spec has no value; `ControlConfig::dead_zone`).
12. **Body against the ground** uses ordered box corners, motion-dependent queries, per-surface
    friction and 16 moment iterations. Game geometry can supply its enabled support clouds.
13. **Sleep:** a car with any wheel on the ground never sleeps (as in the spec); a car resting on its roof
    can. A sleeping car wakes on throttle or handbrake.
14. **Airborne downforce.** The original keeps 0.8 of the downforce in the air; with real aero coefficients
    that pulls a car down about twice as hard as gravity at jump speeds. `AeroSpec::airborne_scale` (default
    0.1) is an optional library tuning. The game adapter explicitly supplies the original 0.8.
15. **Clutch torque follows the original spring coupling** while re-engaging, without an artificial
    peak-engine-torque cap. Gear changes can transfer stored flywheel energy to the wheels.
16. **Gear ratios** are used as magnitudes; shift points are not computed for more than 10 gear entries.

## Tire marks and smoke

`WheelState` carries `slide_speed` (m/s the contact patch slides over the road, from wheel spin and sideways
motion), `skid` (0..1, tire-mark darkness; starts at 2 m/s and is full at 5) and `smoke` (0..1, starts at
3.5 m/s and is densest at 9). Both are 0 for a wheel in the air. A renderer lays marks along the patch
positions while `skid` is above a small threshold; drawing them is a later milestone. The ranges
(`SKID_RANGE`, `SMOKE_RANGE`) are guesses to tune by eye.

## Not implemented

World contacts and prop lifecycle belong to the caller. `react_moments` and `react_bodies` provide
iterated object response with separate per-body restitution and effective mass/inertia, friction,
fixed centres and break thresholds. The caller supplies geometry and gameplay overrides.
See [object contact verification](../../docs/provenance/pc-object-contact.md).

Wall steering and airborne stabilizers now have caller-supplied configuration and
original numeric vectors. Their world geometry and context producers need further comparison.
Still missing here: invulnerability and reset-to-road, perfect launch and drag shift quality, engine heat and sabotage,
catch-up cheats, forced stops, the understeer and oversteer read-outs, and complete
alternative AI powertrains/task scheduling. The yaw-control boost assumes the stability setting is on.

Ordinary racer drift friction/yaw damping and axle traction control follow the
published chassis specification. The reference profile includes asymmetric burnout/fishtail.
`ManeuverAssist` accepts coupled temporary gravity, drag, grip and steering values from the caller;
clock changes and meter state belong to the game. A supplied collision override
scales temporary object-contact mass and inertia without changing the real body.
Drag staging remains absent.
Steering history follows [the documented ordering](../../docs/specs/vehicle-steering-history.md).

## What needs calibration

The object-response kernel matches 13 synthetic measurements of the original PC machine code.
Full driving and collision scenarios still need comparison against the running game.

- **Tire force scale.** The PC lateral multiplier is covered by the current tire specification;
  complete live skidpad/trajectory equality remains unverified. `StepScales` and the 1.5 ellipse stretch (kept in the
  clipped case, so a driven wheel can exceed `torque / r`) are the other grip knobs.
- **Steering tables** are indexed by forward speed over 0..160 assumed to be m/s (question 6).
- **Brake torque units** (`BRAKES * 1.3558 * 4`, `EBRAKE * 1.3558 * 10`) look like tuning fudge (question 8).
- **`SHOCK_VALVING`, `SHOCK_BLOWOUT`, `FLYWHEEL_MASS`** units, and `ROLL_CENTER` as a height above the road
  in inches.
- **Wheel order** against car assembly (question 1).
- **The regularisations above** (0.5 m/s skid floor, quarter-mass brake cap, branch blend) are mine, not the
  game's; compare low-speed behaviour (creeping, stopping, parking on a slope) against the game.
- **Ride and stiffness at 60 Hz:** with very stiff springs (`k dt^2 / m` near 4) the corner goes unstable;
  the example car is far from that.

## Tests

`cargo test -p blackbox-vehicle`: unit tests per module, and whole-car tests on flat and sloped ground:
free fall, settling at the analytic static ride height, a drop, acceleration through every gear,
0-100 km/h, the launch, braking distance, reversing on the brake pedal, gentle and hard cornering,
mirrored turns, nitrous, the handbrake, a slope, a ledge, a flipped car, bit-exact determinism and cloning,
NaN and infinite inputs, and random input across six drivetrain layouts.

License: MIT OR Apache-2.0.

`Vehicle::set_coasting_tires` selects a caller-configured loaded-tire response
without powered slip or handbrake. It uses direct ordinary brakes and axle torque,
rolling wheel speed, an angle ramp and a brake-dependent combined-force limit.
The game selects this only for destroyed police; coefficients and limitations are
in [the contract](../../docs/specs/destroyed-cop-tires.md) and
[provenance](../../docs/provenance/destroyed-cop-tires.md). It is independent of
retirement sleep and does not add a forced stop or extra coast drag.
