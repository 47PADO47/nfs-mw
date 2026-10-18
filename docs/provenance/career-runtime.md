# Career runtime (`crates/nfsmw/src/career`, world and screens)

- **Spec:** [specs/career.md](../specs/career.md) and the pages it indexes: [career-flow](../specs/career-flow.md),
  [career-safehouse](../specs/career-safehouse.md), [career-safehouse-2](../specs/career-safehouse-2.md),
  [career-economy](../specs/career-economy.md), [career-economy-flows](../specs/career-economy-flows.md),
  [career-milestones](../specs/career-milestones.md), [career-blacklist](../specs/career-blacklist.md),
  [career-car-lot](../specs/career-car-lot.md), [career-rapsheet](../specs/career-rapsheet.md),
  [career-rapsheet-2](../specs/career-rapsheet-2.md), [career-save-screens](../specs/career-save-screens.md),
  [career-save-screens-2](../specs/career-save-screens-2.md), [career-regions](../specs/career-regions.md),
  [career-phone](../specs/career-phone.md), [career-text-2](../specs/career-text-2.md); file layouts in
  [formats/career-save.md](../formats/career-save.md) and [formats/career-state.md](../formats/career-state.md).
- **Modules covered:**
  - `crates/nfsmw/src/career/`: `state` (the career being played), `data` (the install tables, read once), `io`
    (profile folder, file read and write), `start` (Career on the main menu), `autosave` (the three triggers).
  - `crates/nfsmw/src/scenes/world/career/`: `safehouse`, `pursuit`, `bust`, `events`, `regions`, `hud`, `phone`.
  - `crates/nfsmw/src/frontend/career_screens/`: `safehouse_menu`, `blacklist`, `rap_sheet/`, `save`, `car_lot`,
    `region_unlock`, `sms`.
  - `crates/nfsmw/src/modding/backends/career.rs`: the `nfsmw.career` 0.2 backend.
  - The rules themselves are in `crates/nfsmw-career` and `crates/nfsmw-data`; their record is
    [career-core.md](career-core.md).
- **Sources read for the spec:** the repository's career specs above, written from `speed.exe` 1.3 (Ghidra, read
  only) and the install's `gameplay.bin`, `English.bin`, `Labels.bin` and FEng packages, as each spec records. The
  world and screen code follows those specs.
- **Implemented:** 2026-10-18, from the specs. Commits on `claude/career-m5-screens` (`git log main..HEAD`, the career commits from
  `feat(data): read the career bins` to `feat(career): flush the phone queue in free roam`), plus the uncommitted screen code.
- **Choices where the specs are silent** (each is marked `[unconfirmed]` in its doc comment; found by grep for
  `unconfirmed` in these folders):
  - `career/data.rs`: a leaf event name with no three leading numbers (for example `epic_pursuit`) gives the leaf name
    as its id. The data name of the final race is not checked, so `1.8.1` may not match it. The handle and the vehicle
    key of a lot car are the name hash (`vlt_hash`); the spec does not say which handle a bought car gets, nor which
    vehicle collection the lot record points at.
  - `career/state.rs`: the milestone block of a new career sets the statistic node keys and the milestone type keys to
    `0`, because `CareerData` does not hold them (`MilestoneBlock::new_career`).
  - `modding/backends/career.rs`: the rival's car is handle 0 and its name is `Rank N`, because the bin data names
    neither. The rival list is in rank order, rank 1 first.
  - `scenes/world/career/events.rs`: the spec unlocks only the races flagged `InitiallyUnlocked` on entry and the rest
    through other races' unlock lists. The Blacklist does not keep those lists, so every free race of an entered bin
    opens. A boss race opens when its rival is unlocked and stays open once the bin is done.
  - `scenes/world/career/hud.rs`: the car is looked up by the name hash of its name; the handle rule is unconfirmed.
    The pending points are the running rep points of the chase (`ai-pursuit-heat.md` section 5.1), banked into the
    record when the chase ends.
  - `frontend/career_screens/car_lot.rs`: a bought car takes the first customisation record no car uses. The spec takes
    a free record; which one the original takes is not confirmed.
  - `frontend/career_screens/region_unlock.rs`: the label string the region text is set on is `0xD6C0E097` (spec
    7.2). Its black twin is not set, and the button's size is decoded but not confirmed.
  - `frontend/career_screens/sms.rs`: hidden for non-story messages. The voice group's role is read from its script
    only.
- **Checked against the game by:**
  - Unit tests: each module has a `*_tests.rs` next to it (for example `career/state_tests.rs`,
    `career/io_tests.rs`, `career/start_tests.rs`, `career/autosave_tests.rs`, `scenes/world/career/*_tests.rs`,
    `frontend/career_screens/*_tests.rs`, `modding/backends/career_tests.rs`) and the `nfsmw.career` tests in
    `nfsmw-modding`. These run with `cargo test`.
  - Real-install test: `career/data_tests.rs`,
    `the_install_gives_fifteen_bins_fifty_five_milestones_and_thirty_nine_traps`. It reads the career tables from a
    real install and checks the counts the specs give (15 bins, 55 milestones, 39 speed traps). It is ignored by
    default; run it with `--ignored` and `NFSMW_GAME_DIR` set to the install. Without the variable it returns early.
  - Screenshot runs: a screenshot run (`--screenshot`) uses a temporary profile folder
    (`nfsmw-career-screenshot` in the temp directory, `frontend/flow.rs`), so it never reads or writes the player's
    profiles.
  - Offscreen runs (2026-10-18): `nfsmw play --ui-script "wait 3;accept;wait 30" --screenshot <png>` presses Career on
    the main menu, drives in free roam in career mode, writes `PLAYER/PLAYER` (63,596 bytes, the size of the original's
    profile files) to the temporary folder and shows the closed-region minimap variant. `nfsmw view-screen <name>
    --screenshot <png>` draws `SMS_Mailboxes.fng` and `Car_Select.fng`.
- **Known differences from the original and gaps:**
  - Profiles are written to this build's own `career` folder next to the config file, never to the original's
    `Documents\NFS Most Wanted`. This is deliberate, so the rewrite cannot damage the original's saves.
  - Races are not run: no race mode, no AI racers, no race pay or event unlocks. Quick Race starts nothing (its
    `StartFreeRoam` command has no sender).
  - The `nfsmw.career` table keeps `BB_ERR_UNSUPPORTED` for `milestone_register`, `milestone_progress`,
    `rival_defeat`, `rival_set_locked`, `give_car`, `remove_car`, `save`, `slot_get` and every safe house and shop call.
  - Open items (see [roadmap](../roadmap.md#career-open-items-milestone-7)): the race mode, AI racers, the
    auto shop prices, and the exact car lot stat values (the bars use the repo's estimator, normalized over the lot).
  - The career screens draw once their opening messages are posted (`career_screens::post_opening`, from
    [career-safehouse-2.md](../specs/career-safehouse-2.md) section 11) and the FEng runtime treats an object's flag 8
    as its start-hidden state that a script set on it clears (`libs/blackbox-feng`). Two of the posted messages
    (`0x5FF45174` for the rival challenge content, `0x8BBAB3ED` for the bio content) come from the packages' message
    responses; who posts them in the original is not traced. The rival pictures load from `GLOBAL/RIVALS.BIN` (the 45
    menu pictures, not the `_IG` variants).
  - The first safe house visit enters bin 15 on engaging the safe house; the original does it when the rival bio opens
    (`career-safehouse.md` 5.3).
  - The 12 statistic types with no producer in the executable and the open questions of [specs/career.md](../specs/career.md)
    apply to the runtime as they do to the rules.
