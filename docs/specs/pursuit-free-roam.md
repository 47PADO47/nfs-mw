# Free-roam pursuit integration

Sources: the existing specs [ai-pursuit](ai-pursuit.md),
[heat](ai-pursuit-heat.md), [cop cars](ai-pursuit-cop-cars.md),
[roadblocks](ai-pursuit-roadblocks.md), [damage](vehicle-damage.md) and
[music](music-graph.md). Their original sources and licenses remain recorded
there. Additional reference-only reading: dbalatoni13/nfsmw (CC0-1.0),
`Frontend/HUD/FeRadarDetector.{cpp,hpp}`, `FeHeatMeter.cpp` and
`AI/Activities/AICopManager.cpp`, retrieved reference revision recorded in
[provenance](../provenance/pursuit-free-roam.md). No code is copied.

## Patrol and detection

- Police models in the patrol plan are police, even while following traffic
  lanes. On a witnessed offence, nearby live patrol cars change to pursuit
  drivers in place, preserving position, velocity and damage.
- A car's retained heat selects detection: at heat four and above, being seen
  suffices; lower heat requires speeding or a witnessed recent collision.
- Arrest presentation must finish before another automatic pursuit can start.
- The original patrol target is the heat row's `NumPatrolCars`, independent of
  civilian density. The rewrite's `cop_share = 0` remains an explicit disable
  switch; positive values use that target rather than a percentage of traffic.
- A free-roam patrol uses the traffic action's normal 35/55 mph street/highway
  targets. Search-mode speeds apply only to units attached to a pursuit whose
  target has left the pursuit's tracked-sight window.

## Contact, search and escape

- Sensing, radar, arrest proximity and the pursuit navigation target use the
  current rigid-body position and orientation; navigation also reads actual
  body velocity. Compare both cars in the same physical frame. Model pivots
  and interpolated display poses must not shift range, height or prediction
  tests. The native radar feeder reads rigid-body position/forward; original
  pursuit actions query the target's physical interfaces. **[decomp]**
- Ground units must pass three-dimensional range/cone and world-occlusion
  tests; wrecked units cannot report sightings. The 1.5-metre vertical gate
  belongs to bust proximity and must not reject an otherwise visible uphill
  target. Unassigned patrols use 150/50 metre front/rear limits; units attached
  to a pursuit use the installed heat's limits.
- A hiding zone is not immediate invisibility. Each cop remembers whether it
  lost the target while the target was inside the zone. A cop watching the
  player enter can keep seeing it; after losing sight that cop cannot reacquire
  inside the zone until the player leaves. This latch is per ground unit.
- Distinguish a raw visibility result from the pursuit's tracked sight. The
  original tracks the target for seven seconds after the latest sighting,
  starting at minus five seconds when a pursuit is created. A raw sighting
  resets that timer to minus 0.25 seconds. The display's 2.5-second gate does
  not decide the driving mode.
- Remember the target body only while a live ground unit or helicopter sees it.
  Runtime air support waits for its model to load, so an invisible helicopter
  cannot keep a pursuit alive. During the tracked-sight grace, chase drivers
  retain the live target, as in the original target interface. After the grace,
  ordinary ground cops switch from pursuit path finding to traffic-lane search
  at the installed default search speeds. They continue along roads instead of
  steering all units at one stationary last-seen point. Reacquisition switches
  the same car and physical state back to the pursuit driver.
- Ground replacements may spawn during search. Search placements select
  ordinary traffic lanes in the 150 to 400 metre spawn annulus; their initial
  driving mode must already be search. While the pursuit state is cooldown,
  new ground units are capped at the heat row's `NumPatrolCars`, subject to the
  configured hard vehicle limit. Existing surplus units do not yet implement
  the original manager's flee/despawn sequence. The rewrite approximates the
  original camera-based gathered-segment sampler and retains its existing
  spawn-site restrictions; it must not claim that sampler is matched.
- Existing ground support ages while searching, and new offensive support is
  suppressed outside tracked sight. Original search helicopters and roadblock
  requests are phase-specific features; their existing installed rules must
  remain distinguished from raw contact and from offensive ground support.
- Existing grace, cooldown, hiding multiplier and bust rules remain the
  authority for pursuit outcomes. Retained heat is session-local until the
  career owns vehicle heat. Heat continues during tracked-sight grace and does
  not escalate while the pursuit is in cooldown, including its gated return
  to the chase display. The PC pursuit task corroborates this state gate.
- Search junction choices currently use ordinary lane-following randomness.
  The original action retains an acquired target to bias road choices; the
  exact road-choice policy and dispatch sectors remain unverified.

## HUD and audio

- Free-roam/cooldown radar targets the nearest eligible live ground police
  vehicle within 300 metres and a 45-degree forward cone, independently of
  whether a pursuit exists. Distance and direction are 3D. There is no separate
  deck-height filter or sight ray. These feeder rules are confirmed in the PC
  executable; exact roll-sensitive lateral orientation remains unverified.
  It is suppressed during active pursuit contact/grace.
  It remains available with the rear-view mirror disabled, and in a pursuit it
  waits for the actual cooldown phase rather than merely the first missed sight test.
- The reference detector identifies LED objects `0x69aa01e7` and `0x9f59065a`.
  Both masks reveal the same amount, from 0.3 to 1.0 during a sweep. A cycle
  repeats after 1.5 times the supplied normalized range, bounded below by a
  floor of 0.1. With no target both masks reveal zero and the cycle timer is
  unset. Reveal grows by 0.1 per HUD update, preserving the original frame
  dependence, and resets to 0.3 only when the period is crossed. A fresh cycle
  discards elapsed overshoot. The PC feeder confirms 300-metre normalization
  and native data confirms the 0.1 floor. See [hud-pursuit-state](hud-pursuit-state.md).
  The original sweep raises a sound message, which remains unwired.
- Show session vehicle heat and engaged/damaged/destroyed counts, replacing
  authored placeholder counts. Both duplicate heat labels must be updated.
  Reference heat text uses `0x7f91da62`; update it and the duplicate package
  template strings `0xa81a2028`. Unverified heat arc masks stay hidden: the
  reference heat update is marked unsolved and its two halves are ambiguous.
- Interactive pursuit music owns music transport while selected. EA Trax
  pause/next/previous must not start songs under that soundtrack. A player's
  paused song must stay paused; pursuit entry/exit must not alter that choice.
- Police speech continues through the installed event dispatcher. A soundtrack
  transition is not an EA Trax song announcement.
- The PC music graph's first group head of each section 1 through 4 is the
  sustained entry used by this host. Installed-data checks identify nodes 0,
  427, 755 and 1127; each keeps playing across repeated intensity-controlled
  bars. The earlier table 409/106/306/213 names internal heads of section 1,
  including a single-bar ending, rather than four sections. Do not fall back
  to that table when a section is missing: report unavailable score data.
  Full original music-event/variable semantics remain unimplemented.
- Only string objects receive heat text. The reference multiplier hash may
  identify a group in the PC HUD; changing a group's host text does not change
  the visible strings below it.

## Damage and roadblocks

- Ordinary collision damage changes visual zones and authored event sequences;
  it must never puncture a tire. Spike-strip contact remains the puncture source.
- Remove procedural bounding-box scrape/crack quads from normal rendering.
  Keep the installed corner overlays, glass swaps and detached parts.
- Follow the navigable road to place a block ahead, use its tangent and lane
  boundaries, and preserve each layout car's lateral position. Roadblock cars
  have police collision identity, damage state and emergency lights.
- A strip must be visible and must match the wheel's deck. Empty/failed
  placements cannot announce a successful roadblock.
- Test the segment between consecutive grounded wheel patches against the
  strip's box inflated by tire radius, including elevation. This prevents a
  fast tire skipping a strip in one fixed step. Discard remembered patches on
  reset or a displacement over 50 m (a rewrite teleport guard).

## Acceptance

Chase radio requests use the installed `SpeechID` values documented in
[speech events](ai-pursuit-speech-events.md), rather than abbreviated symbolic
names that need not equal `CollectionName`. Names are diagnostic labels only.
An installed regression must resolve every requested ID and enqueue it in the
speech dispatcher. That does not establish audible output: composite sentence
assembly remains unavailable; only events with direct recording banks can play.

Synthetic regressions cover patrol promotion, retained heat detection, wreck
sight rejection, tracked sight grace, traffic-lane searches and reacquisition, music
transport ownership, radar direction/cone/3D range, duplicate labels/counters, impacts
without punctures, and roadblock layout preservation. Installed regressions
exercise the actual world, damage data, HUD and pursuit music assets. These
tests do not establish a matched original-game driving or listening comparison.

The installed moving-route fixture uses a kinematic target following installed
traffic lanes and a physically simulated installed patrol car, real resident
collision sight queries and the normal pursuit clocks. Its single-unit wave
isolates mode transitions; it does not establish stock wave population, player
vehicle dynamics, full-route robustness or an original-game comparison.
