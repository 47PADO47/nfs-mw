# Car audio ownership and evidence

## Integration contract **[decision]**

Gameplay supplies ordered physics states and discrete events. Controllers own
their authored modules and voice lifetimes. A single command router chooses
authored playback or a named fallback; an authored owner and its fallback must
never play the same event. Successfully loading a module is not evidence that
its voices decoded or started successfully.

Continuous controller updates run independently of render cadence. Catch-up
work retains both elapsed time and the historical parameters belonging to it.
Sampling the newest frame repeatedly while repaying older clock debt changes
the sound. Short discrete events are queued by their producer rather than
being treated as a persistent flag sampled once per render frame.

## Trace contract **[decision]**

`NFSMW_AUDIO_TRACE` enables a bounded asynchronous JSON-lines trace. Console
`audio-trace <path>` starts a session; `audio-trace off` closes it. Existing
module instances use the same recorder, so enabling capture does not restart
the car. Records contain session sequence, wall time, controller time, stable
instance ID, bank/class, class data, player input words and success/failure.
Raw Q15 gain and Q12 pitch remain separate from backend gain/rate.

Capture module creation, updates, release, starts, updates, pauses, resumes,
stops, decode/backend failures, direct fallback commands and engine requested
versus consumed controller revisions. Output capabilities are stated separately:
the pan/filter/pitch backend and effective authored wet processing cannot become
verified original behavior merely because module outputs match.

The audio callback performs no trace file I/O. Producers never wait for disk;
queue saturation and write failures invalidate complete trace evidence. A
clean session end records dropped counts. Abrupt termination without that
record is incomplete, even when earlier records are usable observations.

## Coverage and comparison **[decision]**

A maintained inventory distinguishes implemented controllers, fallback routes,
missing gameplay producers, missing DSP and unverified original behavior.
Runtime records add observed/replayed coverage without upgrading it to parity.
Every original observed module must either have a classified route or appear
as an unknown in the comparison report. Unknown and absent families are not
passes. Independent drives cannot establish sample-for-sample parity.

Compare controlled scenarios in stages: input/event provenance, class words,
selected recordings and voice lifetime, gain/pitch/spatial/filter trajectories,
then rendered PCM. Random choices require synchronized state or comparison
against the authored allowed set. Tests across render rates and stalls prove
cadence robustness separately from equivalence to the original timer.

## Removal of implementation loudness compensation **[decision, decomp]**

The previous output multiplied authored engine/sample gains by1.5 to retain
pre-map loudness. Matched PC voice creation uses the graph's clamped Q15 word
converted to seven-bit gain, without this multiplier. The remake now preserves
that level with unity makeup. This changes loudness deliberately; unmatched
full-game RMS is not its justification. Master/category settings remain separate.
