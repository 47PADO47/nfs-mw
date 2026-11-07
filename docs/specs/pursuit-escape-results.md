# Pursuit escape results

What the original shows after the player escapes a chase, and the chase's statistics page. Provenance:
[../provenance/pursuit-escape-results.md](../provenance/pursuit-escape-results.md). Evidence tags: **[exe]** read in the
original PC 1.3 executable, **[install]** read in the player's install, **[inferred]** concluded from those.

## 1. Flow

- **Free roam** (no race activity is running): when a chase ends in an escape, the game counts the escape (the
  `pursuits_in_a_row` and `pursuit_evasion_time` statistics), fills the pursuit summary and opens the rewards screen
  `PostRace_MilestoneRewards.fng`, every time, milestones banked or not **[exe]**.
- The rewards screen shows its rows one at a time (section 2). Continue (accept) opens the statistics page
  `PostRace_Pursuit.fng` on its pursuit page (section 3) **[exe]**. That page's Continue goes back to the game
  **[inferred]** (its package leaves through its own script).
- **A race** (a pursuit challenge, a race with cops): the escape path does not open the rewards screen; the race's own
  post-race flow runs ([race-mode.md](race-mode.md) section 11) **[exe]**.
- **During a chase**, the HUD's statistics action opens the same statistics page as an overlay, and in a race it opens
  the race's results **[exe]**. Not done in the rewrite (section 5).
- The rewrite has no generic end card in a career's free roam: the screens above replace it. The busted sequence and
  the sandbox world (no career) keep the end card.

## 2. The rewards screen

The pursuit summary holds the chase's time bounty and destruction bounty, the cop cars immobilized and the milestones
banked **[exe]**. Its rows, in order **[exe]**:

1. `Pursuit Duration` (label `0x4D64888D`) with the time bounty (the normal rep points, earned while the cops see the
   player). Icon: `Milestone_Pursuit_Duration`.
2. Only when the destruction bounty is above zero: `Immobilized Police Vehicles: N` (label `0x23F6E732` and the count
   with the thousands separator) with the destruction bounty. Icon: `Cops_TakenOut_Icon`.
3. Each milestone banked, with its bounty (`BLACKLIST_PURSUIT_MILESTONES_NN_SHORT: value` in the original; the rewrite
   keeps its own words, [career-milestones.md](career-milestones.md) section 3.9).

- Each row sets the big icon's texture, the row's line and the bounty line `Bounty: 2,500` (label `0x29B1B96A`, the
  format `%s: %$0.0f`); a row whose bounty is zero hides the bounty line **[exe]**.
- The total line reads `Total Bounty: N` (label `0x5CCF949A`): the sum of the rows shown so far, not the car's bounty
  **[exe]**.
- FEng script lengths count 1200 ticks a second. The icon plays its reveal (`0x5079C8F8`, 3000 ticks: 2.5 s) **[exe]**.
  When the reveal has played, the icon plays its row type's slot script (360 ticks: 0.3 s), which moves it onto its
  small icon in the row below and lights that one: `POS0` for the duration, `POS00` for the immobilized cars, `POS1` to
  `POS8` for the bounty, infractions, cost to state, roadblocks, spike strips, cops damaged, and the pursuit length
  (minimum time icon) and evasion time (maximum time icon) milestones **[exe]** **[install]**. The slot script's end
  posts `0x0D3C3DE7`, which brings the next row.
- When the rows have run out the big icon hides, both lines play `0x033113AC` (they go), and both panels (`0x962B9C62`,
  `0xEC85C7E4`) play `POS2`, a slide **[exe]**. The bounty line's leave makes the total play its 2 s flourish
  **[install]**.
- The first row comes with the package's message `0x35F8620B`, 500 ticks into its entrance, when the input comes on
  **[exe]** **[install]**.
- **The sounds** are the package's own events **[exe]** **[install]**: the dialog box in on opening (`UISND_COMMON_DLGBOX_IN`,
  trigger 10), `UISND_MILESTONE_REWARD_SYMBOL_ON` (17) as each row's icon appears and with the total's flourish,
  `UISND_MILESTONE_REWARD_SYMBOL_MOVE` (18) as the icon flies to its slot, the dialog box out (11) with the panels'
  slide and on Continue, and the accept on Continue. There is no count-up: the total is set at each row.
- The rewrite follows this (`frontend/rewards.rs`, the sound map in `audio/menu.rs`). It picks the slot by the row
  icon's texture and times the reveal itself; the original checks the reveal on the screen's update. A row icon with no
  slot goes on after the reveal and a slot's time **[ours]**: the original's sequence stalls on such a row until
  Continue. An earlier version timed the reveal as 3 s, filled the first row at once, skipped the end's hide and slides,
  played none of the four screen sounds, and swapped the two time icons.

## 3. The statistics page

`PostRace_Pursuit.fng`'s pursuit page has the heading `PURSUIT` (`0xFEA872D4`) and seven rows, each a language label
and a value **[exe]**:

| Row | Label | Value |
|---|---|---|
| 1 | `Pursuit Duration` `0x4D64888D` | the chase's length, `mm:ss.cc` |
| 2 | `Damaged Police Vehicles` `0xA999F6E2` | cop cars damaged |
| 3 | `Immobilized Police Vehicles` `0x23F6E732` | cop cars wrecked |
| 4 | `Spike Strips Dodged` `0x0291C816` | spike strips passed |
| 5 | `Roadblocks Dodged` `0x29DABA15` | roadblocks passed |
| 6 | `Cost to State` `0xD9BB7D2D` | the chase's cost to state |
| 7 | `Unserved Infractions` `0xB7DFFF96` | the car's unserved infractions |

- The package has nine label and value rows (`0x38B136A8 + n`, `0x85AB7369 + n`) **[install]**; the two past the seventh
  stay empty.
- The screen also has a challenge variant; the rewrite's pursuit challenge keeps its own rows
  ([race-mode.md](race-mode.md) section 11).

### 3.1 The other pages **[exe]** **[install]**

- **Turning the page.** The footer's second button (PC key 2, the pad's button 4, message `0xC519BFC3`) reads the page it
  goes to. The package answers the press itself: the rows play out over 420 ticks, and at 500 ticks it passes
  `0xC519BFC3` on to the screen and plays the rows back in. The screen turns the page on that message only. After an
  escape the pages go pursuit, infractions, milestones, pursuit; in the challenge series, pursuit and infractions only.
- The page's heading and the button's text: pursuit `PURSUIT` (`0xFEA872D4`) with `Infractions` (`0xAEAEB62F`);
  infractions `INFRACTIONS` (`0xFB415E78`) with `Milestones` (`0x74413352`), or `Pursuit` (`0x7448870B`) in the challenge
  series; milestones `MILESTONES` (`0x578B767B`) with `Pursuit`. In a career's pursuit race the milestones page reads
  `CHALLENGE` (`0x334FA7FB`), the infractions page's button `Challenge` (`0x9145A5F2`), and the page has the race's one
  goal; the rewrite shows the page after an escape in free roam only.
- **Lines.** Each of the nine is `LINEn_GROUP` with `COLUMN1_DATAn` (label), `COLUMN2_DATAn` (value), `CHECKn` (a tick)
  and `EMPTYn` (an empty box). Drawing a line first plays `0x16A259` (hidden) on both marks and `0x1744B3` (rest) on the
  group. A line has a state: 1 ticks it (`0x1CA7C0` on the tick), 2 dims it (`0x163C76` on the group and the tick); a
  value line of the pursuit page with state 0 shows no mark, any other line with state 0 shows the empty box (`0x1CA7C0`
  on it).
- **Infractions page:** eight lines with the value hidden, ticked when the chase committed the infraction, in this order:
  `Speeding` (`0x4B0FF103`, mask 2), `Excessive Speeding` (`0x8EC1A6EC`, mask 1), `Reckless Driving` (`0x370F331D`),
  `Ramming a Police Vehicle` (`0x8462A784`), `Hit & Run` (`0xDFF254BA`), `Damage to Property` (`0x7DBD5B34`),
  `Resisting Arrest` (`0x2B1DE2A9`), `Driving off Roadway` (`0xB0EF5C12`) (masks 4 to 128).
- **Milestones page:** one line per milestone of the career's current bin, in the milestone array's order, labelled
  `BLACKLIST_PURSUIT_MILESTONES_NN_SHORT`, with the statistic's current value (below 0 reads 0) and the threshold. State
  1 is at stake, 2 complete, 0 otherwise. An open line reads `value/threshold` with the value clamped to the threshold,
  a ticked one the threshold alone. Pursuit length and evasion time read as times (`mm:ss.cc`, zeros rather than dashes
  when empty), the rest as numbers with the thousands separator.
- Continue's script passes the accept (`PAD_ACCEPT`) on to the screen at its end, and the screen leaves on it. Both
  scripts end with `0xE1FDE1D1`, so that message does not mean the screen is done.

## 4. The rewrite

- `scenes/world/career/pursuit.rs` and `career/rewards.rs` build the report at the end of a career chase: an escape
  outside a race gets the rows of section 2 and the page of section 3; a race's chase keeps the milestones-only report.
- `frontend/rewards.rs` plays the rows, adds up the total, and on its exit opens the page (`Command::ShowPursuitResults`);
  `frontend/pursuit_results.rs` fills the page and turns it, `frontend/pursuit_pages.rs` draws the lines of section
  3.1, and the hint bar's key 2 reads the package's button (`frontend/prompts.rs`).
- `scenes/world/hud.rs` drops the end card of an escape in a career's free roam.

## 5. Open

- The HUD's statistics action during a chase (and the race's results from it) is not bound.
- The milestones page of a career's pursuit race (section 3.1) is not shown: the rewrite opens the statistics after an
  escape in free roam only.
- How the original words a milestone row is not read (section 2, row 3).
- The screens were read from the executable and the packages, not captured from the running original.
