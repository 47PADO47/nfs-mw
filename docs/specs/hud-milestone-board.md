# HUD milestone board

The pursuit HUD's milestone board (`Hud_MilestoneBoard`, in `HUD_SingleRace.fng`): during a free-roam chase in the
career it shows the current Blacklist bin's milestones as icons, how many are done, and one milestone's progress.
The milestones themselves are in [career-milestones.md](career-milestones.md).

Tags: **[decomp]** read from the PC executable's code, **[package]** from the installed package, **[ours]** a remake
choice.

## 1. Objects [package]

| Object | Name or hash | Use |
|---|---|---|
| top group | `0x2269D232` | the board; the layout shows it during a chase |
| `MILESTONE_ICON_GROUP` | `0x5AB1F0A3` | the icons, their backings and the progress line; scripts `0x5079C8F8` (in) and `0x033113AC` (out) |
| `MILESTONE_INFO_GROUP` | `0x5AB48C86` | "MILESTONES >" and the done count; same in/out scripts |
| `MILSTONE_ICON_1..4` (sic) | | the icons, 32 x 32 |
| `MILESTONE_ICON_BACKING_1..4` | | a plate behind each icon: rest `0x001744B3`, highlight `0x249DB7B7`, flash `0x03826A28` |
| done count | `0x894662C5` | the number of milestones done |
| progress line group | `0xF4405EC0` | slides out (`0xAFF37F61`) and in (`0xD6C950A0`) between milestones; shown `0x001CA7C0`, hidden `0x0016A259` |
| progress line | `0xC3E48FBF` | `value/goal` |
| progress backing | `0x5C697702` | shown and hidden with the line, flashed with a backing |

## 2. What fills it [decomp]

The HUD's per-frame update builds the board while the player's pursuit runs (and has not ended), outside a race. It
walks the milestones of the profile's current bin in their order and fills one slot per milestone, at most four:

- the icon: the texture of the milestone's statistic (table below);
- the statistic, which picks the number format;
- the goal: the milestone's threshold;
- the current value of the statistic;
- done: the milestone is at stake or complete.

The slot count is the number of milestones listed, and the board is visible only with a chase on. A pursuit race fills a
single slot with the race's own goal instead.

| Statistic | Icon texture |
|---|---|
| `pursuit_length` | `Milestone_Pursuit_MinTime` |
| `pursuit_evasion_time` | `Milestone_Pursuit_MaxTime` |
| `bounty_in_pursuit` | `0xB4E6456B` (name not resolved) |
| `cops_destroyed`, `cops_destroyed_in_pursuit` | `Cops_TakenOut_Icon` |
| `cost_to_state_in_pursuit` | `Milestone_Cost_To_State` |
| `roadblocks_dodged` | `Milestone_Roadblocks` |
| `cops_damaged` | `Cops_Damaged_Icon` |
| `total_infractions` | `Milestone_Infractions` |
| `tire_spikes_dodged` | `Milestone_SpikeStrips` |
| `speedtrap_speed` | `0x3FFE9EC9` (name not resolved) |
| any other | none (the package's own picture stays) |

The statistic keys are the attribute hashes of the names.

## 3. How it behaves [decomp]

- **No board** (no chase, or no slot): both groups play their out script if they show.
- **Groups:** both play their in script while the board shows.
- **Done count:** the number of slots done.
- **Progress line:** hidden when every slot is done, shown otherwise. It shows the highlighted slot as
  `min(value, goal)/goal`. Pursuit length and evasion time are written as minutes and seconds, the other statistics as
  whole numbers with thousands separators.
- **Highlight:** none when every slot is done; the one open slot when there is one. With two or more open slots the
  line moves on to the next open slot every 3 s: it slides out, and when the slide has ended it slides back in on the
  next open slot.
- **Backings:** the highlighted slot's plate plays the highlight script (not while its flash still runs), the others
  rest.
- **Icons:** a slot's icon shows with its texture at full alpha when open and at 25 % alpha when done; icons past the
  slot count are hidden.
- **A value changes** on an open slot that is not a time: the slot is highlighted at once, its plate and the progress
  backing flash, the line slides in, and the 3 s count starts again.

## 4. Live statistics [decomp]

The board and the milestones read statistics the chase updates as it runs ([career-milestones.md](career-milestones.md)
section 3.4). The remake feeds, every frame of a free-roam chase: pursuit length, cops destroyed in the pursuit, total
infractions, bounty in the pursuit, cost to state in the pursuit, cops damaged, roadblocks dodged and spike strips
dodged. Evasion time and escapes in a row are set at the escape.

## 5. Rewrite

`crates/nfsmw/src/hud/milestone_board.rs` (the objects and the behaviour above) and
`crates/nfsmw/src/scenes/world/career/milestone_board.rs` (the slots from the career). **[ours]** The rotation's 3 s and
the slide's end are timed by the HUD's own frame time and the script's length. The pursuit race's single slot is not
built: the remake has no pursuit races yet.
