# Car lighting provenance

- Date:2026-10-16. Specification:[car-lighting.md](../specs/car-lighting.md),
  written before implementation.
- Matched retail PC image SHA256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Original executable inspected read-only in Ghidra. No original binaries,
  extracted data, decompiler output or private filesystem paths are published.
- Reference names/types from [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0, reference only; independently checked against PC instructions and data.
- Car context selection:`0x0074E160` chooses gameplay rig`0x008F2E70` for
  gameflow6 and the frontend profiles`0x008F2B50` through`0x008F2DD0` otherwise.
  Initial authored default values were read from these image locations.
- Three-light setup:`0x00502130` consumes three entries, converts world,
  camera, sun/opposite-sun and positioned modes, normalizes directions, writes
  local color/direction matrices. Spherical helpers:`0x004FC320`/`0x004FC3C0`.
  FPU instructions establish degree/engine-angle conversions lost by decompiler.
- Runtime upload:`0x006C7420` supplies dynamic-light context matrices;
  `0x006C8000` computes raw per-part local camera position separately.
  `0x004FC530`'s clamped context eye must not be confused with that CAR upload.
- Material setup:`0x006C76E0` corroborates min/range mapping, quarter alpha
  range and car specular scale. The selected shader's environment `.x` access
  establishes scalar reflection ramps, despite the source profile's RGB fields.
- Car render`0x0074E160` installs two light-material replacements before part
  draws: `CARSKIN` and `WINDSHIELD`. The installed slot131 window tint part
  provides `LIGHT_MATERIAL_NAME`, following the existing selected-parts logic.
- `CarSpecScale` attribute hash`0xB656E7C6`: generated layout offset0x60;
  preset preparation`0x00768EE0` writes its prepared-block offset0x40;
  final blend`0x00769580` calls`0x00769190` with destination object+0x20,
  yielding runtime object+0x60, which material setup reads.
- CAR effect updater`0x006C5B80` selects different gameplay/frontend dynamic
  environment textures. Their generation is outside this delivery correction.
- Validation: CPU and installed checks are required after implementation;
  current evidence is static, not a live gameplay visual comparison.
