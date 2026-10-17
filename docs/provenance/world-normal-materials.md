# Normal-mapped world materials

- **Spec:** [Original PC normal-mapped world materials](../specs/world-normal-materials.md).
- **Date:** 2026-10-15; specification precedes implementation.
- **Sources:** retail PC 1.3 executable shader descriptor and vertex declaration,
  embedded WORLDNORMALMAP simple unshadowed Shader Model 2 vertex/pixel programs,
  common world light updater and restricted car LightMaterial setter
  (proprietary; inspected for behavior only; no source or bytecode copied).
  [noclip.website MW renderer](https://github.com/magcius/noclip.website/tree/master/src/NeedForSpeedMostWanted)
  (MIT; read-only corroboration of texture slots and tangent family).
- **Identity:** stock executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
  preferred modded executable SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  All 31 embedded effects match byte-for-byte. This does not establish active
  mod hooks, identical runtime constants or gameplay equivalence.
- **Checks:** bounded declaration inspection establishes tangent offset and
  handedness. Shader registers establish diffuse/normal/specular inputs,
  vertex COLOR clamping, pixel reflection vector and full-light extinction.
  World updater establishes TOD specular RGB and a global power initialized
  to 11.0; car material setter rejects effect ID 3. A complete stock stream
  scan found 8,003 ID 3 shading groups, all with valid normal/specular texture
  indices. This count does not establish resident texture resolution or pixels.
- **Independent measurements:** 21 synthetic original D3D9 shader cases cover
  tangent handedness, normal direction, specular color/texture/power, UV offsets,
  alpha, fog and non-unit bases. Separate equations agree within one UNORM byte.
  These fixtures do not establish whole-scene gameplay equivalence.
- **Remaining:** original projected shadows, runtime sampler/culling and texture
  offset animation, nonuniform transforms, mod hooks and matched gameplay capture.

Some installed normal-map geometry contains nonfinite authored tangents, including
indexed vertices. Upload preserves their exact bits. Finite fixture agreement
does not establish matching original rasterization for those degenerate inputs;
that remains a separate validation gap. No replacement tangent is invented.
