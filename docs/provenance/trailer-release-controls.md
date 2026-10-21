# Trailer release control evidence

- Specification: [trailer release controls](../specs/trailer-release-controls.md),
  written before implementation.
- Sources: read-only PC 1.3 inspection of `RBTractor::UpdateTrailer` and
  `SetHitch`, corroborated by the CC0
  [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `Physics/Behaviors/RBTractor.cpp`, at the revision already credited in NOTICE.
- Target: SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  This is the analyzed PC executable, not a claim that a differently hashed
  modded installation was executed or has identical behavior.
- PC routine `0x69B430` explicitly writes zero to trailer brake and handbrake
  when unhitched. Its support branch advances one stored timer when either
  suspension reports fewer than two wheels; both supported resets the timer.
  `0x69B100` provides the separate immediate release path. Readable-reference
  identification corroborates the interfaces; no decompiled code is included.
- Prior adaptation errors: full braking after detachment was justified with
  real-world spring brakes rather than original-game evidence; separate body
  timers lost continuous unsupported time when support alternated.
- Validation scope: shared-timer boundary/reset controls and an installed
  runtime trailer trajectory compared with an independently stepped brake-free
  body. Neither establishes a full live original/remake truck route match.
