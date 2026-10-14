# Authored FXEDIT wet processor

Sources: independently inspected matching original PC sound-library patch
constructor, processor initialization and scalar mixing routines. The
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) CC0-1.0 reference
`SFXObj_Reverb` describes game patch selection; executable instructions take
precedence for software DSP. Provenance:
[fxedit-processor.md](../provenance/fxedit-processor.md).

## Format and graph

FX/version11 files start with `FX`, byte11, byte0, then a big-endian total
size. The56-byte header contains six node counts and six connection counts,
one layout for each output channel count1..6. Each layout stores its nodes
then connections. Record sizes for IDs0..9 are `[4,8,4,4,8,8,4,8,4,4]` bytes.
Each record is opcode, output selector, and signed big-endian16-bit words.
Connections contain two big-endian32-bit node indices, source output port,
destination input port, and two reserved bytes; length12. Ports are1 or2.

Choose the highest nonempty layout not exceeding the device channel count.
The initial processor supports mono/stereo output; other layouts remain
unsupported. Selector0 is internal. Ordinary selectors1/2 designate output
channels; opcode9 packs two selectors into its nibbles. Nodes retain their
two upstream connections. Duplicate target inputs, invalid indices/ports,
unsupported nodes, cycles and resource overflows are rejected explicitly.

The wet input is a single mono stream. Outputs are evaluated in speaker order.
Node0 copies input and advances its source cursor. Node9 provides the authored
two-consumer cache: a first request evaluates/caches its upstream block; a
second copies that block and marks it consumed; the following request starts
a fresh block. Preserve this state across calls, rather than independently
advancing a shared upstream graph for each speaker. Node8 evaluates both
upstream inputs and sums corresponding samples.

The original buffers are sized for256 f32 samples. Blocks are multiples of16;
the all-pass kernel handles four samples together and delays are multiples
of16. The host can buffer a fixed16 samples independently of render updates.
Patch reset clears all filters, ring buffers and cache state before replacing
the graph. The game fades sends out before reset and fades the new sends in.

## Numeric kernels

All persistent coefficients and ring/sample state are f32. Intermediate
arithmetic uses extended precision in the PC implementation; reconstruction
uses f64 expressions with f32 stores and no fused multiply-add. Coefficient
and sample differences must be bounded against original instruction replay.

| ID | Parameters used | Behavior |
|---|---|---|
| 0 | none | mono source copy |
| 1 | word1 cutoff Hz, word2 Q8 gain | one-pole filter |
| 3 | word0 cutoff Hz | symmetric nine-tap high-pass FIR |
| 5 | word0 center Hz, word1 bandwidth Hz, word2 Q8 gain | resonant two-pole band-pass |
| 6 | word0 Q8 gain | scalar multiplication |
| 7 | word0 feedback, word1 delay milliseconds | stateful all-pass delay |
| 8 | none | sum two upstream blocks |
| 9 | none | two-consumer cached split |

For1, `q = 2*cutoff/sample_rate`; coefficients are `a=(1-q)*gain/256`
and `b=q*gain/256`. Recurrence `y=b*x+a*previous`. The previous state stays
in extended precision within the block and is stored f32 at its end.
The subtraction uses unrounded `q`; store both coefficients to f32 before
multiplying them by the gain.

For7, `g=(feedback>>1)*(1/127)` with the reciprocal stored as f32. Let
`m=floor(delay_ms/16)` and `p` be the first prime at least
`max(3,floor(sample_rate*m/1000))`. Ring length is `16*p`. For each frame,
read ring value `d`; compute `v=x+g*d+epsilon`, output `d-g*v`, and store
`v` into the ring as f32. Increment/wrap the ring index. Epsilon is the
original f32 denormal-prevention bias near `1e-20`; it is also used by5.

For3, clamp normalized cutoff `c=2*cutoff/sample_rate` to0.8. The five
symmetric coefficients start with center `c` and distances1..4 as
`sin(pi*i*c)/(pi*i)`. Apply a Hamming window `0.54-0.46*cos(pi*i/4)` to
the nine-tap impulse, negate it and add1 to the center (high-pass).
Normalize by the magnitude of its Nyquist response. Preserve the eight
previous input samples and the symmetry when evaluating each output.
Store each unwindowed sinc value to f32 before multiplying by its window.
The angle passed to sine uses unrounded `distance*pi`, while its divisor is
stored f32. Accumulate the alternating coefficient sum with f32 stores;
compute the normalization reciprocal in extended precision and multiply the
stored coefficients by that reciprocal.

For5, `r=1-bandwidth*pi/sample_rate` and
`c=2*r/(r*r+1)*cos(center*2*pi/sample_rate)`. Feed gain is
`(1-r*r)*sqrt(1-c*c)*gain/256`. Recurrence is
`y=2*r*c*y_previous + (x+epsilon)*feed - r*r*y_older`.
Bypass when bandwidth is at least sample_rate times the original f32
approximation of2/pi, or center is nonpositive or at/above integer Nyquist.
Compute `r*r` from the unrounded radius and store that square to f32 for
coefficient construction. Use the stored radius for the cosine numerator and
the stored square for its denominator and feed gain. Runtime recurrence uses
the stored radius and cosine; it does not reuse the configuration square.

The trigonometric and square-root helpers are original polynomial
approximations, not platform `sin/cos/sqrt`: cosine Taylor terms through
degree12, sine through degree13, and `sqrt(1+z)` through degree7. Their
coefficients use the original f32 rounded constants and operation grouping;
sine stores its degree11 power to f32. Positive arguments greater than the
f32 approximation of2*pi subtract that period until in range. π,2π and
π/4 also use f32 constants. Changing these to generic math visibly changes
some authored filter responses.

## Host send and mix order

The original voice processor yields one post-filter mono buffer. It is mixed
independently into speaker dry outputs and the mono effect bus. Wet level is
voice volume times the per-voice wet send; dry and pan do not multiply it.
The global graph runs after all voice sends are accumulated, before output
conversion. Changed dry/wet levels interpolate linearly from old to new over
the first16 samples, then use the final level. The host must account for
group/master levels once and preserve requested versus effective wet state.

Live original patch installation/service use is a separate runtime claim.
Isolated constructor/kernel replay proves the DSP behavior, not that an
earlier captured gameplay session enabled the bus. Unsupported host ordering,
speaker modes or device-rate transitions remain explicit limitations.

## Validation

Validate all fifteen installed patches structurally and both mono/stereo
layouts. Compare coefficients, impulse/tail and bounded deterministic signals
against complete original graph replay at44100/48000Hz, with16/64/256-sample
blocks, cache fanout, independent speaker outputs and reset. Native replay may
stub allocation/free only; graph construction, math, connections and kernels
must execute their original instructions. Retain failed attempts separately.
