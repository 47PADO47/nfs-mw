# The reputation overview page (Blacklist N)

What the safe house item "Blacklist N" opens in the Original look: `SafeHouseReputationOverview.fng` (the in-game twin
is `InGameReputationOverview.fng`, opened from the pause menu with the argument 1). The pages it leads to are in
[career-rapsheet-2.md](career-rapsheet-2.md) section 5, the rival data in [career-blacklist.md](career-blacklist.md).
Evidence tags: **[exe]** read from `speed.exe` (Ghidra, read only), **[verified]** checked against the install's
package (`nfsmw dump-screen SafeHouseReputationOverview.fng`), **[unverified]** inferred. Object ids are the name hashes
of the package.

The item does **not** open `SafeHouseRivalChallenge.fng`: that package is the arrival's rival screen and stays empty
when it is opened from the menu, which is what the earlier stand-in did. **[exe]**

## 1. The class

Class constructor `0x00561130` (vtable `0x0089E6FC`), message handler `0x0055A4D0`, init `0x00551070`, panel fill
`0x0052EE50`, rival step `0x00551420`. The page owns three bytes: `+0x148` in game (the pause menu variant),
`+0x149` the challenge button is allowed, `+0x14A` the rival is beaten. The shown rival is the global `0x0091CA3C`
(the safe house item sets it to the current stage); the sheets it opens read the same global. **[exe]**

Init (`0x00551070`) **[exe]**:

- When the career stage byte (settings `+0xB0`) is 15 the two stepper chevrons (`0x47B22FCA`, `0x72AD598C`) are hidden:
  there is no other rival to step to.
- Four options are made for the carousel, option ids 0, 1, 2 and 4, with the labels `0x84E4A54C` "Race Events",
  `0x216F1B81` "Milestone Events", `0xE451941E` "Bounty" and `0x2D159737` "Bio" (`REP_SHEET_RACE_EVENTS`,
  `REP_SHEET_MILESTONE_EVENTS`, `REP_SHEET_BOUNTY`, `REP_SHEET_BIO`). The selected option is 0. Each option has no
  texture of its own (its texture field is 0), so the package's single icon stays. The option title is the string
  `0x5E7B09C9`; the arrows are the groups `0x3DF39A82` (left) and `0xFCD5B255` (right), shown when there is an option
  on that side. **[verified]** that the package has one title string and two arrow groups.
- The rival portrait (`RIVAL_NN`, object `0xC1F62308`) and the graffiti (`RIVAL_NN_GRAF`, object `0xF5A2A087`) are
  looked up and kept; the panel is then filled.

## 2. Messages

| Message | Does | Evidence |
|---|---|---|
| `PAD_UP` (`0x72619778`) | shown rival +1; past 15 it becomes the stage byte (the lowest rival); the portrait plays script `0x9E5E6B5F`; portrait, graffiti and panel are refilled. A career at rival 15 does nothing. | **[exe]** `0x0055A4D0` |
| `PAD_DOWN` (`0x911C0A4B`) | `0x00551420` with 1: shown rival -1; below the stage byte (or below 0) it becomes 15; the portrait plays script `0xAF9D73F2`. Nothing when the stage is 15. | **[exe]** |
| `PAD_LEFT` / `PAD_RIGHT` | the base class steps the option carousel (previous / next, no wrap) and refreshes the title. | **[exe]** `0x00587D30`, `0x00573870` |
| `PAD_ACCEPT` | the base class stores the message and leaves; on `EXIT_COMPLETE` the page opens the selected option's package: 0 `SafeHouseRaceSheet.fng`, 1 `SafeHouseMilestones.fng`, 2 `SafeHouseBounty.fng`, 4 `SafeHouseRivalBio.fng` (the `InGame*` names in game). | **[exe]** |
| `PAD_BACK` | on `EXIT_COMPLETE` opens `MainMenu_Sub.fng` (the pause menu in game). | **[exe]** |
| `PAD_BUTTON4` (`0xC519BFC3`, key 2) | opens `SafeHouseRivalChallenge.fng` (`InGameRivalChallenge.fng`) when `+0x14A` is clear and `+0x149` is set. It does not start the race: the challenge screen has its own button. | **[exe]** |

The shown rival therefore runs from the career's rival up to rival 15 and wraps both ways: it is the rival the career is
at plus every rival already beaten. **[exe]**

## 3. The panel (`0x0052EE50`)

Written for the shown bin from the bin record **[exe]**:

| Object | Content |
|---|---|
| `0x242657CE` (group) | `TITLE_BLACKLIST_NUMBER` (`0x3A64DE21`, "blacklist %d"; `0x96CA2471` in game) with the bin |
| `0x7AC3D0C9` (group) | `BLACKLIST_RIVAL_%02d_AKA` |
| `0x79CF0442` (group) | `BLACKLIST_RIVAL_%02d_CHALLENGE` |
| `0x15D80973`, `0xD802FBA8`, `0x322B18F9` | races won (u16 `+0x1A`), milestones done (u16 `+0x18`), the stable's total bounty (`%$d`): the "Current" column |
| `0xDE7AD199`, `0x7242962E`, `0x055C6E5F` | the bin's races, milestones and bounty needed: the "Target" column |
| `0x4C3B1536`, `0x4C3B1537`, `0x4C3B1538` | a tick per row: script `0xE6361F46` (shown) when the row is met (`have >= need`, signed), else `0x16A259` (hidden) |
| `0xB514E2D8`, `0xF91A59F6` (groups) | `"%s %$d"` of `0xCE6B99B1` "Bounty:" (`BL_BOUNTY_REWARD`) with the stable's total bounty and `0x073B79E0` "Cash:" (key not found; not `BL_CASH_REWARD`) with the cash (profile `+0xB4`); the package alternates the two |
| `0x34D4433B` (group) | the DEFEATED stamp |
| `0x55F6AA1A` (group) | the Challenge Rival button |

The package labels ("Race Wins", "Milestones Completed", "Total Bounty", "Current", "Target", "Challenge Rival") are
its own strings. **[verified]**

`+0x149` is the OR, over the bin's rival events, of a flag-2 test on the event record (the boss race is open;
[career-blacklist.md](career-blacklist.md) section 3.3). `+0x14A` is set when the career is complete (settings flag
bit 14) or the shown bin is above the stage byte: the rival is beaten. **[exe]**

- Beaten (`+0x14A`): the button group is hidden, the stamp shown, and `RIVAL_LIST_READY` (`0xB4C144B1`) is posted, which
  plays the stamp's script.
- Not beaten: the stamp is hidden; `RIVAL_CHALLENGE_ACTIVATED` is posted when `+0x149` is set, and
  `RIVAL_CHALLENGE_DEACTIVATED` otherwise (the button group stays shown, greyed by the package).

## 4. In the Rust rewrite

`crates/nfsmw/src/frontend/career_screens/reputation_overview.rs`. The pure parts (the step rule, the shown rival, the
panel, the sheets) are tested in `reputation_overview_tests.rs`. Differences from the original:

- The race, milestone and bounty sheets are not built, so their options are greyed and Accept does nothing on them;
  the bio opens and comes back to the overview (`Args::overview`), as does the rival challenge screen.
- The Remastered look has no overview page: the item opens the rival list of `SafeHouseRivalChallenge.fng`.
- The first-visit sequence still ends in the safe house menu rather than on this page ([story-flow-2.md](story-flow-2.md)
  step 15).
- The mouse takes the two chevrons, the two arrows, the option and the Challenge Rival button (the original's handler
  has no mouse code of its own; the package's buttons are off screen), and the footer's Back and Accept.

## 5. Open questions

1. What `0x2D62CFE2` and the messages of the group `0x53639A10` do; nothing in the page posts them. **[unverified]**
2. Whether Left on the carousel is "previous" (read from the arrow scripts, not traced to the end). **[unverified]**
3. The exact record flag `+0x149` tests ([career-blacklist.md](career-blacklist.md) section 3.3 gives the rule).
