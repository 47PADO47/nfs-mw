# Vehicle compound dispatch provenance

Spec: [vehicle-pair-dispatch.md](../specs/vehicle-pair-dispatch.md).

Read-only identity-confirmed PC inspection and isolated execution of
RigidBody::OnObjectOverlap at0x6998C0 confirms nested primitive-list visitation,
A preparation once per outer iteration, B preparation per inner pair, and immediate
ResolveObjectCollision calls at every admitted intersection. Seven synthetic cases
cover visitation, pose changes, body refusal, preparation failure and misses.
Separate execution of Geometry::Set at0x4876F0 confirms its centre is copied;
changing the source position leaves all176 local geometry bytes unchanged.
Both targeted functions match the selected D/E executable byte ranges despite the
different full executable hashes. Private code/bytes/results stay outside Git.

The game car-pair adapter previously discarded all but the deepest overlap. This
could suppress a later approaching corner/roof/bumper contact after an earlier
nonclosing overlap. The correction retains per-contact response and feeds every
successful callback to the traffic gameplay path. The existing prop dispatcher
uses a different refresh policy; this evidence does not certify that adapter or
complete global broadphase ordering. No new contact strength or damage constants
are introduced. The CC0 reference RigidBody agrees on nested dispatch and is used
for understanding; no original code is copied into the implementation.
