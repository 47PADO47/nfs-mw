# Split Screen

Two players race each other on one machine, each with a half of the screen, a car and a HUD. The Quick Race menu's third
icon ("Split Screen", `MODE_ICON_2PLAYER_SPLIT`) starts it. This is the design and the rules; the sections for the parts that
are built say where the code is.

- **Where the exe and the decomp were read:** [provenance/split-screen.md](../provenance/split-screen.md).
- **Evidence tags:** **[exe]** read in `speed.exe` with Ghidra. **[decomp]** read in the CC0 decompilation
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) for understanding, no code copied. **[verified]** seen in the
  install. **[ours]** this rewrite's choice. **[unconfirmed]** not traced.
- **Related:** [custom-race.md](custom-race.md) (the screens this flow reuses), [race-mode.md](race-mode.md),
  [hud-viewport.md](hud-viewport.md).

## 1. The mode

`FEDatabase::IsSplitScreenMode()` is "the Quick Race game mode with two players" (`FEGameMode & QUICK_RACE` and
`iNumPlayers == 2`) **[decomp]**; the exe tests the same two fields of the front-end database (`+0x12C & 4`, `+0 == 2`)
before it loads `Global\InGameSplitScreen.bun` **[exe]** (`0x00666AA0`). Choosing Split Screen in the Quick Race menu sets
`iNumPlayers = 2` and goes to the mode select; every other entry sets it to 1 **[decomp]** (`UIQRMainMenu`).

The simulation mode is "user split screen" (`USER_SPLIT_SCREEN`), which two players create (`QuickGame::CreatePlayers`:
a local player on the first controller port with render port 1 and settings 0, and a second on the second port with render
port 2 and settings 1) **[decomp]**. Career mode, free roam, cops and the online modes do not have it (`GameFlow`: the
career's start screen is skipped in split screen).

## 2. The menu flow

```
Quick Race menu -> Split Screen -> mode select -> track list -> [options] -> Press Start (player 1) -> car select (player 1)
  -> Press Start (player 2) -> car select (player 2) -> loading -> race
```

| Screen | What changes in split screen | Evidence |
|---|---|---|
| Mode select | Title `0x43C825ED`. Circuit, Sprint, Drag and Speedtrap only: no Lap Knockout and no Tollbooth (the decomp's split-screen list has neither; the tollbooth is a custom-race addition of the PC build). | **[decomp]** `UIQRModeSelect::Setup`, `RefreshHeader` |
| Track list | As a custom race. For a Drag, Sprint or Speedtrap track, Accept makes the race with one opponent and no cops and goes to Press Start at once; the options screen is skipped. | **[decomp]** `UIQRTrackSelect` |
| Track options | Only Direction and Laps (circuit) or Direction (sprint, drag, speedtrap), and the knockout's rows are not offered. **No Traffic, Opponents, Difficulty or Catch Up rows.** Accept makes the race with one opponent (the second player) and no cops, then goes to Press Start. | **[decomp]** `UIQRTrackOptions::Setup*`, `NotificationMessage` |
| Press Start | Text "Player N press Start" (labels `0xCF538E1C` and `0xA065EFFE`, N the player number). The Start press seats the device that sent it as that player's controller. Player 2 cannot take player 1's controller. Back from player 1 goes to the track options (or the track list for Drag, Sprint, Speedtrap), from player 2 to player 1's car select. | **[decomp]** `uiQRPressStart` |
| Car select | The header says Player 1 or 2 (`0x7B070984`, `0x7B070985`) and the screen plays the "2 players" intro. Player 2's profile is made when their select opens and dropped when player 1's opens. Accept for player 1 goes on to Press Start for player 2; for player 2 it starts the race. Back from player 2 returns to Press Start for player 2, from player 1 to Press Start for player 1. A Drag race skips the transmission question; the others ask each player. | **[decomp]** `uiQRCarSelect` |

**This rewrite [ours]:** the screens are the custom race's (`frontend/custom_race/`) with a split-screen flag in the
session state (`custom_race/split.rs`); the Press Start screens are `PressStart.fng` **[verified]**. A device that already seats
player 1 is refused for player 2. Devices: section 5.

**Status: built up to the hand-over to the world [verified]** (screen tests in `frontend/tests/split_screen_tests.rs` against the
install, and offscreen screenshots of every screen). Where the build differs from the table:

- **The Start press** is the input layer's `SplitInput::start_press`, read by the plugin each frame and handed to the top screen
  (`Screens::set_start_press`, `ScreenLogic::start_pressed`; ignored while the gesture that opened the screen is held). The
  original seats on the Start *release* (`__PAD_START_RELEASED__`); this rewrite seats on the press **[ours]**. A mouse click
  seats nobody. A scripted run (`--ui-script`) seats with `key Space` (keyboard) or `key NumpadEnter` (arrow block).
- **Back** from the Press Start of player 2 goes to player 1's car select with player 1 still seated; from a car select it goes to
  that player's Press Start (player 2's seat is given up); from player 1's Press Start to the options (circuit) or the track list
  (drag, sprint, speedtrap) with both seats cleared **[decomp]**.
- **Gearboxes** are stored per player (`Split::transmissions`); the global transmission setting is not changed by a split race.
- **The options page** has the circuit's Laps only; no race of the install can be reversed, so Direction never shows. The race
  is made with one opponent, no cops, no catch-up (the decomp sets the first two; catch-up off between humans is ours, section 3)
  and the race's own traffic and difficulty.
- **The start:** `Command::StartSplitRace { race, options, setup }` (`setup` is `scenes::world::SplitSetup`; the cars are named
  `folder` or `folder:preset` as for the single-player start). The front end starts the world behind the loading screen as for a
  custom race (origin Quick Race: the pause menu's Quit and the post-race Exit return to the Quick Race menu) and calls
  `WorldScene::queue_split_race` when the world is built (`flow_split.rs`, `flow_loading.rs`).
- **The seats** are set in the input layer by the plugin: `SplitInput::seat(Some(devices))` while a split race is loading, running,
  paused or showing the map, `seat(None)` as soon as the front end is back in the menus (`Frontend::split_seats`). A restart keeps
  the setup. Menu input from either device stays allowed because the seats are empty in the menus.
- **Not verified:** the race itself (the world does not act on `queue_split_race` in this branch), and the "2 players" intro playing
  in the package (the message is posted; only the header text was checked).

## 3. The race

- **Two cars on one grid.** The second player is a racer of the race: the race has one opponent and that opponent is the
  second human **[decomp]** (`SetNumOpponents(1)`), so ranking, laps, checkpoints, the finish and the catch-up rules of
  [race-mode.md](race-mode.md) apply to both. No traffic is offered as an option; the traffic count of the race stays as the
  default. No cops: `SetCopsEnabled(false)` **[decomp]**.
- **Win.** First across the line wins; the other player's place is second. The post-race statistics panel is drawn per
  player (`StatsPanel::Draw(numPlayers)`) **[decomp]**.
- **Catch-up** is for AI racers; between two humans it is off **[ours]** (nothing slows the leader).
- **Scores.** The high scores count a started split-screen race in their own record (`SplitScreenScores.TotalStarts`)
  **[decomp]**; this rewrite keeps no split-screen records **[ours]**.
- **Rain, particles and reflections** are cut in split screen: rain is off unless `EnableRainIn2P` is set, the cars' level
  of detail is scaled by `CarTwoPlayerLODFactor`, scenery uses the lower model sooner, and the emitters use two interest
  points **[decomp]**. This rewrite renders each half with its own full world pass and keeps its detail settings **[ours]**.
- **Sound.** The mix is the two-player mix (`EAXS3D_TWO_PLAYER_MIX`): each car's engine and the 3D objects are placed to
  the player's side (azimuths 5461 and 60074 for the right and left) **[decomp]**. This rewrite plays the first player's
  car as the listener **[ours]** (open question 4).
- **Speech and the radio:** the cop speech, the pursuit sound activity and the speech manager are off in split screen
  **[decomp]**; the minimap does not rotate with the player there (`hud-minimap.md`).
- **Pause.** Either player's Start pauses; the pause menu has Resume, Restart and Quit **[decomp]** (`uiPause`).

## 4. The screen

The exe loads `Global\InGameSplitScreen.bun` (four FEng packages: `HUD_Player1.fng`, `HUD_Player2.fng`, `HUD_Drag_Player1.fng`,
`HUD_Drag_Player2.fng`) and the textures `GLOBAL\HUDTEXSPLIT.BIN` and `GLOBAL\HUDTEXDRAGSPLIT.BIN` **[verified]** (all in
the install). The HUD package for a player is chosen by a layout number: 3 and 4 are the split HUDs of players 1 and 2, 5
and 6 the drag ones, 2 the single drag HUD, anything else `HUD_SingleRace.fng` **[exe]** (`0x005694E0`).

The renderer's view mode is "two horizontal" (`EVIEWMODE_TWOH`) once both players' views have a camera mover **[decomp]**
(`eUpdateViewMode`): player 1's view is `EVIEW_PLAYER1` and player 2's is `EVIEW_PLAYER2`. The picture is the screen cut in two,
player 1 above player 2, each half as wide as the screen **[unconfirmed]** (the packages' extents fix it: section 4.1).
Each camera uses the split-screen set of the seven gameplay views (`CameraInfo::SplitScreenPOVArray`), and a camera setting
whose `SELECTABLE` attribute is off in two-player mode is not offered **[decomp]**.

**This rewrite [ours]:** the world draws each half through the renderer's off-screen view capture (the rear-view mirror's
path: `RenderBackend::request_view_capture`, `app/mirror.rs`) at the half's size, and the UI layer puts the two pictures one
above the other with the player HUDs over them. No renderer backend changes. The cost is a read-back and an upload of two
half-screen images per frame, and the pictures lag by the capture's latency (a few frames on some backends); a native
viewport in the backends is the follow-up that removes both (open question 1).

### 4.1 The HUD packages

See [split-screen-hud.md](split-screen-hud.md) for the objects of the four packages, how they bind to a player's `HudState`
and where each half places them.

## 5. Input

A player holds a device, as the original maps a player to a controller port **[decomp]** (`SetPlayersJoystickPort`,
`FEJoyInput`). `input::PlayerDevice` is the keyboard (ordinary bindings), the arrow block of the keyboard, or one pad.

- **Seating.** On a Press Start screen the device whose Start or accept key went down is seated (`SplitInput::start_press`):
  Enter or Space is the keyboard, Numpad Enter or Numpad 0 the arrow block, a pad's Start or A that pad. Two pads in one frame:
  the same one wins whatever the order. Player 2 may not take player 1's device.
- **Driving.** Player 1's actions are the ordinary `ActionState`; player 2's are `SplitInput::second`, resolved from their own
  bindings. The arrow block drives with Up, Down, Left and Right; Right Shift is nitrous; Numpad 0 or Right Ctrl the handbrake;
  Page Up and Page Down shift; Delete resets the car; End looks back. A player on a pad leaves the keyboard to the other
  player (only Escape and the function keys stay with player 1). A player on the keyboard gives up the arrow block while the
  other player holds it.
- **Pause.** Either player's Start (`Cancel`, `MenuStart`) pauses.
- **A pad that goes away** reads as released; the race goes on **[ours]** (open question 3).

## 6. Code map

| Part | Where |
|---|---|
| Devices, the second player's actions, Start detection | `crates/nfsmw/src/input/split.rs`, `input/systems.rs` |
| The setup the front end hands over | `scenes/world/race/split.rs` (`SplitSetup`), `WorldScene::queue_split_race` |
| The scene hooks | `viewer/mod.rs` (`set_second_input`, `split_active`, `split_view`, `split_hud_state`) |
| Menus | `frontend/custom_race/` (`split.rs` the flag, seats, cars and gearboxes; `press_start.rs`; `car_select/players.rs`; `settings.rs` the rows and options), `icon_items.rs`, `icon_menu/press.rs` |
| The start and the seats | `frontend/flow_split.rs`, `flow_loading.rs`, `plugin.rs`, `screens/start.rs` |
| The second human racer | `scenes/world/ai/car/human.rs`, `scenes/world/race/split.rs` |
| The two views | `scenes/world/split_view.rs`, `app/split.rs` |
| The HUDs | `hud/split/` |

## 7. Open questions

1. Backend viewports instead of two captures (cost and latency, section 4).
2. The exact half rectangles of the PC build (section 4).
3. A pad unplugged mid-race: pause as the single-player game does for the active pad.
4. The two-player sound mix.
5. Split-screen records and the "2 players" statistics page of the post-race screen.
