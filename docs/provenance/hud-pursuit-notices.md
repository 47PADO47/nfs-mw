# Pursuit HUD notices

The parts of the pursuit HUD that tell the player of a change: the infraction notifier, and the further notices that
later sections add.

## Infraction notifier

- Spec: [hud-pursuit.md](../specs/hud-pursuit.md), "Infraction notifier".
- Sources read (2026-11-04): the original PC 1.3 infraction manager's report routine, the message it queues and the
  handler that turns the infraction bit into its language label, the HUD infraction element's constructor, request and
  update, and the group printer, in Ghidra and the disassembly, for behaviour only. The CC0 `dbalatoni13/nfsmw`
  `Frontend/HUD/FeInfractions.cpp`, `FEPkg_Hud.cpp` and `Gameplay/GInfractionManager.h` for the names. Detailed
  executable identity and addresses remain private. No source text copied.
- **[verified]** The installed `HUD_SingleRace.fng` (`nfsmw dump-screen`): the icon, the count, the header and the four
  rows, their scripts and chains, and the messages the icon's scripts raise. The language table for the labels' texts.
- Implemented in `crates/nfsmw/src/hud/infractions.rs` from the spec. The told set (a chase that starts or a log that
  lost an infraction starts again) is the rewrite's own, since it reads the chase's log rather than a report call.
- Checked by `hud::infractions` (synthetic: what is told, the labels) and the installed `hud::pursuit_install_tests`
  (rows, label text, count, icon, timing out).

## Hiding mark

- Spec: [pursuit-hiding.md](../specs/pursuit-hiding.md) section 5.
- Sources read (2026-11-04): the original PC 1.3 pursuit board's update (the cool-down branch that drives the hiding
  bar, and the branch outside it), in Ghidra, for behaviour only. Earlier private notes on the board's constructor
  named the bar's slot. No source text copied.
- **[verified]** The installed `HUD_SingleRace.fng`: the bar's appear and leave and the messages they raise, the
  responses of the group, the label and the icon, and their scripts' tracks.
- Implemented in `crates/nfsmw/src/hud/hiding_mark.rs` from the spec. The leave with no chase is the rewrite's own.
- Checked by `hud::hiding_mark` (the rule), `hud::pursuit_tests` (the cascade through a synthetic package) and the
  installed `hud::police_install_tests` (label included, and nothing outside the cool-down).

## Busted overlay suspect line

- Spec: [busted-sequence.md](../specs/busted-sequence.md) section 6.1, "The suspect line".
- Sources read (2026-11-04): the original PC 1.3 busted overlay screen's constructor (the scene type test, the rule,
  the threshold constant and the arguments of each string), the pursuit timer's print routine and its split into
  hours, minutes, seconds and milliseconds, in Ghidra, for behaviour only. The CC0 `dbalatoni13/nfsmw`
  `MenuScreens/InGame/FeBustedOverlay.cpp` for the names of the pursuit data fields. No source text copied.
- **[verified]** The installed `BUSTED_OVERLAY.fng` (the crawl string `0xf1ed545a`) and the language table (the five
  crawl strings and their printf places).
- Implemented in `crates/nfsmw/src/hud/busted_line.rs` and `busted_overlay.rs` from the spec; the chase's values come
  from the rewrite's police (`scenes/world/hud_pursuit.rs`).
- Checked by `hud::busted_line` (the rule, the duration format, the argument order) and `hud::busted_overlay` (written
  once per showing, sample text kept without a chase).
