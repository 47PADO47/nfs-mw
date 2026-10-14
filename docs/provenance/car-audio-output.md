# Car audio routing, tracing and output

- Specs: [ownership and observability](../specs/car-audio-observability.md),
  [voice output and wet bus](../specs/car-audio-output.md).
- Implementation: game-specific audio adapters, tracing and shared wet output.
- Source behavior: matching original PC voice creation, dry/wet mono mixing,
  16-sample changed-gain interpolation and environment patch-selection table;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  sound library, dynamic mixer and car/environment producers.
- Specifications precede the new implementation. No original tables, decompiled
  code, recordings, patches or executable bytes are included in public source.
- Single-owner routing, retained input timeline, bounded asynchronous tracing
  and failure/completeness reporting are remake integration decisions.
- Numeric isolated processor comparison is recorded separately in
  [FXEDIT provenance](fxedit-processor.md). Bounded passive live observation
  confirms original wet processing; host group/send ordering, complete driving
  scenarios and final output conversion still need separate comparisons.
- Kira mono panning, per-voice filter transfer functions, stereo wet downmix,
  pitch transitions and the fixed 16-sample wet adapter delay remain explicit
  backend differences. See the [coverage ledger](../car-audio-coverage.md).
