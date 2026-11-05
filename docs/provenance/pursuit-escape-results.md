# Pursuit escape results

- Spec: [pursuit-escape-results.md](../specs/pursuit-escape-results.md).
- Sources read (2026-11-03): the original PC 1.3 executable in Ghidra and its disassembly, for behaviour only: the
  pursuit's escape path in free roam (the statistics it counts and the rewards screen it opens), the post-race screen
  event that pushes the results or the pursuit statistics by name, the pursuit summary it fills, the rewards screen's
  message handler (the row order, the next-row message, the Continue that opens the statistics), its row builder (the
  labels, the bounty and total formats, the icons by row type), and the statistics screen's pursuit page builder (the
  seven rows and their labels), and the HUD's in-game statistics action. Detailed executable identity and addresses
  remain private. No source text copied.
- **[install]** `PostRace_MilestoneRewards.fng` and `PostRace_Pursuit.fng` (`nfsmw dump-screen`): the objects, scripts
  and the messages the scripts post; the English language table (`nfsmw strings`): the row labels.
- Implemented from the spec in `crates/nfsmw/src/scenes/world/career/rewards.rs`, `career/pursuit.rs`,
  `crates/nfsmw/src/frontend/rewards.rs`, `frontend/pursuit_results.rs`, `frontend/post_race/mod.rs` and
  `scenes/world/hud.rs`.
- Checked by `scenes::world::career::rewards` tests (the rows of an escape), `frontend::pursuit_results` tests (the
  page's values), `frontend::rewards` tests and the installed `frontend::rewards_tests` (the rows come one after the
  other, Continue opens the statistics page, the page shows the seven labels).
- 2026-11-05, the other pages (spec section 3.1): the statistics screen's message handler (the page turn and the
  continue), its page builder (headings and button texts per page), the infractions and milestones line builders, the
  line's draw (the marks and their scripts by state) and the time format, read in the same executable for behaviour only.
  Implemented in `frontend/pursuit_results.rs`, `frontend/pursuit_pages.rs`, `frontend/prompts.rs`, `viewer/race_report.rs`
  and `career/rewards.rs`; checked by `frontend::pursuit_pages` tests (the ticks, the values) and the installed
  `frontend::post_race_tests` page turn (key 2 shows the infractions, the milestones and the pursuit again, and does not
  leave).
- Not checked against the original's screen: no capture of the screens after an escape was made.
