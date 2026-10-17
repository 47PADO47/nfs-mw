# nfsmw-career (core slice)

- **Spec:** [formats/career-save.md](../formats/career-save.md), [formats/career-state.md](../formats/career-state.md),
  [specs/career-economy.md](../specs/career-economy.md), [specs/career-economy-flows.md](../specs/career-economy-flows.md),
  [specs/career-phone.md](../specs/career-phone.md), [specs/career-text-2.md](../specs/career-text-2.md) (the SMS sender
  table), [specs/career-flow.md](../specs/career-flow.md) (new career defaults, safe house front end, saving), and for
  the `safehouse` module [specs/career-safehouse.md](../specs/career-safehouse.md),
  [specs/career-safehouse-2.md](../specs/career-safehouse-2.md) and section 4 of
  [specs/career-scripts.md](../specs/career-scripts.md). Their own records:
  [career-economy](career-economy.md), [career-phone](career-phone.md), [career-flow](career-flow.md),
  [career-text](career-text.md), [career-safehouse](career-safehouse.md).
- **Sources read for this module:** the repository specs above and the repo's own `libs/save-store` (`digest::md5`). No
  decompiled code, no Ghidra session and no scratchpad dump were open while writing it (spec-first,
  [licensing.md](../licensing.md#spec-first)). The three development saves were opened read only for the optional test
  and for the CRC check; no save bytes are in the repository.
- **Implemented:** 2026-10-17, from the specs only, in `crates/nfsmw-career` (second pass: the `safehouse` module).
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
| New career | the serial stays empty and the gameplay blob zeroed | the serial generator (seed `0x19B3`) is not specified; the blob is the progress slice's job |
| `save::ProfileSave::parse` | refuses a wrong length, magic, size, type id, header checksum or MD5; accepts a wrong descriptor or profile checksum | career-save 1: only the first group is checked by the loader. `to_bytes` rewrites all four |

## Known differences from the original and gaps

- **Left out on purpose:** milestones, the Blacklist graphs, races, the screens, the gameplay-blob contents, the node
  list, the shop unlock table and the popup queue (kept as raw bytes), the customization records (raw bytes), the
  profile pursuit history and the rap sheet rankings, the heat factors (they need `pursuitlevels`), the marker
  inventory (callers say whether a marker is held and remove it), the car lot flows (buy, sell, add car), and the
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
