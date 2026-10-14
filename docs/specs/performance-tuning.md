# Performance tuning sliders

Sources: the original PC 1.3 executable inspected locally with Ghidra on
2026-10-13; the installed language table; and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reference
declarations and behavior in `PhysicsTunings.h`, `CustomTuning.cpp`,
`CustomTuning.hpp` and `PhysicsUpgrades.hpp`. No decompiled code is copied.

## Range, ordering and neutral equipment [verified static PC]

The seven values, in stored order, are steering, handling, brakes, ride height,
aerodynamics, nitrous and induction. All seven use floating-point lower and
upper limits of -1 and +1. Left/right moves by one tenth of the complete range:
0.2. The original screen displays the normalized position from 0.0 to 10.0,
with 5.0 neutral. A neutral stored tuning value is zero.

A remake integer slider from -5 through +5 has eleven positions. Divide its
value by five to obtain the physics tuning value; do not divide ride height by
a different constant. Ride-height tuning is directly an added inch offset.
Clamp and validate integer positions before applying the request.

The original customization record holds three complete tuning profiles and an
active-profile index. The runtime overlay supplies three session-local profiles
and active-profile selection. Career-save persistence and original frontend
presentation remain separate concerns.

## Availability [verified static PC]

The original frontend tests the customization record's ordinary installed
package level, not the vehicle's maximum level, stock hardware or Junkman bits.
The player car and customization record must exist.

| Slider | Required ordinary equipment | Negative/left | Positive/right |
|---|---|---|---|
| Steering | Suspension/chassis stage greater than zero | Slower response | Quicker response |
| Handling | Suspension/chassis stage greater than zero | More sliding | Less sliding |
| Brakes | Brake stage greater than zero | Rear bias | Front bias |
| Ride height | Suspension/chassis stage greater than zero | Lower | Higher |
| Aerodynamics | An installed body kit or spoiler different from the stock part | Less downforce | More downforce |
| Nitrous | Nitrous stage greater than zero | Longer duration | More power |
| Turbo or supercharger | Induction stage greater than zero | Lower RPM boost | Higher RPM boost |

The directions above summarize the installed help text rather than quoting it.
The induction title is Turbo or Supercharger according to the vehicle's
installed-upgrade induction type. Both have the same slider direction.
The aerodynamics gate compares body/spoiler part identity with original
equipment; suspension upgrades, custom paint or a factory spoiler do not unlock
it. The original upgrade conditions do not require the highest package stage.

The overlay must disable editing unavailable sliders with a reason. Until it
can select an actual body kit or spoiler, its aerodynamics tuning remains
locked. Returning a required ordinary package to zero retains the stored tuning
value while locking its editing. The original vehicle tuning getter returns
the active profile directly without checking these frontend equipment gates;
the physics consumers continue to use retained values. Package storage and
tuning-profile storage are separate. Validate ranges even for locked values;
do not clear or mask them merely because an equipment gate is unavailable.
Explicitly resetting a profile can return all seven values to neutral.

## Physics consumers [existing behavior specs]

Pass normalized tuning values to the existing consumers, without changing
base vehicle attributes or applying arbitrary per-car gains.

- Steering range scales by `1 + 0.2 * steering`.
- Handling traction-circle lateral/longitudinal scales are
  `1 + 0.2 * handling` and `1 - 0.2 * handling`.
- Brake front/rear scales are `1 + 0.5 * brakes` and `1 - 0.5 * brakes`.
- Ride height adds the normalized value, in inches, to both axles' installed
  ride height and supplies the resulting suspension/center-of-gravity rules.
- Drag and aerodynamic downforce scale by `1 + 0.25 * aerodynamics`.
- Nitrous uses the power/duration tradeoff in
  [vehicle-input-induction-brakes](vehicle-input-induction-brakes.md).
- Induction uses that spec's low/high-boost and spool-RPM tradeoff.

PC hardware caches use untuned data: the engine's shift-map setup supplies no
tuning pointer to peak-torque/power helpers. The shift-point search also passes
the induction helper's RPM/spool arguments in reversed order. Its effective
boost is zero for ordinary positive idle speeds, leaving naturally aspirated
shift thresholds. Retain this shipped behavior instead of changing automatic
shift maps when the induction slider moves. Hardware package replacement
still rebuilds maps from the new engine/transmission attributes.

The detailed consumers have separate provenance in the corresponding vehicle
specs. Slider range and frontend availability evidence do not establish that
all tire, suspension, powertrain or collision behavior matches a live game.

## Validation boundary

PC limit-table bytes and slider initialization establish the eleven positions;
the availability routine and installed language help establish the gates and
directions. Isolated numeric replay and synthetic tests can verify individual
consumer formulas. Live original/remake driving comparisons, full profile/save
integration and visual menu parity remain separate. Body/spoiler selection and
its availability gate are covered by [garage model customization](garage-model-customization.md).

Isolated PC replay covered seven limit pairs, eleven nitrous tuning positions,
66 induction vectors, three active-profile getter results and three shift-map
builds with different induction strengths. The latter produced identical
thresholds, confirming the reversed-argument behavior. No original process was
launched or modified for this evidence.
