# Original gameplay camera coverage

Scope is Bumper, Hood, Close, Far, Super Far, Drift and Pursuit. Chase and
Dynamic FOV retain their custom follow models. Collision protection shared
with a custom view does not turn its follow model into an original camera.

## Implemented contracts and evidence boundaries

| Stage | Established contract | Validation boundary |
| --- | --- | --- |
| Per-car POV data | Original view order, authored camera values, selectable flags | Installed-data loading and isolated native getters; not every car visually compared |
| Follow and aim | Native target generation, rolling spline history, body up/tilt and camera transients | Native arithmetic/pose fixtures and host timeline tests; a complete matched drive remains necessary |
| Lens | Native FOV conversion and explicitly selected reference scaling | Numeric projection comparison; installed mods can independently change projection |
| Static walls | Two native offset rays, camera-specific barrier producer, authored normal weighting, retained correction | Actual native barrier producer replay plus host world-query checks; arbitrary track scenes are not exhaustively covered |
| Floor | Fresh face selection over predictive cached candidates, minimum clearance, tunnel/overpass ceiling | Native arithmetic and environment-state fixtures; region-boundary and live enclosure transitions require scene validation |
| Avoidable bodies | Rounded roof clearance and nearby-body up influence | Native shape/influence fixtures; exact original registration order is not established by host actor ordering |
| Under another vehicle | Active/loaded other-vehicle predicate and timed pulse | Native predicate, pulse and timer-boundary cases; live trailer/stacked-vehicle behavior remains unverified |
| Main-view visibility | Bumper hides player, Hood keeps bonnet except look-back, other vehicles remain | Native visibility gate and host selection tests; every body-kit/flaring combination is not visually checked |
| High FPS | Retained camera clock and interpolated presentation | Host replay across render cadences; an explicit adaptation to native per-update recurrences |

## Known remaining differences

- Camera rays have a dedicated query with native determinant-numerator
  tolerance, unclamped intersection, stored-radius broad phase and authored
  horizontal normal scale. Its contract is separate from existing vehicle
  rays; their normalized endpoint tolerance must not be substituted back into
  the camera path. These finite native fixtures are not a whole-map guarantee.
- Free-roam treats scenery groups as inactive. It does not reproduce the full
  original race/pursuit group's activation lifecycle. The same eligibility
  policy must apply to camera floor and wall queries.
- Host actor order and lifetime supply the avoidance lists. They preserve
  explicit camera-avoidable flags and physical actor membership, but have not
  been shown to reproduce the original intrusive-list order when several
  roofs overlap or actors stream/spawn simultaneously.
- Active police, trailers and physical camera-avoidable props have host
  adapters. Complete original lifecycle equivalence for despawning, loading,
  detached parts, pursuit-breaker fragments and unusual actor subclasses
  remains unverified; actor presence in one test is not proof of all states.
- Tunnel/overpass state has separate cached precipitation and alternate
  anchor-only producers. Both use zone elevation checks. Exact presentation
  ordering across every weather/render configuration remains a host boundary.
- Drag-specific and cinematic/ICE camera paths are outside ordinary gameplay
  mode coverage. A zero/default input for an unavailable producer must remain
  documented rather than counted as evidence of parity.

## What the checks establish

Isolated native execution validates the instructions reached with the supplied
inputs. Synthetic world or zone replies validate downstream arithmetic, while
the separate actual barrier-producer replay validates its height and eligibility
rules. Neither proves complete original scene behavior by itself.

Underpass suppression skips secondary body elevation only. Static floor and
wall clearance still execute during its pulse. Helicopter noise must be
suppressed in covered state and use horizontal distance from the previous
camera eye; a 3D distance from the car is a different producer contract.

Final visual validation should compare the same car, upgrades, camera mode,
aspect ratio, projection convention, route and input sequence. Include ordinary
bumps, crests, low overpasses, tunnel entry/exit, grouped road closures, nearby
trucks, look-back, gear changes and Speedbreaker. Record the original executable
and loaded mod identities; a high-FPS reference can change both timing and lens.
Check high-FPS operation separately from visual equivalence at matched poses.

Related contracts: [follow](original-camera-follow.md),
[host clock](original-camera-host.md), [obstruction](original-camera-obstruction.md),
[noise](original-camera-noise.md), [transients](original-camera-transients.md).
