# Car audio output and authored wet bus

## Voice mixing **[decomp, bounded]**

Matched PC voice mixing processes a mono source through its processors before
separately adding it to dry speaker channels and the mono effect bus. Wet send
is multiplied by voice gain, independently of dry gain and speaker panning.
Changed speaker/wet gains interpolate over16 samples: sample1 uses old+delta,
sample16 reaches the destination, with delta=(destination-old)/16.

The remake preserves the previous constant-power mono speaker law while moving
it after filtering, so wet input can be taken before panning. Stereo recordings
retain their channels; their wet downmix remains a host approximation. Filters
still use the current backend transfer function, not a measured original match.
Original creation gain and dry words retain their seven-bit conversion. Changed
voice volume retains Q15 precision. The authored controller supplies release
envelopes; the backend does not add its former33ms gain smoothing or40ms stop
fade. Pitch changes remain a separate platform-equivalence question.

## Shared output processor **[decision, decomp]**

All eligible voices contribute pre-dry/pre-pan mono samples to one wet bus.
Category gain is applied once to that contribution; the master gain applies to
both final dry and wet output. Category/master interpolation and quantization
are bounded backend differences, not fully matched PC mixer claims.

The bus runs the installed FXEDIT graph for the requested original environment
row. Parse and instantiate the graph outside the audio callback. Unsupported
patches remain unsupported, preserving dry playback and reporting the failure.
No generic reverb substitutes for an authored patch.

The original processor accepts blocks in multiples of16. A streaming FIFO
retains partial backend blocks without padding or extra state advancement. It
adds a fixed16-sample wet latency, independent of render FPS. This explicit
adapter difference is about0.36ms at44.1kHz and0.33ms at48kHz.

Environment rows select, in order: City_Dense, Alley, City_Open, Garage,
Garage_sml, Simple_Tunnel, Simple_Tunnel_Sml, Hills_Close, Hills, City_Open,
City_Dense, Alley (all `.fx`). These names come from the matched PC table.

## Evidence limits

Isolated original processor replay establishes graph arithmetic separately from
whether a particular running original session enabled the patch. Backend wet
capability and effective patch identity are recorded separately from requested
class/mixer words. Remaining DSP, clock and producer gaps stay in the audio
coverage inventory; neither installed-asset playback nor FPS tests imply full
audible parity.
