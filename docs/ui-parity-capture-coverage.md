# Capture coverage and unfinished parity work

The playtest issue list is not the full capture backlog. A reported bug, a repair candidate and a
completed comparison are different records. The four repair stages do not replace this ledger.

Basis: the original long-recording case inventory and task queue, reconciled with the current repair
plan on 2026-11-01. The windows below are chapter bounds from that inventory, in session seconds.
This document does not claim a new full viewing/listening pass or frame-accurate A/V alignment.
Media, event indexes, executable identities and the original queue remain private.

## Recorded families to reconcile

All rows remain open for capture acceptance. Existing fixes can cover a subset without completing
its row. Reuse the chapter IDs when creating narrower behavior tickets.

| Chapter | Session window | Recorded scope | Unfinished comparison to track |
| --- | --- | --- | --- |
| OP01 | 18-48 s | Options categories, display/calibration, rows | Exact row meanings/defaults/reset, overflow, footer transitions, display calibration, independent missing controls |
| GR01 | 60-71 s | My Cars saved slot, Add tile and stock selector | Saved versus Add tile eligibility, preview/identity, available prompts and transition frames; the original Add-tile pilot was auto-checked, not user-accepted |
| GR02 | 71-123 s | Customization categories/parts, spoilers/rims, cart and dialogs | Carousel motion, selected/locked/installed states, cart contents and answer/Back side effects, interrupted transitions and contextual camera |
| GR03 | 123-151 s | Paint and vinyl browsing | Page grouping, palette/window behavior, color-channel targets, preview versus committed state and complete child-return chain |
| GR04 | 151-176 s | Gauge styles/colors and cart return | Channel semantics, swatch geometry/pulse, style-change retention, default digital colors, preview and saved result |
| QR01 | 181-198 s | Quick Race track selection/options/return | Semantic mode/track identities, available settings/defaults, preview/artwork, remembered selection and complete return route |
| BL01 | 202-222 s | Blacklist overview, milestones, bounty and Bio | Per-page conditions, content/icons, highlight/scroll, progression/Continue/Back, category return and accepted-action sound |
| RS01 | 237-258 s | Rap Sheet login, main and detail/return | Natural versus skipped login, presentation/motion, detail state, sustained cue ownership and stop/re-entry behavior |
| PA01 | 264-285 s | Pause, Performance Tuning and dialog | Entry/exit/Help, preceding-menu state, pause layering, focused row, dialog answer and accepted/rejected input/audio |
| AM01 | 310-320 s | Candidate main-menu idle interval | First establish foreground/render/audio usefulness; then identify room ambience/effect owners. Heartbeats alone do not prove ambience coverage |

The intervals outside these chapters also need classification. Do not silently discard the start,
intervening transitions or ending because they lack a prepared case. Mark each interval as reviewed,
unusable with a reason, or still unreviewed. Conditions never visited remain reference-needed.

## Work missing from the four-stage summary

1. **Recorded flows beyond the recent bug list:** Quick Race, Rap Sheet login/details, cart/modal
   answer variants, display calibration and intermediate menu transitions. Audit before assuming a defect.
2. **Motion and shared widgets:** compare complete entrance/settled/exit and child-return sequences;
   include carousel interpolation, focus/highlight, clipping, captions, glyphs and aspect variants.
   Reconcile the earlier rotation-interpolation candidate with the current source and its evidence.
3. **Sound and ambience:** inventory navigation/accept/back/rejected cues, screen overrides, sustained
   effects and idle ambience separately. Verify requested event, bank/variant, actual voice/PCM and
   lifetime. A fixed Blacklist accept request does not complete the audio audit.
4. **Environment effects:** retain steam/haze/bird and other authored-effect gaps from the earlier
   menu audit. Hidden emitter/helper meshes do not mean their effects were implemented.
5. **Options and conditional states:** carry the full [options inventory](ui-options-coverage.md),
   disabled-action reasons, popup conditions and profile/progression variants. Use existing specs
   and targeted research when the recording does not establish the condition.

## Reconcile before dispatching another implementation batch

- [ ] Compare the original task queue, every chapter above, earlier candidate reports, the options
  inventory and UI-001-061. Keep links to existing fixes instead of creating duplicate fix tickets.
- [ ] For every chapter, list observed behaviors separately. Record reference bookmark, remake route,
  allowed intentional differences, current candidate, latest evidence and remaining acceptance.
- [ ] Give each behavior independent statuses for behavior, pixels, motion, input and audio. Use
  `not-reviewed`, `reference-needed`, `mismatch`, `candidate`, `checked` and `accepted` explicitly.
- [ ] Review the unclassified recording intervals. A missing expected contract remains incomplete;
  package visits, a green unit test or an old queue status cannot supply one.
- [ ] Promote bounded, reproducible mismatches into the four repair stages. Include capture-only
  work even when no user screenshot or numbered UI report exists.

The initial inventory has ten chapter groups; only the Add-tile case had a populated expected-behavior
contract. The others require finer contracts. The old queue's dependency/status fields are historical
and must be reconciled with later batches, not treated as proof that work is untouched or complete.

## Acceptance limits

The recording has variable frame delivery, uncalibrated fine A/V delay, two marked audio gaps and
incomplete save/progression identity. Its final probe tail is not established as complete. Preserve
these limitations. Re-record only a missing/ambiguous state, not the entire menu tour by default.

A retroactive sync estimate over OP01 and BL01 (2026-11-02) measured these pipeline delays:

| Measure | Samples | Median | Interquartile range |
| --- | --- | --- | --- |
| Sound request to audible onset | 300 | 102 ms | 96-110 ms |
| Focus change to first changed frame | 32 | 117 ms | 22-186 ms |

Use the audio figure to align cues. The video figure is too spread to use for frame-accurate motion claims.
New targeted sessions start with a sync routine and snapshot saves/settings, so the profile state is identified.

Keep the new tuning hang and reversed controller inspection report as active regressions alongside
this capture work. Current Pass A/B checks cover named cases; they do not close these chapter groups
or establish complete original-game UI, animation, sound or ambient parity.
