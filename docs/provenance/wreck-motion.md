# Destroyed vehicle motion

Specification: [wreck-motion.md](../specs/wreck-motion.md).

Read-only Ghidra program identity confirmed original executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c` before bounded queries.
`AIActionTooDamaged::Update` at PC `0x4067A0` confirms the already implemented zero controls,
shock gate, 2.5 m/s threshold and quarter brake. `RBVehicle::ShouldSleep` at `0x69AB90` confirms
that a supported vehicle refuses sleep while its dead-on-wheels timer is below 2 seconds, then
delegates the ordinary rigid-body test. The referenced constant at `0x8AAC9C` is `2.0`.

CC0 reference read for understanding: dbalatoni13/nfsmw,
`AI/Actions/AIActionTooDamaged.cpp`, `Physics/Behaviors/{RBVehicle.cpp,RigidBody.cpp}`.
`OnTaskSimulate` increments that timer only for destroyed non-player vehicles with strictly more
than half their wheels supported; other ticks reset it. `GetNumContactPoints` includes wheels,
and the original base sleep predicate uses authored velocity threshold plus contact count.
The implementation follows this behavioral contract without copying original code or bytes.

Installed inspection identifies `SLEEP_VELOCITY = 1.0` for midsize, ghost, GTO, sport and SUV cop
profiles. Runtime continues reading installed values instead of introducing a new stop threshold.
The remake previously skipped every sleep check whenever any wheel was supported, leaving this
destroyed-car exception unreachable. This proves a final-creeping mismatch, not the cause of every
observed long slide. Original live wreck trajectories and injected mod effects remain unverified.

Isolated execution of the original vehicle sleep gate confirms delegation starts at exactly `2.0`
seconds, while `0` and `1.99` refuse it with supported wheels. The ordinary base predicate was stubbed
true solely to observe that gate. Separate original support-counter calls increment for three/four
supported wheels, reset at two, and reset for live/player vehicles. Interface responses were synthetic;
no game process was launched. The generic library exposes an optional caller-selected delay, disabled
by default; only the game adapter selects `2.0` for destroyed non-player cars.
