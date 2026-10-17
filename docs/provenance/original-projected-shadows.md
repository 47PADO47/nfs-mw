# Original projected shadow producer

- **Specification:** [Original PC projected shadows](../specs/original-projected-shadows.md).
- **Date:** 2026-10-16; specification precedes the camera-fit implementation.
- **Source:** matched retail PC executable, read-only Ghidra analysis of shadow
  allocation, vendor capability selection, camera fit, caster submission, common
  effect update and strength production; embedded original receiver shaders.
  Proprietary artifacts are behavioral references only. No original code,
  bytecode, decompilation or assets are distributed.
- **Identity:** executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
  confirmed against the active Ghidra program before address-based analysis.
- **Independent evidence:** six isolated original-instruction camera-fit cases
  using synthetic point sets and views. The original x87 instructions choose
  basis, bounds-relative translation, perspective extents, near/far values,
  post transform, composition and camera stores. D3DX calls use the host library;
  bounding-box accumulation is a synthetic callback. Independent equations agree
  within 0.000005 per matrix element. Fifteen additional unchanged scalar-updater
  cases verify the sun-altitude/cloud strength equation within 0.000001.
  No original game process or GPU was run.
- **Limits:** original frustum/caster collection is not replayed; vendor depth
  comparison/filtering, render-state bias, strength constants and shadow pixels
  are not validated by this oracle. New resolutions and degenerate-input guards
  are enhancements. Custom cascade/AO/contact behavior remains separate.
- **Integration coverage correction:** source review found that the initial
  adapter supplied only frustum corners although the recovered fitter consumes
  selected object corners too. The adapter now includes conservative resident
  caster bounds that overlap the receiver footprint along sunlight. Exact
  parallel camera/sun alignment has a modern perpendicular-axis fallback.
  New synthetic CPU and GPU regressions exercise these integration boundaries;
  this does not extend the original-instruction oracle to original culling.
