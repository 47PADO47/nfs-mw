# Tire visual alignment

## Sources

- **[verified source]** This repository's `CarSim`, `CarRig`, car assembly and
  `Corner::posed`, MIT OR Apache-2.0. Physics wheel arms and displayed ecar wheel
  placement are separate geometries. Only suspension height currently connects them.
- **[verified source]** The original procedural marks code retains a stamp's
  corners while refreshing opacity/age for contact movements below 0.18 m.
- **[new design]** The correction below aligns visuals within this rewrite.
  It does not claim the original game's emitters used this algorithm.

## Placement

Keep the existing loaded-wheel flags, slip intensities, wheel order and physics.
For a car with assembled wheel geometry, obtain the bottom tread midpoint and
scaled width from the actual displayed wheel solid bounds and static transform.
Apply the same suspension and steering transforms as the visible wheel, without
rotating this geometric bottom point with wheel spin. Convert that world point
to physics axes and use the existing short ground projection/exclusions.
No nearby resident ground still means no emission. Missing wheel geometry keeps
the physics contact and the previous 0.24 m fallback width.

Stationary stamps may refresh their corners to the current contact and steering
direction while retaining the original sampling anchor. Finished moving strips
remain fixed on the road; refreshing opacity must not turn a strip into a stamp.
Marks use each contact's displayed tread width. History budgets and lifetimes
remain unchanged. Fixed-step history can still differ from the interpolated
car pose by a fraction of one physics step while moving quickly.

## Checks

Synthetic tests cover unspun bottom geometry, steering/suspension transforms,
varying tread widths and refreshing a stationary stamp without freezing its
position or destroying moving strips. An ignored real-install BMW test measures
physics-to-model mismatch and validates the displayed tread geometry. Local
captures check stationary wheelspin and the same moving handbrake route.
