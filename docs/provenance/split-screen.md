# Split Screen

- **Spec:** [specs/split-screen.md](../specs/split-screen.md).
- **Method:** (1) the CC0 decompilation [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (a shallow clone, read for
  understanding, no code copied): `Frontend/MenuScreens/Safehouse/quickrace/uiQRMainMenu.cpp`, `uiQRModeSelect.cpp`,
  `uiQRTrackSelect.cpp`, `uiQRTrackOptions.cpp`, `uiQRPressStart.cpp`, `uiQRCarSelect.cpp`, `Frontend/Database/FEDatabase.cpp`
  and `.hpp`, `Frontend/HUD/FEPkg_Hud.cpp`, `Frontend/FEJoyInput.cpp`, `Frontend/MenuScreens/InGame/uiPause.cpp` and
  `FEPKg_PostRace.cpp`, `Misc/GameFlow.cpp`, `Sim/Activities/QuickGame.cpp`, `Camera/CameraInfo.hpp`,
  `Ecstasy/EcstasyE.cpp` (`eUpdateViewMode`), `Ecstasy/EcstasyData.hpp`, `World/CarRender.cpp`, `World/Scenery.cpp`,
  `World/WorldModel.cpp`, `World/rain.cpp`, `EAXSound/EAXSound.cpp`. (2) Ghidra on `/speed.exe` (project `nfsmw`), 2026-10-21,
  string search and cross-references. (3) The install's files: `GLOBAL/InGameSplitScreen.bun` (four FEng packages),
  `GLOBAL/HUDTEXSPLIT.BIN`, `GLOBAL/HUDTEXDRAGSPLIT.BIN`, `FRONTEND/FrontB.lzc` (`PressStart.fng`).

## Addresses read in `speed.exe`

| Address | What it gave |
|---|---|
| `0x005694E0` | The HUD package chosen by the layout number: 2 `HUD_Drag.fng`, 3 `HUD_Player1.fng`, 4 `HUD_Player2.fng`, 5 `HUD_Drag_Player1.fng`, 6 `HUD_Drag_Player2.fng`, else `HUD_SingleRace.fng`. Its one caller is at `0x006E9A13`. |
| `0x00666AA0` | The in-game region loader: loads `Global\InGameA.bun`, then `Global\InGameSplitScreen.bun` only when the front-end database's game-mode flags (`+0x12C`) have bit 4 and the player count (`+0`) is 2, then `GLOBAL\INGAMEB.LZC`. |
| `0x00664250`, `0x0057C530` | The users of `GLOBAL\HUDTEXDRAGSPLIT.BIN` (as the decomp's two HUD texture-pack names are swapped between its two constants, the exe's own pairing was not traced). |
| `0x0089FC14` | The string `PressStart.fng`, used by three screens (`0x007AB55F`, `0x007C1730` and a table at `0x008F4360`). |
| `0x008A0874` | The class name `uiQRPressStart`. |

## What the decomp gave

- `UIQRMainMenu::NotificationMessage`: `iNumPlayers` is 2 for the Split Screen icon, 1 for the others.
- `UIQRModeSelect::Setup` and `RefreshHeader`: no knockout in split screen; the split-screen title `0x43C825ED`.
- `UIQRTrackSelect` and `UIQRTrackOptions`: the options shrink to direction and laps; `SetNumOpponents(1)`, cops off, then
  Press Start.
- `uiQRPressStart`: the controller port of the Start press seats the player; player 2 may not use player 1's port.
- `UIQRCarSelect`: player header hashes `0x7B070984` and `0x7B070985`, the "2 players" intro `0x2CF6C390`, the profile of player 2
  made and dropped, the Back chain between Press Start and the car selects.
- `QuickGame::CreatePlayers`: two local players, render ports 1 and 2.
- `eUpdateViewMode`: the "two horizontal" view mode when both views have a camera mover.
- `CarRender`, `Scenery`, `WorldModel`, `rain`, `EmitterSystem`: the detail cuts of the two-view mode.
- `EAXSound`: the two-player mix mode. `FEPkg_Hud`: the four split HUD packages and the two texture packs.

## What was not read

- The PC build's view rectangles and field of view in the two-view mode (the decomp is the GameCube build).
- The two-player sound mix beyond the mode flag.
- The post-race screen's two-player layout (`FEPKg_PostRace.cpp` was only grepped).

## Front-end flow (2026-10-21)

Read for the menu flow in `frontend/custom_race/` (decomp `Frontend/MenuScreens/Safehouse/quickrace/`, no code copied).

| Source | What it gave |
|---|---|
| `uiQRPressStart.cpp` | `__PAD_START_RELEASED__` seats the port that sent it; the test `iPlayerNum != 1 || joyPort != port of player 0` is the refusal of player 2 taking player 1's port; Back from player 2 returns to `Car_Select.fng` (player 1), from player 1 to `Track_Options.fng`, or `Track_Select.fng` in split screen for drag, P2P and speed trap; `EXIT_COMPLETE` opens `Car_Select.fng` with the player number. The texts are `FEPrintf(0xb244cf71, 0xcf538e1c, n)` ("Player %d press START.") and the body `0xa065effe`. |
| `uiQRCarSelect.cpp` | Player group `0xe3fe27fe` set to `0x7b070984` or `0x7b070985`; intro message `0x2cf6c390` instead of `0xde511657`; Accept for player 1 goes to `PressStart.fng` with arg 1, drag skips the transmission question; Back from player 2 clears its port (`SetPlayersJoystickPort(1, -1)`) and goes to Press Start 1, from player 1 to Press Start 0. |
| `uiQRTrackSelect.cpp` | In split screen Drag, P2P and SpeedTrap make the race with one opponent and cops off and go to `PressStart.fng` at once. |
| `uiQRTrackOptions.cpp` | Direction (when reversible) and, for the circuit, Laps only; Accept makes the race with one opponent and goes to Press Start. |
| `uiQRModeSelect.cpp`, `uiQRMainMenu.cpp` | Split-screen list (no knockout), title `0x43c825ed`; the Split Screen icon sets `iNumPlayers = 2`, every other entry 1. |
| `PressStart.fng` (`nfsmw dump-screen`) | Headline string object `0xb244cf71`, body `0x545570c6`, leave handler `0x144873e9` answering `0x587c018b` (our `LEAVE_SCREEN`) with `EXIT_COMPLETE`; the package carries its own Back and Select hints. Language strings checked with `nfsmw strings`. |
| `Car_Select.fng` dump | The "2 players" message `0x2cf6c390` reaches the player group `0xe3fe27fe`, whose string reads "Player 1". |

Not read in the exe: the Press Start screen's own code (only its decomp and the package were read).

## HUD packages (added with `specs/split-screen-hud.md`, 2026-10-21)

- **Method:** `nfsmw dump-screen HUD_Player1.fng` (and `HUD_Player2.fng`, `HUD_Drag_Player1.fng`, `HUD_Drag_Player2.fng`, plus
  `HUD_SingleRace.fng` and `HUD_Drag.fng` to compare) after adding `GLOBAL/InGameSplitScreen.bun` to the screen catalogue;
  names resolved by hashing candidate names from the repository and the decomp with the FEng hash. A probe of the contents of
  `GLOBAL/HUDTEXSPLIT.BIN`, `HUDTEXDRAGSPLIT.BIN`, `HUDTEXRACE.BIN` and `HUDTEXDRAG.BIN` (texture name hashes and sizes).
- **Decomp files read:** `Frontend/HUD/FEPkg_Hud.cpp` (`FEngHud::FEngHud`, `HudResourceManager`, `SetWideScreenMode`,
  `ChooseMaxRpmTextureNumber`), `FeTachometer.cpp`, `FETachometerDrag.cpp`, `FeNitrousGauge.cpp`, `FeSpeedometer.cpp`,
  `FeShiftUpdater.cpp`, `FeEngineTempGauge.cpp`, `FeMinimap.cpp` (`Update`, `UpdatePlayer2Element`, `UpdateElementArt`,
  `AdjustForWidescreen`). Nothing copied.
- **Findings:** the four packages hold 95, 57, 60 and 60 objects; every one is authored on the whole 640 x 480 canvas in its own
  half; the two texture packs are paired with the race and drag packages the other way round from the decomp's constant names;
  `USER_SPLIT_SCREEN` (user mode 1) switches the tachometer to a width-scaled bar, the nitrous mask to 180 degrees, and stops the
  minimap rotating; the minimap and its second arrow belong to player 1's package.
- **Not read:** the exe's own HUD update for the split types (only the decomp's); the shift ratings that choose `PulseBlue`,
  `PulseGreen` or `PulseRed`; the PC build's widescreen handling of the split packages in a capture.
