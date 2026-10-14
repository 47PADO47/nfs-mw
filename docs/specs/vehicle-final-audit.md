# Driving audit corrections

Sources: locally inspected original PC v1.3 routines and independent numeric
execution of wheel feedback; the behavioral references in
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (`Chassis.cpp` and
`SuspensionRacer.cpp`). No decompiled source is included. These findings supplement
the existing tire, chassis, drivetrain and Speedbreaker specifications.

## Confirmed behavior

- Reverse gear uses a maximum tire slip of **71 m/s**, independently of speed.
  Forward gears retain `0.5 + Ramp(speed, 10, 71)`.
- Cached peak torque uses raw torque samples through MAX_RPM, including those
  above redline. Only the returned peak RPM is clamped. A 50 RPM interpolated
  scan through redline can miss the peak and change converter/clutch behavior.
- Reported reference wheel speed substitutes road speed divided by radius when
  **compression is positive** and traction is at least one. Spring force can be
  zero on an extending compressed wheel; force is not the compression gate.
  Brake lock still reports zero. The regularized profile retains its force gate.
- A compressed brake-locked wheel still receives traction-control/burnout grip
  multiplication, even when its drive-torque addition is skipped. Wheel angular
  velocity crossing the remembered sign is clamped at zero without a dead band;
  the dead band applies only when remembering the new sign.
- Speedbreaker tire steering drag blends from 0.15 to 1.0 with breaker strength.
  Its extra downward acceleration acts opposite the **body up vector**, including
  on banks and during roll, rather than along world gravity.
- The chassis angular-velocity snapshot precedes wall steering. Wall steering
  updates the body, but tire point velocities and drift processing in that tick
  retain the snapshot. Landing updates both snapshot and body; jump/sleep gates
  then consume that updated snapshot. Lateral sleep restores snapshot angular
  velocity with yaw scaled, while full sleep damps current body velocity.
- Speedbreaker object contacts override moment mass to twice real body mass after
  material scaling, multiplying inertia by override/old moment mass. Car and
  movable-prop contacts apply it unless the moment is immobile. Integration mass
  and inertia are unchanged.
- Free-roam ground rays exclude nonzero scenery groups, including floor strips.
  A surface exclusion flag alone does not exclude every grouped floor instance.
- Drift recharge follows the continuous updated drift strength each physics tick,
  while breaker is inactive, physical chassis speed is strictly above
  **15.646050453186035 m/s** and absolute body slip strictly above
  **0.5235899686813354 rad**. Gain is `simulation_delta * 0.5 * strength`,
  capped at one. There is no additional grounded-wheel requirement. Body slip is
  velocity relative to the car, not yaw rate or world heading.

## Comparison limits

Matching isolated routines and installed data does not establish matching full
trajectories. Remaining race/drag context producers, dynamic collision geometry,
wall contact geometry, chassis response and live clock integration need separate
original-game comparisons. The existing single-car calibration has nonzero rear
tire/load errors and diverging long trajectories; it is partial validation.
The original clock uses 60 Hz real-time scheduling with variable simulation
delta, while the remake uses fixed simulation-time ticks after a frame integral.
Ordinary impacts currently
produce extra tire damage through a port-specific severity rule. These remain
behavior differences, alongside the absent race/drag and destruction producers.
