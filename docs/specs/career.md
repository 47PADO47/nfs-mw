# Career and progression: overview

How the original's career works end to end, and where each part is written down. The rewrite has no career state yet
(free roam only, see [../app-flow.md](../app-flow.md)); these pages are the specs the career work will be built from.

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP, the install's
  `GLOBAL/gameplay.bin` and `FE_ATTRIB.bin`, and the three saves on the development machine (read only). Each page lists
  its own sources. Nothing was tested in game: the tags are **[exe]**, **[verified]**, **[community]**, **[unconfirmed]**
  ([evidence tags](../README.md#evidence-tags)).
- **Method and provenance:** [career-scripts](../provenance/career-scripts.md), [career-text](../provenance/career-text.md), [career-economy](../provenance/career-economy.md),
  [career-blacklist](../provenance/career-blacklist.md), [career-milestones](../provenance/career-milestones.md),
  [career-flow](../provenance/career-flow.md).

## Pages

| Page | Covers |
|---|---|
| [formats/career-state.md](../formats/career-state.md) | In-memory layout: profile, career settings, car stable, per-car career records, rap sheet sums, script manager tables, race records |
| [formats/career-save.md](../formats/career-save.md) | The save file: header CRCs, block order and sizes, MD5, mapping of the editor offsets |
| [career-flow.md](career-flow.md) | New and resumed career, front-end context bits, safe house menu, autosave |
| [career-safehouse.md](career-safehouse.md), [career-safehouse-2.md](career-safehouse-2.md) | The native side of the safe house: zone kinds, engage rules, the request and world unload, the menu and its sequences, Resume Free Roam, the world reload, heat storage and writers, the respawn marker |
| [career-car-lot.md](career-car-lot.md) | Car catalogue, prices, unlock by stage, buy and sell rules, rival cars |
| [career-text.md](career-text.md), [career-text-2.md](career-text-2.md) | The language table: rivals, milestone labels, rap sheet, markers and popups, SMS ids, tips, shop unlock table |
| [career-economy.md](career-economy.md), [career-economy-flows.md](career-economy-flows.md) | Bounty, infractions and their fines, cash, heat, rap sheet statistics, busted and the impound strikes |
| [career-scripts.md](career-scripts.md), [career-scripts-events.md](career-scripts-events.md) | The compiled Lua state graphs: runtime model, hooks, bin graphs, per-bin SMS and world heat, prologue chain, boss races, final race, challenge ladder, speed traps, OPM |
| [career-rapsheet.md](career-rapsheet.md), [career-rapsheet-2.md](career-rapsheet-2.md) | The rap sheet screens: gate, login, tabs, rank thresholds, summary line, pursuit history, end game |
| [career-save-screens.md](career-save-screens.md), [career-save-screens-2.md](career-save-screens-2.md) | Manual save, autosave gate and triggers, the file write order, load checks and messages |
| [career-regions.md](career-regions.md) | Closed regions as barrier gate groups per stage, the region unlock screen, map variants |
| [career-blacklist.md](career-blacklist.md) | The 15 rival bins as state graphs, the challenge gate, races won, regions, the final race |
| [career-milestones.md](career-milestones.md), [career-milestones-data.md](career-milestones-data.md) | The 30 statistics, 55 milestones and 39 speed traps, states, banking, the tables per bin |

## The loop in ten lines

1. A new career starts at **stage 16** (the prologue, event `16.1.0`) with 0 cash and the preset starter car; the first
   safe house visit moves the stage to 15 (rival 15 is the first). The stage is the bin number and counts **down** to 1
   (Razor). **[exe]**
2. In free roam the player is chased. Every pursuit feeds **statistics** (length, cops damaged and wrecked, roadblocks,
   spikes, helicopters, cost to state). **[exe]**
3. **Milestones** (3 or 4 per bin) watch those statistics. A reached milestone is "at stake" until the pursuit ends: if the
   player **escapes** it is banked and pays **bounty**; if **busted** it falls back to active. **[exe]**
4. **Bounty** is per car (the career record of the car driven) plus a sold-car base. An escape adds the pursuit's reputation
   points; a bust adds none. **[exe]**
5. **Infractions** of the pursuit (8 kinds with fixed fines) go to an unserved set. After a bust the player pays them with
   cash or a marker, takes an **impound strike**, and may lose the car at the strike limit. **[exe]**
6. **Races** pay their event cash on every win and unlock further events on the first win. **[exe]**
7. A **rival challenge** opens when races won, challenges passed (milestones plus speed traps of the bin) and total bounty
   all reach the bin's requirements. **[exe]**
8. Entering a bin (the routine loops down to the target stage) activates its milestones and speed traps, closes or opens
   regions and grants the bin's unlocks and upgrade parts. What moves the stage on after bin 15 in real play is **not
   found**, in the exe or in any script: the only callers are the first safe house visit and an unused event and script
   wrapper. A rewrite should enter bin N-1 when bin N reaches done (our choice, [unconfirmed]). **[exe, open]**
9. Winning event `1.8.1` sets the career-complete flag, unlocks every rival challenge and shows the game-won screen. **[exe]**
10. The safe house saves, changes cars, shows the rap sheet and starts the next challenge; autosave runs at free-roam start
    and after races. **[exe]**

## What the rewrite needs first

1. The state of [career-state.md](../formats/career-state.md) as a Rust module (`crates/nfsmw-data` for the readers,
   a `career` module for the runtime state), with the save format of [career-save.md](../formats/career-save.md) behind
   `libs/save-store`. The modding career table ([../modding/api-nfsmw.md](../modding/api-nfsmw.md)) can then answer its
   queries; its field lists marked [unconfirmed] should be checked against these pages (for example milestones pay
   **bounty** in the original, not cash).
2. The pursuit-end recorder, with the order in [career-economy-flows.md](career-economy-flows.md).
3. The milestone tracker and the bin state machine, driven by the install's own `gameplay.bin` data.
4. The safe house screens and the car lot on top of the existing front end.

## Open questions (all pages)

| Question | Where |
|---|---|
| The divisor read by the bounty award (5 in the file image, almost certainly 1 at run time) | [career-economy-flows.md](career-economy-flows.md); needs a runtime read |
| What moves the stage on after bin 15 (no script and no exe path found; see the loop above) | [career-scripts.md](career-scripts.md) |
| The phone table behind SMS and call ids (the safe house entry itself is closed: no heat or cop reset, [career-safehouse.md](career-safehouse.md)) | [career-scripts.md](career-scripts.md) |
| 12 of 30 statistic types have no producer in the exe; two milestone fields are never read | [career-milestones.md](career-milestones.md) |
| Where "career completed at least once" is set; what "OPM" stands for (bin 20 is a developer test bin with no way in: [career-scripts-events.md](career-scripts-events.md); rival names are read: [career-text.md](career-text.md)) | [career-blacklist.md](career-blacklist.md) |
| Meaning of the settings field at `+0x10`, flag bits 6 to 10, 12, 13 and the 8 extra rap sheet sums | [career-state.md](../formats/career-state.md) |
