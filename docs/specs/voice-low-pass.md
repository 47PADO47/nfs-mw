# Software voice low-pass

## Original PC behavior [decomp]

The music stream's low-pass setter reaches the same software voice filter used
by Ginsu voices. It normalizes integer cutoff Hz by half the mixer sample rate.
Normalized cutoff at least1 removes the filter and frees its history. A later
filtered update creates a new filter with zero history. Coefficient changes
while filtered retain history and apply immediately without a tween.

For an active filter, store normalized cutoff as f32, multiply by sample rate
and store as f32, round to the nearest even integer, then use that integer over
sample rate as the feed coefficient. The feedback coefficient is1 minus that
fraction, computed before the f32 feed store. This is the FXEDIT first-order
processor's coefficient arithmetic at unity gain.

Each sample computes `y = feed*x + feedback*previous`. Keep the accumulation
at double precision within a processing block; output samples are f32 and the
last history is stored as f32 at block end. Channels have independent history.
A zero cutoff retains the last history value; it does not forcibly mute.

The original audible Speedbreaker music output comes from its installed mixer
filter route and leaves moment effects on their own SFX bus. The separately
updated internal music fader is not consumed by the inspected stream path.
The final stream cutoff is an integer.
The original's voice filter is a first-order software filter, not a two-pole
Butterworth filter. A host-rate adaptation supports44.1/48/96kHz without
dependent render-FPS changes; native PC kernels are compared at44.1/48kHz.

## Validation and boundaries

Retain unchanged native coefficient/sample-kernel vectors for impulses,
coefficient retarget, zero cutoff, filter removal and fresh re-entry. Installed
audio checks establish routing and high-FPS lifecycle, not matched full-game
PCM. Shared music-bus filtering is linear but native per-stream allocation,
callback grouping and final mixer quantization remain separate boundaries.
High-pass processing is outside this correction.
