# Vehicle force scalar order

Date: 2026-10-26. Sources: original PC executable behavior inspection,
retained player-vehicle boundaries and existing remake source. No restricted
implementation or captured game data is included here.

A carried-state diagnostic exposed force differences before the tire update.
Matched query geometry, compression and wheel velocity isolated speed-scale
interpolation and spring arithmetic. Original instruction-block replay confirms
weighted endpoint interpolation, progressive-spring product grouping and the
rounded reciprocal used for compression speed. The behavior contract was written
before changing production code.

The speed-scale candidate matches 88 outputs across 44 synthetic speeds under
the declared single-precision contract. The spring candidate matches 500
synthetic cases; grouping alone matches 491. Original live precision was not
captured, and extended-precision replay can differ. These results justify local
arithmetic corrections, not a global floating-point mode change or a full
vehicle parity claim. Public tests use generic synthetic inputs.

After these corrections, all mapped tire-call inputs agree at the initial
retained vehicle response. An isolated lateral helper comparison identifies
weighted load interpolation and distinct gain order for the angle-interpolated
and plateau branches. The specified candidate matches all 168 synthetic helper
cases under single precision with one prescribed descriptor. The live descriptor
count is not captured. Sliding friction and joined world behavior remain separate.

Further original-block inspection identifies division-before-load multiplication
for sliding friction and a rounded reciprocal in its low-speed branch. Applying
these orders reproduces both initial recorded rear-wheel slide components under
the same declared precision. Separate synthetic instruction probes distinguish
each operation order. This does not close all tire branches or world response.

The next unchanged-input comparison matches three complete wheel outputs and
isolates the fourth wheel's friction-ellipse limit. Original instructions multiply
drift, traction boost, static grip, load and pilot in that order, storing the
maximum before dividing by magnitude. The previous implementation starts with
load. These products differ by one single-precision step in the retained case.
The candidate matches 216 synthetic instruction-block cases; the previous order
differs in 42. A separate generic complete-tire case distinguishes final force
outputs with the two orders under the same declared precision and descriptor.

The complete-tire regression also distinguishes rounded squares/sum/square-root
lengths from the library hypotenuse function. Native skid, patch and ellipse
instructions use the former. A body-exact checkpoint can still conceal tire
history differences, so state and accumulated force agreement are reported
separately. The later wheel forward-projection block adds X and Z before Y;
direct original execution matches that order in 120 synthetic vector cases,
while the previous sum order differs in 42. Neither contract changes global
vector math or the stabilized tire policy.

Original loaded-tire instruction execution confirms that gripping feedback
divides by wheel radius before multiplying inertia and adding the stored base
force. Under the declared single-precision mode, this matches 16 captured
gripping boundaries and 45 synthetic cases; 16 synthetic cases distinguish the
previous ordering. The live floating-point mode remains unknown. This result
does not establish agreement for tire history or accumulated vehicle torque.

An original instruction-block probe also confirms division of traction by
absolute slip before multiplying the slip limit. All 32 retained tire boundaries
match this contract under the same declared precision. Thirty-six synthetic
block cases match; seven distinguish the previous grouping. This establishes
the scaling boundary, not all subsequent wheel-spin arithmetic.

The terrain-query tolerance instruction block adds horizontal travel to downward
travel before adding the radius. Original-block replay reproduces all 32
retained tolerances under the declared precision, including five differences
from the previous grouping. Thirty-two synthetic cases match, with six
distinguishing the groupings. The retained queries selected the same planes;
that observation does not establish behavior at a support threshold.

After those changes, the remaining observed torque differences are already
present at the aerodynamic boundary. The original downforce product starts
with speed, coefficient and 2000, then applies forwardness and upness. Its
instruction block matches 108 synthetic cases under the declared precision;
26 distinguish the prior product order. Original full-block replay identifies
downforce-vector differences at the three retained torque-divergence frames;
the matched drag and application coordinate do not alone prove the whole aero
path. Live precision and other driving situations remain unverified.

The original direction calculation first multiplies by a rounded speed
reciprocal and adds the Z and Y products before X. The complete instruction
block matches 54 generic synthetic cases, with 13 distinguishing the previous
sum order. Under the declared precision this also reproduces all eight retained
forwardness inputs; two differ from the previous sum order. Low-speed behavior
and the existing positive clamp are retained.
