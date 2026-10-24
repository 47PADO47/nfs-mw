# Per-unit pursuit visibility clocks

Evidence: reconstructed `AIVehiclePursuit.cpp` constructor, `ResetInternals`,
`Update`, and `AIPursuit.cpp` pursuit update. The PC task scheduler's fractional
rate semantics are independently documented in `police-ai-parity-core.md`.

**[decomp]** Each ground pursuit vehicle runs its AI on its own staggered
one-in-eight fixed simulation ticks. Its visibility timer and time since seen
advance by that task's elapsed time. When the visibility timer reaches 0.25
seconds, subtract 0.25 and test the target once. A successful test sets the unit's
time since seen to -0.25 seconds. A failed test leaves that timer advancing.
Consequently a failed ray does not immediately erase a recent sighting.

The constructor starts visibility time at zero and time since seen at 99 seconds.
A vehicle outside a valid pursuit target resets both timers to 0.25 seconds.
Thus a promoted patrol may test at its first pursuit task, while a newly created
pursuit vehicle starts with the constructor's timer state. No random visibility
phase is introduced; it follows the vehicle's existing AI task stagger.

**[decomp]** Each pursuit decision advances its shared time since seen, then
takes the minimum with every active, undestroyed unit's sight timer. That includes
the positive ages between sight checks. A roadblock that sees the target supplies
zero. The final pursuit also supplies zero. This preserves the chase's initial
negative-five-second observation age until a fresher observation replaces it.
Do not repeatedly reset the shared timer to -0.25 from a held boolean.

Patrol detection retains its existing global check schedule. Roadblock proximity,
per-unit hidden-zone latches, and the cooldown display's reacquisition gate remain
independent rules. Helicopter and secondary-player visibility remain independent
current observations rather than borrowing a ground unit's age.

The implementation queues a sight update when the car actually executes its AI
task, then resolves its ray in the world's existing post-contact sensing pass.
This preserves deterministic serial/parallel integration and ensures a suspended
car does not invent task updates. The post-contact ray pose differs from the
original task's exact scheduling position; that ordering remains an explicit
adapter boundary pending native scheduler integration evidence.

**[adapter]** Cars suspended while collision is not resident do not contribute
their frozen sight ages to pursuit aggregation. Reactivation resets the target
observation before the next scheduled check. This maps the original active-unit
filter to the remake's streaming boundary and prevents an unloaded cop from
holding a pursuit in sight indefinitely.
