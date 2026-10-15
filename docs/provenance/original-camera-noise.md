# Original camera noise provenance

Native PC executable identity is the same checked hash as
[original-camera-follow](original-camera-follow.md). Inspection and isolated
instruction replays were completed on 2026-10-15. Binary code, extracted values
and raw results remain private; the public spec describes behavior and the
regressions contain synthetic outputs only.

## Trace

- Cubic update `0x0047d3c0` invokes per-view handheld, helicopter and vehicle /
  terrain noise after constructing the follow matrix and before clearance.
- `0x0047bec0` produces two four-channel frequency/amplitude groups from speed,
  acceleration, drag, over-rev, nitrous, brake and the current surface's
  `CAMERA_NOISE` pair. Its two-index lookup is preserved independently.
- `0x00467420` samples those channels and composes screen translation and two
  FOV-relative rotations into the view matrix.
- `0x004673c0` scales each angular displacement against the camera's current
  half-FOV and the checked reference angle.
- `0x00467350` sums six octaves of deterministic one-dimensional value noise;
  `0x00467290` interpolates integer-hash lattice values using floor.
- `0x00470a60` applies a separate transform for each active police helicopter
  within 40 horizontal metres; it retains the preceding second noise group.
- `0x00468870` uses the per-view handheld weight; the checked default PC table
  contains zero for every original view and has no matched write reference.

The reference project's `CameraNoise.hpp` helped identify semantic vector names.
Its platform's handheld frequency differs from the PC executable; the PC values
were checked directly. The integer hash is the Hugo Elias style value-noise
polynomial, corroborated by the native instructions. No reference implementation
or native pseudocode was copied into the public source.

## Regressions and limits

Eleven phases, including negative and non-integer phases, replay the unchanged
six-octave native routine. The independently written behavioral model matches
every output bit. Native updater calls use a synthetic stack and floating-point
control mode matching the PC game. A synthetic return-store wrapper reads the
result; it does not change the native routine.

Five native full-noise application cases cover motion, negative and positive
rotations, ordinary and Speedbreaker FOV, and a later phase. The harness supplies
only the external standard 4x4 matrix multiplication primitive. Ten captured
whole-angle rotation values match the behavioral model. The public matrix
regression checks transform order and screen-axis conversion. Standard-library
trigonometry differs from the native lookup approximation by a few millionths
in some matrix entries; that tolerance is explicit.

These establish isolated arithmetic, not live route or pixel equivalence.
Surface selection, helicopter candidates, phase continuity and high-refresh
clock presentation require host integration checks. The host starts a new
controller's phase at zero instead of inheriting the original process timer.
