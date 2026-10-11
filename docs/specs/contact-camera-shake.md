# Contact camera shake

Target: Most Wanted PC 1.3. The original was inspected privately before this
specification was implemented. No binary, asset, disassembly or decompiled
expression is included in the source tree.

- Original PC executable SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Read-only PC inspection: shaker selection `0x64C5F0`, start `0x64C1D0`,
  update `0x64C2D0`, sampling `0x64C270` and view application `0x64C420`.
  Addresses identify evidence; they are never runtime dependencies.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reference:
  `Misc/Rumble.cpp` and PC symbols. Read for behavior only, with the
  [spec-first process](../licensing.md#spec-first).

## Behavior

The camera observes target acceleration in world/render axes, including impacts
and landings. Bias horizontal components by 0.7 and the vertical component by 1.
Below 100 m/s² there is no trigger. Clamp acceleration length at 600, then map
the 100..600 range linearly to translation magnitude 0.1..0.4 m and duration
0.3..0.7 seconds. The direction opposes the biased acceleration.

The normalized envelope has points `(0,1)`, `(0.1,0.7)`, `(0.3,0.5)`,
`(0.5,0.4)`, `(1,0)`. Interpolate linearly, returning zero outside 0..1.
A new pulse replaces an existing one only when its requested magnitude is
greater than the current normalized envelope value. This compares to the
envelope itself, without multiplying by the stored translation magnitude.

Phase advances by seconds divided by duration. Translation is gated on even
`floor(phase * 11.74265)` intervals and zero on odd intervals. The camera also
rotates by the translated vector expressed in view axes: X is -1200 angle units
times local Z plus Y, Y is 2000 angle units times local X, Z is zero. One turn
is 65536 angle units. Apply translation in view axes, then quantized X and Y
rotations to the entire view matrix, including its translation row.

## Integration and checks

Measure acceleration from velocity before integration to velocity after all
world and car-pair responses, divided by the fixed tick duration. Triggers and
lifetimes advance on the shared 60 Hz clock. Drawing evaluates the waveform at
the same interpolated time as the car and chase camera. Sampling cannot change
the follow pose, collision probes or cleanup view. A teleport resets the pulse.

Synthetic inputs executed by the original PC sampler provide thirteen reference
vectors. An isolated PC matrix application fixture uses documented D3DX math
for external DLL calls. Tests check those vectors, the matrix, thresholds,
retrigger rules, interpolation before a contact, fixed-tick follow equivalence
across frame rates and reset behavior.

Explosion camera events and cinematic shakes are separate authored paths and
remain outside this contact feedback pass. A full original/remake gameplay
capture comparison is still required to establish perceptual parity.
