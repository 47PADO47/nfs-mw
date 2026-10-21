# Car audio integration

Implemented from [car-audio-integration.md](../specs/car-audio-integration.md),
the existing engine/controller specifications and installed data readers.
References: CC0 https://github.com/dbalatoni13/nfsmw and original PC 1.3 static
collision-controller comparisons, researched 2026-10-14. No code or assets copied.

- **2026-10-21 collision lifetime:** matched PC collision-event constructor and
  collision sound object's process update corroborate the CC0 SoundCollision.cpp
  and SFXObj_Collision.cpp behavior: the combined SMOKABLE/WALL description has
  a creation-only quantized intensity gate, and impact event azimuth is captured
  once while scrape azimuth remains live. The changes preserve authored levels
  and do not establish perceptual loudness parity or owner-transform tracking.

- **Implemented:** 2026-10-14, from the behavioral specification and existing readers.
- **Automated coverage:** collision ancestry and response ramps; collision mixer
  precedence; contact coalescing; planar listener coordinates; filter open/closed,
  frequency attenuation and device-rate limits; coherent engine updates and stalled
  callback recovery. Installed tests render the actual playback graph, isolate
  scraping and the engine, cover eight ordinary road terrain IDs, exercise scrape
  refresh/release clocks and compare engine loudness at 5–360 FPS and 44.1/48 kHz.
  Representative engine families and upgrade selection/stock restoration are covered.
- **Second pass:** matched PC sample-wrapper, landing and smackable callbacks
  established the corrected piece record, separate landing route and prop-owned
  OnDetached producer. Installed effect-bank probes confirmed mono channels and
  authored stitch-module compatibility. Backend documentation established the
  signed panning range. Added actual output tests for left/centre/right placement,
  gentle landing, physical impact completion and five prop profiles.
- **2026-10-20 prop callbacks:** matched PC Effects hit handling and the
  independently readable Effects collision listener establish per-owner
  WORLD/OBJECT/GROUND actions and source profile selection. Runtime records now
  include prop pairs, successful ground support and authored barriers, consumed
  on the same completed simulation tick without duplicating player/prop events
  or feeding surrounding prop impacts into the player's camera response.
- **Directional lookup:** original PC selector branches were executed in
  isolation for ten signed-normal and strict-boundary cases. Installed data has
  3,674 car/prop collision linkage records across 121 effects wrappers; 1,526
  have directional audio fields and 1,274 inherit at least one audio field.
  Resolving exact inherited fields replaces arbitrary own-reference enumeration
  and DESCRIPTION-based front selection. Installed regressions compare every
  reachable zone to the source reference, including fallback and silent rules.
- **Audio intensity/culling:** matching PC one-shot world-effect handling and its
  clamped polynomial helper establish wrapper AudioQuadratic and AudioCullDist.
  Thirty-six isolated original curve cases passed. All 121 reached installed
  wrappers have authored culling radii; the common nonlinear curves affect
  both impact stitch levels and scrape strength. The port applies those fields
  once after physical acceptance, retaining physical detachment acceptance
  separately from whether its audio is audible.
- **Playback limits:** collision family limit25 and the three-landing pool are
  retained. Piece variation uses the port RNG; its sequence is not a PC match.
  Mono panning follows the ideal native speaker law rather than its byte-quantized
  table. Matched live original audio and global voice competition remain open.
- **Known differences:** no live original-game recording has established audible
  equivalence or absolute loudness calibration. Environmental reverb, signed skid
  telemetry/modules, road transitions/wet overlays, transmission modules,
  Speedbreaker audio treatment and component-specific upgrade crossfades remain
  incomplete. Continuous module parameters are batched on the game thread. See the
  specification for lifetime envelopes and backend filter approximations.

- **Endpoint delivery correction:** host inspection found car/prop callbacks
  stored both owners in the vehicle's returned impact, which AI/trailer stepping
  discarded. Prop-owned contact records now enter the world queue at the shared
  collision response, independent of the driver. A synthetic production collision
  regression checks endpoint ownership, detachment, material and single drain;
  this fixes delivery, without introducing new sound selection or loudness values.

- **Vehicle endpoint continuation:** source inspection found AI/trailer callers
  discarded vehicle impact records and pair audio was gated to the player.
  The CC0 collision event reader accepts owner objects independently of the
  primary camera target; target participation adds a description flag, rather
  than enabling callback existence. Existing PC-verified effects listeners and
  per-pvehicle linkage readers establish owner-specific selection. Traffic,
  police and trailer contacts now retain their own profile and unique body ID
  through fixed-tick audio delivery. Selection/output regressions are separate
  from a live original-game loudness or voice-priority comparison.
- **Heavy endpoint intensity:** the existing PC-verified object dispatcher
  specification distinguishes physical response from notification intensity.
  The CC0 `Effects.cpp` object listener forwards the corresponding endpoint's
  reported scalar to hit audio. Pair audio previously used zero physical
  response for an immobile endpoint; it now retains the positive notification.
  Synthetic and installed heavy-vehicle tests exercise that distinction without
  changing mass or collision response.
