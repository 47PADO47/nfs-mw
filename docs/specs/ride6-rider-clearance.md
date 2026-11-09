# The rider kept out of his bike's body when it leans

A user's playtest: "when leaning around, biker parts go through the bike". At a large lean the rider's legs, torso and arms ended up inside the tank, the
fairing, the seat and the exhaust. This page says why, what is measured and what is done about it. Provenance:
[ride6-rider-clearance](../provenance/ride6-rider-clearance.md). Code: `crates/nfsmw/src/scenes/world/drive/rider/{clearance,pose}.rs`, the converter's
`mods/ride6-bikes/src/rider/body_section.rs`. Earlier: [ride6-rider-knee-foot](ride6-rider-knee-foot.md) (the lean clips are sweeps),
[ride6-rider-animation](ride6-rider-animation.md).

## 1. Why he clipped

The riding pose is one animation, authored on one bike (a supermoto), and **swept by the lean**: the lean clips take the pelvis 15 cm over and the knees 19 to 21 cm out
at the full lean ([ride6-rider-knee-foot](ride6-rider-knee-foot.md), section 2), and the physics' rider adds his own hang-off shift (`Inertia`, up to 15 cm sideways) and roll on
top. Nothing in the pose knows the shape of the bike under him: the outer knee, which the sweep brings **toward** the bike's centre line as the body goes over, the
elbows and the chest meet a tank, a fairing or a scooter's body that is wider or taller than the reference bike's. RIDE 6 corrects this per bike with the animation correction table
(`knee`, `lean_knee`, `elbow`, `pole_elbow_cm`, `aero_tank`, `muffler`: carried in `contacts.json` as `correction`, **not played**: they need the blueprint's control rigs).
This page replaces them with a measurement of the bike's own mesh.

## 2. What is done

**The bike's cross-section** (`body_section` of `contacts.json`, [formats](../formats/ride6-rider.md)). The converter cuts the side view around the rider's root (0.55 m behind it,
0.65 ahead, 0.70 below and 0.35 above) into cells of 3 cm and writes, for each, the largest `|y|` of the vertices of the body-role parts that fall in it (the chassis, the exhaust, the
hand guards: the tank, the fairing, the seat, the frame, the engine), in millimetres; a cell with fewer than 2 vertices is 0 (a bolt is not a wall). Only what is fixed to the chassis counts: unskinned
parts and vertices skinned mostly to `BONE_CHASSIS` or `VEHICLE_BASE`, not the fork, the handlebar, the levers or the wheels, which move, and
which the hands hold on purpose. A cell is read as the largest of the four nearest, so a point near a wall is taken for being at it.

**The rider's body** is the capsules of his own physics asset (`ragdoll.json`: the thigh, calf, upper arm, forearm and the torso's bones other than its first), three
sample points on each axis. A point is **inside** when it is in a cell with a body and `|y| - radius < half_width - TOLERANCE_M`; its depth is the difference.
The pelvis (it sits on the seat on purpose: [render-integration](ride6-rider-render-integration.md), section 8), the hands (on the grips) and the feet (on the pegs) are not guarded.

**What a rider does about it**, in `pose::animate`, after the body has leaned for the grips and the hands and feet have reached their targets:

1. **A knee or an elbow inside the body is swung out** about the line from hip to foot (shoulder to hand): the same two-bone solve is run again with its pole at the joint's place
   moved sideways, outward on the limb's own side, by the deepest depth of any point of the limb divided by the share of the segment the joint carries (a point at the knee moves with it
   entirely, one half way along the thigh by half; a point nearer than `MIN_LEVER` = 0.25 to the hip, the shoulder, the foot or the hand is not chased), at most `MAX_PUSH_M` = 0.12.
   The hand and the foot stay on their grip and peg (the solve reaches the same target). A leg let off its peg by half or more (the old-school foot out) is the clip's own and is left alone.
2. **A chest on one side of the tank is moved over** (before the limbs are solved, so they follow): the deepest torso point that is beyond `TORSO_OUTER` = half of the section's half width
   from the centre line is moved outward by its depth, at most `MAX_TORSO_PUSH_M` = 0.05, by shifting the body bone (the same one the hang-off moves). A point nearer the centre line is over the
   tank, which a sideways push cannot cure; it is left alone (it is what the lean's `reach` does with the spine).

| Constant | Value | Why |
|---|---|---|
| `TOLERANCE_M` | 0.01 m | the suit's padding and the cell's slack (3 cm cells read as the largest of four) are a centimetre either way; also the check's tolerance below |
| `MAX_PUSH_M`, `MAX_TORSO_PUSH_M` | 0.12, 0.05 m | bounds for a wrong section, not modelling values; the clips' own knee-out is 19 to 21 cm |
| `MIN_LEVER` | 0.25 | a push at the knee moves a point a quarter of the way along a segment by a quarter of the push: more is chasing |
| `TORSO_OUTER` | 0.5 | a chest point past half of the half width is on a flank, nearer the line it is over the tank |
| Cell, extent, vertex count | 3 cm; 0.55/0.65/0.70/0.35 m; 2 | the knee is 15 cm across and a flank changes over several centimetres; the extent holds the pegs, the bars and the chest |

Nothing is done without a measured section (`contacts.json` made before this has none), without a ragdoll, or with the environment variable `NFSMW_RIDER_CLEARANCE=0` (to compare).
A rider is posed as before then. Cost: three samples on about nine capsules and, for a limb that clips, one more solve and one more forward pass of the skeleton.

## 3. The numeric check

`clearance_probe` (`rider/clearance_probe_tests.rs`, ignored, needs a scratch mod folder):
`R6_CLIP_FILES=<mod>/files R6_CLIP_CAR=<ID> [R6_CLIP_GRAPH=graph_race.json] cargo test -p nfsmw clearance_probe -- --ignored --nocapture`. It poses the rider of one bike at 0 to 60 degrees of lean in steps of 5, to
each side, with the body shifted to the outside and to the inside of the lean by the largest the rig allows, as the game does, with the keeping-out off and on, and prints in centimetres the deepest
capsule point and the deepest **skin vertex** of the torso and head, the arms and the legs inside the section (the vertices skinned mostly to the pelvis, the hands and the feet excluded: they are
meant to touch). A clip is a vertex deeper than `TOLERANCE_M` plus 1 cm.

## 4. Result

Not measured when this page was written (the code was written without a build, by order): the table is to be filled in from the probe on a sport bike, a supermoto, an adventure bike and a scooter.

## 5. Not done

The bike's own `knee`, `elbow` and `aero_tank` corrections; the clearance of the head and the helmet against a screen; a push **up** for a chest that is over the tank; the feet against the exhaust and
the swingarm; a bike with a section but no ragdoll rider.

## 6. Tests

`clearance_tests.rs`: the section's lookups and its damaged forms; a clear rider is left exactly as he is; a knee inside a tank is swung out to its own side by no more than the bound, the foot
stays on its peg and the depth falls; a chest on a flank is moved over and one over the centre line is not; only the limbs' and the torso's capsules are guarded. `body_section.rs`: the grid's cells and counts.
