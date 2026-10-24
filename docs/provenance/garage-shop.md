# Garage menu, shopping and unlock awards

- **Specs:** [garage-shop](../specs/garage-shop.md),
  [garage-records](../specs/garage-records.md),
  [garage-frontend](../specs/garage-frontend.md),
  [garage-decals](../specs/garage-decals.md).
- **Reference:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0, the declarations and behavioral routines listed in garage-shop.
  Read for understanding. No original code, installed assets or extracted
  tables were copied into the implementation.
- **Date:** 2026-10-23.
- **Original binary check:** PC 1.3 executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Ghidra program identity matched the selected unmodified executable.
  Cart-total and checkout instructions independently confirm one-quarter
  trade-in credit, integer truncation and negative-delta cash refunds.
  The trade-in predicate confirms the excluded slots in the spec.
  The performance screen also confirms the Burger King reward bit and the
  completed-profile alternatives for My Cars Junkman availability.
  This resolves an inconsistency in the incomplete reference reconstruction.
- **Marker check, 2026-10-24:** the same matched binary independently confirms
  owned state 1, exact kind/parameter redemption, first-match removal by clearing
  kind and state, and the vinyl-only wildcard in parameter-zero counts.
  The visual-slot overload explicitly requests vinyl kind 14, parameter zero.
  Private bounded inspection evidence retains the routine addresses and output.
- **Implementation:** runtime menu rows use installed catalogues and a separate
  preview/confirm/cancel contract. Price and unlock adapters follow the written
  schema. Native record fields preserve the existing profile layout.
- **Validation scope:** synthetic tests cover quote gating, preview isolation,
  number pairing, body-dependent decal rows and device-independent input.
  Ignored installed tests cover the price and bin-award readers when run.
  Passing those checks does not establish original runtime parity.
- **Remaining comparison:** live original cash totals and refunds, backroom marker
  behavior, heat ordering, animations, camera transitions, rendering and real
  controller interaction require direct original/remake comparison.
