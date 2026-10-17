# Provenance: career phone and SMS

Method, addresses and dead ends for [career-phone.md](../specs/career-phone.md). Ghidra auto names appear only here.
Image base `0x00400000`; program `/speed.exe`.

## Method

1. The Lua wrappers and event handlers of the three calls were found through the event tables of
   [career-scripts.md](../provenance/career-scripts.md) (hash table `0x008A5418`, handlers `0x008F5F40`, Lua wrappers
   `0x008F6450`). `EAddSMS` is index 100, `EShowSMS` 93, `ECellCall` 95.
2. The work of an event is done by a small request object kept in a ring (`0x00920390`, 16-byte slots). The request is
   filled by the wrapper; its virtual slot 0 (a destructor that does the work) runs when the queue drops it. This is why
   the callers of the working functions look like they have none.
3. The slot layout was confirmed against the three local saves (slot table file offset `0x4043`, counter `0x429B`)
   and the strings of `English.bin` (`blackbox-text`, a throwaway binary outside the repo).
4. Senders were collected from: the 203 distinct Lua bodies (`EAddSMS`, `ECellCall` constants), the bin attributes
   `EntryCellCallID`, `SMSRivalChallenge`, `SMSCellChallenge`, the NIS scene event lists (scan for the `ECellCall` hash
   `0x885F756E`, with the id read from the event's static data), and the native callers of the add function.
5. Voice: `copspeech.idx` banks were parsed with the layout of audio.md; event 204 has 4 banks (44 takes of the cop
   radio), the sentence record for 204 in `copspeech.evt` names event 425 and tests the two request masks.

## Addresses in `speed.exe`

| Role | Ghidra name | Address |
|---|---|---|
| `EAddSMS` Lua wrapper / event handler / request destructor | `LAB_0062a700`, `LAB_0062a6d0`, `FUN_0062a6a0` -> `FUN_00620290` | `0x0062A700`, `0x0062A6D0`, `0x0062A6A0` |
| add to inbox (immediate set 10..14, 95, 96, else queue) | `FUN_005f5120` | `0x005F5120` |
| deliver one id (state 2, sequence, post call) | `FUN_005ddd20` | `0x005DDD20` |
| slot lookup (`id < 150`: base + `4*id`) | `FUN_0056d720` | `0x0056D720` |
| next sequence (counter at `+0x26A`) | `FUN_0056d740` | `0x0056D740` |
| story predicate (false for 110, 111, 120..130, 132..137) | `FUN_0056d750` | `0x0056D750` |
| pending-list append / reload | `FUN_005f3390` / `FUN_005f50c0` | `0x005F3390` / `0x005F50C0` |
| flush of the pending list | `FUN_005edfe0` (called from the manager update `LAB_006036c0` at `0x00603706`) | `0x005EDFE0` |
| can-show test | `FUN_005ec040` | `0x005EC040` |
| batch delivery (pick story id, assign sequences) | `FUN_005ec160` | `0x005EC160` |
| "pending list empty" test (autosave overlay, HUD) | `FUN_005edfc0` | `0x005EDFC0` |
| `ECellCall` wrapper / handler / request destructor | `LAB_0062b320`, `LAB_0062b2f0`, `FUN_00637d00` -> `FUN_0062b180` | `0x0062B320`, `0x0062B2F0`, `0x00637D00` |
| `EShowSMS` wrapper / handler / request constructor | `LAB_006315d0`, `LAB_00631580`, `FUN_00631420` | `0x006315D0`, `0x00631580`, `0x00631420` |
| front end may open overlay | `FUN_00542670` | `0x00542670` |
| language id global | `DAT_008f41c0` (`FUN_0056bb20`), table `0x008F40F8` | ids 0 en, 1 fr, 2 de, 3 it, 4 es, 5 nl, 6 sv, 7 da, 12 pl, 13 fi |
| call id -> speech masks | `FUN_00701540` | `0x00701540` |
| queue a speech request ("CellCall") | `FUN_0071d860`, `FUN_00713b20` | `0x0071D860`, `0x00713B20` |
| speech flush after queueing | `FUN_00714410` | `0x00714410` |
| inbox list build | `FUN_00548ae0` | `0x00548AE0` |
| list row construction (sort key = sequence) | `FUN_0051fdf0`, comparator `LAB_00511e40` | `0x0051FDF0` |
| inbox screen constructor | `FUN_00553260` | `0x00553260` |
| message screen body fill | `FUN_00549150` | `0x00549150` |
| row / message handler (open, replay, delete) | `FUN_00531d40` | `0x00531D40` |
| row labels `SMS_MESSAGE_%d_FROM` / `_SUBJECT` | `FUN_00531cf0` | `0x00531CF0` |
| new career slot init | `FUN_0057f4d0` | `0x0057F4D0` |
| impound countdown that sends 120 | `FUN_00580990` | `0x00580990` |
| game manager update (calls the flush) | `LAB_006036c0` | `0x006036C0` |
| the pending list in the manager | `+0x2A4` list head, `+0x2A8` sentinel, `+0x2AC` count | `DAT_0091e00c` object |
| Game block save / load of the list | `FUN_005f7410`, `FUN_005f50c0` | `0x005F7410` |

Pad hash check: `PAD_BUTTON0` is `0xC519BFBF` (multiplicative hash, seed `0xFFFFFFFF`), so `0xC519BFC3` is button 4 and
`0xC519BFC4` is button 5; `PAD_BACK` is `0x911AB364`; `EXIT_COMPLETE` is a string of the executable.

## Dead ends

- The caller of the second flush path in the manager vtable: none needed; the update calls the flush directly.
- The HUD "new message" icon: no string, widget or code besides the autosave overlay's `HIDE_INCOMING` /
  `SHOW_INCOMING` and the list's `NEW_MESSAGE_ICON_<n>`. Not found does not prove absent in an FNG package; the packages
  were not unpacked here.
- The three player-car interface calls and the pursuit-count call in the can-show test (virtual calls, slots `+0xBC`,
  `+0x20`, `+0x5C`, `+0xAC` then `+0xA0`) were not resolved to names.
- Mapping of the three screen-message hashes to physical controls.
- `EShowMessageScreen` (message screen ids, for example `IntroMessageID` 2 in race `16.1.1`) is a different
  system (generic message screens), not the SMS inbox.
