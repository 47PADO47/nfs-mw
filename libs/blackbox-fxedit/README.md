# blackbox-fxedit

Data-only FX/version11 reader and software processor for authored effect graphs.
No game names, filesystem access, sample assets or generic room presets.

`Patch::parse(bytes)` reads all six layouts. `Processor::new(&patch, rate, channels)`
selects/compiles a mono or stereo layout. Prepare processors before the audio
callback. `process_mono(input, left, right)` consumes16..256 samples in multiples
of16, produces separate speaker buffers and allocates no heap memory. `reset()`
clears all filter/delay/cache state while keeping the authored configuration.

Unsupported processors, layouts and malformed connections are explicit errors.
Implemented IDs:0 source,1 one-pole,3 nine-tap high-pass,5 resonant band-pass,
6 gain,7 all-pass delay,8 sum,9 cached split.

Spec/provenance:
[`fxedit-processor.md`](../../docs/specs/fxedit-processor.md),
[`fxedit-processor.md`](../../docs/provenance/fxedit-processor.md).
Game-level installed tests compare original instruction replay and asset identity.

`LowPass` exposes the software voice form of the first-order processor with
integer cutoff normalization, immediate retargeting and history removal on
bypass. It allocates no memory during processing. Spec:
[`voice-low-pass.md`](../../docs/specs/voice-low-pass.md).
