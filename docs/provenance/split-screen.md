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
