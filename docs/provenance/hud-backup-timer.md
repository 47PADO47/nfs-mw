# Pursuit board backup timer

- Spec: [hud-pursuit.md](../specs/hud-pursuit.md), "Backup timer"; the ETA itself in
  [ai-pursuit-heat.md](../specs/ai-pursuit-heat.md).
- Sources read (2026-11-02): the original PC 1.3 pursuit board's constructor, interface setter and update, and the HUD
  feeder that passes the pursuit's backup ETA to the board, in Ghidra and the disassembly, for behaviour only (the
  thresholds 0 and 10 s, the scripts started, the rounding and the formatter shared with the pursuit time, and the early
  return during the cool-down display). The CC0 `dbalatoni13/nfsmw` `Frontend/HUD/FePursuitBoard.cpp` for the names.
  Detailed executable identity and addresses remain private. No source text copied.
- **[verified]** The installed `HUD_SingleRace.fng`: the backing (`Outrun_Backing.tga`), the row group with the
  `HUD_BACKUP` label and `BACKUP_TIMER`, their scripts and the messages that chain the backing's scripts to the group's.
- Implemented in `crates/nfsmw/src/hud/backup_timer.rs` from the spec. One guard is the rewrite's own: the leave script
  only takes down a row shown in this chase.
- Checked by `hud::backup_timer_tests` (synthetic: the script for each ETA, the timer text, the cool-down hold) and the
  installed `hud::pursuit_install_tests` (the backing's appear shows the label and timer, the flash in the last 10 s).
- 2026-11-03: a stock-game capture (the pursuit's status, ETA and budget fields over one pursuit from heat 1 to 5)
  confirmed the ETA rules the row reads. The board's own scripts were not recorded, so the look of the leave on a hidden
  row and of the row during a cool-down remain open.
