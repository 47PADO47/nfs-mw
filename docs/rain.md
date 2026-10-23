# Original PC rain

Rain uses the installed PC textures, surface particle profiles and sound banks.
It applies to both car material styles and both sky paths. Enhanced road shading
is separate; this pass does not add new puddle or reflection materials.

- Primary precipitation uses 350 camera-relative textured points, with Z-up
  motion and camera-speed streaking. The previous blue untextured streaks are removed.
- Authored tunnel and overpass regions suppress camera droplets and nearby tire
  spray. A separate 200-point curtain draws rainfall beyond a shelter boundary.
- Twenty independent camera droplets use the installed `RAINDROPPC` displacement
  map. They bend scene color before the HUD, and never appear in reflection captures.
- Wet tire drive, slip and slide effects come from each surface's records. Player,
  nearby traffic and police cars use the same profiles. Airborne wheels stop emission.
  The old procedural dry smoke is suppressed while the player uses wet surface effects.
- The authored rain sound bank uses the rain mixer object and hood/bumper variants.
  The original close/distant thunder recordings use the stock moment stream selectors.

## Controls

F8 > Environment > Rain offers location weather or forced precipitation. Location
weather reads the original `Rain` parameter map. The amount controls primary point
population; zero disables precipitation. Cloud comparisons and road-wetness overrides
remain independent. Road dampness follows rain history and pauses with simulation.

Tire spray needs active rain, exposed ground contact and movement. On the stock
asphalt profile, driving spray starts above 1 m/s (3.6 km/h) and reaches its full
emission rate at 5 m/s (18 km/h). A wetness-only comparison does not activate
spray. `vehicle-effects status` includes live and emitted tire-water counts.
The camera droplets and low-opacity stock spray are separate effects.

Opaque projected-reflection faces use the road pass's reversed orientation.
This prevents ordinary road geometry from covering reflected sky and trees,
including the highway transition at (-130, 4089) to (-78, 4108).

## Scope and remaining comparisons

This is a reconstruction from [the PC specification](specs/rain-pc.md), not a claim
of complete original-game parity. Original RNG, every camera-mover transition,
curtain hysteresis, race/NIS weather overrides and exact audible mix still need
comparison against the running unmodified PC game. Free-camera curtain height
uses the camera anchor. Thunder uses resident-section readiness as its streaming
guard, and the audio speech queue as its channel-busy guard.

The implementation deliberately retains the existing authored wet-road reflection
capture. Road roughness, puddle masks, improved water response and an Enhanced rain
presentation remain separate work.

### F8 adjustments

Environment > Lens droplet size defaults to 0.5x, matching the smaller
Widescreen Fix preference. Set zero to hide lens water, or 1x for the previous
size. This changes only camera droplets. Rain, tyre spray and audio remain active.

Materials > Road reflection resolution improves image detail and geometry coverage.
Enhanced resolutions retain smaller reflected objects and cull the reflected mesh
rather than its main-view source box. Original resolution keeps the original
visibility thresholds. Existing capture filtering is unchanged.
