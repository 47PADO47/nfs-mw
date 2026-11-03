# Pursuit HUD takedown line

- Spec: [hud-pursuit-takedown.md](../specs/hud-pursuit-takedown.md).
- Sources read (2026-11-03): the original PC 1.3 executable in Ghidra and its disassembly, for behaviour only: the
  pursuit's cop-destroyed handler (the multiplier, the window, the bounty and the counters), the HUD feeder that passes
  the wreck count, the vehicle key, the bounty and the multiplier to the pursuit board, the board's setter that picks
  the label and sends the line, the generic message's constructor, show and update routines (the scripts, the icon, the
  priority), and the other callers of the generic message for their priorities. Detailed executable identity and
  addresses remain private. No source text copied.
- **[install]** `HUD_SingleRace.fng` (`nfsmw dump-screen`): the generic message group, its player's seven scripts and
  lengths, the icon `0x6DD754EC` with its script `0x0013FF94` and the glow `0xCAEC9D04`; the English language table
  (`nfsmw strings`): the nine `Combo Bounty` labels.
- Implemented from the spec in `crates/nfsmw/src/scenes/world/ai/police/rep.rs` (the latest wreck),
  `crates/nfsmw/src/scenes/world/feed/takedown.rs` (the diff and the line's time), `crates/nfsmw/src/hud/takedown.rs`
  (the label and the text) and `crates/nfsmw/src/hud/race.rs` (the scripts).
- Checked by `scenes::world::feed::takedown` tests (synthetic: the baseline, a new chase, the line's time and serial),
  `hud::takedown` tests (the labels, the text) and the installed `hud::race_install_tests` (the objects and scripts
  exist in the package, and the line plays them).
- Not checked against the original's screen: no capture of the HUD during a takedown was made. The stock-game pursuit
  capture of 2026-11-02 records the latest wreck's bounty, multiplier and the wreck count per sample, which agree with
  the handler read (the multiplier stays within 1 to 3).
