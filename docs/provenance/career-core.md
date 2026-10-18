# nfsmw-career (core slice)

- **Spec:** [formats/career-save.md](../formats/career-save.md), [formats/career-state.md](../formats/career-state.md),
  [specs/career-economy.md](../specs/career-economy.md), [specs/career-economy-flows.md](../specs/career-economy-flows.md),
  [specs/career-phone.md](../specs/career-phone.md), [specs/career-text-2.md](../specs/career-text-2.md) (the SMS sender
  table), [specs/career-flow.md](../specs/career-flow.md) (new career defaults, safe house front end, saving), and for
  the `safehouse` module [specs/career-safehouse.md](../specs/career-safehouse.md),
  [specs/career-safehouse-2.md](../specs/career-safehouse-2.md) and section 4 of
  [specs/career-scripts.md](../specs/career-scripts.md); for `rapsheet`
  [specs/career-rapsheet.md](../specs/career-rapsheet.md) and [specs/career-rapsheet-2.md](../specs/career-rapsheet-2.md)
  (with the corrected tables of career-economy-flows 5 and the verdict rule of career-text 6.2); for `savefile`
  [specs/career-save-screens.md](../specs/career-save-screens.md) and
  [specs/career-save-screens-2.md](../specs/career-save-screens-2.md); for `regions`
  [specs/career-regions.md](../specs/career-regions.md); for `milestones` [specs/career-milestones.md](../specs/career-milestones.md)
  and [specs/career-milestones-data.md](../specs/career-milestones-data.md); for `blacklist`
  [specs/career-blacklist.md](../specs/career-blacklist.md) and sections 3 and 5 of
  [specs/career-scripts.md](../specs/career-scripts.md); for `carlot` [specs/career-car-lot.md](../specs/career-car-lot.md).
  Their own records:
  [career-economy](career-economy.md), [career-phone](career-phone.md), [career-flow](career-flow.md),
  [career-text](career-text.md), [career-safehouse](career-safehouse.md), [career-rapsheet](career-rapsheet.md),
  [career-save-screens](career-save-screens.md), [career-regions](career-regions.md).
- **Sources read for this module:** the repository specs above and the repo's own `libs/save-store` (`digest::md5`). No
  decompiled code, no Ghidra session and no scratchpad dump were open while writing it (spec-first,
  [licensing.md](../licensing.md#spec-first)). The three development saves were opened read only for the optional test
  and for the CRC check; no save bytes are in the repository.
- **Implemented:** 2026-10-17, from the specs only, in `crates/nfsmw-career` (second pass: the `safehouse` module; third pass: `rapsheet`, `savefile` and `regions`). Fourth pass, 2026-10-18: `milestones`, `blacklist`, `carlot`, `settings::new_career` and `save::milestones_block`, with the `nfsmw-data` readers they use (see the last section).
- **Checked against the game by:** nothing was run in the original. Checked against the three development saves
  (`NAME`, `mwo`, `stock`): every file parses, passes the container checksums, the profile MD5 and the gameplay blob
  MD5, and is rewritten byte for byte after the settings and the stable were parsed and written back
  (`tests/real_saves.rs`, enabled by `NFSMW_SAVE_DIR`, see [testing.md](../testing.md)). The same test checks the values
  the specs state for those saves (flags word, stage per save, counter 74 and 80 read slots in `mwo`, the sold-history
  bounty 43,116,691 and evaded 111, the six calls without a sequence number). The descriptor checksum `0x28A9C0BB` is a
  unit test of the CRC. Everything else is unit tests built from the numbers in the specs.

## What is covered

| Module | Spec sections |
|---|---|
| `settings` (`CareerSettings`, `Flags`, `SmsInbox`) | career-save 3, career-state 3, career-flow 3 (new career), economy-flows 7 (cash) |
| `stable` (`Stable`, `CarRecord`, `CareerRecord`, `SoldHistory`, totals, `impound`) | career-save 4, career-state 4, economy 1.2 and 1.3, economy-flows 6.2, 6.3 and 6.5 |
| `economy` (`pursuit`, `infractions_screen`, `cash`, `infraction`) | economy 1.4, economy-flows 2, 3.4, 6.1, 6.2, 6.4, 7 |
| `phone` (`ids`, `queue`, `inbox`, `voice`) | career-phone 1 to 4, career-text-2 1.1 and 1.2 (sender ids only) |
| `save` (`ProfileSave`, `container_crc`) | career-save 1, 2 and 7 |
| `safehouse::zone`, `engage` | safehouse 2, 3.1 to 3.5 |
| `safehouse::respawn`, `first_visit`, `resume` | safehouse 3.5, 3.6, 4, 5.3 (first visit); safehouse-2 6, 7.3, 7.4, 9 |
| `safehouse::menu`, `stepper`, `autosave`, `phone` | safehouse 5.1, 5.3; career-flow 7.2, 8; career-phone 3.5 |
| `safehouse::heat` | safehouse-2 8; economy-flows 4 |
| `rapsheet::nav`, `session` | rapsheet 1 to 3; rapsheet-2 1.1, 3 |
| `rapsheet::pages`, `rank`, `time` | rapsheet 1.4, 4 to 8; rapsheet-2 1.2 to 1.5; economy-flows 5.3 |
| `rapsheet::history`, `card` | rapsheet-2 2 and 4; economy-flows 5.2 |
| `savefile::autosave` | save-screens-2 8, 14; career-flow 8 |
| `savefile::write`, `state` | save-screens 2, 5, 6; save-screens-2 10, 12 |
| `savefile::load` | save-screens-2 9.3, 14; career-save 1 |
| `regions` | regions 1 to 3, 6, 7, 9 |
| `milestones::stat` | milestones 2.2, 3.3, 3.5; milestones-data table 1 |
| `milestones::state` | milestones 2.3, 3.2, 8 |
| `milestones::set` | milestones 3.2 to 3.6, 3.8 (the milestone part) |
| `milestones::speedtrap` | milestones 1.3, 2.4, 3.7, 3.8; milestones-data table 3; scripts-events 7.2 |
| `blacklist` (`Blacklist`, bin walk, events) | blacklist 2 to 5, 7; scripts 3 and 5 |
| `blacklist::bin` | blacklist 2.1 to 2.6, 1 (side bins); scripts 3.2 |
| `blacklist::gate` | blacklist 2.5, 3.1, 3.3 |
| `blacklist::results` | blacklist 4.2, 5.2, 5.4, 6, 7.1; economy-flows 7; scripts-events 6.2, 7.2 |
| `carlot::catalogue` | car-lot 1, 2, 6 |
| `carlot::purchase` | car-lot 1, 3 |
| `carlot::sale` | car-lot 4 |
| `carlot` (`owned_count`, `custom_count`, `Limits`) | car-lot 3.2, 8 |
| `settings::new_career` | career-flow 3.1; career-save 3.2, 3.3 |
| `save::milestones_block` | milestones 7; career-save 3.1, 3.2 (raw block, decoded parts only) |
| `nfsmw-data::gameplay::bins` | blacklist 1, 8 |
| `nfsmw-data::gameplay::milestones` | milestones 1, milestones-data |
| `nfsmw-data::career_cars` | car-lot 1, 2, 8 |
| `nfsmw-data::rank_thresholds` | rapsheet-2 1.2, 1.5; economy-flows 5.3 |
| `nfsmw-data::pursuit::career_heat` | economy-flows 4; milestones 3.3 |
| `nfsmw-data::gameplay::races` (`barriers`) | regions 2 |

## Choices where the specs are silent, ambiguous or marked unconfirmed

Each is also a doc comment at the code. Nothing here is a new game constant.

| Where | Choice | Why |
|---|---|---|
| `economy::AWARD_BOUNTY_DIVISOR` | 1 | economy-flows 3.4 says to treat the divisor as 1 (the file image holds 5, never written, **[unconfirmed]**) |
| `economy::pursuit` | `busted` is a plain flag of the result | economy-flows 2.2: model slot `+0x74` as "ended in a bust" |
| `stable::impound` countdown | the state becomes 4 on the tick that brings the countdown from 1 to 0 (five ticks after the impound) | economy-flows 6.2 "ticks reach 0" and the summary "five ticks"; the 6.3 wording could also mean one tick later |
| `stable::impound::release_fee` | `round(0.25 * cost)` with halves rounding up | the rounding mode for an exact half is not stated; a car price that is not a multiple of 2 is the only case that differs |
| `economy::release_with_cash` | refuses (no change) when the cash is below the fee | the spec only says the fee is paid by `SpendCash`, which clamps at 0; giving a car away for less than its fee was judged worse. `CareerSettings::spend_cash` itself still clamps as specified |
| `CarRecord::counts` | the class mask test is "any bit of `0xF0000`" together with the owned bit | career-state: "a record counts only if both are set"; the weaker reading was taken. In the three saves it gives the totals the specs list (NAME 900, mwo 43,116,691, stock 28,800 total bounty) |
| `Stable::live_career_indices` | a career record is counted once even if two cars point at it | not stated; no save has the case |
| Totals as `u64` / `u32` | the totals return wider integers than the `u32` of the original | the original wraps; no value in the specs comes close |
| `CareerSettings::add_cash` | saturates at `u32::MAX` | economy-flows 7: no cash cap found; wrapping to a small balance was judged worse |
| `Flags` | bits `0x8000`, `0x10000`, `0x20000` carry both names | career-state and career-save name them differently (see below) |
| `phone::PhoneQueue::add_sms` | an id with no slot (150 and up) is queued, the flush skips it; an immediate id with an invalid (`0xFF`) slot is still delivered | career-phone 3.1 and 3.4 test only "slot exists" (`id < 150`), not the label |
| `SmsInbox::open_message` | does nothing for a slot that is not listed (state 0 or invalid id) | the spec sets state 4 unconditionally but the screen cannot reach such a slot |
| `SmsInbox::delete` | keeps the sequence number | the spec only sets the state to 0 |
| `phone::process_cell_call` | the "already open" check of the text-language branch is left to the caller | the overlay state is not part of the inbox |
| New career | the gameplay blob is left as the progress slice's job; the serial is generated (see the last section) | `settings::new_career` now builds the serial from the seeded generator; the blob is not built here |
| `save::ProfileSave::parse` | refuses a wrong length, magic, size, type id, header checksum or MD5; accepts a wrong descriptor or profile checksum | career-save 1: only the first group is checked by the loader. `to_bytes` rewrites all four |

## Known differences from the original and gaps

- **Left out on purpose:** race records and the race event graphs (the bin walk and the gate are in `blacklist`), the
  screens, the gameplay-blob contents except the milestone block, the node list, the shop unlock table and the popup
  queue (kept as raw bytes), the customization records (raw bytes), the profile pursuit history and the rap sheet
  rankings, the heat factors (they need `pursuitlevels`), the marker inventory (callers say whether a marker is held
  and remove it), the car lot's add-car step (`carlot` has the purchase and sale checks), and the
  pending SMS list inside the gameplay blob (the queue lives in memory).
- **Default stable hash:** kept as stored, never computed. It depends on the installed car data, so a save written by
  the rewrite loads in the original only for the same car set (career-save 2.2 and 7).
- **Flag bits `0x8000`, `0x10000`, `0x20000`:** career-state reads them as "event 19.8.69 done", "every challenge-series
  race done" and "first safe-house visit done"; career-save reads them as unlocks for `BONUS_SL65`, `BONUS_GT2` and
  "bonus-content popup queued". The bits are the same; the crate exposes both names and interprets none of them.
- **Release SMS:** the win tick fires script event `0x78`, which equals SMS id 120 (the impound release notice of
  career-phone 5.1). The crate reports how many cars became releasable and names the id `phone::IMPOUND_RELEASE_SMS`;
  queueing it is the caller's job.
- **Sender and subject text:** only the numeric sender per id is in the code. Subjects, senders and bodies are read at
  run time from the install language table by label (`SMS_MESSAGE_<n>`, `_FROM`, `_SUBJECT`).

## Safe house module: choices and gaps

| Where | Choice | Why |
|---|---|---|
| `safehouse::engage` | the engage decision reads a prompt-ready flag (property `0x0280164F`) and a node flag (`0x2B1F54F6`) as plain booleans | their meaning is unconfirmed in the spec; the input struct names them without interpreting them |
| `safehouse::engage` | a zone string of no known kind posts no zone message (`PostEnterMessage(None)`) | safehouse 3.2: engage zone only posts for the three kinds |
| `safehouse::heat` | factors are parameters; the 0.95, 0.94 and 0.93 constants are named defaults | the values come from the install `pursuitlevels/default`; the milestone pairing is unconfirmed in economy-flows 4 |
| `safehouse::heat::unload_world` | writes only in a career context and only when the current car has a record; a NaN heat is written as 5.0 | safehouse-2 8.3; the NaN case is not in the spec |
| `safehouse::heat::spawn_heat` | 1.0 when the car has no record or the context is not career | safehouse-2 7.5 gives the default 1.0 outside a career; the no-record career case is not stated |
| `safehouse::respawn::heading_rotation` | returns `heading * (-1/360)` and builds no direction vector | the spec gives the factor but not the rotation axis or unit |
| `safehouse::respawn::Respawn::select_key` | the fallback key becomes `home`; collection existence and the name hash are caller closures | safehouse-2 9.3; the hash of collection keys is the attribute-system hash, not implemented here |
| `safehouse::first_visit` | stage 16 to 15 only; no SMS, no flag | safehouse 5.3 lists no SMS or flag for this step (bin 15 has no entry message, career-scripts 3.3); nodes, rewards and manager updates of the enter-bin routine are the progress slice's |
| `safehouse::stepper` | only the counter rule; where the stepper resumes after the first-visit bio is not modelled | safehouse 5.3: unconfirmed. Step 7 is reached on any call after step 6; the spec does not say what calls it after the stage 15 overview |
| `safehouse::autosave` | one gate for both uses; a refused request is dropped, not retried | safehouse 5.3 (`autosave possible`) and career-flow 8 describe two overlapping tests; no retry is documented |
| `safehouse::phone` | in the safe house nothing is delivered (all flush conditions false) | not stated in the safe house specs; it follows from career-phone 3.5 (flush needs the game state 6) |
| `safehouse::resume` | with the debug byte set the prologue check is skipped; a custom start position, the `+0x2D4` byte and the previous node are inputs | safehouse-2 6.1, 7.3 |

Not modelled in `safehouse`: the front-end request resolver of safehouse section 4 beyond the respawn flag rule, the
pause menu path that clears event states (3.6), the marker choice screen, the region unlock screen, the rap sheet
screens, the car-select flow, the world loader and the per-frame pursuit/results screen plumbing. No spec file was
edited.

## Rap sheet, save screens and regions: choices, corrections and gaps

| Where | Choice | Why |
|---|---|---|
| `safehouse::autosave` | removed; the path re-exports `savefile::autosave` | the earlier model posted `EAutoSave` at race end, took a "profile loaded" and a manual-save input into the gate and treated `BossRace` as an exemption flag. The spec (save-screens-2 8) gives the gate order, the three triggers (roaming start, event start, pursuit evaded) and the one-shot skip |
| `safehouse::menu::bindings` | `CAREER_DATA` is the career percentage, not the stage | career-safehouse 5.1 was corrected to point at rapsheet-2 section 4 |
| `stable::Stable::fines_due` | live cars only, no sold-history part (and `fines_paid`) | rapsheet 1.4; the infraction counts keep their sold part. This corrects a pass-1 choice |
| `rapsheet::history` | sums and the top-5 bounty compare as signed 32-bit; sums saturate | rank rule: "all comparisons are signed 32-bit"; the original wraps |
| `rapsheet::history` | the stored-record count at block `+0xA0C` is not touched by `record` | the recording rule never names it; career-save 5 lists it (5 in one save, 97 in another). Left as found |
| `rapsheet::history::case_id_text` | the caller supplies the 8 characters | the generator is random (alternating digit and capital letter, start unspecified) |
| `rapsheet::pages::vehicle_rows` | uses `CarRecord::counts` and a career index below 25 | rapsheet 1.4 says "below 26" for a 25-slot table; index 25 would read past the table |
| `rapsheet::pages::infraction_rows` | the swapped speeding labels are kept | rapsheet 5, unconfirmed whether intended |
| `rapsheet::time::format_ticks` | default separators; the seconds case is not padded, the hour case uses a two-wide blank-padded hour | rapsheet-2 1.3 writes `SS.cc` and "blank instead of a leading zero" without examples; per-language separators are not in the spec |
| `rapsheet::card` | `game` is summed in `f64` over the truncated bytes; a zero denominator gives 0 | the width of the original float and the zero case are not stated |
| `rapsheet::rank` | the threshold tables are kept as defaults; the rank function takes any 15-long slice | the original reads them from `FE_ATTRIB`; a slice of another length gives 16 |
| `rapsheet::session::EndGame` | a step past 4 opens nothing | the switch has five cases; where context bit 18 is cleared again is unknown |
| `savefile::load::check_load` | a file longer than the sizes describe is refused as a bad container; a header whose total is not the file length is refused before any size check | not described by the spec; this follows the listed order and refuses the case the reader cannot read |
| `savefile::load` | the default stable hash is a caller input | it depends on the installed car data (career-save 2.2) |
| `savefile::state::manual_save_panels` | exposed as an enum list documented as reconstructed | save-screens 5: the prompt order was rebuilt from the library and not seen in game; no logic depends on it |
| `savefile::write` | `WriteOutcome::resolve` reports an open failure, a cancelled overwrite, and nothing else | only a failed open is reported; a full disk or short write is ignored (a rewrite may choose a safer write, which the spec allows) |
| `regions::gates` | the closed sets come from a fallback table; `gates_in_list` reads a `Barriers` list | regions 2: the rewrite should read the list so a modded bin works, and keep the table as a test |
| `regions::gates::RegionGates` | models the trigger rules of 3.3, not the scenery instances, the road flags or the collision world | those need the world data; `group_state` and `instance_blocks` give the two pure rules |
| `regions::unlock` | the leave is 600 FEng ticks at 1200 per second | regions 7.4; the sound ids other than the accept click are not named in the repo |

Spec inconsistencies found (no spec file was edited): rapsheet 1.4 "career record index below 26" against 25 slots;
rapsheet 1.4 "(the sold part wraps at 16 bits)" for the all-infractions total is not modelled (the crate sums in
`u32`); regions section 0 and 9 say "stage 12 opens {11, 14, 15}" while gate 10 also **closes** at stage 12 (it is
open at stages 16 to 13), which section 1.3 states and `regions::apply_stage` reports; career-flow 8 was already
corrected for the autosave triggers, and no code comment still says "race end".

## Milestones, Blacklist, car lot and new career: choices and gaps

Each choice is also a doc comment at the code (`**[unconfirmed]**` or the module docs).

| Where | Choice | Why | Spec section |
|---|---|---|---|
| `milestones::set` | the pursuit summary holds at most `PURSUIT_SUMMARY_MAX` keys; past the cap a milestone still goes to stake and is only left off the list | the spec gives no cap | milestones 3.2 to 3.6 |
| `milestones::set` | the summary is kept after a bust | the spec only says it is emptied at the start of a pursuit | milestones 3.2 to 3.6 |
| `milestones::set` | banking is not gated by play mode; the debug-only forced bit is not modelled | the spec gates only the threshold test; in a challenge race the test never runs | milestones 3.5 |
| `milestones::set::pay_banked` | the bounty goes through `award_player_bounty` with divisor 1 | the economy divisor (see `economy::AWARD_BOUNTY_DIVISOR`) | milestones 3.6, 9 |
| `milestones::state` | only the threshold is modelled; the goal and step ladder are not used | no code of the exe reads them | milestones 1.2 |
| `milestones::speedtrap` | a trap with no threshold in the data uses 88 km/h; with no bounty, 100 | the spec gives the values but not the code path that uses them | scripts-events 7.2 |
| `milestones::speedtrap::SpeedTrap::is_active` | a crossing can complete a trap when it is armed and not done | the body of `IsActiveSpeedTrap` is not shown; this matches the completion routine | milestones 3.7 |
| `blacklist::Blacklist::advance` | when bin N reaches `done` while it is the current bin, bin N-1 is entered; bin 1 stays current | the spec has no rule for `CurrentBin` after a rival is beaten | blacklist 5.3; scripts 5.1 |
| `blacklist` (walk) | a bin that is `done` on entry does not advance the walk again | not stated | blacklist 2, 5.3 |
| `blacklist` (counters) | races, challenges and bounty are counted from the records; entering a bin does not reset them | section 2.3 resets the script copies only | blacklist 4.3, 2.3 |
| `blacklist` (gate) | bounty going down does not lock an unlocked rival | the spec is silent | blacklist 3.2 |
| `blacklist::FINAL_RACE_EVENT` | `1.8.1` is the event that sets `CAREER_COMPLETE` | the flag is tied to the post-race return, not the race itself | blacklist 7.1 |
| `blacklist::results` | race cash is paid on a first place only | economy-flows 7 says "placed first"; blacklist 4.2 step 4 says "on every finish" | economy-flows 7 |
| `blacklist::results::EventResult::challenge_series_complete` | the caller computes it (every race of bin 19 done except `19.8.31` and the optional ones) | the unlock lists belong to each race record, which the caller owns | blacklist 7.1 |
| `blacklist::results` | the region screen is reported, not shown; the caller shows it at the safe-house arrival | the spec puts the screen on the arrival stepper | regions 7.1 |
| `blacklist::bin` (`Opm`) | the side bin's name "OPM" is not expanded | the spec does not give it | blacklist 1 (notes) |
| `carlot::catalogue::LotCar` | the price is read as `i32` and clamped to 0 by the caller; a class value the game does not produce is never offered | the game's compare with a negative price is not checked | car-lot 1 |
| `carlot::purchase` | the handle of a bought car is supplied by the caller | not stated whether the lot's handle or a new one is used | car-lot 3 |
| `carlot::purchase` | `NotOffered` and `Locked` are refusals | the spec shows the car as not for sale or greyed out but names no refusal | car-lot 1 |
| `carlot::purchase::buy` | the flag word gets `CLASS_MASK` and `FLAG_OWNED` | the stable writes owned cars this way; the spec names only the class bit | car-lot 3 |
| `carlot::purchase::buy` | the prologue's plain confirm (stage 16) uses `Delivery::Deliver` | the only choice that leaves the current car alone | car-lot 3 |
| `carlot::purchase::buy` | the customization index is the caller's; preset parts are not built | the customization slots are raw bytes in this crate | car-lot 3 |
| `carlot::purchase::buy` | two checks are added after the spec's three: a free career record (25) and a free car slot (200) | a full table cannot take the car | car-lot 3 |
| `carlot::sale::sell` | the place and stage checks and the price lookup run before the spec's refusals | the spec lists refusals only | car-lot 4 |
| `carlot::sale::sell` | the sold car's career record is cleared in the same step | no record is left pointing at nothing | car-lot 4 |
| `carlot::sale::replacement_current` | the new current car is the first free car in slot order | the spec says only that the screen re-reads the current car | car-lot 4 |
| `carlot` (`FLAG_MARKED`, `0x40`) | a marked car is left out of the owned count; what the mark means is unknown | the spec says only that the count skips it | car-lot 3.2 |
| `settings::new_career::SerialRng::draw` | with `n == 0` the state is left as it is | the spec says only that `n = 0` returns 0 | career-flow 3.1 |
| `settings::new_career::format_serial` | the bytes after the letters are zero | the save field is 16 bytes; the original may leave them as they were | career-save 3.2, 3.3 |
| `settings::new_career::generate_serial` | the four letters come from the caller | their source is open in the spec | career-flow 3.1 |
| `nfsmw-data::gameplay::milestones` | `unconfirmed_goal`, `unconfirmed_step` and `unconfirmed_field` are read and not used | no code reads them; the field names are unknown | milestones 1.2, 9 |
| `nfsmw-data::rank_thresholds` | the threshold array is read by field key `0xF9A7D5F7` as the spec gives it | the field name is not in the specs | rapsheet-2 1.2 |
| `nfsmw-data::pursuit::career_heat` | `milestone_factor` (0.93) is read; its pairing with the milestone moment is not used | the spec does not pair it and no caller was found | economy-flows 4; milestones 3.3 |

Gaps in the new modules:

- `save::milestones_block`: the count fields at `0x18..0x34` and the start offset of each array after `0x80` are not
  decoded, so the block is kept raw and records are read and written at caller-given offsets (milestones 7).
- `blacklist`: the transient states of a bin (`initial`, `unlock_races`, `introduce_rival`, `check_rival`) run inside
  the entering call and are not stored; the `initial` wait of bin 16 is not modelled.
- `blacklist` and `carlot`: race results, pursuit results and the flows that call `buy` and `sell` are the caller's.
  `carlot::catalogue::state_at` gives the stage-based availability only.
- `milestones`: the milestone heat adjustment is not applied here (`safehouse::heat` takes the factor as a parameter).
