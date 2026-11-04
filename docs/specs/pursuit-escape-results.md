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
- The icon plays its reveal (`0x5079C8F8`, 3000 ms) **[exe]**. When the reveal has played, the icon plays its row
  type's slot script (360 ms), which moves it onto its small icon in the row below and lights that one: `POS0` for the
  duration, `POS00` for the immobilized cars, `POS1` to `POS8` for the bounty, infractions, cost to state, roadblocks,
  spike strips, cops damaged, minimum and maximum time milestones **[exe]** **[install]**. The slot script's end posts
  `0x0D3C3DE7`, which brings the next row; when the rows have run out, both lines play `0x033113AC` (they go) and the
  total stays **[exe]** **[install]**. The rewrite picks the slot by the row icon's texture and times the reveal itself
  (the original checks the reveal on the screen's update); a row icon with no slot goes on after the same time.
- The first row comes with the package's message `0x35F8620B`, 500 ms into its intro **[install]**; the rewrite fills it
  when the screen starts.

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
- The screen also has an infractions page (heading `INFRACTIONS`) and a challenge variant; the rewrite's pursuit
  challenge keeps its own rows ([race-mode.md](race-mode.md) section 11). The page switch is not done.

## 4. The rewrite

- `scenes/world/career/pursuit.rs` and `career/rewards.rs` build the report at the end of a career chase: an escape
  outside a race gets the rows of section 2 and the page of section 3; a race's chase keeps the milestones-only report.
- `frontend/rewards.rs` plays the rows, adds up the total, and on its exit opens the page (`Command::ShowPursuitResults`);
  `frontend/pursuit_results.rs` fills the page.
- `scenes/world/hud.rs` drops the end card of an escape in a career's free roam.

## 5. Open

- The HUD's statistics action during a chase (and the race's results from it) is not bound.
- The infractions page of the statistics screen and its page switch are not done.
- How the original words a milestone row is not read (section 2, row 3).
- The screens were read from the executable and the packages, not captured from the running original.
