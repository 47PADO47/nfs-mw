# Prop and articulation phases

Sources: original PC scheduler/body-end behavior (proprietary inspection) and
CC0 rigid-body reference, read for behavior only. No restricted code ported.
See [provenance](../provenance/prop-articulation-phases.md).

Integrate all participating vehicle, trailer and active prop bodies before the
prop-pair pass. Preserve sorted prop-pair ordering and the existing deterministic
car-pair ordering. Separate prop-pair work from prop lifecycle preparation so
it can run after all body begins. A unified original mixed-body pair traversal
is not established by this change; vehicle-prop admission retains its own pass.

A prop's body finish resolves world and ground contacts before applying drag,
then evaluates sleep from the resolved support. Keep the existing contact sound,
effect and damage snapshots, friction and restitution, and sleep thresholds.
The standalone prop step composes the same phases without duplicate integration.

After every vehicle and prop body finish, solve existing trailer articulations
before engine/suspension response. Preserve solver iterations, masses, limit
values and deterministic ordering. Separate articulation solving from final
pose publication. Recheck attachment state through existing release logic;
its precise original task priority remains a separate open question.

Final car/trailer poses publish after all these stages. Current pursuit/property
and sound-frame observations follow completed car response. Contact damage and
severity retain incoming-contact snapshots as specified by
[driving state consumers](driving-state-consumers.md).

Regression checks must show a constraint's corrected velocity reaching that
tick's response, prop drag consuming post-contact velocity, and property events
being observed once in the same completed tick. This is phase correctness,
not proof of original pair traversal, primitive geometry or route parity.
