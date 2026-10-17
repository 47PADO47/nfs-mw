# World window material provenance

- **Spec:** [Original world window materials](../specs/world-window-materials.md).
- **Date:** 2026-10-16; behavioral spec written before implementation.
- **Source:** shipped PC shader descriptor/effect, world light uploads and named
  reflection texture binding (proprietary, inspected for behavior only). No
  original bytecode, assembly or assets are included in public source.
- **Executable identity:** SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **GlossyWindow effect identity:** 38140 bytes, SHA-256
  `8dac7eb15a1d0db283d381e9d0eed3b177cafcb0426e1bd5c9847c2d926489bf`.
  The static modded/stock resource comparison must not be interpreted as proof
  of equal active runtime hooks or constants.
- **Verified static behavior:** effect ID 5 ordinary vertex declaration,
  unshadowed and programmable shadowed shader pairs; vertex reflection highlight
  exponent ten, texture-alpha gloss mask, per-vertex COLOR saturation, ambient
  extinction, shared world scattering and view-normal reflection coordinates.
  Runtime resolves WINDOWREFLECTION by name and binds its resident texture before
  shared time-of-day light delivery. The installed image is in InGameA, 256 by
  128 with five mips.
- **Validation:** independent original D3D9 fixtures use synthetic geometry,
  textures, light/scattering constants and view-normal variations. Current
  validation outcomes are recorded by the matching implementation checks;
  static shader inspection is not installed gameplay observation.
- **Remaining:** original sampler and mip policy, projected-shadow reconstruction,
  blend/cull state coverage, nonrigid context selection and gameplay composition.
