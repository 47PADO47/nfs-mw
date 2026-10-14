# The pursuit HUD: the board, the cool-down bar, the heat meter and the cost to state

What the in-game HUD shows during a chase, which objects of `HUD_SingleRace.fng` carry it, and what the rewrite
drives. The chase itself (heat, evade, busted) is in [ai-pursuit.md](ai-pursuit.md) and [ai-pursuit-heat.md](ai-pursuit-heat.md);
the HUD framework is in [feng-runtime.md](feng-runtime.md) section 8.

**Evidence tags** as in [docs/README.md](../README.md#evidence-tags). **[verified]** = read from the install's
package or checked on screen; **[binary]** = a string or code in `speed.exe`; **[decomp]** = the decompiled
sources cited by the pursuit specs; **[inferred]** = a reading not yet confirmed.

**Sources.** The package `HUD_SingleRace.fng` in `GLOBAL/InGameB.bun` (install, `nfsmw dump-screen`); the object
names are FEng hashes, so each candidate name was hashed with `fe_hash_upper` and looked up in the package
**[verified]**. The HUD class names in `speed.exe` (`Hud_PursuitBoard`, `Hud_BustedMeter`, `Hud_HeatMeter`,
`Hud_CostToState`, `Hud_RadarDetector`, `Hud_GetAwayMeter`, `Hud_SpeedBreakerMeter`) at `0x008a2840`–`0x008a28c0`
**[binary]**: these are the classes that drive the elements, not object names (none of them hashes to a package
object). The strings `HEAT_X%.0f`, `HEAT_BASE_LED_%d`, `BOUNTY_%d`, `cost_to_state_in_pursuit` **[binary]**.

## 1. The objects

Names are the FEng object names; the hash is in brackets. The parent is the object's parent in the package.

| Object | Kind | What it is | Status |
|---|---|---|---|
| `PursuitBoardGroup` [0xde89e070] | group | The pursuit board: backing top and bottom, the leader-list pulse, the bounty row and the timer. Parent: group 0x5c7a5c8a, root 0x5d0101f1 (the right context). | **bound** |
| `PursuitTime` [0xfc39cb0a] | string | The timer, `00:00.00` (font `FONT_MW_BODY`), under `Timer_Icon` in the board. | **bound**: `mm:ss.cc` |
| bounty row [0x8674e6d4] | group | `BOUNTY >` and the value `9999999` (placeholder). | held hidden: no bounty in the feed |
| `Cooldown_Meter_Group` [0x84a226ec] | group | The cool-down (evade) bar: the text `COOLDOWN`, `Outrun_Backing`, the track `BasePoly_Add` [0xa1eb7e6e] and the fill `BasePoly` [0xcf817638] (pivot x = -108, the left end). Parent: the radar group 0xab9c1d4d. | **bound** |
| `HIDING` group [0x60b45523] and `Hide_Car_Icon` [0x38074901] | group, image | The hiding indicator under the cool-down bar. | held hidden: the hiding timer is not in the feed |
| `Heat_Meter_Group` [0xc46a80a9] | group | The heat meter's backing, `Heat_Icon`, `Shift_Up_Icon`, `Heat_Icon_Additive` and the `x` label group. | held hidden (see 2) |
| `Heat_Meter_Multi_Image` [0x862824c9], second arc [0x4b2cbe1b] | multi image | The two half arcs of the heat gauge (`Meter_Backing.tga`; the game turns each arc's mask). Parent: group 0xb0f63386 under the group 0x1603009e. | **not bound**: which arc takes which half of the heat is not confirmed |
| heat label `x10` [0xa81a2028] | string (two objects, same hash) | The heat multiplier text, `x{int}` (`HEAT_X%.0f` **[binary]**). | **not bound**: the two objects share one hash, so a lookup by name finds only one of them |
| `CostToState` [0x3ff5f33c] | string | The cost-to-state text, `1000000` (placeholder). | **not bound**: the feed has no cost |
| cops group [0xe0b1430b] | group | The group holding `Cops_Icon` [0x3787231c] with its count `99`, `Cops_TakenOut_Icon` [0x0fe608e6] and `Cops_Damaged_Icon` [0xb224abde] with their counts (the dump lists them as its children). The group's own name is not among the candidates tried. Its fade-in scripts start on `0x3c97521a` / `0x5c930796`, the same messages as the board. | **bound** (E8b): shown with the board, held at full alpha in a chase |
| `Cops_TakenOut` [0xa16f9f1e] | string | The cops taken out (wrecked) count, right-justified beside `Cops_TakenOut_Icon` (placeholder `99`, `FONT_MW_BODY`). Hash checked with `fe_hash_upper`. | **bound** (E8b): the wrecked count of the chase (`PursuitHud::cops_destroyed`) |
| `Cops_Damaged` count [0x5fa70d4c] | string | The damaged count beside `Cops_Damaged_Icon` (placeholder `999`). | not bound: the damaged count is not in the feed |
| `Cops_Icon` count [0x814918ca] | string | The cops in pursuit beside `Cops_Icon` (placeholder `99`). | not bound: the engaged count is not in the feed |
| `Radar_Icon` [0x6ccbbbf1] | image | The radar's green laser. | bound already (`cop_nearby`) |
| `Radar_DirectionArrow` [0x24196ad4], `RADAR_GROUP` [0x062743f5] | image, group | The radar's arrow and group. | not bound (the radar detector's target is not in the feed) |
| busted bar | — | The bust meter (five segments, [ai-pursuit.md](ai-pursuit.md) section 4.4). | **not located**: no object name hashes to a candidate; the group 0x3345911d with four `button_ring.tga` images under 0x1603009e is unverified |

The original's HUD classes are `Hud_PursuitBoard`, `Hud_BustedMeter`, `Hud_HeatMeter`, `Hud_CostToState`,
`Hud_RadarDetector` [binary]. Only the board, the timer and the cool-down bar have their objects identified above.

**Other objects of the chase in the package.** `INFRACTIONS` [0xc0f39f88] and `Ramming a Police Vehicle`, `CODE: 404`
text (placeholders), `Generic_Infraction.tga` [0x69d4e157]. These are the infraction list, not bound.

## 2. What the rewrite shows

Driven by `HudState::pursuit` (`Option<PursuitHud>`, `None` while no cop chases the player). The world fills it in
`scenes/world/hud.rs` from `TrafficWorld::pursuit_feed()`.

- **The board** (`PursuitBoardGroup`): shown while a pursuit runs; hidden otherwise (`crates/nfsmw/src/hud/elements.rs`,
  `Layout::update`). Its group and the cool-down group are held at full alpha during a chase, because their fade-in
  scripts (`0x033113ac`, `0x5079c8f8`, `0x13f51124`) start on messages this build does not raise (the cool-down group
  has `on 0x3c97521a`) **[verified]**. The fade is skipped, as for the mirror.
- **The timer**: `secs` (the feed's seconds since the chase began) as `mm:ss.cc`, the package's template **[verified]**.
- **The cool-down bar**: shown when the evaded share is at least 0.05 (the spec's `evade_level >= 0.05`, ai-pursuit.md
  section 4.4 **[decomp]**). Its fill grows from the left end: width = full width × evaded share. The share is
  `1 − secs_to_evade / timeout(heat)` (`blackbox_pursuit::evade::timeout`), which is the progress the world already
  computes **[decomp]** (`scenes/world/ai/police/world.rs`, `pursuit_feed`). Shown on screen: the fill starts at the
  left end and grows right **[verified, offscreen screenshot]**.
- The cool-down group is inside the radar group in the package, so the layout decides it on its own flag, and the
  hiding parts and the bounty row are held hidden under any feature that shows around them.

**Not done, with reasons.** The heat arcs and label (the arcs' mask mapping and the label object are not resolved),
the cost to state and the bounty (not in the feed), the busted bar (no object located), the infraction list, and
the radar detector beyond the laser (its target source is not in the feed). Each is a feed or an object question,
not a rendering one.

## 3. How to check it

1. `nfsmw view-world --drive BMWM3GTR --open-console --exec "wait 4" --exec "pursuit 5" --exec "wait 3" --screenshot-delay 10 --screenshot out.png`:
   the timer shows top right and, once the evade share is at least 5 per cent, the cool-down bar shows at the bottom
   with its fill at the left end.
2. Without a chase the board and the bar are hidden.
3. Tests: `cargo test -p nfsmw hud::pursuit` (the evade share, the threshold, the timer format, the board and bar
   gating, the fill's left end), and with the install `NFSMW_GAME_DIR=… cargo test -p nfsmw hud:: -- --include-ignored`
   (the objects above exist in the package).

## Open questions

- The heat arcs: which of `0x862824c9` and `0x4b2cbe1b` is the first half of the gauge, and the mask rotation per heat.
- The `x10` label: the two objects share a hash; the game sets its text through the object's own pointer, which
  the runtime does not expose by name.
- The busted bar's object name (see the table).
- The message that starts the board's fade in the game (`0x3c97521a` on the cool-down group is one candidate).
