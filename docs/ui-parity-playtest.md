# Original UI playtest backlog

Reported: 2026-10-27, on the batch 6 UI parity test build, with batch 8, batch 9 and 2026-10-28 batch 12 follow-up findings. Scope: Original UI.
All items remain **open for playtest acceptance**. Reproduced failures and candidate coverage are recorded below.
UI-005 has scoped gated-release and high-frame-rate repairs; its full physical-input report remains unverified.
Other reports require reproduction before a repair or closure can be claimed.
The [batch 12 coverage record](ui-parity-batch12.md) records the later source repair pass and remaining limits.
Twenty-seven supplied screenshots are preserved in private evidence archives; game images do not belong in this repository.
Screenshot observations establish visible state, not input history, sound, timing or original-game equivalence.

This backlog supplements [menu parity](menu-ui-parity.md), [garage parity](garage-parity.md) and
[missing options](ui-options-coverage.md). Existing implementation and passing tests do not close these reports.

## State loss and invalid input — address first

Batch 7 candidates: EA Trax now completes its authored Accept exit and returns to the remembered menu; retained
Garage carousels replay their entrance on Back; name entry validates submission without starting the notice's exit.
Installed regressions cover repeated returns, visible text/footer, saved play order, held-input guards, empty/whitespace
names and valid recovery. Garage controls include nondefault selections and Showcase return. These checks do not
close the other EA Trax or garage reports, prove physical input, or certify original animation timing.

- [ ] **UI-001 — EA Trax saving leaves a blank UI.** Reported path: change/save EA Trax settings.
  Reproduce the exact save/Back sequence. Validate a visible, usable return screen, retained focus and saved values
  after reopening. Test controller and keyboard; retain an end-to-end capture of the transition.
- [ ] **UI-002 — My Cars garage loses UI after Back.** Some carousel graphics can remain while other UI disappears;
  screenshot S1 shows incomplete chrome. Record the child screen and Back route before diagnosing the cause.
  Validate title, footer, selection and input after returning from each affected child and after repeated visits.
- [ ] **UI-003 — Empty career name breaks the UI.** Reported trigger: submit a career name with no characters.
  Validate that an empty name is rejected without leaving a usable entry screen or creating an invalid profile.
  Cover empty submission, cancel/reopen and then entering a valid name. Test whitespace-only input separately.

## Input, prompts and EA Trax

Batch 8 candidates: settings replacement footers suppress their separate native key artwork, including PC keys
re-exposed by exit scripts. A repeat-state correction
resets the fast-repeat flag when its direction is released while package input is gated. The latter isolates a stale
state defect; it does not establish every reported physical D-pad inconsistency. Both items remain open for playtest
acceptance. See [settings input](specs/settings-input-parity.md) for scope and transition checks.

Batch 9 follow-up: removing the key alone left its striped hint box visible. Replacement presentation now suppresses
that complete native hint. A separate repeat correction preserves fast repeat on frames that advance zero UI ticks;
the prior 60–144 Hz controls missed the failure at the reported 2,163 FPS. Installed cadence checks now include
2,163/2,400/4,800 Hz, before and after a menu round trip. These are controlled runtime checks, not a physical-input
or frame-pacing certification.

- [ ] **UI-004 — Keyboard prompt briefly overlays a settings button during Back, even on controller.**
  Capture the exit frame sequence with controller input held constant. Validate that legacy keyboard art does not
  flash through the controller footer. Cover main and pause settings, including an actual device switch control.
- [ ] **UI-005 — Held D-pad Up/Down moves between settings inconsistently.** This is not closed by earlier fixes
  for duplicate D-pad/wheel input. Record initial press, sustained hold, direction reversal and release separately.
  Compare selection changes and repeat timing with the reference; check that render rate does not change navigation.
  Follow-up: choppy immediately after launch, smooth after entering a menu and returning. Cover both states; do not
  hide the issue by imposing a frame limit.
- [ ] **UI-006 — EA Trax scrolling is buggy.** Follow-up identified Left/Right paging instead of editing the mode.
  Batch 9 makes Up/Down scroll a sliding four-row window and updates the scrollbar; Left/Right keep the selected
  track in place and change its mode. The original's exact scrolling animation and the full playtest remain open.
  Check held D-pad, individual presses, keyboard and wheel at the first, middle and last rows. Validate that selection,
  visible row window, displayed track and preview action stay aligned, including after leaving and reopening.
- [ ] **UI-007 — EA Trax defaults are incorrect.** Record the observed initial values and the Defaults action
  separately. Establish reference defaults before changing them; distinguish a fresh profile from retained settings.
  Validate reset scope, displayed values and persistence after save/reopen.
- [ ] **UI-008 — EA Trax mode choices are incorrect or unavailable, including menu/race track choices.**
  The source inventory's related jukebox modes do not prove usable on-screen controls. Establish the reference's
  exact choices and cycle order, then check display, input, playback context and persistence for each supported mode.
  Batch 9 writes the current mode into the actual right-side column, keeps song titles clean, and makes Accept
  save/exit without cycling the mode. Defaults and exact original cycle order are not certified.
- [ ] **UI-009 — EA Trax footer lacks controller input icons.** Validate prompts for every action available on the
  current screen and state. Test controller/keyboard switching, entrance, scrolling, save and return. A displayed
  controller icon must correspond to the bound action; do not add decorative hints for unavailable actions.
  Batch 9 repairs the missing native footer-layout mapping and supplies hints for the remake's actual bindings.
  Batch 12 adds the recorded X Preview/Y Grab actions; see the source candidate record. Initial
  selection checks passed; the later user clarification identified the separate Left/Right mapping defect.

## My Cars garage presentation and behavior

- [ ] **UI-010 — Lock and checkmark indicators are missing or incorrect.** Capture locked, unlocked, selected,
  previewed and installed states separately; establish which indicators each state requires. S3 contains one visible
  lock, so it is not evidence that every lock is missing. Validate each marker against the actual item state.
  Batch 12 follow-up separately reports marker alignment in the shopping cart; see UI-038 and S21.
- [ ] **UI-011 — Garage text overlaps.** S2 visibly shows performance/footer text colliding. Validate title, item
  label and footer bounds throughout entrance/exit and selection changes, with keyboard and controller prompts.
  Include long labels and supported aspect ratios; settled screenshots alone cannot close transition overlap.
- [ ] **UI-012 — Garage UI colors and active highlights are wrong or absent.** Record selected, unselected,
  disabled, locked and installed states. Compare authored colors and highlight animation with the reference.
  Validate state changes as well as a static frame; do not infer correct eligibility from color alone.
- [ ] **UI-013 — View Car plays the wrong sounds while moving the camera.** S4 identifies the Showcase context,
  but a still image provides no audio evidence. Correlate orbit/zoom inputs with requested and played sounds.
  Compare reference audio; check for incorrect menu-navigation effects, duplicate triggers and continuous holds.
- [ ] **UI-014 — Garage/Showcase vignette overlay does not match.** Compare the same screen, camera, aspect ratio
  and transition phase before adjusting artwork, opacity or bounds. S1/S4/S5 preserve remake examples; they do not
  independently establish the correct reference appearance.
- [ ] **UI-015 — Some part/style screens use the wrong camera angles.** Record the specific part/style and car,
  including direct entry, category changes, Back and revisit. Validate the intended part is framed after asynchronous
  car loading and after a later upload completes. Capture transitions, not only the final pose.
- [ ] **UI-016 — Gauge customization targets the wrong element.** User reports the speed number changes where
  the gear display should change. S5 records the gauge preview. Reproduce the exact style/color selection and
  identify its intended element before remapping it. Validate preview and applied Original HUD independently;
  changing one channel must preserve unrelated gauge elements.

## My Cars loading

- [ ] **UI-017 — Car selection/loading is laggy and slow.** Measure first load and repeated visits per car, including
  rapid selection changes. Separate asset reading, decoding, upload and first correct visible frame. Investigate reuse
  before promising instant loading; do not invent a performance threshold or accept a stale preview as completion.
  Batch 12 playtest: switching cars still hangs. The requested result is quick entry and smooth switching in both
  standalone and career My Cars; the preview cache candidate has not satisfied this report. See also UI-043.
- [ ] **UI-018 — Loading has no useful visible state.** Evaluate this alongside UI-017. If noticeable waiting remains,
  show clear pending/ready/error state and prevent the displayed car, selected name and accepted action from
  disagreeing. Batch 12 clarification: the full-screen garage loading screen is incorrect for My Cars. Keep the menu
  visible; if a car needs noticeable loading, use an indicator scoped to its preview rather than replacing the whole
  screen. Validate first entry, first unseen car, cached cars, rapid browsing, cancel/Back and failed loads.

## Shared slider artwork

- [ ] **UI-019 — Slider fills stretch instead of repeating their segmented texture.** Batch 8 Audio screenshot S8
  shows continuous/stretched gold fills; provided reference S9 shows segments. Inspect the shared settings slider
  path used by other categories too. Check zero, partial and full values, main and pause layouts, selected/unselected
  rows and repeated visits. Batch 9 uses texture pitch for the UV span; rendered comparison and playtest acceptance
  are required in addition to geometry tests.

## In-game tuning, pause and controls — batch 9 playtest

These are new user reports, not independently reproduced repairs. S10–S19 are the ten supplied follow-up images.
The user notes that some behavior may also appear in the existing reference video; exact reference timestamps
have not been matched. Stills do not establish animation, input history or audio behavior.

- [ ] **UI-020 — In-game tuning row presentation is inconsistent.** S10–S11 show differing label colors,
  selection treatment and slider artwork: some rows contain segmented fills over the native centered scale.
  Check each row's selected/unselected/disabled state, centered and nondefault values, and preset changes.
  Compare with reference before deciding which layers or colors should remain. Keep separate from UI-019's
  standard settings slider fix; its passing checks do not cover this tuning screen.
  Batch 10 candidate: native row scripts explicitly restore departed-label color and preserve disabled alpha.
  Installed selection regressions cover this subset; slider layering and rendered/reference acceptance remain open.
- [ ] **UI-021 — Tuning Help text overlaps and overflows.** S12 shows overlapping description lines and text
  reaching the popup footer. Validate layout, clipping and scrolling for every tuning help description, including
  long text, opening/closing Help, input-device changes and repeated visits.
  Batch 11 candidate: manual wrapping now uses the renderer's unscaled layout width; installed checks cover
  all seven topics and scrolling. This fixes the reproduced second wrap per slot, not every popup layout concern.
- [ ] **UI-022 — Tuning footer mixes duplicate keyboard and controller prompts.** S10 shows overlapping Cancel,
  Accept and Help labels plus a separate keyboard Help hint. Validate one usable hint per action for the active
  device, during entry, selection, Help and exit. S11 preserves a keyboard-state comparison; do not infer input
  history from either still. Cover the Help popup's Hide binding too.
  Batch 11 reproduces the native PC footer returning after Help closes and suppresses those complete roots for
  tuning presentation. The popup's own keyboard Hide hint still needs a controller-aware replacement.
- [ ] **UI-023 — Back from in-game tuning has no exit animation.** Capture the complete Cancel/Back route and
  compare the reference's exit sequence, destination and input gate. Check Accept separately, repeated visits
  and held/spammed Back; do not substitute an arbitrary delay for the authored transition.
  Batch 11 candidate: clean Cancel, Accept and Start request the native leave sequence and wait for EXIT_COMPLETE
  before closing/saving. Inputs are ignored while leaving; failed-save return restores the retained screen.
  Changed-value discard confirmation's final exit remains separate and unverified. See [lifecycle](specs/tuning-ui-lifecycle.md).
- [ ] **UI-024 — In-game menu background lacks the expected darkening.** Reported with tuning; S10–S16 also
  show the driving scene behind pause/options/controls screens. Establish darkening scope and opacity per screen
  from reference, including Help overlays and entry/exit. A blurred background alone does not establish parity.
- [ ] **UI-025 — Controls navigation leaves previous fields highlighted.** D-pad, keyboard arrows and stick
  reportedly accumulate highlights along the navigation path, described as a snake trail. S14 preserves the
  controls context. Reproduce with each input source; validate that only the current eligible field stays active,
  including row/column changes, scrolling, remap capture, cancellation and leaving/reopening the page.
  Batch 10 candidate: controls logic explicitly restores the native unselected field script, absent from the
  package's lose-focus responses. Installed checks cover pad navigation, pointer focus and the resulting remap target.
- [ ] **UI-026 — Controller remapping page is still missing.** User cannot reach the controller remap page;
  S14 shows Keyboard Configuration only. Record the route and connected device, compare available reference
  pages, and validate page switching, actual reassignment, cancel/reset behavior and persistence. This is an
  open user report; the screenshot alone does not prove the underlying input system lacks remapping support.
- [ ] **UI-027 — Controls page bottom strip has no action prompts.** S14–S15 show an empty footer bar below
  the remapping instructions. Validate the available actions and matching controller/keyboard icons, including
  normal navigation, active remap capture, cancel and return. Check other affected in-game pages separately.
  Batch 10 candidate: pause controls now uses its native animated footer frame for replacement binding-aware
  Select/Column/Remap/Back hints and suppresses the old PC Back art. Other pages and physical playtest remain open.
- [ ] **UI-028 — Pause menu lacks Back to Safe House.** S16 preserves the pause/free-roam context. Establish
  when the reference offers this action, including career state and pursuit restrictions; validate visibility,
  enabled/disabled behavior, confirmation, destination and return. Do not add an unconditional shortcut without
  checking those conditions. Link this missing action in the options inventory.

## Career pages — batch 9 playtest

- [ ] **UI-029 — Milestones page lacks list icons, working scrolling and selection highlights.** S17 shows a
  populated description above an empty-looking list region, scrollbar and footer captions without input icons.
  Validate row population, icons, selected state, description synchronization, scrollbar range and input at the
  first/middle/last entries. Cover controller, keyboard, wheel, repeated visits and launch/Back behavior.
- [ ] **UI-030 — Race and bounty views have similar missing UI and navigation failures.** User reports the
  same class of faults as Milestones. Record each affected page separately; do not assume a shared cause.
  Validate icons, rows, highlights, scrolling, details, footer actions and return state for each page.
  Batch 12 playtest: bounty-page icons are still wrong. S25 shows the same police-badge artwork repeated across
  the visible grid and in the selected detail header. Establish the correct per-entry reference art, then check
  header/grid agreement, selected/unselected states, scrolling and return. Populated slots do not close this report.
- [ ] **UI-031 — Career race-list icons appear incorrect.** The user clarified the affected page on 2026-10-28.
  Batch 12 replaces each list slot's Circuit placeholder with the actual event-type icon, synchronized with details.
  Validate selected, unselected, locked and completed variants against the reference.
  The later report also identifies the Blacklist menu's Race Events category icon; track that distinct surface as
  UI-044. The per-event list binding change does not establish the category icon is correct.
- [ ] **UI-032 — Blacklist Bio page appears incomplete.** S18 shows Razor's text and graffiti with an otherwise
  sparse right side and a Continue caption without an input icon. Inventory expected portrait/media, information,
  layout and available actions against reference; do not invent missing content from empty space alone.
  Validate multiple rivals, entry, scrolling if applicable, Continue/Back and controller/keyboard prompts.

## Leaving gameplay and animation input — batch 9 playtest

- [ ] **UI-033 — Gameplay motion blur persists after returning to the main menu.** S19 visibly shows radial
  blur in the garage menu; the user reports it remains after exiting gameplay. Capture driving/pursuit state,
  exit route and settled menu frames. Validate reset of gameplay effect state without changing the user's saved
  graphics preference, including repeated drive/menu cycles and a cold-menu control. Cause not yet established.
- [ ] **UI-034 — Police radio sometimes continues after exiting gameplay.** Intermittent audio report, not
  verifiable from stills. Capture audio and event/lifetime traces across pursuit-to-menu transitions; check queued,
  active and newly requested speech, repeated exits and a subsequent gameplay session. Validate intended menu
  audio remains correct. A cleanup defect is a hypothesis, not a confirmed cause or a reason to mute all sound.
- [ ] **UI-035 — Repeated button presses during animations repeatedly trigger sounds.** Identify the affected
  screens, actions and transition phases. Correlate raw presses, accepted actions, queued requests and audible
  playback. Validate rejected/gated actions do not spam feedback or replay transitions; preserve intended held
  navigation sounds and valid actions after the transition. Include rapid presses, holds, release and re-entry.
  Batch 11 gates additional tuning edits, Help toggles, pointer and wheel input during its new pending exit.
  This is scoped input protection, not a reproduction or closure of the broader audible-spam report.

## Garage and career follow-up — batch 12 playtest, 2026-10-28

The following user reports arrived after the batch 12 build handoff. S20–S27 are the eight supplied screenshots,
in attachment order. Their executable identity has not been independently matched. These are open findings;
no repair or independent runtime reproduction is claimed. Loading and bounty findings extend UI-017–018 and UI-030
above. The report asks for documentation only.

- [ ] **UI-036 — Spoiler item carousel is broken.** S20 shows the Spoilers / Groundmaster screen at item 4/15,
  with distorted/overlapping-looking thumbnail artwork and placeholder circles. Reproduce item and style changes
  separately. Validate each visible thumbnail's bounds, scale, selected marker, item name/counter and car preview
  through forward/reverse browsing, wraparound, style changes and Back/re-entry. Do not infer the cause from a still.
- [ ] **UI-037 — Scrollbars are broken in some option menus, including Graphics.** Other affected categories
  were not named, and no screenshot isolates this fault. Check every standard options category with overflow.
  Record whether the defect is visual, range/position, input or row synchronization. Validate first/middle/last rows,
  held navigation, keyboard/controller, wheel/drag where supported, category changes and reopening, in main and pause
  menus. Keep this separate from the EA Trax scrollbar report.
- [ ] **UI-038 — Checkmark icons are misaligned.** S21 shows shopping-cart checks above/left of their item text
  rows. Compare authored marker/row alignment with the reference. Validate first and later rows, selection changes,
  scrolling, removal/clear/checkout, transition phases and supported aspect ratios. Check shared checkmark consumers
  separately; correct marker visibility or state under UI-010 does not establish correct placement.
- [ ] **UI-039 — Custom paint colors are split into one-color pages with internal names.** S22 shows PEARL1
  with a single swatch and 1/1 counter. The requested presentation is one **Custom** page containing these colors,
  rather than a page per internal material name. Validate complete swatch population, ordering, selection, preview,
  applying/canceling, saved color restoration and Back/re-entry. Preserve the distinction between a display label
  and the material identity needed to apply the chosen color.
- [ ] **UI-040 — Back from Vinyls returns to the main menu instead of one level up.** Record the entry route
  and selected category. Validate keyboard Back/controller B restores the immediate parent garage menu, its selected
  item, car preview and footer. Cover repeated visits and pending vinyl/cart changes; do not confuse a deliberate
  full garage exit with leaving the vinyl child screen.
- [ ] **UI-041 — Car performance-stat colors are incorrect.** S23 and S26 show blue segmented Top Speed,
  Acceleration and Handling bars with yellow markers. Establish the expected reference palette before changing it.
  Validate base/upgrade/comparison segments and markers, selection changes, standalone and career car selection,
  and repeated visits. Correct stat values or segmented geometry alone do not close the color report.
- [ ] **UI-042 — Car names are incorrect in the My Cars garage.** S24 shows the internal-looking label
  `caymans` above the manufacturer grid. Validate the selected car's display name, manufacturer and model artwork
  against its actual preview and saved slot. Cover multiple cars, modified cars, rapid switching and reopen; internal
  identifiers must not appear as normal display names. Career-specific wrong names are also tracked in UI-043.
- [ ] **UI-043 — Career My Cars is broken: preview, names, options and icons.** The user reports no working car
  preview, incorrect names, wrong options and missing icons. S26 contains a Porsche preview but a Mitsubishi badge
  and `Debug: carrera_gt` text; a visible car in this still does not establish correct live preview updates. Record
  the exact safe-house/career entry route. Validate owned-car population, preview/selection synchronization, proper
  display names and manufacturer art, heat/stats, eligible actions and their keyboard/controller icons, saved-car
  selection, switching, Back and re-entry. Treat career My Cars separately from the standalone My Cars manager;
  startup or Add New Car smoke checks do not cover this route. Include UI-017–018's loading acceptance.
- [ ] **UI-044 — Blacklist menu Race Events category icon still appears incorrect.** S27 identifies the selected
  Race Events icon beside Milestones and Bounty. Match this exact category icon to the reference; it is a separate
  surface from UI-031's per-event race-list icons. Validate native resolution, aspect/scale, selected/unselected
  appearance, transition states, rival changes and enter/Back behavior. The expected replacement art is unverified.

## Evidence and closure rules

Private screenshot IDs, in attachment order:

| ID | Visible context | Relevant reports |
| --- | --- | --- |
| S1 | Red Porsche, tilted garage view, carousel icons, incomplete surrounding chrome | UI-002, UI-014 |
| S2 | Performance carousel and overlapping footer/item text | UI-011, UI-012 |
| S3 | Paint PEARL2 page, counter, empty-looking tile and a visible green lock | UI-010, UI-012 |
| S4 | Porsche Showcase side view and keyboard footer | UI-013, UI-014, UI-015 |
| S5 | Custom Gauges / Needle preview and color swatches | UI-015, UI-016 |
| S6–S7 | Batch 8 HUD footer and enlarged crop; old hint box remains behind replacement hints | UI-004 |
| S8 | Batch 8 Audio sliders with stretched fill appearance | UI-019 |
| S9 | Provided reference Audio screen with segmented fills; executable identity unverified | UI-019 |
| S10 | In-game tuning, mixed slider layers and overlapping keyboard/controller footer | UI-020, UI-022, UI-024 |
| S11 | Tuning with keyboard footer and inconsistent row colors/slider layers | UI-020, UI-022, UI-024 |
| S12 | Steering Help popup with overlapping/overflowing text and keyboard Hide hint | UI-021, UI-022, UI-024 |
| S13 | In-game Options carousel over the blurred driving scene | UI-024 |
| S14–S15 | Keyboard Configuration and crop of its empty bottom strip | UI-025, UI-026, UI-027 |
| S16 | Pause carousel showing Resume Free Roam | UI-024, UI-028 |
| S17 | Milestones description with empty-looking list region and missing footer icons | UI-029 |
| S18 | Razor Bio page with sparse right side and Continue caption without input icon | UI-032 |
| S19 | Main-menu garage retains visible radial blur after reported gameplay exit | UI-033 |
| S20 | Spoilers / Groundmaster, item 4/15, distorted-looking carousel thumbnail | UI-036 |
| S21 | Shopping cart with checkmarks offset above/left of the item text | UI-010, UI-038 |
| S22 | Paint PEARL1 page with one swatch and 1/1 counter | UI-039 |
| S23 | Blue performance-stat bars, yellow markers and cropped GTO model artwork | UI-041 |
| S24 | Manufacturer grid with internal-looking `caymans` heading | UI-042 |
| S25 | Bounty page with repeated police badges, selected Ocean Hills Police Station | UI-030 |
| S26 | Career My Cars context: Porsche preview, Mitsubishi badge and debug car name | UI-041, UI-043 |
| S27 | Blacklist menu with Race Events selected beside Milestones and Bounty | UI-031, UI-044 |

Association identifies context, not proof of the full reported fault. In particular, S4 alone cannot establish which
physical input device was active. Reports without a screenshot remain valid open user findings.

For each fix:

1. Record a reproducible trigger, current state and expected result. Obtain reference evidence where behavior is unknown.
2. Keep one issue or proven shared cause per task. Save a failing regression or before-capture before changing code.
3. Validate the real menu route, transition and return state, including repeated visits and affected input devices.
4. Review sound, presentation and persistence where relevant. Unit tests, traces and pixels prove different things.
5. Keep the item open until its acceptance checks have evidence. Record partial coverage and unresolved behavior explicitly.

Suggested order: UI-001–003, then shared input/prompt and EA Trax issues, then separate garage presentation tasks.
Measure loading independently; neither a loading indicator nor a cache alone establishes acceptable responsiveness.
Track gameplay-exit effect/audio cleanup separately from static page artwork. For batch 9 follow-ups, establish
the runtime trigger for UI-033–035 and the controls focus defect before grouping fixes by an assumed shared cause.
For the batch 12 follow-up, reproduce wrong-parent Back and career My Cars state first. Audit option scrollbars
separately, then group remaining presentation defects only when a shared cause is demonstrated. UI-017–018 and
UI-030 remain open despite the earlier preview-cache and populated-grid candidates.
