# Original UI repair plan

## Decision

Stop selecting arbitrary groups of six symptoms. Work in dependency order, reuse existing
shared components, and require the kind of evidence that can actually detect each defect.
Use small repair tickets inside the work packages below. Do not rewrite the UI framework.

This plan covers UI-001 through UI-061, the outstanding options inventory and capture-only work. All reports
remain open for acceptance. Some have tested repair candidates; others are repeated failures
or unresolved reference questions. An open item does not automatically need another code change.

Basis: current contribution checkout, HEAD `7b9eae1`, including its uncommitted repairs;
batch 18 playable build and subsequent reports. Upstream was fetched for this review, without
merging or altering source. Historical tests are not a fresh validation of this planning edit.

Sources: [main backlog](ui-parity-playtest.md), [batch 14 findings](ui-parity-batch14-followup.md),
[batch 18 findings](ui-parity-batch18-followup.md), [options](ui-options-coverage.md),
[batch 17 candidates](ui-parity-batch17.md), [batch 18 candidates](ui-parity-batch18.md).

## Scope correction: capture work is a separate backlog

The four-stage summary was too narrow as a dispatch checklist. The reported UI issues do not
cover all unfinished work from the original recording. Use the [capture coverage ledger](ui-parity-capture-coverage.md)
alongside them: ten chapter families, unclassified intervals, Quick Race, Rap Sheet, modal/cart
variants, motion, sound/ambience, environment effects and conditional states.

Reconcile that ledger before choosing another implementation batch. This expands the existing P00
work; it does not mean the video has now been fully reviewed or every old queue task is still undone.
The four stages remain dependency groups. Capture audits feed concrete tickets into each stage;
they are not deferred until all reported bugs are fixed. Keep missing options as an explicit stream.

## Inspection findings when the plan was first written

- `frontend/challenge_series/mod.rs` hides `CONSOLE_FOOTER` in both start and tick, and
  posts `SHOW_HELP`, documented there as the keyboard footer. This directly supports UI-053;
  adapt this page to existing binding-aware prompts rather than adding another icon system.
- `frontend/pause_background_playtest.rs` checks package count, visibility, dimensions and
  stack retention. It does not render pixels. These assertions cannot detect UI-058's white sides.
  The background is installed through `frontend/screens/pause_background.rs`.
- `frontend/garage_backdrop.rs::set_stage` reads/uploads a replacement room and calls the
  camera rig's `begin` when the stage changes. `flow_backdrop.rs::show_backdrop` resets the
  camera when constructing a new scene. Both are leads for UI-057, not a reproduced diagnosis.
- `garage_ui/palette/highlight.rs` moves the authored border using a swatch-position delta
  and parent transform. It preserves the initial authored offset; rendered transformed bounds
  need checking. A selected-index assertion cannot prove correct border placement.
- Existing `footer.rs`, `prompts.rs`, `logic/feedback.rs`, preview jobs/cache and `CameraRig`
  already provide shared mechanisms. Prefer completing their contracts over parallel replacements.
- `Cx::press_sound` infers acceptance from a changed command count. Blacklist overview pointer
  paths call it, while pad paths call accept directly. Audit host feedback before changing this:
  differing paths are verified; the actual missing/duplicate audible event still needs capture.
- Preview decode jobs already run asynchronously and reject stale completed requests. Room
  reads/uploads and car upload remain separate costs; increasing the cache alone cannot fix cold stalls.

These are historical remake-source observations. Pass A/B changed several named paths; see their coverage records. They do not establish that original code is responsible.

## First: make existing evidence usable (P00)

- [ ] Reconcile the original recording queue and [capture ledger](ui-parity-capture-coverage.md) with all playtest and options backlogs. Create one linked behavior/issue index. Keep IDs, latest user report,
  tested build, candidate change, evidence, next experiment and acceptance state per issue.
  Keep historical prose as evidence; make the index the current dispatch/status view.
- [ ] Use states: reported, reproduced, reference-needed, candidate, checked, accepted, reopened.
  Latest reported failure overrides candidate/checked status. Never infer acceptance from silence.
- [ ] Index the existing long reference recording by menu, transition and sound event. Reuse clips;
  record only missing states or ambiguous behavior. Do not request another full menu tour by default.
- [ ] Add a small reusable real-route capture runner around existing `--ui-script`, screenshots,
  `tools/ui_capture` and trace files. Store fixture setup, commands, expected checkpoints and output
  locations in scenario data. Capture entrance, settled state, child return and exit, not just startup.
- [ ] Fix diagnostic blind spots only where needed: optional final transformed bounds/visibility for
  selected objects, accepted/rejected action with reason, requested/played sound, preview generation
  and load/upload/publish timestamps. Current capped local-state traces are not final presentation.

Keep recordings, extracted frames/audio, profiles and executable/install identities private.
Public scenario files/specifications must not contain game assets or machine-specific paths.
Do not make a large new test platform a prerequisite: start with Pause and Challenge Series scenarios.

## Work packages and complete issue coverage

Each row is a work package, not permission to make one large patch. Split by the tickets below.
Shared membership means related acceptance coverage, not an assertion of a shared root cause.

| Package | Issues | Result |
| --- | --- | --- |
| P01 Pause/background composition | 014, 024, 058 | Correct native darkening, alpha, aspect and screen coverage |
| P02 Footer ownership | 004, 009, 022, 027, 053 | One binding-aware footer, no legacy flashes or duplicate hints |
| P03 Lifecycle and cleanup | 001, 002, 003, 023, 033, 034, 040, 052, 060 | Correct return, retained state, transitions and teardown |
| P04 Action/audio agreement | 013, 035, 055 | Correct feedback once per accepted action; correct orbit behavior |
| P05 Selection, scrolling and focus | 005, 006, 025, 029, 030, 037 | Selection, visible window, focus and scrollbar agree |
| P06 EA Trax semantics | 007, 008 | Verified defaults, mode order and context playback |
| P07 Palette behavior | 016, 039, 046, 054 | Correct transformed frame, channel targets and style policy |
| P08 Authored widget presentation | 010, 011, 012, 019, 020, 021, 036, 038 | Correct item state, bounds, textures, text and layout |
| P09 Car identity and presets | 042, 043, 056 | Preview, caption, artwork and default preset agree |
| P10 Performance-stat semantics | 041 | Correct base/current/preview extents and markers |
| P11 Camera and navigation policy | 015, 048, 051, 057, 061 | Correct fresh entry, child return, part framing and intro |
| P12 Preview lifetime and performance | 017, 018, 045, 049, 050 | Responsive preview, no blank room, bounded rebuilds, correct floor |
| P13 Career content and eligibility | 026, 028, 031, 032, 044, 047 | Working remap, correct actions, Bio flow and icons |
| P14 Environment and lighting | 059 | Verified room selection and matched rendering |
| P15 Missing options | Options inventory | Supported controls with proven semantics and persistence |
| P16 Capture completion | OP01, GR01-04, QR01, BL01, RS01, PA01, AM01 and unclassified intervals | Per-behavior comparisons, motion/audio/ambience coverage and new bounded mismatch tickets |

## Order and bounded repair tickets

### Pass A: regressions and obvious disconnected paths

All four have repair candidates; see [implementation and validation scope](ui-parity-pass-a.md).
The acceptance gates below remain open where direct playtest/reference evidence is missing.

- [ ] **A1 / P01: reproduce and repair pause composition.** Capture actual gameplay > Pause >
  Options > Back and Tuning/Help > Back. Inspect texture availability, blend/alpha convention,
  UV/clamp, layer order and viewport/aspect. Fix the established cause in its owning layer.
  Compare 4:3 and 16:9 plus a wide-window control. Gate: no white side panels or opaque central
  block; intended scene remains visible; exit removes the overlay. Extend the current test with
  actual rendered evidence. Do not hide the package or hardcode a dark rectangle to pass.
- [ ] **A2 / P02: Challenge Series prompts.** Describe its actual available actions in the existing
  Footer/Prompt path. Suppress complete keyboard/native groups, including late script reactivation.
  Gate: locked/unlocked rows, tutorial return and device switch show one usable prompt per action.
- [ ] **A3 / P04: Blacklist accept feedback.** Trace navigation versus accepted category entry for
  keyboard, pad and pointer. Find which layer owns playback. Repair that route without adding a
  second sound to working paths. Gate: one reference-matching effect on accepted entry, no spam
  during transitions; verify audio output as well as request count.
- [ ] **A4 / P07: gauge swatch alignment.** Compare final frame/swatch bounds, inherited transforms
  and authored border padding after animation. Reuse paint-picker coordinate conventions where
  equivalent. Gate: all swatches, scrolling and aspect changes align; preserve border animation.
  Keep the already-correct Lettering contract: printed dial/units tint; digital speed/gear defaults.

These four tickets deliver visible improvements and exercise the validation loop before broader work.
They should not wait for an environment overhaul or a universal widget abstraction.

### Pass B: navigation, input and shared presentation contracts

Latest playtest: [Pass A+B follow-up](ui-parity-pass-ab-followup.md) adds a tuning-entry hang
(UI-060) and reversed controller inspection input (UI-061). Reproduce these before treating B1/B2
as accepted; previous route checks did not cover the reported preceding menu sequence or physical input.

See [Pass B candidates and audit coverage](ui-parity-pass-b.md). Return-camera and settings-footer
repairs have regressions; remaining physical-input/reference gates are explicitly retained.

- [ ] **B1 / P03: return-state audit.** Re-run EA Trax save, empty/whitespace profile name,
  Vinyls child Back, customization/Showcase return and each Blacklist category return. Repair only
  failing routes. Distinguish enter, resume and exit; deliver input/Back to its owner once. Save
  selected item and scroll explicitly. Gate: real-route replay twice, rapid Back and failure recovery.
- [ ] **B2 / P11: return-camera repair.** Trace stage/room identity, camera request and intro owner
  across customization > My Cars. Separate new-room initialization from returning to a retained
  overview. Gate: continuous appropriate return motion, no main-menu intro replay or terminal snap.
  Separately compare initial intro clipping and per-part angles; one fix does not certify all three.
- [ ] **B3 / P02: complete footer migration.** Audit settings, gauge, EA Trax, tuning Help and
  controls adapters using the Challenge Series pattern. Keep page-specific actions/layout in data;
  centralize suppression and binding/device rendering. Gate: enter/resume/Help/exit frame sequences
  contain one footer, including temporarily re-exposed native hint groups.
- [ ] **B4 / P05: selection/scroll contract.** Audit EA Trax, Graphics overflow, controls fields and
  career grids as separate adapters. Share current index/visible window/scrollbar calculations only
  when semantics match. Preserve EA Trax Left/Right mode editing. Gate: first/middle/last, wrap,
  disabled/empty/single-item cases, resize and repeated reopen; exactly one active field.
  Repeat-rate checks include cold launch and return, 60/144 and very high render rates. Use an
  actual XInput capture for the remaining physical-hold report; scripts alone cannot close it.
- [ ] **B5 / P08: widget tickets.** Separate slider UV repetition from tuning's centered scale;
  separate spoiler icon transforms from cart glyph alignment and Help wrapping. Reuse shared
  world-bound, text measurement and installed/locked/selected styling helpers. Gate: final bounds,
  transition frames and selected/unselected/disabled states. Remove superseded patches only once
  their real-route regression is covered by the shared implementation.
- [ ] **B6 / P03-P04: cleanup and audio gates.** Test pursuit > menu, queued speech at exit, another
  drive session and animation spam. Tie effects/voices to the owning scene/session and clear its
  transient state on teardown. Preserve settings and legitimate menu audio. Gate: no old blur or
  pursuit speech in menu; rejected input produces no duplicate transition/feedback.

### Pass C: reference decisions and semantic correctness

Reference decisions can be investigated while Pass B runs, but uncertain behavior must not be guessed.

- [ ] **C1 / P09: identity/default-car ticket.** Trace the default M3 GTR preset through assembly,
  vinyl application and publication; distinguish it from owned/customized slots. Recheck bonus cars,
  aliases, fallback artwork and standalone/career names. Gate: default vinyls appear and user-owned
  changes remain intact; displayed identity and published preview always match.
- [ ] **C2 / P10: stat-bar contract.** Observe one stock and one upgraded car plus a proposed upgrade
  in the reference. Identify meanings of dim base, lit extent, yellow marker and comparison state.
  Write that short spec, then use one semantic stat model with separate authored layout adapters.
  Gate: career selection and customization show correct meanings, not merely nonzero blue geometry.
- [ ] **C3 / P13: career content tickets.** Separate Bio state/progression/Continue rules, race-list
  per-event art, Blacklist category art and bounty art. Verify against matched reference clips.
  Gate: rival changes, completed/locked/current states, child Back and action prompts. Avoid replacing
  every race symbol with one guessed icon. Validate remap capture/persistence and Safe House
  eligibility on their actual routes; retain distinct career and standalone rules.
- [ ] **C4 / P06-P07-P11-P13: explicit reference questions.** Resolve EA Trax fresh/reset defaults
  and mode order; gauge color retention on style change; Junkman availability by garage/reward;
  carousel reset/restore policy per route. Record reference identity and decision before coding.
  Modded behavior is labeled separately. New profiles and existing profiles need separate fixtures.
- [ ] **C5 / P15: missing options one domain at a time.** Inventory: standard category Defaults;
  independent Race Information/Split Time/Score/Leaderboard; Game Moment Camera/Car Damage;
  Interactive Music/EA Trax/Audio Mode; Connection Quality Warning; in-menu Resolution/calibration.
  For each: verify meaning, existing consumer, saved scope, live apply and reset. Add only functioning
  controls. Preserve batch 18 independent volumes, context maps and profile Autosave as regression
  controls. Keep extra graphics controls in F8 unless main-menu placement is justified/approved.

### Pass D: performance and environment

- [ ] **D1 / P12: measure before redesign.** Capture option request, disk/decode, room setup, GPU
  upload, shadow/reflection submission and first correct preview frame. Separate cold car, cold part,
  material-only edit, UI/gauge-only edit and cached revisit. Record frame-time spikes, not just FPS.
- [ ] **D2 / P12: repair the measured bottleneck.** Classify UI-only/material/geometry/car/room
  changes. Avoid model/room rebuilds for UI-only edits; update material state without geometry work
  where supported. Retain valid room allocations and static shadow inputs. Genuine geometry changes
  still require correct shadows. Use bounded uploads if upload stalls dominate; stage room work if
  room loading dominates. Do not promise smoothness from decode workers or a larger cache alone.
  Gate: A/B measurements on matched settings, correct final selection under rapid requests, bounded
  resources, no black room, preview-scoped pending/error state, cancel without stale publication.
- [ ] **D3 / P12: tire contact.** Recheck main, standalone and career with different tire/ride-height
  combinations. Existing double-sink correction is a candidate, not a reason for another global lift.
- [ ] **D4 / P14: environment comparison.** Match route, room/camera, car, installation and graphics
  settings before comparing lighting. Check environment choice first, then lights/fog/shading and
  reflection treatment. Tie camera clipping to the verified room geometry. Gate: matched frames
  and transitions; no global brightness compensation for an incorrectly selected environment.

## One validation loop for every ticket

1. **Reproduce:** exact build/source, isolated fixture, real entry route and symptom evidence.
   Capture a failing assertion or before video/pixels/audio appropriate to the fault.
2. **Define:** observed versus expected; reference clip/spec where original behavior is uncertain.
   Select one proven cause. If multiple causes emerge, split the ticket before editing.
3. **Implement:** use the existing owner/shared helper. Keep page differences explicit; do not
   mix career progression/save rules into generic widget code. Keep public files under 500 lines.
4. **Check:** targeted logic regression plus the real route and neighbors using that shared helper.
   Visual fault needs final pixels; audio needs a recording; performance needs frame-time evidence;
   physical input needs a device run. Cover relevant transition frames, not just a settled screenshot.
5. **Review:** save before/after clips or a contact sheet and a concise result manifest. Record
   reference gaps, missing adapters and failing checks. Zero selected tests is never a pass.
6. **Integrate:** run formatting, guards, affected native-only compile/tests and Clippy through the
   maintained validation helpers. Serialize shared-cache/GPU operations. Run required final checks
   on settled source; do not rerun broad suites for every swatch tweak or count executions as parity.

Promote a candidate to checked only for the evidence actually obtained. A full issue is accepted
only when its listed acceptance is covered; keep missing physical/reference checks explicit.
Run the shared cross-screen regression set once after integrating a pass, not once per worker.

## Small-model task format and safe parallelism

Dispatch one ticket, not this entire plan, to a fast coding model. Supply:

- Issue IDs, exact route and fixture, expected behavior/reference anchor, and before evidence.
- Existing owner and allowed files; named nearby screens that must remain working.
- One concrete deliverable and exact focused check commands selected from current tooling.
- Stop conditions: reference behavior unresolved, fixture cannot reproduce, or change needs
  unrelated renderer/save/flow work. Return findings instead of inventing behavior.
- Required return: cause, patch, checks run with actual results, before/after evidence and limits.

Reuse agents by domain if parallel execution is requested: UI presentation; flow/input/audio;
preview/rendering. Assign one owner per shared file and integrate sequentially. Workers may
prepare disjoint source changes in parallel; release builds, shared Cargo and GPU cases remain
serialized. A reviewer checks actual route/rendered evidence, not only the worker's summary.

Do not hand a cheaper model a vague instruction to fix all garage bugs or refactor the framework.
Reconcile evidence before changing tests that fail: historical fixture failures and actual product
failures must stay distinguishable. Do not waive the seven historical installed failures in the
[handoff](ui-parity-handoff.md), or infer that they still fail on current source without a rerun.

## Build and handoff policy

A test build contains a coherent checked pass, exact revision/dirty-tree and executable hashes,
private evidence references and current saves/settings copied without touching the prior bundle.
Use a matching validation manifest and copy the artifact under the build lock. Smoke-check actual
career routes and the changed surfaces; an Add New Car tile cannot certify an owned-car garage.
Update the desktop shortcut only after those checks. Do not produce a new build for each patch.

Give the user a small checklist focused on changed behavior and regressions. Keep the broader
61-issue index and missing-options status in the documentation. Do not ask the user to rediscover
known failures as the main validation method. No implementation, new build or parity sign-off is
claimed by this planning pass.
