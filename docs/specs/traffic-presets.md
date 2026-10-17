# Traffic presets and supported spawn ranges

Reference selects the installed PC experience used for comparisons. Extended
and Custom are remake enhancements, independent of geometry LOD. Sources and
limits: [traffic provenance](../provenance/ai-traffic.md) and
[original spawning](ai-traffic-spawning.md). No original code or assets copied.

## PC reference correction

Static checks of two installed PC executable images agree on a 350 +/-50 m
spawn band, positive velocity projected onto the randomly selected direction,
a final-lane minimum distance of 300 m and a population gate of 20 active
vehicles across classes. The GameCube-derived 200 +/-50 m / 150 m / 10 values
in the older spec are not the installed PC values. The pattern-record maximum
remains ten; it is separate from the active-vehicle budget. Runtime mod patches
and complete Xbox 360 simulation equivalence have not been established.

## Settings

| Policy | Reference (default) | Extended |
|---|---|---|
| Final minimum spawn distance | 300 m | 450 m |
| Random probe maximum, before projected speed | 400 m | 600 m |
| Total active vehicle budget | 20 by default, independent F8 control | Preserved |
| Maximum unseen range | 650 m | 1000 m |
| Minimum offscreen grace | 4 s | 8 s |
| Density scale | 100% | 100% |

The Video menu offers Reference, Extended and Custom beside detail options. Custom retains
the current values and directs the player to F8 Traffic. Editing an advanced
range value selects Custom. Density and actor budget stay independent of the
range preset and are only exposed in F8. F8 provides reset, live previews, named presets and A/B
comparison; Save profile persists startup settings. Old files without traffic
tuning keep existing explicit traffic counts and gain Reference range defaults.

The active budget is a ceiling, not guaranteed density: the player, chasing
cops and trailers consume space. Installed neighbourhood type rates, model
share caps and rare-vehicle maxima still apply. The density scale reduces the
context density before its original rate/time/distance tables; pursuit retains
its factor. Zero density stops new spawns without deleting visible actors.
Race-specific density/route plumbing remains incomplete in the world adapter;
presets do not manufacture a fixed race traffic setting.

Use a deterministic 30 Hz simulated-time activation clock and cached models.
Each activation tries one random probe, so failed lane/ground searches wait
at most another 1/30 second while the manager is eligible. The previous
half-second clock, even with four retries, could starve traffic on highways.
The original registration runs roughly every two rendered frames; 30 Hz
matches that cadence at 60 FPS while keeping simulation independent of the
actual rendering rate. This is not a verified high-FPS original schedule.
Disabled traffic resets the clock rather than accumulating a catch-up burst.
Original frame-dependent timing, inactive actor reuse and the
four-second new-instance timer remain timing-parity gaps. No startup or live
preset change flushes existing traffic. Lower budgets drain through recycling.

## Placement and retirement

Sample the wedge within +/-45 degrees around the focus direction. Add the
positive component of focus velocity projected onto that direction to the
probe distance. Preserve the driven centre's elevation for the road's 3D
closest-point search; flattening it to sea level biases sloping-road projection.
The free-camera focus still lacks a supplied elevation and retains its zero-height fallback.
Apply oncoming probability to this same projection. After
road projection, reject missing support, excessive snapping, final positions
inside the configured minimum and overlapping vehicles, including trailer
spacing. Explicit mod summons retain their deliberate near-player behavior.

Retirement must not overlap the spawn band: sanitize maximum unseen range to
at least spawn maximum plus 200 m. Keep the original density-dependent unseen
distance/time checks, but require configured grace before normal distance,
stuck or maximum-range removal. Visible finite cars, attached trailers and
visible detached parts stay protected. Invalid physics and off-world recovery
remain exceptional cleanup. Pursuit wave retirement remains separate.

The settings are bounded: spawn minimum 150-1200 m, spawn maximum up to
1400 m, unseen range up to 2400 m, grace 4-30 s, total budget up to 64 and
density scale 0-100%. Missing road support rejects activation; distant physics
waits while required collision is loading instead of integrating into a hole.

## Collision support

Request collision independently of colour visibility and custom shadows.
Reuse section workers but parse only collision for traffic requests: no texture
decode, mesh upload or expansion of main-view scenery. Prioritize live actors
and their short predicted paths, then a focus disk covering the spawn band and
projected speed. Prioritize sections near the centre of the forward spawn band,
after live actors, rather than filling the cache with near-player sections
that cannot host a new car. Bound support to 128 extra sections and two requests in flight.
Core scenery loading/readiness remains independent of auxiliary requests.

Collision ownership is shared across colour and traffic residency. Retiring
one owner cannot remove packs another owner still needs. Hand collision to
traffic when a full scenery tile retires, without retaining its GPU resources.
Offscreen support expires with the bounded policy; failures are reported and
never interpreted as valid ground. Mod map packs retain their existing path.

## Validation

Synthetic checks cover settings/profile migration, finite/bounded policies,
presets and menu cycling, projected speed, final-lane range, unseen grace,
visible-car protection and collision ownership across scenery retirement.
Installed checks must exercise real extended-distance ground and cached models
with ordinary shadow support disabled. A stationary screenshot establishes
startup/readiness, not absence of pop-in during a route or a performance claim.

F8 reports actual actors, occupied slots, counts within 150/300/600 m, collision
waits and cumulative activation/rejection counts. Range/grace sliders control
visibility, not density; increasing every slider can reduce nearby encounters.
