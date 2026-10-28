# UI parity repair coverage: batch 12

Scope: the thirty-five reports in [the playtest backlog](ui-parity-playtest.md), Original UI only.
This records the source repair pass before its subsequent playable-build handoff. Playtest acceptance stays open.
Installed package data supplies artwork, scripts and text. No reference artwork or generated game code is copied.

## Implemented candidates

| Reports | Candidate behavior | Check focus |
| --- | --- | --- |
| UI-001–003 | Earlier fixes retain EA Trax/garage return state and reject empty/whitespace career names. | Repeated returns and valid recovery remain regression controls. |
| UI-004–005 | Earlier fixes suppress complete PC hint roots and retain repeat state on zero-tick/high-FPS frames. | Cold entry, round trip, release/reversal and high-FPS cadence. |
| UI-006, 008–009 | EA Trax uses a sliding row window, Left/Right edits modes, X previews and Y grabs. Native mode labels replace invented labels. | Mode/selection isolation, preview state restoration, reordering and binding-aware footer. |
| UI-010 | Garage markers run their native reveal scripts; unhiding alone left authored alpha/scale at zero. | Locked, available and installed markers. |
| UI-011 | Category footer reserves the selected caption's actual glyph bounds before fitting action hints. | Performance selections, keyboard/controller labels and drawn text bounds. |
| UI-012 | Garage controls use authored highlight/unhighlight tracks where present and avoid refocusing unchanged controls. | Selected/departed state and animation clocks during idle frames. |
| UI-013 | Showcase camera direction input no longer requests generic menu navigation feedback. | Directional sound filtering, with Back and explicit native sound requests preserved. |
| UI-014 | Showcase selects its native normal/wide vignette script when opened, resized or restored. | Actual image visibility/alpha, current aspect and retained-screen return. |
| UI-015 | A screen camera request made during a stage intro is applied after the intro. | Deferred target and later screen changes. |
| UI-016 | Opening a color channel no longer previews the first swatch in unrelated channels. | Unedited gauge/vinyl channels remain unchanged; actual edits update one channel. |
| UI-017–018 | My Cars avoids unchanged geometry uploads, retains three recent geometry previews, and selects the first saved slot before uploading. Native initial loading remains visible and cancellable. | Handle reuse/eviction/cleanup, failed loads, first saved slot and Back release latch. |
| UI-019–020 | Settings fills use texture pitch; tuning uses signed native geometry and positive UV spans. | Zero/partial/full settings and both tuning directions. |
| UI-021–023 | Earlier help wrapping fix is retained. Help gets one device-aware Hide hint. Confirmed discard runs the native tuning exit. | Seven topics, help return, changed-value discard and gated exit. |
| UI-024 | Pause stack loads the previously omitted native InGameBackground package. | One authored backdrop beneath pause, tuning, options and Help. |
| UI-025–027 | Earlier controls highlight cleanup is retained. Keyboard/controller pages share real capture and binding-aware footer actions. | Neutral release, capture/cancel/disconnect, conflicts, axis direction, persistence and page switching. |
| UI-028 | Supported career free roam offers Safe House with context-specific confirmation, including the pursuit-loss warning. | Cancel/recheck, destination, retained career and discarded unbanked pursuit progress. |
| UI-029–031 | Career sheets populate the native nine-slot grids, markers, highlights and scrollbar. Each race slot binds its event type. | Three-column navigation, partial/overflow rows, details, launch eligibility and return selection. |
| UI-032 | Bio binds each rival's background/graffiti and runs the native media-ready reveal. Continue gets a binding-aware hint. | Rivals 1/7/15, native text/media and footer placement. |
| UI-033–034 | Scene transitions reset runtime focus blur and stop/reset police-radio speech state. Pause stops radio-event advancement. | Active/queued/finished speech, pending 911, next session and unchanged saved graphics/audio settings. |
| UI-035 | Leaving carousels, track selection and settings reject further input. A pending tuning confirmation keeps its accepted answer. | Rapid Back/Start/direction presses, supplemental garage actions, pointer fallback and rejected sound feedback. |

Details: [garage](specs/garage-presentation-state.md), [controller remapping](specs/controller-remap-ui.md),
[career pages](specs/career-ui-playtest.md), [lifecycle](specs/frontend-lifecycle.md),
[EA Trax](specs/eatrax-jukebox.md), [tuning](specs/tuning-ui-lifecycle.md).

## Remaining evidence and implementation limits

- **UI-007:** current defaults use the installed song table's DefPlay values. A wrong fresh-profile value or reset
  order has not been demonstrated. The four captured reference rows match current defaults, but the recording does
  not establish a fresh profile or show a Defaults action. Retained settings must be distinguished before changing data.
- **UI-008:** exact reference mode cycle order remains unverified; the four native labels and context modes exist.
- **UI-014:** selecting the existing normal/wide variants repairs a concrete missing message. It does not establish
  matched reference opacity, camera or transition appearance; no replacement artwork or arbitrary opacity was added.
- **UI-015–016:** deferred camera state and unwanted color mutations have concrete repairs. Exact part framing and
  the reported speed-number versus gear-display mismatch still need matched reference checks.
- **UI-017–018:** first-time unseen car decoding/upload still runs synchronously after initial setup. Recent preview
  reuse does not prove smooth first-time browsing or eliminate every loading stall.
- **UI-028:** the Safe House race-session variants are not implemented by this candidate.
- **UI-032:** the installed Bio has no portrait object. Background/graffiti and reveal fixes do not prove every
  expected reference element is present.
- Physical D-pad cadence, audible sound matching, rendered animation/artwork and end-to-end career acceptance
  remain separate from source/runtime-tree tests. The modded install can differ from stock behavior.

## Validation record

Focused groups pass against both the modded and original-style reference installations. These include fifty new
batch regressions, seven career-page checks, sixty-five garage checks, EA Trax, held repeat, name entry and returns,
settings exits, slider geometry, footer bindings, and the batch 10/11 controls. Synthetic preview playback,
controller capture/slots, mode settings and unbanked pursuit checks also pass. Tests use private profile/save paths.

The installed checks use the native renderer configuration with multiplayer enabled. Runtime-tree and tracked-handle
checks do not establish physical-controller behavior, audible quality or matched rendered reference appearance.
Private manifests retain source hashes, exact test binaries, installation identities and command logs. Five controlled
failure checks restored individual old defects, confirmed the corresponding regressions failed, then restored the
exact candidate bytes. The complete contributor run kept source and installation identities unchanged.

The full run passed 10,317 workspace tests, seven native GPU checks, Python tests, licenses, guards and formatting.
Its overall result remains **failed**: Clippy reported 34 warnings (23 recorded previously, eleven new reports at six
locations), and the installed frontend stage recorded 163 passes and seven failures, including child-process results.

## Handoff: remaining work

The user requested stopping the source pass here and continuing later. A subsequent build request packaged this
candidate and updated the test shortcut. The later [batch 12 playtest follow-up](ui-parity-playtest.md#garage-and-career-follow-up--batch-12-playtest-2026-10-28)
records UI-036–044 and continued loading/bounty-icon failures under UI-017–018 and UI-030. Those findings remain
open; startup and focused checks did not establish their acceptance. No follow-up source repair is claimed.

- [ ] Fix the eleven new lint reports: nested conditions in career icons, caption layout and preview eviction;
  indexed loops in native-hint suppression; redundant SpeechTune struct update in the dispatcher test.
  PR preparation addresses these six locations; fresh diagnostic outcomes are recorded in the PR description.
- [ ] Triage the career cash-payment return and native-carousel question-reopen failures. These are additional
  failing case names compared with the last recorded full run; their causes and when they began remain unverified.
- [ ] Retain the five prior failing cases: arrest-ticket return, third-strike return, the modded cursor's size,
  windowed-category pointer selection, and the main-menu Challenge Series fixture. They are not passes or waivers.
- [ ] Rerun the contributor workflow after cleanup. Keep the focused checks and controlled failures alongside it.
- [ ] Complete the reference/physical checks and implementation gaps listed above, including first unseen car stalls
  and the exact gauge-display report. Passing source tests does not close playtest acceptance.

The [continuation handoff](ui-parity-handoff.md) carries the later playtest requests and exact installed failure
names. This historical record is not a claim that the integrated PR revision passed the same checks.
