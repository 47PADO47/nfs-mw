# Contact camera shake provenance

- **Spec:** [contact camera shake](../specs/contact-camera-shake.md).
- **Implemented:** 2026-10-11, from the written behavior specification.
- **Sources:** privately inspected original PC 1.3 executable, hash and routine
  addresses recorded in the spec; the CC0
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `Misc/Rumble.cpp`
  behavior and PC symbols. No original code was copied or translated.
  Installation paths and private report paths are omitted.
- **Validation:** thirteen isolated native sampler vectors and one native
  view-transform fixture, threshold/retrigger checks and pre-impact
  interpolation. Presentation tests check that render sampling leaves the
  physics follow/cleanup view unchanged across frame rates and that teleporting
  clears the pulse. World integration reads acceleration after shared contacts.
- **Remaining:** original/remake gameplay capture comparison, authored explosion
  camera events and cinematic shakes. These checks establish numerical and
  integration coverage, rather than full perceived parity.
