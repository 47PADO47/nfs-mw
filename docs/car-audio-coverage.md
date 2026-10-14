# Car audio coverage

Implementation and verification are separate. Authored playback means the remake
loads the player's installed module and supplies its controller inputs. It does
not establish complete audible parity with an independent original-game drive.

| Area | Current implementation | Remaining boundary |
| --- | --- | --- |
| Engine and limiter | Ginsu loops, CAR sample graph, sputters, distance/environment inputs; unavailable sample layers retain real engine loops | Original timer/random sequence, final host filtering and pitch interpolation |
| Tyres, transmission, road and wind | Authored skid/transmission, road transitions, wet road, stitch road and weather wind | Complete tyre damage/terrain overrides and backend filter/spatial equivalence |
| Collision and scrape | Surface/profile selection, paired wheel landings, consecutive impact levels, authored scrape/stitch collision, prop-owned impacts | Other prop-ground/prop-prop producers and original random sequence |
| Nitrous and drivetrain effects | Authored nitrous, purge, shifts, sweeteners, turbo and reverse whine; exclusive fallback ownership | Special camera/NOS states and controlled original comparison |
| Traffic | Moving traffic engines, ENV_STATIC horns, authored near-miss whooshes, installed vehicle profiles and source lifecycle | Original RNG, camera velocity averaging, additional AI requests, truck auxiliary/scenery drive-bys |
| Environment | Installed spatial Reverb map and tunnel/overpass/garage zone selection; authored patch transitions | Dynamic zone visit mutations, occlusion and tunnel drive-by producers |
| Reverb | Installed FXEDIT processor graphs and shared pre-pan mono wet bus at device sample rate | Stereo wet downmix, final output quantization, fixed 16-sample wet FIFO delay, matched live driving comparison |
| Speedbreaker | Authored begin/end effects and selected moment recording, shared real-time mode envelopes, consumer-verified music cutoff and native voice low-pass kernel, native HUD fade/active bar messages and pulsing icon, paired camera distance/FOV targets and shader-verified additive screen mask | Per-stream callback grouping/quantization, full native Cubic camera follow, music branch arbitration, other original blur passes, optional renderer support and matched driving PCM |
| Other callers | Explicit coverage inventory | Remote racer/cop engine producers; static radio chirps and other environmental attachments |

## Evidence available

Synthetic and installed-asset tests cover controller catch-up, historical input
retention, event ownership, actual voice starts, release/failure paths and PCM
across render rates from 5 to 360 FPS. Output tests cover 44.1 and 48 kHz,
irregular callback boundaries, wet gain independent of dry panning, patch reset
and every installed FXEDIT patch. These are scoped regressions, not complete
original-game parity certification.

Independent isolated original-code comparison covers all 15 installed FXEDIT
patches at both output rates; the largest measured sample difference is below
0.00000003. A bounded passive original-game capture separately confirms active
wet processing. It does not establish equivalent sends throughout a drive.
Original observations, extracted assets and numeric dumps remain private.

## Tracking new gaps

The `audio-audit` console command lists the maintained runtime inventory.
`audio-trace <path>` starts a bounded asynchronous JSON-lines capture;
`audio-trace off` flushes its final completeness record. `NFSMW_AUDIO_TRACE`
enables tracing at startup. No trace files are written by the audio callback.

[`tools/audio_coverage.py`](../tools/audio_coverage.py) joins original module
observations with remake controller and voice records. Unknown modules, missing
routes, decode failures and dropped/incomplete traces remain visible. Independent
recordings always retain `parity: not_established`; a matching family name alone
cannot approve a gain, pitch, timing or waveform mismatch.

Resolve a reported mismatch in order: gameplay inputs, class words, selected
recordings and voice lifetime, gain/pitch/spatial/filter trajectory, rendered
PCM. Record a regression and update both this ledger and the runtime inventory.

Contracts: [ownership and tracing](specs/car-audio-observability.md),
[output](specs/car-audio-output.md), [power effects](specs/car-audio-power-effects.md),
[traffic](specs/authored-traffic-audio.md), [spatial maps](specs/spatial-parameter-maps.md),
[common events](specs/common-audio-events.md), [FXEDIT](specs/fxedit-processor.md),
[Speedbreaker presentation](specs/speedbreaker-audio.md).
