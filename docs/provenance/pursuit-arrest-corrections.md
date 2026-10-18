# Provenance: arrest and pursuit meter corrections

Specification: [pursuit-arrest-corrections.md](../specs/pursuit-arrest-corrections.md).

- Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
  revision `80993f35224c61d2f1b135beba76c6f8234c7940`.
- Read `AI/Common/AIPursuit.cpp`: `OnTask` (timer, stop goals and delayed NIS),
  `TimeUntilBusted` (signed proximity/timer blend).
- Read `AI/Common/AIVehicle.cpp`: busted perpetrator control override.
- Read `AI/Actions/AIActionStopShort.cpp`: stopped police controls.
- Read `Frontend/HUD/FePursuitBoard.cpp`: five bar pieces and the pre-bust 0.99
  clamp. No code was copied from the reference.
- [Bartender author's documentation](https://github.com/rng-guy/NFSMWBartender)
  independently identifies distinct orange warning and red busted bar behavior,
  and lists broken-line-of-sight bust prevention as a mod correction. The remake
  previously required immediate visibility. The base gate now follows the
  reference's seven-second tracked-contact grace instead of silently adopting
  that mod correction.

The remake previously kept driving controls live through its fallback arrest,
removed all police on the same frame, and left the terminal flag latched after
payment. Its `HudState::busted` had no bar consumer. These findings came from
source tracing; a fresh runtime check is required after implementation.

The native PC executable's `TimeUntilBusted`, pursuit task and variable scheduler
were inspected in a separate read-only analysis pass. They corroborate the
signed timer blend and steady +1/-2 scheduling conversion. The executable's
SHA-256 was `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
Static evidence does not establish exact gameplay timing under all frame rates.

The original `TimeUntilBusted` machine instructions were also executed in an
isolated instruction emulator, using the same executable identity. Twelve
boundary cases passed. The Rust regression retains representative vectors:
timer/warning/evade of `(1, 0, 0)` gives 0.2; `(2, 0.5, 0)` gives 0.7;
`(4.9, -0.8, 0)` gives approximately 0.964; `(0, 0.3, 0.05)` gives -1; and
`(0.5, 0.3, 0.5)` gives 0.37. This corroborates the arithmetic and branch
thresholds; it does not constitute a live pursuit or rendered HUD comparison.

The exact NIS-to-career sequence remains incomplete. The remake now follows a
three-second hold with the no-NIS ticket fallback; it does not pretend a timed
overlay is a played arrest animation.
