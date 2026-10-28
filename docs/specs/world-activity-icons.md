# World activity icons

Original retail PC 1.3 stores a table of world-icon types, model hashes and
particle-group hashes. The type numbers and world model/particle pair are
cross-checked against the GIcon interface in the CC0 reference and the names
in the user's installed InGameA and ATTRIBUTES files. See provenance.

Sprint, circuit, drag, knockout, tollbooth, speed trap and rival use their
matching CHECKPOINT_GEOMETRY_*_00 models and fxgame_icongrp_* groups. Safe
houses, car lots and custom shops use SAFEHOUSE/safe, CARLOT/lot and
CHOPSHOP/chop respectively. Hiding spots use HIDECAR/hidecar. Pursuit breaker
icons have no model in the table and use the pursuit particle group.

## Placement and visibility

Retail PC static inspection of `GZoneTrigger` construction, `RefreshZoneIcons`,
`UpdateTriggerAvailability`, `UpdatePursuit`, `UpdateIconVisibility`,
`GIcon::SnapToGround` and `GIcon::SetPosition` establishes these rules:

- Race icons use the engage trigger's Position and TargetActivity, not the
  race start grid. Place icons use their zone Position. Both receive zero
  rotation. Geometry and particles share that transform.
- Hiding icons use the HIDDEN track zone's stored position and elevation,
  not its polygon centroid, and also receive zero rotation.
- Ground snapping only applies to an authored zero elevation. Explicit
  elevations must survive, including on bridges and stacked roads.
- In free roam, event, shop and car-lot icons disappear during a pursuit.
  Safehouse icons remain available during cooldown. Hiding icons appear only
  during cooldown, not during the active chase or after escape.
- Place-zone `BinIndex` (field 0x6CE23062) is an unlock threshold: the current
  bin must be no greater than that positive threshold. It is not an icon index.
- Race icons follow career event availability. Unsupported race categories do
  not produce a replacement icon. Model and particle assets are independently
  optional.
- Boss races use the rival model and `fxgame_icongrp_rivalr` group rather than
  the underlying circuit/sprint icon. The retail table's type 7 emitter hash
  matches this installed group.

These are static PC findings, not a matched original-game frame comparison.
The remake retains a 250 m admission distance; exact section-spawn ordering
and original global icon-pool limits are not established. Race/cutscene hiding
remains the world scene's lifecycle policy. Synthetic datasets without engage
triggers retain grid-based entries for compatibility; installed engage records
take precedence whenever present.
