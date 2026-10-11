# Trailer constraint evidence

Spec: [trailer constraints](../specs/trailer-constraints.md), written before
implementation. The private executable has the identity recorded in
[ground response](../specs/ground-contact-response.md). CC0 reference revision:
`80993f35224c61d2f1b135beba76c6f8234c7940` (credited in NOTICE).

Inspected PC entry points include `0x48AF30` (fulcrum placement), `0x48AD40`
(rotation arc), `0x48A910` (joint moment reaction), `0x486EC0` and `0x486BD0`
(point tensor), `0x48C270` and `0x48B290` (angular limit and correction),
`0x48D050` and `0x48B070` (limit construction), `0x6718F0` (named joint),
and their matrix/quaternion helpers. Five synthetic joint vectors and 24 angular
cases were executed in isolated x86 memory. Regression tests compare measured
linear/angular velocity, constraint frames and exceeded angles.

Installed checks match both conical/prismatic constraints for all six variants:
about 10.98 degrees tilt and 80.54 degrees yaw. Five variants, including every
tractor used by installed road traffic patterns, remain finite and hitched in
flat-ground driving; maximum settled anchor separation is under 0.00002 m.
Complete synthetic turning/braking/impact traces agree bit for bit at
5/15/30/60/120/144/240 FPS. The unused cement model's rotated body/suspension
frame still needs a spawn adapter and original-game comparison. Native angle
lookup differences, singular rotations and the immobile limit branch are
explicit limits; these checks do not establish full live-game parity.
