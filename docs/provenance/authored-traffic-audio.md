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
