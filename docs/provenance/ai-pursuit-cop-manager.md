# Provenance: cop manager parity pass (lockout, placement, recycle, budget, backup, heat bins, ticket)

Date: 2026-10-20. Specs: [ai-pursuit-cops](../specs/ai-pursuit-cops.md) (section "Implemented in the rewrite"),
[ai-pursuit-heat](../specs/ai-pursuit-heat.md), [ai-pursuit](../specs/ai-pursuit.md) section 6,
[busted-sequence](../specs/busted-sequence.md) section 6.3.

The code was written from those specs. The behaviours below were learned by reading the public CC0 reconstruction
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) for understanding; nothing is copied or translated, and no
decompiler names appear in the repository.

| Behaviour | Read in | Notes |
|---|---|---|
| Lockout timer, `VehicleSpawningEnabled`, removal of every active cop while it runs, 60 s start, `TimeInactiveFor911` after an evade or restart | `AI/Activities/AICopManager.cpp` (`LockoutCops`, `ResetCopsForRestart`, `UpdatePatrols`), `AI/Common/AIPursuit.cpp` (the evade branch) | The 120 s is the install-verified value in the heat spec. Which of the 60 s and the 120 s applies at a free-roam session start is not established; the rewrite uses 60 s. |
| Out-of-play recycle: `RespawnAvailable` (cone `[-0.707, 0.8]`, mid-point 275 m), `CanRespawn` (8 s latch, `kNeverVisibleRespawnTime` 10 s), wreck 3 s, 75 m ahead point, 375 m / 600 m | `AI/Common/AISpawnManager.cpp`, `AI/Common/AIVehicle.cpp`, `AICopManager.cpp` (`UpdatePatrols`) | "Age" is taken as the seconds since spawn. The base is the simulation camera. |
| `CheckSpawnPosition`: 150 to 400 m window, `NO_COP_SPAWN` zones | `AI/Common/AISpawnManager.cpp` | The 15 m / 5 m same-lane spacing, the camera base and the seek-ahead walk were added later: see [ai-pursuit-cop-fleet](ai-pursuit-cop-fleet.md). |
| Cops-to-evade budget, fully engaged radius, evaded count, backup trigger and countdown, `LockInPursuitAttribs` | `AI/Common/AIPursuit.cpp` (`Update`, `RemoveVehicle`, `LockInPursuitAttribs`) | `BackupCallTimer`, `NumCopsToTriggerBackup`, `FullEngagementRadius` verified against the install by `the_backup_rows_match_the_documented_table`. |
| Adjusted wave counts, surplus selection, `FleeCopOfType`, `StartFlee` | `AI/Common/AIPursuit.cpp`, `AI/Common/AIVehiclePursuit.cpp` | Per-car sight is now each cop's own last sight test (see [ai-pursuit-cop-fleet](ai-pursuit-cop-fleet.md)). The type pick keeps the original's first-type bias. |
| Cool-down bar width `1 - remaining / required` with `required` = timeout - 7 | `Frontend/HUD/FePursuitBoard.cpp`, `AI/AIPursuit.h` | |
| Ticket: the payment tells Cash (`0xB8A7C6CD`) from Marker (`0xB8A7C6CC`) by the pressed object, and continues to the car select screen | `Frontend/MenuScreens/Safehouse/career/uiInfractions.cpp` | The package dump (`nfsmw dump-screen Infractions.fng`) shows its only game responses are `PAD_LEFT` and `PAD_RIGHT`; the rewrite resumes driving instead of the car select. |
| Heat range and growth by Blacklist bin | `docs/specs/ai-pursuit-heat.md` 3.1 and 3.2 (already verified) | The `race_bin_NN` records are not read; the table is hard-coded from the spec. |

Not verified in a running original: any of the above as a play experience. The tests are synthetic or installed-data
tests, not a matched comparison.
