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
- **Remaining:** original/remake gameplay capture comparison and cinematic
  shakes. These checks establish numerical and
  integration coverage, rather than full perceived parity.

## Native gameplay-view integration audit

Read-only inspection of the same hash-matched PC executable confirmed the
camera action calls the shaker from anchor acceleration, and both inside and
outside movers apply it. The previous integration fed only the independent
custom chase controller, leaving stock Far/Close/Hood/Bumper views without this
pulse. Native views now share the verified sampler while using their own
scalar-speed derivative and real-time camera clock. No decompiled code was
copied. Regressions cover native admission, quiet low-load changes, reset,
fixed-rate equivalence, and isolation from custom views/follow state. This is
static call-chain and synthetic integration evidence; gameplay feel remains
unmeasured.

The same action's spatial-list calls were traced to `IExplosion` registration
(`0x675E10`, constructor `0x6872D0`). The CC0 `IExplosion.h` getter order and
derived PC getter instructions (`0x6874C0`, `0x6874D0`) identify expansion speed
and maximum radius; the camera's radius margin is 2 m. Active wave snapshots
now feed that native admission path, in order after the body request. Synthetic
checks cover radius boundaries, zero distance, real elapsed seconds and
same-tick admission order. This does not establish live explosion timing or
complete cinematic effect parity.

## Wall-contact and pursuit-moment clock audit

The 2026-10-21 source audit found that native acceleration and custom contact
shake already read the completed player wall response. A regression now drives
the actual player integrator into a synthetic resident collision barrier and
checks native Bumper/Hood/Close/Far admission and custom presentation, rather
than injecting a shake request. Low-load thresholds remain unchanged. This does
not resolve an unrecorded report of a particular wall impact feeling too quiet.

The audit also found a host-time mismatch: physics included the pursuit moment's
speed multiplier, but the camera's within-frame sampling cursor omitted it.
The camera now receives the frame-frozen multiplier used for those physics ticks,
including any game-speed override, and retains the integrated Speedbreaker ramp.
This prevents consuming later contact timestamps during a slow pursuit moment.
The clock correction follows the written host invariant; no additional original
binary behavior or universal camera-shake amplification is inferred from it.
