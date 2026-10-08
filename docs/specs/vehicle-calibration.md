# Vehicle model checked against the running original

How the tire, load and steering model of [vehicle-suspension-tires.md](vehicle-suspension-tires.md) and
[vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md) compares with the PC v1.3 game.

A 230 s capture of the PC v1.3 game (Cheat Engine, 64 samples per second: per-tire load, lateral and
longitudinal force, slip angle, `grip_boost`, `traction_boost`, road speed, drive and brake torque and the front
wheel angles) was taken with a Porsche 911 GT2 in free roam. The object layouts of the decompilation
(`SuspensionRacer::Tire` at 0x100 specs, 0x10C axle, 0x128 grip boost; `Steering` at 0x114) hold for the PC
build **[install]**. Findings:

- `grip_boost` was 1.24 and `traction_boost` 1.20 (1.23 rear) at 24 m/s: the §3.7 tables and factors. `drag_reduction`
  0.15, `max_slip` 0.5 at low speed, `YawControl` 1.0.
- The tires attribute block of the car equals the data we read (`STATIC_GRIP` 2.075/2.1, `DYNAMIC_GRIP` 1.85,
  rim 18, 315/30).
- The steering speed argument (`state + 0x48`) is in m/s, like the table domain 0..160 of §6.1.
- Rest load 14.3 kN (ours 14.1 kN for 1440 kg); total load at 36 m/s about 32.7 kN (ours 32.4 kN).
- **Replay.** Driving our car with the original's front wheel angles and throttle (`Vehicle::forced_wheel_angles`,
  test `replay_capture` in `crates/nfsmw-data/tests/real_install/compare.rs`) reproduces the original's tire
  forces in hard corners at 20 to 33 m/s: front axle lateral force within 1 to 5 % at most sampled instants
  (for example 30.4 against 30.8 kN at 61.5 s and 28.7 against 28.3 kN at 63 s), rear axle within about 5 to 25 %
  (worst sampled instants: 29.6 against 38.5 kN, and 31.5 against 28.5 kN on the front), total load within 5 to
  10 %. The two cars drift apart over a long replay because the speeds differ by a few m/s (ours leaves corners
  faster, so later instants differ by more). The coast-down from 38.8 m/s matches to within 0.2 m/s. The step
  response of the wheel angle (about 0.15 s to 40 degrees) matches the capture.

So the tire, load and steering model reproduces the original on flat ground; what remains is the world (surface
faces, collision), input shaping and the camera.

