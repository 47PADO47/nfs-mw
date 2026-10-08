# blackbox-vehicle

Deterministic, fixed-step vehicle physics for EA Black Box style cars: one rigid body on four spring and
damper corners, a slip-based tire model, engine, clutch, automatic gearbox, differentials, nitrous and
forced induction, brakes, speed-sensitive steering with Ackermann geometry, aerodynamics, and driver-input
shaping.

- **Pure.** `f32` only, no I/O, no allocation in the step, no game-install access, no dependency on the
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
| `rigid_body` | `RigidBody`: semi-implicit Euler, rotation about the centre of gravity, box inertia, quadratic drag, sleep, `react_plane` (impulse with Coulomb friction) |
| `engine` | `EngineSpec` (torque curve clamped at the red line, engine braking, inertia), `Clutch` |
| `drivetrain` | `TransmissionSpec`, `ShiftPoints` and the automatic box, `calc_split` (differential), `split_drive_torque`, `Powertrain` (clutch, torque loop, limiter, speedometer) |
| `induction`, `nos` | turbo or supercharger spool and boost; nitrous tank, burn and recharge |
| `brakes` | `BrakeSpec` in ft*lb and the per-wheel brake and handbrake commands |
| `tires` | `Tire` state, `update_loaded` and `update_free`, the lateral force table, per-step grip and traction scales |
| `suspension` | `ChassisSpec`, wheel `Geometry`, centre of gravity, the ground probe, compression, spring, damper and anti-roll force |
| `steering` | speed-sensitive angle and rate, counter-steer, post-collision limit, `ackermann` |
| `aero` | drag (doubled off the throttle) and linear downforce |
| `input` | `InputState` to `Controls`: dead zone, automatic reverse, idle auto-brake, handbrake priority, shift requests |
| `ground` | the `Ground` trait, `GroundHit`, `SurfaceGrip`, and `FlatGround`, `SlopedGround`, `NoGround` |
| `vehicle` | `Vehicle`, `VehicleSpec`, `Tunings`, `WheelState`; `VehicleSpec::example()` is a plausible rear-drive saloon for tests, not any game car |

Each file stays under 500 lines; modules with many tests keep them in a `tests` folder or file.

## The ground

```rust
pub trait Ground {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit>;
}
pub struct GroundHit { pub distance: f32, pub normal: Vec3, pub surface: SurfaceGrip }
pub struct SurfaceGrip { pub lateral: f32, pub drive: f32, pub rolling: f32 }
```

`hit` casts a ray from `origin` along the unit vector `dir` for at most `max_distance` metres and returns
the nearest ground surface. The vehicle only casts straight down: from 0.5 m above each wheel contact point
(so a wheel pressed into the road still finds it), and from 0.3 m above each corner of the body box. The
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
12. **Body against the ground** uses the eight corners of the box cast from 0.3 m above, a single impulse
    pass with the body's own friction (no per-surface multiplier, no 16 sub-steps, no AI skip rule).
13. **Sleep:** a car with any wheel on the ground never sleeps (as in the spec); a car resting on its roof
    can. A sleeping car wakes on throttle or handbrake.
14. **Gear ratios** are used as magnitudes; shift points are not computed for more than 10 gear entries.

## Not implemented

Walls, props and car-versus-car response (the caller finds the contact and uses `RigidBody::react_plane`
or changes velocities, then calls `Vehicle::notify_collision`), invulnerability and reset-to-road, burnout
and fishtail, drift state and its yaw damping, the axle traction control, wall steer, the airborne
stabilisers, speed-break, perfect launch and drag shift quality, engine heat and sabotage, blown tires,
catch-up cheats, forced stops, the understeer and oversteer read-outs, and the traffic and AI chassis
variants. The yaw-control boost assumes the stability setting is on.

## What needs calibration

Nothing here is checked against the running game; the numbers come from the specs, which are [decomp] only.

- **Tire force scale.** The lateral table is the console build's; the PC build has an extra multiplier
  (spec open question 2). Calibrate with a skidpad run. `StepScales` and the 1.5 ellipse stretch (kept in the
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
