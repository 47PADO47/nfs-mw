# Countdown controls provenance

- Date: 2026-10-26.
- Contract: [manual shifting](../specs/vehicle-manual-shifting.md), section 2 and player input order.
- Selected PC executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- Clean analysis executable SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Original PC player input routines at `0069cef0` and `0069ce20` corroborate the
  decompiled PInput control order: shift request, auto-reverse, idle braking,
  then handbrake priority. Staging rejects desired button gears above first;
  auto-reverse does not query staging.
- An isolated execution of the original FetchInput tail covers 48 directed
  boundary cases. It includes gas plus brake, staged neutral/reverse, speed
  thresholds and two gear changes in one callback. Original instructions,
  private harnesses and vectors remain outside the public tree.
- GO remains after the last staged physics tick, consistent with retained
  callback captures. This evidence does not establish every countdown frame,
  complete drag heat behavior, physical trajectories or live gauge acceptance.
- Clearing drag launch mode on race teardown is a remake lifecycle correction:
  race-owned rules and drag RPM presentation must not remain in free roam.
