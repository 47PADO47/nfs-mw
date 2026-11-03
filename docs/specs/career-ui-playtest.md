# Career UI playtest candidates

Scope: UI-029 through UI-032 in [the playtest backlog](../ui-parity-playtest.md). These candidates do not close
the reports before the full reference comparison and user acceptance.

## Evidence and reproduced source defects

- **UI-029/030:** the sheet's refresh path hid `0xCA9FA6F5`, the complete icon grid. It updated only a counter and
  selected-entry details. It did not bind slot artwork, play highlights or update the scrollbar.
- The installed race, milestones and bounty packages each contain nine `EVENT_ICON_n` buttons, arranged in three
  columns and three rows. Their native highlight/unhighlight scripts and scrollbar images are present.
- **UI-031:** the user identified the Career race list. Its native list slots retained `Mode_Icon_Circuit`
  placeholders, even when the selected event's detail icon used another `RaceType`. Each visible slot now binds
  its event type. Milestones and bounty also retained the Circuit detail placeholder and now bind their own icons.
- **UI-032:** Bio filled its text but did not set the rival's media or notify the package that textures were ready.
  The installed Bio has a background and graffiti. It has no portrait object; empty space does not justify adding one.

These observations come from the repository source and private installed-package dumps. The media-ready sequence
already appears in [career-regions.md](career-regions.md) sections 7/8; no new decompiler evidence is used here.

## Candidate behavior

The sheets use their authored nine-slot layout. Their entry model supplies each slot's icon and locked/completed
marker. The selected slot plays the native highlight, and the departed slot plays its unhighlight. A shared index
drives the slot focus, counter, details and action. See [career-rep-sheets.md](career-rep-sheets.md) for navigation,
scrollbar and selection-memory rules.

Race slots use the existing event-type icon mapping. Milestone slots reuse the installed statistic artwork already
used in the remake's challenge series and pursuit rewards. Speed traps reuse the speedtrap mode icon; bounty uses
the installed `MAIN_ICON_BOUNTY`. These last two choices need reference comparison before claiming exact artwork parity.

Bio binds `RIVAL_NN_BG` and `RIVAL_NN_GRAF` to its existing images and posts the documented texture-ready messages.
Those messages start the installed reveal scripts. Text still comes from the installed language table. No portrait,
video or additional information is fabricated. Footer binding is shared with the frontend's device-aware hints.

## Focused validation

`career_screens/rep_sheets/playtest.rs` contains ignored installed-package regressions for all three grids, markers,
three-column movement, first/last rows, overflow, wheel scrolling, click selection, locked launch rejection, remembered
selection, confirmation and keyboard/controller footers. A synthetic grid test covers wrap and partial-row boundaries.

`career_screens/blacklist_playtest.rs` checks Bio backgrounds/graffiti for rivals 1, 7 and 15, plus visible body text.
`career_screens/rep_sheets/race_playtest.rs` covers all six event icon types in a thirteen-event fixture and checks
the actual race rows in all fifteen installed career bins. It checks selected details and visible slot artwork
separately, including locked/completed markers and overflow.
These checks inspect the runtime/presented trees. They do not establish physical controller input, animation quality,
matched original appearance or the full career launch/return playtest.

Filter the installed regressions with `career_playtest_`; they require `NFSMW_GAME_DIR` and never modify the install.
Baseline copies of the changed source and private package dumps are retained outside the repository. The final
validation record reports executed red/green outcomes separately from these candidate specifications.
