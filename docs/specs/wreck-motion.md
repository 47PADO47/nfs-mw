# Destroyed vehicle motion

Destroyed police use the original retirement controls: zero gas, steering and handbrake; no regular
brake while shocked or below 2.5 m/s planar speed; otherwise 0.25 regular brake. Continued motion
after a disabling impact is therefore partly intentional. Do not replace this with full braking,
extra friction or an immediate position/velocity freeze.

One missing terminal state is supported-body sleep. A destroyed non-player vehicle accumulates
`dead_on_wheels` time only while strictly more than half its wheels are supported. Any tick that
does not meet both conditions resets that timer. Before 2 seconds, supported wheels prevent sleep,
as they do on a live vehicle. At 2 seconds or later, supported wheels no longer prohibit the ordinary
body sleep test. The candidate must still have:

- no admitted object overlap in the current tick, even if no approaching response results;
- `|linear velocity| + |angular velocity| * body radius <` its authored `SLEEP_VELOCITY`;
- more than two contact points, counting body-ground contacts plus supported wheels.

Apply this check after the tick's world and vehicle/prop contacts, so an incoming hit retains its
pre-response velocity and can prevent sleep. For the Reference profile, this is the shared body-end check: it reads the prior
support timer. The later behavior callback advances support time before suspension.
The Regularized profile retains its late contact-finalization check.
Simulation advances the support timer once per tick;
repeated pose synchronization must not advance it again. A later contact can wake the body through
the existing response path. Authored mass and friction remain unchanged. Reference intact sleepers continue
engine and suspension; caller-tagged retired sleepers run engine only. The tag
currently excludes destroyed players, whose full task policy remains unverified.

This correction targets final low-speed creeping. It does not establish parity of high-speed wreck
sliding, the complete SuspensionSimple tire/engine path or complete original driving trajectories.
Validate the support-duration boundary, loss-of-support reset, angular-speed gate, collision wake,
live/player exclusion and installed destroyed-cop feed independently of the retirement brake rule.
