# Weather (rain intensity and wind)

The current PC reconstruction is specified in [rain-pc.md](rain-pc.md).
That document supersedes the older incomplete weather analysis in this file.

The original-PC executable uses independent primary precipitation, shelter-bound
curtains, screen water, regional parameter maps and authored sound modules.
The 350-point primary rain pool is distinct from the 200-point curtain pool.
Both the primary atlas `RAINDROP` and camera displacement texture `RAINDROPPC`
come from the user-installed global texture packs.

The weather gate includes position and time. Earlier analysis omitted FPU
operands, incorrectly concluded that the gate was always zero, and confused
sine with cosine. Both rain and clouds now share the corrected free-roam gate.
Earlier claims that stock PC contained no thunder were also incorrect: the
rain sound class can queue distant and close thunder moments.

The pure `Weather` type retains the script override and original easing model.
The world precipitation adapter uses elapsed-time equivalents of the original
60 Hz rates. Regional free-roam rain comes from the installed `Rain` layer;
forced rain and amount overrides remain available for comparison.

Implementation provenance and runtime-comparison limits are recorded in
[rain-pc.md](../provenance/rain-pc.md). Static reconstruction, synthetic tests,
and remake captures do not establish complete original-game parity.
