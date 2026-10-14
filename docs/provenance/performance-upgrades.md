# Performance packages

- **Spec:** [performance-upgrades](../specs/performance-upgrades.md).
- **Sources read for the spec:** original PC 1.3 executable inspected locally
  with Ghidra (proprietary, behavior reference only);
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0, declarations
  and generated attribute layouts listed in the spec. No decompiled expression
  or original game data is included in the implementation.
- **Research:** 2026-10-13, with program SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **Implementation:** follows the separately written behavioral spec; package
  resolver implementation and tests are tracked with the current feature.
- **Original comparison:** static package routines confirm stage fractions,
  endpoint indexing, integer truncation, array lengths and Junkman conditions.
  Reading installed values can confirm adapter resolution but is not an
  original-game runtime driving comparison.
- **Known limits:** this evidence does not establish complete vehicle handling,
  gearbox, contact, collision, assist or audio parity. Installation-specific
  modified vehicle collections remain the installation's own data.
