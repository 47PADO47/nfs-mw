# Shared vehicle phase provenance

Date: 2026-10-26. Spec: [shared vehicle phases](../specs/shared-vehicle-phases.md).
Sources: original PC 1.3 scheduler/body routines (proprietary); retained private
player capture; dbalatoni13/nfsmw `RigidBody.cpp` and `RBVehicle.cpp` reference
(CC0, decompiled, behavior-only corroboration). No restricted code is ported.

The PC reference body scheduler integrates all bodies, dispatches pair contacts,
then finishes all bodies before engine and suspension tasks. Vehicle body finish
refreshes terrain, applies world response and then drag. A retained player
capture confirms body-stage output equals engine input and engine input body
state equals suspension input body state in all 1,967 complete sampled frames.
Every observed boundary uses the same simulation delta, including scaled time.

The capture had substantial recorder overhead. It supports phase boundaries
and state handoffs, not normal-frame-rate trajectory or timing acceptance.
Observed bodies remained awake; staging, frozen and sleeping paths additionally
require explicit regression tests and static contract review.

Public changes express the behavior contract without binary extracts, private
recordings, addresses, installed assets or machine-specific paths. Player,
traffic, guest and trailer actors share the physical phase barrier; AI policy
and tuning are outside this change. Full tick parity remains open.

## Sleep task admission

Original-PC instruction replay covers eight bounded dispatcher cases, thirty
force-helper cases, fifteen sleep-decision cases and thirty-six support-timer
cases. Intact sleepers reach engine and suspension; zero aerodynamic resolve
wakes them. Destroyed sleepers pause suspension but retain engine updates.
Vehicle and individual behavior pause flags remain separate admission gates.
The support callback follows global body-end and uses strictly more than half
the wheels, so body-end reads the prior timer. Declared external harness
boundaries and early returns limit this evidence to the measured dispatch and
force prefix. All retained live capture bodies were awake. Live overturned-car
settling and destroyed-player admission remain unverified. No restricted code
or captured game values are included in the implementation.
