# After the ticket: impound notices, career car select, paint cooling

Date: 2026-10-20. Specs: [busted-aftermath](../specs/busted-aftermath.md), [career-cooling](../specs/career-cooling.md).

Written from those specs and the existing career specs ([career-economy-flows](../specs/career-economy-flows.md),
[career-safehouse-2](../specs/career-safehouse-2.md), [career-rapsheet-2](../specs/career-rapsheet-2.md)). Sources read
for understanding, without copying or translating code:

- **`speed.exe` (Black Edition PC v1.3), Ghidra read only.** The infractions screen's message handler at `0x005a15b0`
  (not a function in Ghidra; its bytes were read): the two button name hashes, the compared messages, the two impound
  message hashes and reasons, the package switch to `Car_Select.fng` (string at `0x0089d998`) with argument `0` for a
  marker, `0x100` for cash and `0x200` after the notice, and the `fine > cash` and `strikes >= limit` tests. The car
  select's message handler at `0x007ad490`: the cash release, the marker release (no state test before it consumes an
  Impound Release marker, type `0x15`), the Extra Impound Strike marker (type `0x14`) and the game over message
  `0xe0b38195`. No instruction or decompiled text is included.
- **[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0** (a public decompilation of the same game),
  read for understanding only: `MenuScreens/Safehouse/career/uiInfractions.cpp` (the payment flow and screen
  switches), `MenuScreens/Safehouse/quickrace/uiQRCarSelect.cpp` (the career select mode: rows, the header with strike
  pips and impound stamp, back and accept rules, the release choice, the strike marker, `CalcGameOver`),
  `Database/VehicleDB.cpp` (`FEImpoundData`, `FECareerRecord::AdjustHeatOn*`), `MenuScreens/Safehouse/customize/
  CustomizeManager.cpp` (`UpdateHeatOnVehicle`, the backroom 0.75, the heat preview) and `CarCustomize.cpp`
  (`CustomizeMeter`, the heat meter rule). Points taken only from it are tagged **[community]** in the specs.
- **The install, read only.** The `fecooling` collection through `nfsmw-data` (eleven factors, asserted by an ignored
  test), the English strings table (the message and label hashes in the specs resolve to the quoted texts) and the
  `Car_Select.fng` package dump (`nfsmw dump-screen`): the object, script and image names for the strike pips, the
  stamp, the value lines and the heat meter.

Choices where the sources are silent, marked `[unconfirmed]` in the specs and code: the car select's argument bits, the
respawn flag on the bust path, the order of the heat factors in a multi-part garage commit.
