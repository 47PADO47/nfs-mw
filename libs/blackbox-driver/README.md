# blackbox-driver

The **controllers** of computer-driven cars in EA Black Box games (Need for Speed: Most Wanted today):
they turn a target point and a wanted speed into gas, brake, handbrake and steering.

- `Driver` with `ControllerKind::Simple` (traffic: steer at the target, full gas or full brake) or
  `ControllerKind::Pid` (racers, cops: adaptive steering PID, velocity-form throttle PID), plus the
  reverse logic and a forced-reverse manoeuvre.
- `StuckDetector`: a car that presses the gas and does not move.
- The building blocks: `PidError`, `AdaptivePid` (MIT rule), `Table`, `Graph`.

Pure maths on plain numbers: no I/O, no vehicle or road dependency. Physics space (x right, y up, z forward).

Specs: `docs/specs/ai-driver-control.md`, `docs/specs/ai-driver-control-pid.md`.

License: MIT OR Apache-2.0.
