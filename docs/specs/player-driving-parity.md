# Player driving parity corrections

Current implementation coverage, the user-reported countdown rev defect and
remaining gaps are tracked in [player driving status](../player-driving-status.md).

## Reference and scope

The comparison target is PC v1.3 with the large-address-aware memory flag and
the user's installed fixes. Original executable inspection and isolated numeric
execution establish the steering rule below. The selected executable's code
sections match the existing PC research executable; only a PE header flag differs.
Sources and evidence limits are in
[the provenance record](../provenance/player-driving-parity.md).

This correction affects human steering: gamepad/keyboard and speed-sensitive
racing wheels. AI steering bypasses human range shaping. Fixed-range racing
wheels retain their existing 45-degree range. Rate tables and the rate algorithm
are unchanged; the corrected history curve changes their inputs.

## Steering range multiplier

The absolute recent steering-history average selects a six-point, linearly
interpolated multiplier over 0..1:

| History | 0 | 0.2 | 0.4 | 0.6 | 0.8 | 1 |
| --- | --- | --- | --- | --- | --- | --- |
| Range multiplier | 1 | 1 | 1.1 | 1.2 | 1.25 | 1.35 |

The rate multiplier uses a separate table: `1, 1.05, 1.1, 1.2, 1.3, 1.4`.
Do not substitute the rate table for the range table. Retain the existing
braking/tuning/countersteer factors, remembered maximum and final 45-degree cap.

Isolated original-code range outputs, with full gas, no braking, no tuning,
no collision limiter, no rear slip and previous maximum zero:

| Speed (m/s) | History 0.2 | History 0.8 | History 1 |
| --- | --- | --- | --- |
| 10 | 28.75 | 35.9375 | 38.8125 |
| 40 | 8.875 | 11.09375 | 11.98124981 |
| 80 | 3.875 | 4.84375 | 5.23125029 |

These are maximum angles before multiplying by raw steering input. A
speed-sensitive wheel at 40 m/s and history 0.8 has maximum 12.8125 degrees.
At steady raw input 0.8 its output is therefore 10.25 degrees. Negative input
is symmetric. The fixed wheel remains `45 * input`; AI keeps its direct mapping.

## Partial-stick history remap

The PC human-steering routine selects a specific 21-point curve over raw input
-1..1. The negative half through zero is:
`-1, -0.712, -0.453, -0.303, -0.216, -0.148, -0.116, -0.080, -0.061, -0.034, 0`.
The positive half is its odd mirror. Clamp inputs outside the table domain and
linearly interpolate between points.

This curve records the remembered previous input and magnitude history. The
target steering angle continues to use raw input. The next rate estimate uses
current raw input minus the previous remapped value. Do not apply this curve to
racing-wheel or AI input, or substitute the separate rate/range coefficients.

Original numeric examples: raw 0.2 records 0.061; 0.4 records 0.116; 0.8 records
0.453. With raw input 0.4 then 0.2 at 60 Hz, previous magnitude 0.116 and previous
rate coefficient 3.1, the original rate helper returns 408.224884 degrees/s
before the tire-steering factor. At tire steering 0.1, a prior angle of 9.3
degrees therefore moves to 8.619625 degrees. Tuning is neutral in this vector.

Forty-three isolated original curve samples, an original rate-helper vector,
the loaded descriptor/points, and guarded passive player callbacks establish
the selection. See [provenance](../provenance/player-driving-parity.md).

## Comparison-driver configuration

Each matched comparison must record the car identity, installed performance
packages/Junkman parts, aerodynamic body parts, active tuning profile and all
seven slider positions. The three profiles store signed positions -5..5;
the selected profile maps to physics values -1..1. Neutral means all seven zero,
not merely the stock hardware package. Stored tuning can survive hardware changes.
Steering tuning multiplies the corrected human range by `1 + 0.2 * tuning`.
Compare neutral first, then slider endpoints, without changing the hardware.

The installed calibration driver must select the car's `tire_integration` and
`jump_config`, with the reference wall-steering configuration when applicable,
as gameplay does. Constructing only `Vehicle::new(physics.spec)` selects the
generic regularized tire profile and cannot measure current gameplay parity.
Report the selected profile alongside diagnostic output.

Historical capture replay reconstructs binary throttle/brake from wheel torque
thresholds and assumes flat ground. It is diagnostic output, not an exact-input
acceptance test. Do not tune current tire constants to its historical errors.

## Open comparison boundaries

The selected mod configuration changes the simulation target to monitor refresh
rate and uses an input cutoff deadzone. A guarded passive session measured 144
real-time player updates per second, including during full Speedbreaker, when
simulation delta is 1/576 instead of 1/144. Histories retain 33/9 samples. These
are measurements of this installation, not a general rule for all patched games.
Additional matched routes and full dynamics acceptance remain required.
Flat-tire handling is excluded from this correction.

The steering correction leaves collision ordering and AI control unchanged.
The separate [shared clock](shared-physics-clock.md) supplies real-time cadence
and per-tick Speedbreaker deltas to every body; 144 Hz is an explicit offline
comparison mode rather than a rendering-FPS dependency.
Matched multi-car driving, upgrades, terrain, Speedbreaker and long trajectories
remain required before claiming complete player-driving parity.
