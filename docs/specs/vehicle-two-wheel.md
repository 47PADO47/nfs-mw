# Two-wheel vehicle dynamics (`blackbox-vehicle::bike`)

The model of a motorcycle: a rigid body on a fork and a swingarm, two round tyres, an engine with an automatic gearbox,
and a rider's balance. It is a new model, written from published vehicle dynamics and from the parameter sets a bike
game ships as data. Nothing in it comes from decompiled code ([licensing.md](../licensing.md)); the code is
`libs/blackbox-vehicle/src/bike/`.

## Sources

| What | Source | Licence / status |
|---|---|---|
| Tyre forces | The Magic Formula 94 of Pacejka, *Tyre and Vehicle Dynamics*, appendix 3 (lateral `a0..a13`, longitudinal `b0..b10`, aligning `c0..c17`; loads in kN, angles in degrees, slip in percent) | Published formulas |
| Inverted-pendulum balance of a bike | Standard single-track roll model (Whipple and Carvallo, Sharp; Meijaard et al. 2007) | Published equations |
| Which parameters exist, and their names | The RIDE 6 SDK dump (Dumper-7, from the user's own install: names of classes and fields, no code) and the values of its physics data assets | Facts about the user's own data; none is stored in the repository |

## The body

A `blackbox_vehicle::rigid_body::RigidBody` carries the bike (frame, engine, wheels and rider together). Physics space is
x right, y up, z forward; the body origin is on the ground midway between the contact patches at rest. The inertia is the
box inertia of the body's half extents scaled per axis (`tensor_scale`: about x pitch, about y yaw, about z roll), the
same convention the car uses.

## The front end and the rear end

- **Steering head**: the steering axis tilts back by the rake. It meets the ground one `trail` ahead of the contact patch,
  which puts the hub at the perpendicular offset `R sin(rake) - trail cos(rake)` from the axis. Steering turns the wheel
  about that axis, so the hub moves sideways and down as the bar turns (`geometry.rs`).
- **Fork**: compression moves the hub along the steering axis. **Swingarm**: compression moves the hub up the body's y
  axis (the arc is ignored).
- **Suspension** (`suspension.rs`): a spring (rate at the shock, divided by the square of the leverage ratio at the wheel),
  a two-slope damper (slow slope up to a threshold speed, fast slope above, separately for bump and rebound), a top-out
  spring at full extension and a bump stop past the travel. The wheel assembly has no mass in the ground solve: the
  compression is whatever puts the tyre on the ground (`wheel.rs`, Newton on the gap), the weight of the wheel is added to
  the tyre load. The damper is applied implicitly against the mass the contact moves, so a stiff damper cannot overshoot.

## The tyre

- **Contact**: the tyre is a torus, a ring of radius `R - r` in the wheel plane with a tube of radius `r`, where
  `r = toroid_factor * width / 2`. The lowest point of the ring plus one tube radius down the ground normal is the contact
  patch, so a leaned bike stands on the shoulder of the tread and the patch moves sideways with the lean.
- **Forces**: `Fy = -MF94_lateral(Fz, alpha, -gamma)` and `Fx = MF94_longitudinal(Fz, kappa)` with
  `kappa = (w R - v_long) / max(|v_long|, 1)` and `alpha = atan2(v_lat, max(|v_long|, 1))`. The pair is held inside the
  ellipse of their peaks (`D_x`, `D_y`) and the lateral force inside `lateral_grip_limit * Fz`. The camber is negated so
  that a positive `a111` pushes a bike towards its lean.
- **Spin**: `I dw/dt = T_drive - T_brake - R Fx(w)` is solved for the new spin by bisection every step. An explicit step
  flips the force each step at a crawl.
- **Rolling resistance**: `Fz (c0 + c1 v + c2 v^2)`.

## The engine and the box

`T = peak * curve(rpm) * throttle - friction * rpm/max * (1 - throttle)`, cut at the rev limit; the wheel gets
`T * gear * final * efficiency`. The clutch slips below idle: the engine runs at a launch speed that follows the throttle.
Gears change automatically (up above a share of the rev limit under throttle, down below another), with a short cut of
the drive. This is the part of the model that is a choice, not a fact of the game: the data gives the gears and the
limiter, not the shift logic.

## The rider's balance (`controller.rs`)

The roll of a bike at speed `v` is an inverted pendulum whose support is moved by the steering:
`I phi'' = m g h sin(phi) - m h (v^2 / b) delta`. The controller asks for a critically damped approach to the target
lean, `phi'' = -kp (phi - phi_t) - kd phi'`, and solves for the handlebar angle `delta`. The result is the real riding
behaviour: to lean right it steers left first, then steers into the turn by `b g sin(phi) / v^2`. The target lean is the
stick times the lesser of the steepest lean and `atan(lateral grip)`. Below 2 m/s the handlebar follows the stick; below
2.5 m/s a torque stands in for the rider's feet.

## What is data and what is a choice

| Quantity | Origin | Confidence |
|---|---|---|
| Mass, centre of gravity height and balance, rake, trail, wheel mass and inertia | Data | Certain |
| Spring rate, travel, damper slopes and thresholds, leverage curve | Data | Certain; the leverage ratio is read as wheel travel per shock travel |
| Tyre coefficients `a`, `b`, `c`, rolling resistance, `lateral_grip_limit` | Data | Certain (the numbering is that of the published formula) |
| Torque curve, peak torque, rev limits, gear ratios, final ratio | Data | Certain |
| `brake force` as the force at the tyre patches of both brakes | Data | A reading of an unlabelled number |
| `friction_torque` as the closed-throttle drag at the rev limit | Data | A reading of an unlabelled number |
| `tensor_scale` multiplying the box inertia | Data | A reading: the box the scale applies to is not in the data |
| Friction ellipse for combined slip | Choice | The data's solver is not known |
| Slip regularisation speed, shift logic, launch speed | Choice | Numerical or behavioural |
| Controller gains | Choice | The game's controller (`LeanTimeScale`, `HandlebarK`, rider help) is not known |

## Not modelled yet

Gyroscopic torque of the wheels, the chain's effect on the swingarm, the swingarm's arc, tyre temperature and wear,
the aligning moment (the handlebar has no feel), a rider's mass moving on the bike, wheelies and stoppies as controlled
manoeuvres, ABS and traction control, falling and sliding on the ground, the damper's effect on the contact normal when
the tyre leaves the ground.

## Verified by tests

Stands on both wheels with the static load split by the weight balance; accelerates in a line without wobble; brakes and
moves weight forwards; countersteers before a lean and then holds it; leans the same angle each way; is held up at a
walking pace; the tyre formula's stiffness and symmetry; the implicit wheel solve at a crawl.
