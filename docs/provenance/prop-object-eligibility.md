# Prop object collision eligibility

The [contract](../specs/prop-object-eligibility.md) was written before the runtime
change. Read-only inspection of the matching original PC executable on
2026-10-20 confirmed its primitive-count, linked `NO_OBJ_COLLISIONS`, and frozen
state checks. The original routine was executed in isolated emulated memory for
24 combinations of primitive count, source flag and body state; all matched the
contract. The same routine bytes were verified in the selected modded executable.

The existing independently readable reference's `RigidBody::CanCollideWithObjects`
and `UpdateGrid` agree with those checks. The source flag removes an endpoint
from object broad phase rather than changing its physical mass or contact
material. Installed assets, private addresses, captured records and emulator
output remain outside the public implementation.
