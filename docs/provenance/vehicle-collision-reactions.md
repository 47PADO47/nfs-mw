# AI collision reaction integration

- Sources and layout are recorded in
  [the specification](../specs/vehicle-collision-reactions.md), written before
  implementation. No original binary, attributes or private inspection text
  enters the repository.
- The isolated PC selection routine chooses front, front-side, rear and
  rear-side exactly as the specification states. A point on the transverse
  centre plane selects rear; absolute normal-forward projection exactly 0.707
  selects the end record. Six synthetic PC cases confirm these boundaries.
- Installed-data audit resolves all 31 collision profiles, 22 inherited AI
  defaults, 23 goal overrides and 113 linked vehicles. Individual police
  variants retain their own records; null default profiles remain null.
- Runtime regressions check partner eligibility, grounded player centre,
  positive versus nonpositive scales, goal selection and unchanged stored mass
  and centre of gravity. Compound player/traffic/racer collision traces are
  identical at 5, 10, 30, 60, 120, 144 and 240 FPS and through uneven frames.
- Original goal state machines, stun handling, pursuit mass/centre overrides,
  and live original-game impact comparison remain unverified or unimplemented.
