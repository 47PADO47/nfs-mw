# Loaded-tire capture corrections

- Date: 2026-10-25.
- Spec: [loaded-tire capture corrections](../specs/vehicle-tire-capture-corrections.md).
- Sources: original PC v1.3 executable, guarded passive entry/leave observations
  during a user-driven race, retained static routine evidence and isolated
  execution of unchanged original instructions. Proprietary implementation is
  a behavioral reference only; no original code or bytes are included.
- Selected executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- Captured inputs include exact previous tire state, actual tire-call arguments,
  tire/brake attributes and surface scalars. Replays initialize each tire call
  independently; they do not reconstruct a car trajectory or pedal input.
- Nonzero-patch evidence: the original loaded routine tests skid against its
  epsilon and patch components against zero. One captured near-stationary call
  produced about 1,773 N longitudinal force while the old Rust branch produced
  zero. Original-code replay reproduces that force within one float ULP and
  exactly reproduces the captured wheel rate. A reduced power-of-two vector
  establishes the exact 512 N and -256 N m regression outputs in the spec.
- Direction evidence: the original loaded routine calls its sign-history helper
  after the wheel update, including the locked branch. Four captured lock calls
  recorded zero direction where Rust retained a preceding nonzero direction.
  Isolated original-code replay confirms the zero-direction outputs.
- Implementation preserves the generic Regularized profile's branch choices
  and locked-wheel behavior. Existing AI scheduling/control are unchanged.
- Angle follow-up (2026-10-26): original loaded-tire helper `005cb5d0` applies
  the near-zero component gates and calls signed quantized angle helper
  `0045ded0`. Inspection used the hash-identified clean PC analysis program;
  numeric execution used the selected memory-patched executable above. All 257
  octant nodes equal mathematically generated rounded arctangents. No extracted
  lookup bytes are required in the implementation. A private model agrees with
  4,056 directed/random isolated original-helper executions under x87 control
  word `007f`, including axes, equal components, zero thresholds and table-cell
  boundaries. Its angle matches all 38,764 captured loaded-tire calls exactly.
  Double-precision ratio calculation differs from four live angles by one
  `1/65536`-turn unit; each case is resolved by single-precision ratio rounding.
  This establishes captured arithmetic behavior, not a direct observation of
  the live control word. Existing rolling-response precision remains unchanged.
- Limits: full vehicle dynamics remain separate. A runtime
  initialized lateral-table descriptor differs from its on-disk sentinel;
  isolated original numeric execution must initialize the verified live value.
  This is not permission to tune grip around an uninitialized numeric oracle.
