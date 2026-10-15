# Original gameplay camera follow provenance

## Sources

- Matched PC 1.3 `speed.exe`, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Identity confirmed through Ghidra program metadata and independently before
  isolated instruction execution on 2026-10-15.
- Community PC symbol map: function names only, matched to native addresses.
- `dbalatoni13/nfsmw` reference declarations for `CameraAnchor`, `CameraMover`,
  `tCubic1D` and `tCubic3D`; used to interpret fields, not copied into this code.
  The reference Cubic mover source/header are empty. Its platform layouts are
  not substituted for the checked PC layout.

## Native observations

| Routine | Observation |
|---|---|
| Cubic update `0x0047d3c0` | Speed sampling, durations, advance-before-target ordering, separate directional/local/up states |
| Target producer `0x0047cf00` | Actual eye/target placement, acceleration modifiers, up target, snap admission |
| Nitrous initializer `0x00884f30` | FOV target increment of 1638 angle units; target producer gates it by outside/moving/NOS |
| Direction producer `0x0046b1f0` | Reverse reflection, speed-direction blend, post-airtime vertical attenuation |
| Basis producer `0x0046b410` | Normalized filtered direction and two unnormalized cross products outside; body orientation inside |
| POV setter `0x0046b050` | Preserve outside-to-outside state, snap crossing inside/outside boundary |
| Reset `0x0046b0c0` | Snap retained scalar and vector states |
| Cubic constructor `0x00481bd0` | Separate spline allocation, rolling flag, five-sample acceleration average |
| Scalar update `0x006513c0` | Hermite coefficient/evaluation arithmetic and retained derivative |
| Target setter `0x00649cb0` | Rearm each scalar when desired differs from current value |
| Acceleration producer `0x0047cdd0` | Saturating five-knot per-axis nonlinear response |
| Averager `0x00478b70` | Five entries; startup count uses populated entries |
| Speed scale `0x0047ce80` | Five-point lag extension by fraction of anchor top speed |
| Tweak sampling `0x0046f2b0` | Independent two-row speed response for each view |
| Main loop `0x00663d30` | Uses wall-clock scheduler return as camera-dispatch input |
| Scheduler `0x00642e60` | Timestamp-based elapsed/period accounting |
| Camera dispatcher `0x0064a680` | Shares unscaled accumulated elapsed between actions and movers |
| Mover dispatcher `0x0047c880` | Forwards elapsed to active mover virtual update |
| Anchor update `0x00468fa0` | Longitudinal speed derivative, geometry rotation, 300 m/s teleport gate |
| Camera action `0x00472740` | Grounded slip/speed drift, real-time impact timer decay and normalization |
| Collision callback `0x00472460` | Three thresholded closing-velocity impact timers |
| Car acquisition `0x004725d0` | Top-speed anchor input is transmission GetMaxSpeedometer |
| Cubic transient blocks `0x0047d53d`, `0x0047d88d` | Instantaneous mapped-acceleration pulse admission and local offset response |
| Static initializer `0x00884f00` | Original pulse admission acceleration threshold is 24 |

The spec records the behavioral scalar responses using semantic names. No
executable code, complete binary tables, original assets or decompiler output
are part of the implementation or fixtures.

## Validation and limits

The unchanged scalar-update routine was executed in isolated x86 memory with
synthetic state, a private copy of the matched executable, a synthetic stack
and its original constants. No original-game process was launched or patched.
Fixtures include consecutive 60 Hz rolling updates, nonzero derivatives,
zero-duration updates, large steps and an overshoot case. The regression
values are synthetic outputs of that routine, not game assets or pseudocode.

Fifty-four unchanged collision-callback replays cover all three collision
kinds, nine closing speeds and two preceding timer values. Seven native anchor
updates cover longitudinal/sideways velocity changes, a rotated body basis,
deceleration, zero elapsed and both sides of the teleport threshold. The latter
harness supplies only the external standard D3DX vector-matrix transform; game
instructions remain unchanged. Both sets agree with the behavioral equations.
The host's new anchor history seeds the first sample without a derivative,
avoiding a synthetic zero-to-current-speed impulse when attaching to a car.

The first zero-to-ten scalar sequence at duration 1.25 seconds is
0.005285926, 0.020814722, 0.04603459, 0.08041237, 0.12343314. The half-second
sequence at duration one is 5, 9.375, 10.15625. These distinguish the native
rolling cubic from exponential interpolation and a one-shot smoothstep.

Static reconstruction establishes state/target arithmetic, not full live
camera equivalence. The host deliberately updates this rate-sensitive original
algorithm at 60 Hz and interpolates presentation at other refresh rates. Noise,
transients and obstruction require separate checks. Native camera cut admission
can seed derivatives from the previous mover; startup initialization in this
host snaps to the selected view instead of reconstructing unsupported ICE cuts.

Twenty-seven regressions retain 209 synthetic output poses from the unchanged
whole Cubic update through final eye, target, up and camera FOV production. They
cover startup for all seven modes, sustained turning, reversing, high speed,
airborne motion, acceleration, body pitch, Hood banking and upside-down body
orientation, zoom, NOS, lookback, and outside/inside view switches. The harness
supplies controlled authored POV fields, false pause/under-vehicle queries, empty
nearby-car influence, and standard external D3DX vector/matrix arithmetic. It
stops before noise and obstruction. Impact and gear-change transients are inactive
in the initial twenty scenarios. Seven additional sequences activate world-impact
pulses in outside, inside and lookback configurations, exercise stock zero
vertical displacement, and check gear pulses with inside/slow admission and
active-pulse retrigger suppression. Their elapsed steps align with the original
4000 Hz timestamp granularity. The separate transient spec records the derived
responses and distinguishes unused ground/object impact fields.

These composed sequences exposed two errors that scalar spline checks could not:
the direction and up spline durations had been interchanged, and normalizing
the outside placement basis's cross products changed its airborne height. Both
were corrected from the matched native call graph. Vector comparisons permit
0.0003 world units for floating-point and native 4000 Hz timestamp rounding;
FOV comparisons remain exact after native lens finalization. The host integrates
airtime continuously at its fixed camera step rather than reproducing the
original integer timestamp epoch.
