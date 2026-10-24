# Garage performance-bar provenance

Specification: [garage-performance-bars](../specs/garage-performance-bars.md).
Date: 2026-10-24.

Sources:

- User-installed PC executable with SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Ghidra program identity was checked before bounded inspection of the raw
  estimator, population initializer, package blend, shift-point helper,
  peak-torque helper, eligibility and maximum-package helpers, and slider calls.
  This is static inspection; no original process execution is implied.
- User-installed `attributes.bin`: engine, tires, chassis, induction, nitrous,
  transmission and pvehicle records. Proprietary assets are read at runtime.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0, reference only:
  generated attribute field names, `PhysicsInfo.hpp`, `CustomizeManager.cpp`,
  `CarCustomize.cpp` and the common `Slider.cpp`. No source was copied.
- Existing [car-lot spec](../specs/career-car-lot.md) supplied the previous
  investigation's entry points. The new inspection resolves `HandlingRating`,
  `junkman_current`, combined stock/max normalization and fill-versus-tick roles.

Original decompilation output, exact procedure addresses, executable constants,
and installation identity stay in private research evidence. Public files keep
behavioral facts, formulas and generated type/field identifiers only.

The new implementation follows the written contract rather than the prior
physical vehicle launch/grip proxy. It remains separate from live driving and
AI planning, so changing a menu estimate cannot change a driven car's physics.
