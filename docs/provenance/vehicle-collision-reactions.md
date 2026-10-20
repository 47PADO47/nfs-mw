# AI collision reaction integration

2026-10-20 follow-up: installed E: source records identify 17 vehicle profiles
whose linked body sets object-contact immobility. A live construction audit
contained 186 carrier/carried-car contacts exposing the missing runtime flag.
Matched PC constructor and base-modifier inspection confirms the Moment starts
with its own source/attachment immobility. Partner source immobility only gates
assignment of the attachment breakaway impulse; it does not clear initial
immobility. A later capture exposed 86 attached prop appearances against an
immobile vehicle which retained immobility with zero breakaway impulse. The
corrected two-stage rule matches 35,292 classified prop appearances and their
breakaway impulses. This is independent of temporary `NO_CAR_EFFECT`.
The material reader and native vehicle/prop and vehicle/vehicle
paths now preserve that flag. This changes temporary contact moments rather
than stored mass, integration, or ground response. Synthetic regressions cover
unchanged carrier motion with retained notifications, retained bilateral
immobility, and attachment breakaway eligibility; installed coverage checks
every linked vehicle body.

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
- Native live-goal integration now selects installed pit, ram, pull-over and
  static-roadblock overrides. Stored physical mass/inertia remain independent
  of these temporary contact parameters. The complete original goal state
  machines, stun handling and matched live impact comparison remain open.
