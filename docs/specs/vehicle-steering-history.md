# Steering history order

Supplement to [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md) §6.1.
Source read: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
`Physics/Behaviors/SuspensionRacer.cpp`, `DoHumanSteering`, `CalculateMaxSteering`
and `CalculateSteeringSpeed`. Read for behaviour only; no code copied.

**[decomp]** The range and input-magnitude coefficients use the history from the
previous tick. For a gamepad, the rate estimator subtracts the previous tick's
remapped input from the current **raw** input and divides by the fixed step.
Its absolute value is looked up in the rate-coefficient table **before** recording
the coefficient into the 0.15-second average. Averaging derivatives before the
nonlinear lookup gives a different rate.

Compute the target from raw input and the previous magnitude history, then clamp
its movement using the averaged rate coefficient. Finally record the remapped
current input into the 0.55-second magnitude history and remember it for the next
rate estimate. Racing wheels remember and average raw input; they have no rate
limit. A speed-insensitive racing wheel returns the absolute 45-degree range
directly, before braking, tuning, countersteer or collision range changes.

This is a static GameCube-reference observation. Existing PC calibration confirms
the speed axis and full-lock response but does not establish these partial-stick
history details in the running PC game. Synthetic regressions establish agreement
with the documented order, not complete original-game driving parity.
