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

For the independent custom chase controller, measure acceleration from velocity
before integration to velocity after all world and car-pair responses, divided
by the fixed tick duration. Drawing evaluates the waveform at the same
interpolated time as the car and custom chase camera.

Native gameplay views use the camera action's anchor acceleration instead:
the derivative of scalar speed along the body's forward axis, with the anchor's
300 m/s placement gate. The PC camera action calls the same shaker selector
after updating this anchor. Both its inside and outside movers apply the shaker;
it is not restricted to custom chase. The outside mover applies it after its
follow/transient and under-vehicle offsets, before noise and clearance.

Native triggers, lifetimes and posed samples advance on the existing real-time
60 Hz camera clock. This follows the native follow clock adaptation, including
Speedbreaker inputs sampled on simulation time. Transform the posed sample's
view, including eye, forward and up; leave the follow spline state untouched.
Subsequent noise/clearance use that posed sample, and presentation interpolates
the resulting native camera keys. Custom views already carry their own shake
and must not receive a second native pulse. A teleport resets the pulse.

Read-only PC call-site inspection: camera action `0x472740`, anchor update
`0x468FA0`, inside mover `0x476690`, outside mover `0x47D3C0`. These call sites
establish the native producer and application order; they are static evidence,
not a live driving comparison.

Synthetic inputs executed by the original PC sampler provide thirteen reference
vectors. An isolated PC matrix application fixture uses documented D3DX math
for external DLL calls. Tests check those vectors, the matrix, thresholds,
retrigger rules, interpolation before a contact, fixed-tick follow equivalence
across frame rates and reset behavior.

## Explosion admission

The same PC camera action additionally visits active `IExplosion` instances
after the body-acceleration request. The CC0 interface identifies the getters
as origin, expansion speed and maximum radius; the derived PC getter instructions
confirm their event-backed fields. The camera does not use the current expanding
shell, damage bit or player target flags for this visual response.

Use the previous camera eye in render coordinates. When its distance from the
explosion origin is strictly positive and strictly less than maximum radius
plus 2 m, request outward acceleration with magnitude expansion speed divided
by the supplied real camera elapsed seconds. There is no distance attenuation.
Then run the existing biased selection and waveform. Body and explosion requests
are selected immediately in that order, preserving source registration order:
a newly admitted pulse starts at envelope one, which can suppress later requests
on the same tick. Do not replace these calls with a strongest-request batch.

The host snapshots active world explosions alongside fixed-physics camera
inputs, converts their position to render axes and visits them on its real-time
60 Hz camera clock. Completed waves are absent. These snapshots use the same
simulation timestamps as the native body inputs, including Speedbreaker.
Independent custom chase behavior remains separate. Cinematic/ICE shake tracks
and a full original/remake gameplay capture comparison remain outside this pass.
