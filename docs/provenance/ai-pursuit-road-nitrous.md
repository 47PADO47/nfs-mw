# Road pursuit nitrous provenance

Date: 2026-10-24.

Specification: [road pursuit nitrous](../specs/ai-pursuit-road-nitrous.md).

- Original PC executable inspected after confirming its supported identity.
  Bounded native inspection covers road-action potential acceleration,
  available nitrous, update decisions and action-entry initialization. Both
  locally retained PC executables have identical bytes over those inspected
  functions. Static inspection establishes rules, not their execution in every
  encounter.
  Two additional bounded physics-information functions verify the boost and
  bottle-duration tuning used when matching nitrous capability.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reference,
  `AIActionRace`: used to identify and cross-check the behavior. The code here
  is independently written from the specification.
- A privately retained modded-PC user pursuit capture across heats 1-5 shows
  nitrous controls on ordinary heat-5 reinforcements, including distant units.
  The capture did not record action kind, so distance alone is not proof of
  which action selected a particular control. No traces are included here.

The existing close-pursuit nitrous action has separate rules and state. This
change must not replace those rules with the road-action hysteresis.
