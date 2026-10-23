# The reputation sheets: race events, milestones, bounty

What the rewrite does with the three pages behind the safe house item "Blacklist N" (the overview's options
`REP_SHEET_RACE_EVENTS`, `REP_SHEET_MILESTONE_EVENTS`, `REP_SHEET_BOUNTY`). The original's classes, handlers and the row
rules are in [career-rapsheet-2.md](career-rapsheet-2.md) section 5 and [career-reputation-overview.md](career-reputation-overview.md);
the addresses and what was read this pass are in [the provenance](../provenance/career-rep-sheets.md). Code:
`crates/nfsmw/src/frontend/career_screens/rep_sheets/`.

Evidence tags: **[exe]** read in `speed.exe` (Ghidra, read only), **[verified]** checked in the install's data or its
packages (`nfsmw dump-screen`, a real-install test), **[ours]** a choice of the rewrite where the original is not known.

## 1. The shared layout **[verified]**

The three packages (`SafeHouseRaceSheet.fng`, `SafeHouseMilestones.fng`, `SafeHouseBounty.fng`, all in
`FRONTEND/FrontB.lzc`) are one layout built from the same template as the custom race's track select: a title group
`0x242657CE`, the page counter ("3 / 9": the two static strings `0x5B805EE5` and `0x2E481779` and the "/" `0xF6586F97` are
hidden and the shadows `0x6F25A248` and `0xB2037BDC` carry the numbers), the map picture `0x0F365871`, two scroll arrows
`0x444969FD` and `0x444969FE`, and a group `0xCA9FA6F5` of 12 icon buttons (`EVENT_ICON_n`). The package answers PAD_UP,
PAD_DOWN, PAD_LEFT and PAD_RIGHT with a game message and Accept and Back with pad bits only.

The original fills the 12 icon buttons from the rows and moves the focus between them. **[ours]** The rewrite keeps the
focus out of the package: the group is hidden and the pad walks the rows one at a time (Left and Up back, Right and Down
forward, wrapping at the ends), the counter shows the position, and the panel shows the selected row. Back (the pad bit
or the footer) goes to the overview at the rival the sheet was opened for, the mouse takes the two scroll arrows and the
wheel steps the walk. A real-install test checks that every object the sheets fill is in the package.

## 2. Race events **[exe]** + **[ours]**

Rows are the events of the viewed bin: its free races (`WorldRaces`), then its boss races (`BossRaces`). The original builds
rows from `FUN_00601060` event keys that `FUN_0051f120` accepts, in one of two modes: the viewed bin only (a flag on the
page, set when it is opened from the overview) or every bin from the career's stage up (see the provenance). The rewrite
has the first mode.

| State | Rule |
|---|---|
| Locked | the bin is not entered; or a boss race while the rival is still locked |
| Open | a free race of an entered bin; a boss race once the rival is unlocked or beaten |
| Done | the race is in the bin's won list (`races_won` or `rival_races_won`, saved as flag 2 of its race record) |

A done race can be run again. The panel shows the race kind and its icon (`Mode_Icon_*`), the track name
(`TRACKNAME_<event>`), the best time, the cash a win pays (`CashValue`), the route length and, for a circuit or knockout, the
laps. The average and the top speed are the best of the player's finished runs (`records.toml`, from the racer's distance and top
speed; **[ours]**: the original keeps two derived u16 in the race record, not read). The "Cop Probability" row is hidden:
the data it comes from is not known.
The map is the city picture of the race, zoomed to its route as the track select does.

The sheet opens on the first open race that is not done. Accept asks `REP_SHEET_RACE_CONFIRM` and, on Yes, leaves for the
world, which starts the event as a career race (`Command::LaunchEvent`, the same path as the rival challenge button). Its
result counts in the Blacklist.

## 3. Milestones **[exe]** + **[ours]**

Rows are the pursuit milestones of the bin (every state), then its speed traps. A milestone is locked (state 1), open
(2 and 3) or done (4); a trap is locked until the bin opens it and done once crossed fast enough. The title is
`BLACKLIST_PURSUIT_MILESTONES_<NN>_SHORT` and the text `BLACKLIST_PURSUIT_MILESTONES_<NN>` with `%s` replaced by the goal
(minutes and seconds for pursuit length and evasion time, a count otherwise), `NN` being the statistic type's label field.
A trap shows its threshold in the player's speed unit. The bounty reward is the row's bounty.

Accept on a done row does nothing (the original plays sound 7). On an open row it asks `REP_SHEET_MILESTONE_CONFIRM`
(`REP_SHEET_PHOTOTICKET_CONFIRM` for a trap) and, on Yes, the world starts with the car on the row's spawn point
(`Command::JumpTo`, section 5). All 55 milestones and 39 traps have a spawn point **[verified]**.

## 4. Bounty **[exe]** + **[verified]**

The 12 places are the collections `bounty_jump/<place>`; their number (field `0xDB89AB5C`, 1 to 12) is the `NN` of
`BLACKLIST_BOUNTY_SPAWN_POINT_<NN>` (the name) and `_DESC` (the hint). The first four places are always on offer, places 5
to 8 while the career's bin is below 13, places 9 to 12 below 9. Accept asks `REP_SHEET_BOUNTY_CONFIRM` and, on Yes, jumps
like a milestone. The one-time warning `blacklist_bounty_spawn_point_first_time` is not shown.

## 5. The jump **[ours]**

The original writes a pending jump (spawn key, flag `+0x111`) and leaves the front end as "Resume Free Roam" does; the
reader of the spawn key was not found. The rewrite leaves the safe house for a new world and puts the car on the road
nearest the point, as a respawn does (`WorldScene::start_at_point`). A co-op guest is put behind the lead.

## 6. Progress that is saved **[ours]** + **[exe]**

The races of a bin the player has won are kept as flag 2 ("won in career") of their race records, the list the original
saves after the stable; the bin counters keep the passed challenges. The wins of the side bins (18, 19, 21) are written the same way and the ladders read them back when the world takes the career.
A loaded career rebuilds each entered bin from them
(`Blacklist::restore_progress`): the races won, the boss races won, the challenges passed (milestones banked and traps
done) and the resting state. A bin above the career's bin is done; the others follow their gate. Before this was added a
reload forgot every race won and every rival state.

## 7. From the pause menu and in the Remastered look **[ours]**

The original opens twin packages (`InGameReputationOverview`, `InGameRaceSheet`, `InGameMilestones`, `InGameBounty`, whose
objects differ from the safe house ones; not bound). The rewrite lists "Blacklist N" in the pause menu while the career is
played outside a race and opens the safe house packages over the frozen world, with no room behind them. Back goes to the
pause menu. A Yes on a race starts it in the running world (`Scene::start_career_event`) and a Yes on a jump moves the car
(`Scene::jump_player`), then the game goes on.

The Remastered look has no overview package. Its Blacklist wall (the list of rivals) opens a list for the pressed rival:
Race Events, Milestone Events, Bounty, Bio and the Challenge Rival button (greyed while the rival's button is off, absent
for a beaten rival). Back from a sheet returns to that list, and from the list to the wall.

## 8. Open

1. The "Cop Probability" value (the script scan of the label's users is disabled in this Ghidra setup, so its code was
not found). 2. The sheet's second row mode (every bin from the career's bin up) and what switches it. 3. The one-time bounty
warning `blacklist_bounty_spawn_point_first_time` (its flag is not known). 4. The in-game twins' own object layout.
