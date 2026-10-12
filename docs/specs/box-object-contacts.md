# Box object contacts and penetration correction

Evidence: private inspection and isolated execution of the user's PC 1.3
executable (SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`).
Geometry setup `0x4876F0`, box intersection `0x4879B0`, and segment/plane
intersection `0x4875B0` were measured with synthetic boxes before implementation.
Original code, executable bytes and private reports are excluded from the repo.
The CC0 reference revision and licensing boundary are in
[pc-object-contact.md](pc-object-contact.md).

## Face selection

Each box supplies current centre, three unit axes, nonnegative half dimensions,
and centre displacement since its previous prepared pose. Include movement of
the primitive's offset during rotation. Visit the first box, then the second;
within each box visit axes in order right, forward, up (0, 2, 1).

Treat the visited box as the reference and the other as the incident box. Orient
the reference axis toward the reference minus incident **previous** centre;
an exactly zero projection keeps the positive axis. Find the incident support
point along this axis. Each incident extent contributes positively/negatively
for a strictly positive/negative projection; exactly parallel extents contribute
zero, producing a face centre rather than an arbitrary corner.

Signed overlap is the current centre difference projected onto the oriented
axis, minus the reference half dimension and the three absolute projections of
incident extents. Reject the pair when any signed overlap is positive. Score
each axis by signed overlap minus relative travel projected onto that axis.
Keep the greatest score, comparing the extended intermediate with the stored
single-precision score. Subtract each extent projection with a single-precision
store between subtractions; rounding can break mathematically equal scores.
Keep its
current overlap, oriented normal, support point and owning reference box.
The output normal points toward the first box. The output point belongs to the
incident box; its identity must survive normal reversal.

This is a displacement-aware six-face test, not a least-current-overlap test
over fifteen axes. A box that crossed a thin object in one tick can therefore
be corrected toward the side it came from rather than ejected out the far side.

## Contact point clipping

Test the support point against the reference face's other two axes, in ascending
order: (1,2) for face 0, (0,2) for face 1, (0,1) for face 2. Orient each side
axis using point minus reference centre **plus** relative travel. A point on a
side plane is inside. If it is inside both side planes, return it unchanged.

For a point outside a side plane, form incident edges by reflecting the support
point across each incident centre axis, in order 0, 1, 2. Intersect each segment
from reflected point to support point with the reference side plane. Reject
parallel segments with absolute normal projection at most `0.000001`, and
intersections outside the closed segment.

Accept the first intersection strictly behind the selected reference collision
face and inside the remaining two reference planes. For the collision axis,
depth is minus projection onto the selected oriented normal; for the other axis,
choose its sign using candidate displacement plus relative travel. Update point
and penetration from that intersection. Otherwise continue to the next side;
if neither an initial inside point nor a valid edge intersection exists, report
no hit. Some corner overlaps intentionally produce no hit in the original.

## Separation before response

The original higher-level handler adjusts the point with the body whose surface
contains it. Box reference A means point on B; reference B means point on A.
Sphere/sphere points belong to A; sphere/box points belong to the box.

Reject penetration at most `0.000001`. An immobile body receives no displacement.
For two mobile bodies, use stored mass proportions, except two smackables use
their normal linear speeds when their sum has magnitude above `0.000001`.
For approaching smackables, each movement share is its own normal speed divided
by their sum. For a separating pair, swap the shares. If one exceeds one,
clamp it to one and set the other to zero.

Translate the contact point with its owning body's displacement, and translate
both temporary collision centres with their bodies. Temporary mass, inertia and
local centre modifications are selected **before** separation. When both moments
are immobile, release any with a positive breaking threshold before deciding
whether separation is possible. Then run the 32-iteration object response.

## Measured examples and scope

### Compound dispatch

**[verified]** For an eligible pair of bodies, visit the first body's enabled
object primitives in list order, then each enabled primitive of the second body.
Prepare the first primitive from the body's current pose at the start of its
outer iteration. Prepare each second primitive when its pair is visited. Run
response immediately for every successful narrow phase; do not retain only the
deepest overlap. Keep the original previous primitive centres for the fixed tick.
Separation also translates the prepared current collision centres. Repreparing
from the updated poses before the next pair retains this position change;
the response does not rotate either body pose.
An earlier separation can change a subsequent primitive's position and travel,
even when its impulse solver reports no approaching contact.

The installed car/lamp regression has a deeper nonclosing intact-body contact
followed by a shallower approaching roof contact. Read-only PC dispatch and
isolated impulse execution confirm the later pair releases the attachment;
the measured one-mobile/one-attached separation is applied between those calls.
The slow control remains attached. This establishes the compound pair behavior,
not the global broad-phase ordering or a full original gameplay trace.

A synthetic compound/attached-pole control has a nonclosing downward overlap
of 1.75 m. Separating that pair brings its upper primitive to a valid rightward
contact of 1 m, which exceeds the attachment threshold. Preparing every contact
before response would miss this later pair. Disabling the upper primitive keeps
the attachment. Native dispatch and isolated impulse execution confirm both
outcomes with the same measured separation adapter.

Equal boxes with half dimensions (1,1,2), centres (0,0,0) and (0,0,3.8),
no travel: point (0,0,1.8), normal (0,0,-1), penetration 0.2, point on B.
A small box with half dimensions (0.2,0.7,0.3) at (0,0,2.2): point (0,0,2),
penetration 0.1, point on A. A first box at (0,0,5), travel (0,0,8), crossing
that small box at the origin: normal (0,0,-1), point (0,0,-0.3), penetration 7.3.

Regression tests use synthetic measurements. Randomized private comparison
checks hit/miss, point, normal, penetration and point ownership, with tolerances
for x87 versus Rust arithmetic. This does not establish live gameplay parity,
ground/wall parity, or agreement for an entire compound body interaction.
