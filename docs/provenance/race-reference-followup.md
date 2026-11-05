# Custom-race reference follow-up

- **Spec:** [race-reference-followup](../specs/race-reference-followup.md).
- **Started:** 2026-11-04.
- **Sources:** a user-driven modded PC 1.3 sprint, video and timestamped runtime
  observations; existing PC analytical-estimator research in
  [ai-pursuit-acceleration-handoff](../specs/ai-pursuit-acceleration-handoff.md);
  retained PC race/racer accessors, finish callback and statistics-reader analysis.
- **Reference declarations:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0-1.0, read for calling contracts and field meaning. Cross-platform struct
  offsets were not used without PC verification. No source was copied.
- **Executable identity:** runtime target SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  Saved PC analysis image SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Selected hook bodies were validated in the actual target. Modded behavior is
  identified as such; this recording is not an unmodded comparison.

The user recorded a complete Hard sprint with catch-up on, six opponents and
maximum traffic. Racer progress, physical pose, requested/actual speed, driver
controls, engine and race state were observed. All selected sites attached; no
race-probe drops/read/hook errors were reported, and all 153 checked ranges were
restored. Video decoded successfully. One explicitly logged audio gap prevents
frame-accurate audio claims. Raw media, native layouts and traces remain private.

The photo's displayed speed agrees with the recorded finishing speed. Six
opponent timer values agree with elapsed time divided by fractional completion.
These observations are independent of synthetic report tests. The action-curve
integration follows the existing estimator spec; this capture does not itself
recover a complete acceleration table or establish the cause of every weak-racer
failure. The regression runs the production six-opponent loader with installed
data and checks physical course progress, separately from scalar report fixtures.

Extended installed course runs, with catch-up disabled and the human stationary,
also exposed opponents leaving the drivable path around a junction. Recovery follows the existing off-path
contract in [ai-driver-speed-skill](../specs/ai-driver-speed-skill.md), section 1:
check corridor bounds, fresh-road membership and whether the new road is closer.
The racer decision-rate correction follows the existing 30 Hz contract in
[ai-simulation-lod](../specs/ai-simulation-lod.md). The old integration inherited
the police's eight-tick interval instead of the racer's two-tick interval.

The failed course runs separated a 31-segment shortcut from the 36-segment base
leg. A physical collision query hit an enabled race barrier across the shortcut.
The corresponding barrier group was also enabled in the original capture.
Shortcut construction used a graph whose temporary race flags had already been
cleared. The fix passes active barriers into that search without enabling global
race road flags; the existing base-route fallback limitations remain separate.

Actual finish times and gameplay finish flags remain distinct from report-only
projections. Extra racer counts and custom police options remain intentional
extensions. Further difficulty, catch-up, police and route cases are still needed
before claiming full race parity.


The user's next capture has Hard difficulty, six opponents, Minimum traffic, catch-up Off and police disabled. The human
finishes second. Telemetry is retained, but encoder finalization failed; recovered media remains marked incomplete and
contains two audio discontinuities. It is not evidence of frame-perfect audio synchronization. Its unfinished-rival
timers also demonstrate that the projection boundary can follow the human finish; this remains a result-timing limit.

The tested remake intro for event 15.2.1 places one authored camera eye below its streamed road. Car channels were linked
by nearest civilian proximity and could exchange rivals. A fixed six-Hz body loop and blanket intro-engine muting were
also confirmed in the source. The new code binds race channels by racer ID, removes that loop, reads installed engine
curves, and handles authored body/burnout controls by CARP system identity. The PC rev-file loader's sixteen-channel layout
was independently checked in the saved analysis image identified above. Body pulse and burnout contracts use the reference
interface and suspension declarations; their PC runtime amplitude remains unverified. The camera ground clearance is an
explicit remake safeguard. Installed structural, decoding and physics checks are distinct from a user-observed visual pass.

The PC start-selector initializer independently confirms that scene table rows and selector enum bits differ. The old
row-as-bit lookup chose a different scene's audio for IntroNis04. That early correction still used an incorrectly
inferred bank assignment. The later native sample capture in
[race-presentation-followup](race-presentation-followup.md) supersedes it: IntroNis04 uses bank 2, takes 14/15.
Declared PCM counts agree with decoded stream lengths, but waveform similarity and nonzero PCM alone did not establish
the countdown cue's identity. End sections include authored tails and need not last exactly three seconds.

Following a rival continuously through event 15.2.1 exposed a second route failure. The route window revisits road IDs;
both the navigator and the periodic route refresh selected the first occurrence. Consuming an accepted path edge and
preserving its remaining suffix repairs traversal identity. The correction does not skip gates or move a physical car.
