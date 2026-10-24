# UI themes: the HUD slots

The HUD (state-fed) slots of the element catalogue and where the Original theme draws each from. It extends
[ui-themes.md](ui-themes.md) section 6, which keeps the front-end and career slots and the numbering rules: numbers are
part of the mod ABI, never reused, never renumbered, and pinned by a test in `crates/nfsmw/src/ui/theme/tests.rs`.

Slots 1 to 12 were confirmed in the code in T6 (the `?` entries of the T1 draft resolved there). Slots 13 to 19 were
added for the Custom HUD 2026 ([ADR 0013](../decisions/0013-custom-hud.md), PR 5), in the free block before the front
end. `ElementId::is_menu` (raw 20 and up) does not class them as menus.

## The table

| Slot (id) | Original source (confirmed in code) |
|---|---|
| `Tachometer` (1) | Group guid `0x5164D4EA` (`TACHOMETER`, `hud/elements.rs`) in `HUD_SingleRace.fng`. Needle `3rdPersonNeedle`, face `TAC_Lines_7500`, redline `RPM_REDLINE`, shift light `Shift_light`, fill guid `0x456E`. Bound by `HudBinding::tachometer` (`hud/bind.rs`) |
| `Speedometer` (2) | Group guid `0x941FFF09` (`SPEEDOMETER`). Digits `SPEED_DIGIT_1..3`, units `3rdPersonSpeedUnits`. Bound by `HudBinding::speedometer` |
| `GearIndicator` (3) | Object `3rdPersonGear`, inside the tachometer group. Text and shift-time alpha set in `HudBinding::tachometer`. **Confirmed** (was `?`) |
| `Minimap` (4) | `hud/minimap.rs` (`GROUP`, `BACKING`, `ARROW`), shown by `hud/elements.rs`, placed by `hud/viewport.rs` ([hud-minimap.md](hud-minimap.md)) |
| `Blips` (5) | `hud/blips.rs`; styles from `hud/blip_style.rs` and `hud/blip_styles.rs` |
| `PursuitMeter` (6) | `hud/pursuit_meter.rs` (meter guid `0x7B422BA3`, cool-down guid `0x84A226EC`). The pursuit board (timer and cops count, groups `0xDE89E070` and `0xE0B1430B`) is bound in `HudBinding::pursuit`. **Decided (T7):** the board is part of this slot (section 8, slot map). The bounty row inside the board is its own slot, `BountyRow` (15) |
| `Nitrous` (7) | Group guid `0x87C38E97`, bar guid `0xEDFB6D37` (`nos_bar`), icon guid `0x27DDF583` (`nos_icon`, scripts `SCRIPT_NOS_BURNING` and `SCRIPT_NOS_READY`). Bound by `HudBinding::nitrous`. **Confirmed** (was `?`) |
| `Turbo` (8) | Group `TURBO_GROUP` (`hud/elements.rs`), needle `3rdperson_TurboDial`. Bound by `HudBinding::turbo`. **Confirmed** (was `?`) |
| `Speedbreaker` (9) | `hud/speedbreaker.rs`: icon `SPEEDBREAKER_METER_ICON_GROUP`, group `Speedbreaker_Meter_Group`, mask `Speedbreaker_Meter_Multi_Image`, linear group guid `0x82D60021`, bar guid `0x1FDAF669`. **Confirmed** (was `?`) |
| `RadioCard` (10) | Original: the EA Trax chyron, `hud/trax.rs`, package `EA_TRAX.fng` (`GLOBAL/GlobalB.lzc`). Custom: `ui/theme/custom/radio_card.rs`, fed by `hud/radio.rs` data (built in T9, [ui-themes-custom.md](ui-themes-custom.md)) |
| `PoliceRadar` (12) | `hud/radar.rs`, `hud/roots.rs::police_radar`: the detector group (laser, arrow, two LED rows) and its backing in either layout. Original: the package's objects. Remastered: a pill with dashes and a dial ([ui-themes-remastered-screens.md](ui-themes-remastered-screens.md)) |
| `BustedOverlay` (11) | `hud/busted_overlay.rs`, package `BUSTED_OVERLAY.fng` (`GLOBAL/InGameB.bun`), loaded by the HUD while `HudState::busted_overlay` is set. **Confirmed** (was `?`) |
| `RaceStatus` (13) | The race information group `0xB1C5ED0D` with the clock `CurrentLapTime`, its icon `TIMER_ICON` and `POSITION_GROUP` (`hud/race.rs::root_hashes`, `hud/roots.rs::race_status`). Inside it: the place, the progress group and the held tollbooth group. Custom: the race strip (PR 8) |
| `Leaderboard` (14) | `LeaderBoardGroup` `0x5ED609A1` and its four rows (`hud/leaderboard.rs`, `hud/roots.rs::leaderboard`). Custom: the compact leaderboard (PR 8) |
| `BountyRow` (15) | The career bounty row `0x8674E6D4` (`hud/elements.rs::BOUNTY_ROW`, `hud/roots.rs::bounty_row`). It sits inside the pursuit board (below). Custom: the bounty in the pursuit panel or the free-roam extras (PR 8) |
| `Banner` (16) | The generic message group `Hud_GenericMessage` `0xE0BA07EC` and the `321_GO_GROUP` countdown (`hud/race.rs`, `hud/roots.rs::banner`). Custom: the intro, countdown, escape and busted cards (PR 8) |
| `Notifications` (17) | No Original source: empty under Original. Custom: the notification stack (PR 4, drawn in PR 8) |
| `WorldMarkers` (18) | No Original source: empty under Original. Custom: the world markers and edge arrows (PR 9) |
| `VehicleCondition` (19) | No Original source: empty under Original. Custom: the condition chip (PR 8) |

## Where the new slots' objects sit **[verified]**

Read from `HUD_SingleRace.fng` in `GLOBAL/INGAMEC.BUN` (the object list's parent links, no decompilation):

| Object | Parent chain |
|---|---|
| race information `0xB1C5ED0D` | `0x5C7A5C8A` → `0x5D0101F1` (no slot's root) |
| `LeaderBoardGroup` `0x5ED609A1` | `0x5C7A5C8A` → `0x5D0101F1` |
| bounty row `0x8674E6D4` | `PursuitBoardGroup` `0xDE89E070` → `0x5C7A5C8A` → `0x5D0101F1` |
| `Hud_GenericMessage` `0xE0BA07EC` | the radar's group `0xAB9C1D4D` (no slot's root) |
| `321_GO_GROUP` | the radar's group `0xAB9C1D4D` |

So the race readout, the leaderboard and the banner were drawn as unclaimed nodes before their slots, and their adapters
emit the same nodes. The bounty row was not: it sat in the `PursuitMeter` slot, inside the board.

## The bounty row inside the pursuit board

- `PursuitMeter`'s roots exclude the bounty row, and `BountyRow`'s adapter emits it, so no node is taken twice.
- The row went with the board before it had a slot, and it still does: when the active theme replaces `PursuitMeter`
  but leaves `BountyRow` to Original, the composer draws no FEng bounty row (`original::HOSTS`, `compose.rs`). The
  Remastered theme, which draws its own pursuit timer, therefore still draws no FEng bounty row.
- A theme that provides `BountyRow` draws its own row whatever it does with the board.

## Parity

Under Original the composed HUD is the package's tree node for node and in the same draw order. The one documented
difference grows: a slot root whose parent lies outside its slot has no `parent` in the composed tree. Besides the gear
and the blips, those are now the race information group, `LeaderBoardGroup`, the pursuit board, the bounty row and the
two banner groups. Nothing draws from that link after composition ([ui-themes-compose.md](ui-themes-compose.md)).
`compose_tests.rs` checks both, and that Remastered draws no FEng bounty row.
