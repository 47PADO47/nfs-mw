# Career milestones: the data

The numbers behind [career-milestones.md](career-milestones.md). All values were read from this install (PC v1.3,
`GLOBAL/attributes.bin` and `GLOBAL/gameplay.bin`) with a throwaway script on top of the `blackbox-attrib` reader.
**[verified]** unless a column says otherwise. For the tag meanings, see [evidence tags](../README.md#evidence-tags).
Field hashes are explained in section 1 of the main file.

## Table 1: the 30 statistics (`milestonetypes`)

`Type` is `MilestoneType` (1 or 3 bigger is better, 2 smaller is better). `Per pursuit` is the bool `0xAB4A196F`.
`Label` is `0xDB89AB5C`, the index of `BLACKLIST_PURSUIT_MILESTONES_NN` (`-` = absent, which the code reads as 0).
`Updated at` is the producer address of section 3.4 of the main file **[exe]**.

| Statistic | Type | Per pursuit | Label | Updated at |
|---|---|---|---|---|
| `pursuit_length` | 3 | yes | 1 | 0x00443CE7 (pursuit tick) |
| `pursuit_evasion_time` | 2 | yes | 2 | 0x004110D8 (escape) |
| `bounty_in_pursuit` | 1 | yes | 3 | 0x006013F0 |
| `cops_destroyed_in_pursuit` | 1 | yes | 3 | 0x00418F30 |
| `cops_destroyed` | 1 | no | 3 | 0x0043E8D0 |
| `cost_to_state_in_pursuit` | 1 | yes | 5 | 0x006013F0 |
| `cost_to_state` | 1 | no | 5 | 0x006013F0 |
| `roadblocks_dodged` | 1 | yes | 6 | 0x00439EE0 |
| `cops_damaged` | 1 | yes | 7 | 0x0040AF40 |
| `total_infractions` | 1 | yes | 8 | 0x005FDDA0 |
| `tire_spikes_dodged` | 1 | yes | - | 0x00439EE0 |
| `pursuits_in_a_row` | 1 | no | - | 0x004110C2 (escape) |
| `speedtrap_speed` | 1 | no | - | 0x0060A7B0 |
| `cops_in_pursuit` | 1 | no | - | 0x004190C0 |
| `helis_in_pursuit` | 1 | no | - | 0x004190C0 |
| `total_cops_in_pursuit` | 1 | no | - | 0x00423DF0 |
| `insurance_claims` | 1 | no | - | 0x004299B0 |
| `heat_meter` | 1 | no | - | 0x006F13F0 |
| `margin_of_victory` | 1 | no | - | none found |
| `hit_and_run_ticket` | 1 | no | - | none found |
| `damage_property_ticket` | 1 | no | - | none found |
| `911_calls` | 1 | no | - | none found |
| `outrun_pursuits_won` | 1 | no | - | none found |
| `fleeing_ticket` | 1 | no | - | none found |
| `outrun_races_won` | 1 | no | - | none found |
| `driving_sidewalk_ticket` | 1 | no | - | none found |
| `racing_ticket` | 1 | no | - | none found |
| `speeding_ticket` | 1 | no | - | none found |
| `reckless_ticket` | 1 | no | - | none found |
| `opponents_damaged` | 1 | no | - | none found |

Label meanings (own wording; the texts are read in [career-text.md](career-text.md) section 3): 0 spike strips dodged, 1 pursuit time, 2 evade within a time, 3 bounty in one pursuit,
4 consecutive evaded pursuits (no milestone uses it), 5 cost to state, 6 roadblocks dodged, 7 cops tagged ("trade
paint"), 8 infractions, 9 speed trap speed (used by the speed traps, not by a milestone collection).

## Table 2: the 55 milestones (`milestones/bin_NN/challenge_N_<kind>`)

Counts per kind: cops tagged 9, bounty 9 (collection name `cops_destroyed`), evade-time 8, pursuit length 8, cost to
state 7, roadblocks 6, spike strips 4, infractions 4. Three or four per bin: bins 1 to 10 have four, bins 11 to 15
have three.

`Kind` is the collection name suffix. The statistic it tracks is in section 1.2 of the main file (bounty goals track
`bounty_in_pursuit`, cost goals `cost_to_state_in_pursuit`). `Threshold` is field `0x8445AF47` (the value that
completes it; seconds for the two time kinds, points or dollars for bounty and cost, counts otherwise). `Goal`
(`0x3B9BBFC2`) and `Step` (`0x006EC903`) are read by no code found. `Bounty` is the reward in bounty points. `-` = not set
in the collection (reads as 0).

| Bin | Kind | Threshold | Goal | Step | Bounty |
|---|---|---|---|---|---|
| 1 | `cops_damaged` | 35 | 50 | 5 | 220000 |
| 1 | `cops_destroyed` | 850000 | 1000000 | 100000 | 220000 |
| 1 | `pursuit_evasion_time` | 120 | 60 | -30 | 220000 |
| 1 | `pursuit_length` | 780 | 840 | 60 | 220000 |
| 2 | `cost_to_state` | 200000 | 400000 | 50000 | 180000 |
| 2 | `pursuit_length` | 540 | 780 | 60 | 180000 |
| 2 | `roadblocks_dodged` | 12 | 24 | 2 | 180000 |
| 2 | `tire_spikes_dodged` | 8 | 16 | 2 | 180000 |
| 3 | `cops_damaged` | 32 | 36 | 4 | 140000 |
| 3 | `cops_destroyed` | 600000 | 750000 | 100000 | 140000 |
| 3 | `pursuit_evasion_time` | 120 | 60 | -30 | 140000 |
| 3 | `pursuit_length` | 480 | 720 | 60 | 140000 |
| 4 | `cops_damaged` | 28 | 34 | 4 | 100000 |
| 4 | `cops_destroyed` | 325000 | 500000 | 50000 | 100000 |
| 4 | `cost_to_state` | 150000 | 300000 | 50000 | 100000 |
| 4 | `total_infractions` | 6 | 8 | 1 | 100000 |
| 5 | `cops_destroyed` | 200000 | 275000 | 50000 | 90000 |
| 5 | `cost_to_state` | 100000 | 250000 | 50000 | 90000 |
| 5 | `pursuit_evasion_time` | 120 | 60 | -30 | 90000 |
| 5 | `tire_spikes_dodged` | 6 | 12 | 2 | 90000 |
| 6 | `cops_damaged` | 22 | 28 | 4 | 80000 |
| 6 | `pursuit_length` | 420 | 660 | 60 | 80000 |
| 6 | `roadblocks_dodged` | 10 | 20 | 2 | 80000 |
| 6 | `tire_spikes_dodged` | 4 | 8 | 2 | 80000 |
| 7 | `cops_destroyed` | 75000 | 150000 | 25000 | 70000 |
| 7 | `cost_to_state` | 50000 | 150000 | 20000 | 70000 |
| 7 | `pursuit_evasion_time` | 180 | 90 | -30 | 70000 |
| 7 | `roadblocks_dodged` | 8 | 16 | 2 | 70000 |
| 8 | `cops_damaged` | 18 | 24 | 5 | 60000 |
| 8 | `pursuit_length` | 360 | 600 | - | 60000 |
| 8 | `tire_spikes_dodged` | 2 | 4 | - | 60000 |
| 8 | `total_infractions` | 5 | 7 | 1 | 60000 |
| 9 | `cops_damaged` | 12 | 18 | 4 | 46000 |
| 9 | `cops_destroyed` | 30000 | 50000 | 5000 | 46000 |
| 9 | `pursuit_evasion_time` | 180 | 120 | -30 | 46000 |
| 9 | `roadblocks_dodged` | 6 | 12 | 2 | 46000 |
| 10 | `cops_destroyed` | 10000 | 25000 | 2500 | 35000 |
| 10 | `cost_to_state` | 30000 | 75000 | 10000 | 35000 |
| 10 | `pursuit_length` | 300 | 540 | 60 | 35000 |
| 10 | `roadblocks_dodged` | 4 | 8 | 2 | 35000 |
| 11 | `cost_to_state` | 20000 | 40000 | 5000 | 30000 |
| 11 | `roadblocks_dodged` | 2 | 4 | - | 30000 |
| 11 | `total_infractions` | 4 | 6 | 1 | 30000 |
| 12 | `cops_damaged` | 8 | 12 | 4 | 19000 |
| 12 | `pursuit_evasion_time` | 180 | 120 | -30 | 19000 |
| 12 | `pursuit_length` | 240 | 480 | 180 | 19000 |
| 13 | `cops_damaged` | 4 | 6 | 2 | 11000 |
| 13 | `cops_destroyed` | 5000 | 7500 | 1000 | 11000 |
| 13 | `pursuit_evasion_time` | 240 | 180 | -30 | 11000 |
| 14 | `cops_destroyed` | 1000 | 4000 | - | 6000 |
| 14 | `cost_to_state` | 15000 | 25000 | - | 6000 |
| 14 | `total_infractions` | 3 | 4 | - | 6000 |
| 15 | `cops_damaged` | 2 | 4 | - | 5000 |
| 15 | `pursuit_evasion_time` | 240 | 180 | - | 5000 |
| 15 | `pursuit_length` | 120 | 120 | - | 5000 |

## Table 3: speed traps (`speedtraps/bin_NN/speedtrapK`)

Thresholds in km/h (`ThreshholdSpeed`) in trap order 1 to 3; the bounty is the same for all traps of a bin. The trap
must be crossed faster than the threshold. Three traps of the data carry an extra field `0x39BF8002` (25 or 30)
whose use was not found. Every trap states its trigger size in `Radius` (20 m; 25 m on one trap, 30 m on two), which the
rewrite uses for the free-roam trigger **[verified]** (install).

| Bin | Traps | Thresholds (km/h) | Bounty per trap |
|---|---|---|---|
| 1 | 3 | 320, 225, 241 | 136000 |
| 2 | 3 | 220, 266, 313 | 114000 |
| 3 | 3 | 255, 285, 225 | 94000 |
| 4 | 3 | 265, 245, 290 | 76000 |
| 5 | 3 | 300, 290, 210 | 60000 |
| 6 | 3 | 173, 255, 180 | 46000 |
| 7 | 3 | 180, 255, 230 | 34000 |
| 8 | 3 | 214, 238, 238 | 24000 |
| 9 | 3 | 225, 220, 240 | 16000 |
| 10 | 3 | 175, 185, 225 | 8000 |
| 11 | 2 | 190, 190 | 6000 |
| 12 | 2 | 185, 195 | 5500 |
| 13 | 2 | 175, 170 | 5000 |
| 14 | 2 | 145, 150 | 4500 |
| 15 | 1 | 130 | 4000 |

## Table 4: rival requirements and milestone totals (`race_bin_NN`)

`RequiredRacesWon` (`0xD617FEDC`), `RequiredBounty` (`0xD3657D92`), `RequiredChallenges` (`0x6DD4B98B`). "Available" is
milestones + speed traps in the bin; each bin has more than it requires. **[verified]**

| Bin | RequiredRacesWon | RequiredBounty | RequiredChallenges | Available |
|---|---|---|---|---|
| 1 | 9 | 10000000 | 5 | 4 + 3 = 7 |
| 2 | 8 | 7550000 | 5 | 4 + 3 = 7 |
| 3 | 8 | 5550000 | 5 | 4 + 3 = 7 |
| 4 | 7 | 4050000 | 5 | 4 + 3 = 7 |
| 5 | 7 | 3050000 | 4 | 4 + 3 = 7 |
| 6 | 7 | 2300000 | 4 | 4 + 3 = 7 |
| 7 | 7 | 1680000 | 4 | 4 + 3 = 7 |
| 8 | 5 | 1180000 | 4 | 4 + 3 = 7 |
| 9 | 5 | 790000 | 4 | 4 + 3 = 7 |
| 10 | 5 | 500000 | 4 | 4 + 3 = 7 |
| 11 | 5 | 300000 | 3 | 3 + 2 = 5 |
| 12 | 4 | 180000 | 3 | 3 + 2 = 5 |
| 13 | 4 | 100000 | 3 | 3 + 2 = 5 |
| 14 | 4 | 50000 | 3 | 3 + 2 = 5 |
| 15 | 3 | 20000 | 3 | 3 + 1 = 4 |

Other `race_bin_NN` fields seen but not decoded: `0xBB30C804` (20 in bin 15 up to 88 in bin 2), `0xE2D26232` (0 up
to 89), `0xE8C24416` (2.0 to 5.0, close to `RequiredChallenges`).
