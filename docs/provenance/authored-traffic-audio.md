# Authored traffic audio

- Date: 2026-10-14.
- Specification: [authored-traffic-audio](../specs/authored-traffic-audio.md).
- Sources: owner's installed PC game, passive native sound-controller captures,
  authored banks and mixer map; [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
  (CC0), traffic effect producers, spatial controller, stitch wrapper and class
  declarations. No reference implementation code or installed assets were copied.
- Method: matched PC identity before static queries; independent semantic spec,
  then Rust implementation. PC horn constructor directly confirms `ENV_STATIC`
  common type and rules out the previous traffic-engine-bank assumption. The
  PC whoosh producer confirms speed gating, rotating variant state and family 1.
- Verification: synthetic event/order/spatial/class-data checks plus installed
  module lifecycle checks. Commands and outcomes are recorded by local validation.
- Limits: see specification; backend DSP and complete original gameplay routing
  remain independently measurable. Tests establish selected contracts and FPS
  consistency, not complete listening parity.

## Actor roles (2026-11-06)

- Sources: installed `ATTRIBUTES.BIN` vehicle records read through the remake's
  attribute reader; CC0 reference state managers for traffic, AI-racer and cop
  cars, the AI engine effect, the engine controller and the main map state list.
  Read for understanding; no reference code copied.
- Findings: every installed race-car record inherits `HornType` 5,
  `TrafficEngType` 0 and `WooshType` 9 from `racers`/`cars`. The reference
  separates contexts; only traffic cars attach traffic effects. AI racers use the
  `CAR` engine module in map state 3, four slots, 135 m connection distance.
- Verification: synthetic connection-order and class-word checks; an installed
  regression drives six race opponents through the real map and engine banks. It
  requires engine PCM, at most four voices, no traffic horn/whoosh/source records,
  and release when opponents leave. The first run exposed a missing low-pass word
  that silenced every voice; the filter is now published.
- Limits: PC addresses for the AI engine path were not inspected; listening level
  against the original is unmeasured.
