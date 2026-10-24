# Provenance: road trail curvature

This record supplements [the trail specification](../specs/ai-road-nav-trail.md#6-curvature-for-speed-choice-verified-pc-math).
The original reference source labels this function unsolved. The arithmetic below was checked against the
PC executable rather than inferred from that label or adjusted to a desired driving speed.

## Evidence

- Reference: `World/Common/WRoadNetwork.cpp`, `WRoadNav::CookieTrailCurvature`, in the CC0 reference listed
  by the trail specification. Used for names and comparison; no decompiled code copied.
- Native PC function `0x00778910`, size `0x53d`: clamps each cookie's signed curvature to ±0.01,
  averages adjacent signed values, weights by the later cookie's length, and takes the absolute value
  after the final division by total length.
- In the same function, the angle passes through the absolute-value helper `0x00401d10` after arcsine,
  before the negative-dot-product branch subtracts the angle from pi. Left and right mirror images
  therefore have the same apex curvature.
- The startup initializer at `0x0088d600` stores `0x40490fdb` in the apex clamp's upper-bound global.
  This is single-precision pi. The road average constants are ±0.01 and the adjacent-pair weight is 0.5.
- Ghidra identified the inspected program as `/speed.exe`, x86 little-endian PE, base `0x00400000`,
  SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  An independently hashed second installed executable has SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  The bounded curvature function is byte-identical in both:
  SHA-256 `3257877bd9a73b0be60672cb4ec2e5b008b039c1e52ea4aca82748897934c24f`.
  These identities establish static evidence, not a running encounter's patch state.

## Regression scope and limits

Synthetic regressions cover a mirrored 140-degree occluded bend and a length-weighted S-bend with
signed curvature clamping. They distinguish absolute angle handling, signed accumulation, and adjacent
averaging. They do not establish the surrounding visibility sweep, obstacle prediction, road branch
selection, apex-cookie selection, or avoidable-speed projection.

Correcting these arithmetic differences alone does not establish a sustained police pursuit or explain
every severe braking event. An observed acute-angle road occlusion can produce the same curvature with
both angle formulas.
