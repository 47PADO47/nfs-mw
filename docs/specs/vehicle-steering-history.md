# Steering history order

Supplement to [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md) §6.1.
Source read: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
`Physics/Behaviors/SuspensionRacer.cpp`, `DoHumanSteering`, `CalculateMaxSteering`
and `CalculateSteeringSpeed`. Read for behaviour only; no code copied.

**[decomp]** The range and input-magnitude coefficients use the history from the
previous tick. For a gamepad, the rate estimator subtracts the previous tick's
remapped input from the current **raw** input and divides by the supplied simulation delta.
Its absolute value is looked up in the rate-coefficient table **before** recording
the coefficient into the 0.15-second average. Averaging derivatives before the
nonlinear lookup gives a different rate.

Compute the target from raw input and the previous magnitude history, then clamp
its movement using the averaged rate coefficient. Finally record the remapped
current input into the 0.55-second magnitude history and remember it for the next
rate estimate. Racing wheels remember and average raw input; they have no rate
limit. A speed-insensitive racing wheel returns the absolute 45-degree range
directly, before braking, tuning, countersteer or collision range changes.

The initial reference was static GameCube behavior. PC consumer inspection,
isolated human-steering sequences and passive player callbacks now confirm the
selected PC remap, prior-history order and averaged rate coefficients. At the
observed modded 144 Hz cadence the histories are limited to 33 and 9 samples,
including during Speedbreaker, whose supplied delta becomes 1/576. The remake's
[shared clock](shared-physics-clock.md) now separates real cadence and scaled
simulation delta. Matching component inputs still does not establish complete
driving trajectories.
