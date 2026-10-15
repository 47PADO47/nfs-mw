# Original camera obstruction and view gates

## Scope and evidence

**[PC native checked]** Covers the seven original driving POVs and their
look-back, player visibility and static-world camera clearance. The custom
Chase and Dynamic FOV cameras are outside this contract.

The matched PC Cubic update and its vtable consumers establish the behavior
below. Isolated unchanged native instruction runs corroborate 14 view gates,
49 view switches, 12 wall-clearance cases and 16 floor-height cases. Synthetic
world query replies isolate camera math; they do not establish complete world
collision or visual parity. See [provenance](../provenance/original-camera-obstruction.md).

## Outside, visibility and look-back

An outside POV is Close, Far, Super far, Drift or Pursuit (indices 2 through 6).
Bumper and Hood are inside POVs.

| POV | Looking forward: render anchored car | Looking back: render anchored car |
|---|---|---|
| Bumper | no | no |
| Hood | yes | no |
| Outside views | yes | yes |

The suppression applies only to the camera anchor's vehicle, identified by its
world ID. Other cars remain visible. Native flare rendering uses the same
gate. Hood is an active hood camera only when its look-back control is released.
Suppressing the entire player car in ordinary Hood view is incorrect.

Look-back is a held boolean. The reported look-back angle is half a turn when
held, zero otherwise. Cubic eye and look target horizontal local coordinates
are multiplied by -1 while held, retaining their vertical components. This
switch is immediate; a decorative orbit or 180-degree interpolation is not
part of this path.

Selecting an unchanged view does nothing. Switching between two outside views
preserves the follow state. Any changed-view transition involving Bumper or
Hood requests a follow-state snap. Snap remains pending until the next camera
update. This is separate from Speedbreaker zoom and normal follow dynamics.

The ordinary camera hotkey skips original views whose selected car profile's
fullscreen SELECTABLE field is false. Custom Chase and Dynamic FOV remain in
the host cycle. Direct overlay selection still exposes every original POV.
Body masking affects only the main driving view; free-camera inspection,
shadow feeds and reflection feeds keep their existing vehicle instances.

## World wall clearance

Only outside POVs run the world-clearance stage. It follows the initial camera
matrix, feedback and noise stages. Input positions and normals here use the
camera's Z-up coordinate system.

Let C be the car geometry anchor and E the current eye. Define:

- D = C - E and d = length(D).
- For d below 0.001 m the wall stage does not produce a correction.
- n = D / d.
- The native FOV angle uses a full turn of 65536 units. Set
  q = clamp(FOV_units * 0.00007847377128200606, 1, 1.5).
- side = cross(world_up, n), without normalizing side.
- width = q * d * 0.44.
- r1 = normalize(D + side * width).
- r2 = normalize(D - side * width).

Cast two rays from C to E - 0.9 * r1 and E - 0.9 * r2.
The original query passes collision flag 2, selecting barrier lines only.
It does not test road triangle strips in this stage. Its exclusion mask is
zero; barrier hits are accepted from either side with normals facing the
ray origin. Ground sampling is a separate later operation.
The native instance collector first rejects nonzero scenery groups whose
activation byte is zero. An exclusion mask of zero does not bypass this
separate eligibility check. The current free-roam host has no active groups,
so camera rays must skip grouped instances, consistently with floor queries.
Otherwise invisible race/scenery barriers can contract the camera on open
roads, with small height changes making their strict vertical ranges switch
between hits and misses. Ordinary ungrouped walls still obstruct both ways.
These are two splayed rays with
endpoints extending past the eye, not a single center ray or swept sphere.

For each hit H with unit normal N, compute:

    deficit = (distance(C, ray_end) - distance(C, H))
              * (1 + 0.6 * (1 - abs(dot(N, n))))

The desired clearance is the maximum deficit and zero. With retained clearance
history A, the native response is:

    applied = (desired + A) * 0.5
    A = A + desired - applied
    E = E + n * max(applied, 0)

The correction moves the eye toward the car. History persists between updates.
With no new hit, a prior positive clearance decays by half each native update.

The camera collider broad phase uses midpoint (C + E) / 2 and radius
min((distance(C,E) + 1) / 2, 24.9). That broad-phase sphere does not replace the
two fine rays. A separate 0.7 m virtual minimum-distance getter exists but is
not consumed by this wall-clearance stage; do not add it to these equations.

Its cached triangle collection uses the shared predictive-region policy and
horizontal candidate predicates described in
[cached terrain reference](vehicle-ground-reference.md). The inspected PC
instance collector ignores the nominal sphere/cylinder distinction and has
no additional vertical cutoff. The camera's instance exclusion mask is zero.

## Ground clearance and preserved aim

After wall clearance, normal-mode ground sampling uses the eye's horizontal
position and a query height at least C.z + 0.5. A valid sample raises the eye
to at least ground_z + 0.25. The original tunnel/overpass state instead samples
at the eye's actual height and also caps the eye to ground_z + 4.5.

The state comes from the view's retained precipitation state, independently
of visible rain. Its producer queries the first containing track zone of
each kind: Tunnel (3), Overpass (4), OverpassSmall (5), Garage (7). Polygon
queries use the previous published camera's horizontal position; elevation
checks use the current Cubic geometry anchor's height. A zone qualifies only
when anchor height is strictly below its elevation. A qualifying tunnel
sets the state and establishes the nearest-cover elevation; otherwise the
initial elevation is 9999. Overpass/small/garage zones additionally require
elevation below that retained nearest cover, then lower it. The floor branch
consumes the OR of tunnel and overpass flags, not the nearest zone identity.
Render Z-up coordinates map directly to zone (x,y), with anchor.z as height.

This is a cached environment result, not a new ray to arbitrary overhead
geometry. A host must preserve the previous-view sampling relationship and
must not classify the player as covered merely from polygon membership while
above a bridge or garage.

This is a fresh selected-face query over the pre-wall collider's triangle
collection. Lift its query height by 0.025 m; accept containing faces whose
vertical gap is strictly greater than -1 m. Thus a face up to 1.025 m above
the raw query can be eligible. Preserve native face order/continuation rules;
there is no finite downward-ray endpoint. Replacing it with an arbitrary
deep ray changes both stacked-surface selection and above-point acceptance.

The original rebuilds the matrix after moving the eye. It preserves the
original car-relative projected aim and roll rather than simply replacing the
eye in a fixed target matrix. At a minimum, a host must keep the original
target direction/aim contract stable through obstruction correction.

No valid ground sample means no floor clamp. Bumper and Hood skip the
outside-view wall and floor stage entirely.

## Secondary avoidable-body clearance

The camera maintains a list of avoidable IBody objects, including attached
vehicles and hierarchy props whose CAMERA_AVOIDABLE attribute is true.
Each body supplies its transform, half dimensions and linear velocity.
This is not a proximity search over every rendered prop.

The outside camera has a separate elevation stage after static clearance.
For each avoidable in list order, inflate each half dimension by 0.85 m.
Let L, W, H denote inflated forward, right and up half extents. Reject bodies
whose horizontal center distance squared is at least L*L + W*W, or whose
absolute vertical center distance is at least 2*H.

Transform the eye into the body's local axes. For local forward x and right y:

    fraction = (x/L)^4 + (y/W)^4
    roof_z = H * sqrt(sqrt(1 - fraction))

If fraction is at least one or the eye is above this roof, continue to the
next body. Transform the roof point (x,y,roof_z) back to world space. Return
its vertical distance above the eye only if that distance is no greater than
sqrt(L*L + W*W); otherwise return zero. The first qualifying body wins.

Raise the temporary eye by positive corrections until clear, at most 16 times.
If horizontal car speed is below 1 m/s, the saved and new first view-matrix
rows have dot product greater than 0.9, and the new raise is smaller than the
saved raise, retain the saved raise. This prevents stationary eye bobbing.
Retain the applied raise using the same half-weight recurrence as wall
clearance. Rebuild the view with the aim-preservation helper.

The original skips this stage during the vehicle-underpass pulse below.

## Avoidable-body up influence

Above 5 m/s, the follow target adds an up-vector influence from the same
body list. For each body, use its center relative to the car and reject squared
distance outside the strict interval (0.64,49), or forward cosine at most 0.5
against normalized car velocity. Set:

    attenuation = clamp(4.84 / distance_squared, 0, 1)
    relative = clamp((length(car_velocity - body_velocity) - 15) / 85, 0, 1)
    opposing = max(-dot(normalized_car_velocity, normalized_body_velocity), 0)
    influence += normalized_relative_position * attenuation * (relative + opposing)

Zero body velocity produces zero normalized body direction. There is no
extra arbitrary strength or camera-mode-specific multiplier.

Native body eligibility and transforms must be supplied by the host. Using
all props, omitting the player's body or reordering overlapping candidates
changes the original behavior. Twenty-one isolated native cases corroborate
the shape and velocity equations; live crowded-scene validation remains open.

## Vehicle-underpass pulse

The original separately tests active, fully loaded other vehicles, excluding
the camera anchor's world ID. This test does not include hierarchy props.
Take the old camera forward vector and the candidate vehicle forward vector.
Reject absolute dot product at least 0.707. Predict the anchor position by
adding 0.05 seconds of its current velocity. The predicted point must be
strictly inside the candidate's native rigid-body radius, and its local
forward and right coordinates must be strictly inside the corresponding
uninflated half dimensions. No extra local-height test is added.

When any candidate passes, refresh the under-vehicle timestamp. The native
timer is an integer counter at 4000 ticks per second. Convert elapsed host
seconds to the nearest timer tick, retaining sub-tick time between updates.
Let t be that integer delta multiplied by the authored float 0.00025:

    phase = clamp(1.2 - 1.25*t, 0, 1.2)
    strength = phase when phase <= 1, otherwise 2 - phase
    eye.z -= 0.9 * strength

Evaluate the phase subtraction with extended intermediates before storing
the float phase. This matters at 3840 ticks (0.96 seconds): the original phase
is nonpositive and secondary elevation resumes. Tick 3839 still suppresses it.

Apply this to outside POVs before feedback/noise and static clearance.
Skip secondary body elevation while strength is positive. The target is
unchanged. Twenty-three unchanged native predicate cases corroborate
registration filters, prediction, radius, orientation and local bounds.

## Frame-rate behavior and validation

The original wall and secondary height response include per-update halves.
Do not run those recurrences once per render frame in a high-FPS host.
Advance retained camera state on a stable update clock and interpolate only
presentation, documenting the chosen clock as a host adaptation. Never
multiply the per-update coefficient directly by render dt.

Required checks:

- Every visibility gate and all inside/outside transitions, including reentry.
- Held look-back flips horizontal coordinates without an orbit transition.
- Both offset ray endpoints, FOV factors and oblique-normal weighting.
- Positive history decays without new hits; no-hit zero history stays zero.
- Floor minimum, invalid floor samples and special-state ceiling.
- Equivalent replay durations at 5, 60, 144 and 360 render FPS.
- Other vehicles remain rendered when the player is hidden.
- Custom cameras retain their existing behavior.

Full live original scene comparison, dynamic body registration/list order,
all noise sources and rare special-state floor behavior remain separate
validation requirements.
