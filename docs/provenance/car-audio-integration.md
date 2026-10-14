# Car audio integration

Implemented from [car-audio-integration.md](../specs/car-audio-integration.md),
the existing engine/controller specifications and installed data readers.
References: CC0 https://github.com/dbalatoni13/nfsmw and original PC 1.3 static
collision-controller comparisons, researched 2026-10-14. No code or assets copied.

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
- **Playback limits:** collision family limit25 and the three-landing pool are
  retained. Piece variation uses the port RNG; its sequence is not a PC match.
  Mono panning follows the ideal native speaker law rather than its byte-quantized
  table. Other prop-prop/prop-ground sound producers remain incomplete.
- **Known differences:** no live original-game recording has established audible
  equivalence or absolute loudness calibration. Environmental reverb, signed skid
  telemetry/modules, road transitions/wet overlays, transmission modules,
  Speedbreaker audio treatment and component-specific upgrade crossfades remain
  incomplete. Continuous module parameters are batched on the game thread. See the
  specification for lifetime envelopes and backend filter approximations.
