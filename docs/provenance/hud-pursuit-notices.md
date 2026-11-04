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
