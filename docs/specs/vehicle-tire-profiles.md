# Tire integration profiles

The [tire specification](vehicle-suspension-tires.md) §3.4 records the reference
force branch and wheel integration. Sources and static/runtime qualifications are
those of that specification. The existing generic solver regularizes these rules
to keep arbitrary synthetic cars stable. The reference profile selects the
documented rules; the regularized profile retains existing generic behaviour.

Reference differences: choose gripping or sliding from the previous traction;
use kinetic friction without the low-skid-speed floor; preserve road-speed and
angular-acceleration history at touchdown; apply the specified viscous stop torque
without the quarter-mass cap; apply the same-sign sliding torque limit even for
a locked wheel (opposite-sign/zero-torque contact friction remains);
integrate a gripping wheel toward road acceleration without explicitly cancelling
remaining slip. Brake-lock evaluation occurs only while the moving patch has skid,
as described in §3.4. Ordinary racer asymmetric burnout (§4.3 of the assists spec)
is enabled with the reference profile.
Reverse maximum slip is 71 m/s in the reference profile; regularized integration
retains its speed-dependent slip threshold to preserve generic braking/reverse stability.

Reference chassis sleep follows §7.4. Damping acts on net pending force including
gravity and on the original additive lateral/yaw cancellation. This prevents an
idle stop from creating a rolling-speed bias that the reference tire integration
would retain when the driver starts accelerating.

The profile persists over placement, cloning and package replacement. Changing it
clears burnout state but does not teleport, repair, or change the current wheels'
velocities. Generic callers default to regularized integration. The game adapter
may select reference only after installed stock and upgraded car driving checks.

This profile does not supply drag staging, speed-breaker forces, original jump
stabilizers or wall steering. Ground queries and rigid-body/world integration are
independent; selecting it is not a claim of complete original-game parity.
