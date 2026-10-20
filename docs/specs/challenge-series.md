# Challenge series screen

The main menu's Challenge Series icon opens `ChallengeSeries.fng` (the original's `UIQRChallengeSeries`). Code:
`crates/nfsmw/src/frontend/challenge_series/`.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0): `uiQRChallengeSeries.cpp`,
  `feArrayScrollerMenu.cpp`, `uiTrackMapStreamer.cpp`, `FEAnyTutorialScreen.cpp`, `feDialogBox.cpp`, `feWidget.cpp`
  (`FEScrollBar`), `FEDatabase.cpp` (icon and text hashes), `UnlockSystem.cpp`. Ghidra on `speed.exe`: the screen's
  message handler (code at `0x007AE680`, not a defined function; read with `read_memory` and disassembled outside Ghidra).
  Read for understanding; no code copied. Tags: **[decomp]**, **[exe]** (checked in `speed.exe`), **[verified]** (checked
  against the install), **[ours]**, **[unconfirmed]**.

## 1. The list

Bin 19's `WorldRaces`, in the bin's order (70 in this install: 35 tollbooth trials, 34 pursuit challenges, the burger-chain
race `19.8.31` and the Black Edition race `19.9.70`) **[verified]**. The original hides `19.8.31` until an easter egg and
`19.9.70` outside the Black Edition; this build shows both **[ours]**. Grid 4 x 3 (`ArrayScrollerMenu(sd, 4, 3, true)`).
Left and right wrap inside a row, up and down stop at the ends, the window scrolls a row at a time **[decomp]**.

## 2. What the selection writes **[decomp, exe]**

- Header label `CHALLENGE_SERIES_DESC_<tag>_HEADER` and text `CHALLENGE_SERIES_DESC_<tag>`; `<tag>` is the race's
  `LocalizationTag` (equal to the event number) **[verified]**. `%s` is the goal: minutes for `pursuit_length` and
  `pursuit_evasion_time`, else the number (thousands separators **[unconfirmed]**).
- Event picture by `MilestoneName` (attribute hash = the type key the executable switches on **[verified]**); no statistic
  is the tollbooth picture.
- Counter `n/total`; padlock on a locked event, tick on a won one; scroll bar (`FEScrollBar::Update`).
- Locked panel text from the position alone (`CHALLENGE_SERIES_UNLOCK_MSG_01/02`): index < 61: place = (i+1) % 5, page =
  (i+1)/5; places 1,2 name `page*5`; 3,4 name `page*5+1` or `+2`; 0 names `(page-1)*5+3` or `+4`; index >= 61 names `i`.
- Messages to the package: `0xB295482E`/`0x0F7B54C7` tutorial hint on/off, `0xC5DD9D68`/`0x38091FA1` locked panel
  on/off, `SHOW_HELP` for the keys footer **[ours]** (the package needs it on a PC).

## 3. Map

`TRACK_MAP_<event id with underscores>` in `TrackMaps.bin` (1024 x 1024; all 70 exist **[verified]**); the route or the
spawn arrow is baked into the picture. The view is the square around the box of start, checkpoints and finish plus 0.125
on each side, eased over 1 s from the previous view (`ZoomToTrack`, `PanToTrack`). A view that would run off the picture
is moved back inside **[ours]**.

## 4. Unlocking

Open when `InitiallyUnlocked`, or a won event lists it in `UnlockRaces` (the original keeps this in the profile's race score
flags). Wins are kept in `challenge_series.toml` next to the config file **[ours]**.

## 5. Launching **[exe]**

Accept on an open event: alert dialog "Are you sure you want to launch this event?" (Yes/No, No focused); Yes: selection
dialog "Select your transmission." (Manual `0x317D3005`, Auto `0x8CD532A0`, the current setting focused); the choice is
stored as the transmission setting and the event starts. Back leaves for the main menu.

The tutorial button (`PAD_BUTTON4`, key 2): the executable plays `tollbooth_tutorial` only for events with no statistic;
this build also offers `pursuit_tutorial` (length, evasion time) and `bounty_tutorial` **[ours]**.

## 6. Wins and what is open

A won event outside the career (a race finished first, or a pursuit challenge escaped with its goal reached) is written by
`challenge_series::record_win` (`scenes/world/race/finish.rs`, `challenge.rs`). The screen does not yet reopen after a race;
the pause menu's Quit to Main Menu is the way back **[open]**.

## 7. Open

- Wrong-answer sound on a locked event; mouse clicks on icons untested; the launch and the tutorial movie not run end to end.
- The route minimap and the barriers during a race: [race-minimap.md](race-minimap.md), [race-barriers.md](race-barriers.md).
