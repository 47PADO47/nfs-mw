# Player race launch

The caller supplies the launch configuration and race staging/release context.
Generic vehicles keep this feature disabled. AI activation is outside this change.
See [provenance](../provenance/player-launch.md).

**Known defect (2026-10-26 playtest):** countdown engine revs are buggy and
behave differently from the original. The cause is unconfirmed. Component tests
below do not establish countdown producer timing or complete launch parity.
See [the open defect and acceptance criteria](../player-driving-status.md).
The [countdown drivetrain follow-up](player-countdown-drivetrain.md) corrects two
clutch-state rules without claiming full race-start acceptance.

For the selected PC game, first gear alone supplies a launch band. Its width is
`(redline - idle) * 0.25`; its lower edge is
`min(peak_torque_rpm + width, redline - 500) - width`. Other gears have no band.
While staging, positive lower edge and width are required. Displayed RPM must
be strictly inside the band. Strength is `0.5 * (1 + (rpm - lower) / width)`;
both endpoints produce zero. This band does not replace peak RPM in converter
or clutch calculations.

An explicit release starts a 4.5 simulation-second timer when stored strength
is positive and no launch is already running. Effective strength is zero while
staging or when the timer is zero. Outside staging the timer decreases only
while no gear change is active, and clears strength at zero. Body speed strictly
above `26.821800231933594` m/s also clears it. Drag-specific shift cancellation
is outside the ordinary race caller; the generic state can optionally enable it.

During an active launch with positive effective throttle, replace engine torque
with cached peak torque times NOS boost, set throttle to one and engine braking
to zero. Do not multiply this torque by launch strength or induction boost.
Zero throttle and a blown engine retain normal zero-throttle behavior. The
override has no additional gear gate after activation. Automatic shifts are
blocked while staging. Tire setup adds `strength * 0.25` to its traction scale.
All staged cars also multiply all four tires' traction boosts by `0.25` after
the ordinary tire assists. This is independent of drag mode. The caller supplies
this staging multiplier; unconfigured vehicles retain a multiplier of one.

Staging holds the chassis independently of its pedals. Save the prior world
position, run normal rigid-body integration, then use the updated rotation to
convert velocities to local space. Zero local sideways/forward velocity and
local yaw angular velocity. Keep vertical velocity and roll/pitch angular
velocity. Restore prior world X/Z position while keeping integrated Y position.
Do not reset wheel speed, engine speed, pedal input, or chassis orientation.
Staging also bypasses the chassis low-speed sleep/damping rule, so released
throttle or a driver's brake input cannot trigger its wheel-speed reset.

The library evaluates launch state once per vehicle step from the previous
displayed RPM and current gear/shift state. Race staging and release are supplied
by the scene. Component numeric agreement does not alone establish countdown
producer timing, complete drag behavior or full original race trajectories.

## Human race caller

The scene configures human cars after placement; generic and AI cars stay
disabled. A placement resets launch state. Grid holding supplies staging and
retains the driver's gas, brake and handbrake inputs. A passive Golf race capture
has zero handbrake during every staged callback and confirms these controls are
not forced to full brake/handbrake. The body hold prevents departure. NOS and
Speedbreaker remain unavailable while staged. The explicit GO event releases staging
and requests a launch once. Cancelling or ending a race only clears staging;
it must not award a launch. Rolling starts never stage and earn no launch.
Drag races select shift cancellation, independently of ordinary race launches.
Initial grid gear/clutch sequencing and tachometer needle-color feedback remain
separate comparison limits.

A successful human launch shows the installed `HUD_LAUNCH_PERFECT` language
label using the existing generic message script, alongside the GO countdown.
The launch message has its own lifetime; a later race notice can replace it.
Once replaced, it cannot reappear when that newer notice expires. Message
identities remain distinct across race restarts so repeated launches replay
their animation.
