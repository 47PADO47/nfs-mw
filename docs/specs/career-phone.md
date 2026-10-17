# Career phone: the SMS inbox, cell calls and their delivery

How the original sends, stores, shows, reads and deletes the story messages and phone calls of the career. This
page covers the 150-slot inbox of the save, the three ways a message arrives (queued text, immediate call, a call
from a cutscene), the rules that decide when a message may appear, the inbox screens, the voice lines behind the
calls, and the complete list of which message is sent by what. The text, sender and subject of each message are in
[career-text-2.md](career-text-2.md); the per-bin message table (which bin sends which id) is in
[career-scripts.md](career-scripts.md) section 3.3; the save layout is in [career-save.md](../formats/career-save.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, base `0x00400000`) through the Ghidra MCP and a linear
  disassembly; `LANGUAGES/English.bin` read with the repo's `blackbox-text`; the 258 Lua handlers of
  `GLOBAL/gameplay.bin` (disassembled); the NIS scene event lists; `SOUND/SPEECH/copspeech.idx` / `.evt`; the three
  local save files (structure only). No game text beyond short subjects is quoted.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked in install data or
  saves. **[unconfirmed]** a guess or a name I could not prove. Addresses are `speed.exe` addresses; Ghidra names are
  only in [the provenance file](../provenance/career-phone.md).

## 0. Summary

1. The inbox is 150 slots of `{id, state, sequence}` inside the career settings object. A slot is valid when the
   language table has `SMS_MESSAGE_<id>`. **State 0 = never delivered, 2 = delivered and unread, 4 = read.** State
   4 is set the first time the player opens the message in the inbox. **[exe]** + **[verified]**
2. Script and data code use three calls: `EAddSMS(id)` (queue a text message), `ECellCall(id)` (a phone call:
   voice, or a text message in five languages) and `EShowSMS(id)` (open the inbox). Only `EAddSMS` is used by the
   Blacklist scripts; `ECellCall` comes from three race scripts and three cutscenes. **[exe]** + **[verified]**
3. Queued text messages wait in a list and are flushed once per game update, but only when the game is in free
   roam with no race, no front-end overlay, no call speaking and no pursuit. All messages in the list become
   delivered at the same moment (state 2, increasing sequence numbers); one of them is opened in the inbox. **[exe]**
4. The inbox list is sorted by sequence, newest first, in two tabs: story messages and "system/tip" messages. The
   screen marks unread messages. Opening a message marks it read and, for story messages, plays its voice line.
   The player can replay the voice and delete a message (the slot goes back to state 0). **[exe]**
5. Every valid message from id 10 to 105 except id 16 has a recorded voice take (75 of 76 valid ids), selected by a
   (group, line) pair derived from the id. **[verified]**
6. 13 of the 95 valid messages are never sent by any script or native code in this build (16, 21, 32, 47, 50, 74,
   97, 98, 105, 110, 111, 122, 129). Of the other 82, two are conditional (120, impound release; 121, a slow
   drive); the late-game save holds exactly the remaining 80 in the inbox. **[verified]**
7. The sender and subject mapping of career-text-2.md is confirmed: all 95 rows match `English.bin`. **[verified]**

## 1. Data

### 1.1 Slot table **[exe]** + **[verified]**

Inside the career settings object (profile offset `0xA8`), 150 four-byte slots start at `+0x12` (file offset
`0x4043` in a save); the sequence counter is a u16 at `+0x26A` (file `0x429B`).

| Byte | Meaning |
|---|---|
| 0 | `id`: the slot's own index, or `0xFF` when the language table has no `SMS_MESSAGE_<index>` |
| 1 | `state`: 0, 2 or 4 (below) |
| 2..3 | `sequence` (u16): the delivery order, 0 when the slot never received one |

Slot access is `slot(id) = base + 4 * id`, and `id >= 150` returns "no slot". On a new career and on reset all
150 slots get `id = index`, `state = 0`, then `id = 0xFF` where the label is missing; the counter is 0. The
sequence counter is incremented first and the new value is stored (the first message gets 1). **[exe]**

States:

| State | Meaning | Set by |
|---|---|---|
| 0 | not delivered (also: deleted) | initial; the delete confirmation |
| 2 | delivered, unread | delivery (section 3) |
| 4 | read | the inbox when the player opens the message (section 4.3) |

State 6 (both bits) is never produced. The list screen shows slots whose state has bit 1 or bit 2 set, that is 2
and 4. **[exe]**

Cross-check on the saves: the new profile after the prologue holds only id 15 with state 2 and sequence 1 (counter
1); the late-game save has the counter 74, 80 slots in state 4 with sequences 0 to 74. Sequence 0 occurs for ids
10, 11, 12, 13, 95 and 96 only: these are the immediate calls, which in a text-language game are marked
delivered without a sequence number (section 3.3). **[verified]**

### 1.2 Story and non-story ids **[exe]**

A predicate splits the ids. Non-story ids are 110, 111, 120 to 130 and 132 to 137; everything else is a story
message. The predicate is used by the delivery flush (which message to open), the list (which tab) and the message
screen (whether voice is available). The non-story ids are the tutorial tips, the impound notices and extra texts.

### 1.3 Labels and the sender / subject mapping **[verified]**

Each message has three labels in `English.bin`: `SMS_MESSAGE_<n>`, `SMS_MESSAGE_<n>_FROM`, `SMS_MESSAGE_<n>_SUBJECT`. The
table has 95 messages. All 95 sender and subject strings agree with the table of career-text-2.md (subjects are
truncated there with `..`). The only difference of wording is id 120, whose sender label reads "Rockport Police
Department" (the other page abbreviates it). Senders: Rog 45, Mia 26, Razor 11, Cross 4, Vic 3, Ronnie 2, Earl 2, the
police 1, and `-` for id 122. Ids 21 and 137 share one subject and one body length. Fixed captions: `SMS_MAILBOXES`,
`SMS_FROM`, `SMS_SUBJECT`, `SMS_TEXT`, `SMS_VOICE`, `SMS_INCOMING`, `SMS_MOBILE_MESSAGE`, `SMS_REPLAY`.

### 1.4 Voice groups **[exe]** + **[verified]**

A function maps a call id to a pair of one-bit masks for the speech system. Ids outside 10 to 105 have no voice.

| Ids | Group g | Line = id - first id |
|---|---|---|
| 10-16 | 16 | 0-6 |
| 17-22 | 15 | 0-5 |
| 23-29 | 14 | 0-6 |
| 30-36 | 13 | 0-6 |
| 37-41 | 12 | 0-4 |
| 42-46 | 11 | 0-4 |
| 47-52 | 10 | 0-5 |
| 53-57 | 9 | 0-4 |
| 58-63 | 8 | 0-5 |
| 64-69 | 7 | 0-5 |
| 70-72 | 6 | 0-2 |
| 73-76 | 5 | 0-3 |
| 77-79 | 4 | 0-2 |
| 80-84 | 3 | 0-4 |
| 85-88 | 2 | 0-3 |
| 89-96 | 1 | 0-7 |
| 97-105 | 0 | 0-8 |

The request carries `1 << g` and `1 << line`. The groups follow the Blacklist bins (group = bin, 16 = prologue, 0 = the
post-game set). The speech event is `cellcall` (id 204: priority 100, interrupting, not interruptable, never dropped,
expiry 55 s); its sentence record names the phrase event 425, whose single bank (any speaker) has 80 takes, each tagged
with a (group, line) selector. 75 of the 76 valid ids from 10 to 105 have a take; id 16 has none; four selectors have no
id (spare). The record format and the unexplained part are in [speech.md](speech.md) and
[audio.md](../formats/audio.md).

## 2. The calls the scripts can make

All three are events with a Lua wrapper (the wrapper takes exactly one argument; any other argument count does
nothing, which is why `ECellCall(1, 14)` and `ECellCall()` in two unused graphs are inert).

| Call | Argument | Effect |
|---|---|---|
| `EAddSMS(id)` | message id | the rules of section 3.1 |
| `ECellCall(id)` | call id | a request that plays the call (section 3.3) |
| `EShowSMS(id)` | slot id, or a negative number | opens `SMS_Mailboxes.fng`, at that slot, or at the list for a negative number (section 4.1). No script uses it |

Two other message names concern calls: `MNotifyCellCallStarted` and `MNotifyCellCallComplete` are broadcast by the
speech queue when a call's speech starts and ends (both at once if speech is unavailable). One race script waits for
the first (section 5.2); the handlers for the second in `drive_to_target` are empty.

## 3. Delivery

### 3.1 `EAddSMS(id)` **[exe]**

```
if id in 10..14 or id == 95 or id == 96:       # the "immediate" ids
    deliver(id)                                 # 3.2
else:
    slot = slot(id)
    if slot is none or slot.state == 0:         # not delivered yet (or deleted)
        if id is not already in the pending list:
            append id to the pending list       # oldest first
    # a slot in state 2 or 4: ignored
```

Re-sending an id that is already in the inbox does nothing, so the "wins == n" triggers of the bins cannot duplicate a
message (this settles the doubt of career-scripts.md 3.3).

### 3.2 `deliver(id)` **[exe]**

```
slot = slot(id)
if slot exists and slot.state == 0:
    slot.state = 2
    counter = counter + 1; slot.sequence = counter
    post an ECellCall(id) request              # 3.3
```

### 3.3 The `ECellCall(id)` request **[exe]**

Executed when the request is processed. The language id is a global; the five **text languages** are Dutch (5),
Swedish (6), Danish (7), Polish (12) and Finnish (13). The others (English 0, French 1, German 2, Italian 3,
Spanish 4) are **voice languages**.

```
if language is a text language:
    if the front end may open now (section 3.5, second test):
        if slot(id) exists: slot.state = 2            # no sequence number is given
        push a pause request, then open SMS_Mailboxes.fng
            at slot(id)       (or at the list when id < 0)   # unless it is already open
else:                                                  # voice language
    if id in 10..105:  queue a "CellCall" speech request {1<<g, 1<<line}   # 1.4
    flush the speech queue now
```

So in a voice language a call does **not** touch the inbox; the message exists in the inbox only if it was queued or
delivered through `EAddSMS`. In a text language the call is shown as a message, and a message that cannot open
now is simply lost (the slot stays at 0).

### 3.4 The flush, once per game update **[exe]**

The game manager's update calls the flush after the race-status update. It runs only if the pending list is not
empty and the "can show" test of 3.5 is true:

```
chosen = none
for id in pending list, oldest first:
    slot = slot(id)
    if slot exists:
        if story(id) and chosen is none: chosen = id
        slot.state = 2
        counter = counter + 1; slot.sequence = counter
clear the pending list
if chosen is none and the list was not empty: chosen = the oldest id
if chosen is set:
    open the inbox at chosen  (the EShowSMS request, 4.1)
    tell the HUD engage object (sets a flag on it)
```

All queued messages are delivered together; sequence numbers follow the queue order (oldest smallest, so the newest
shows first). The first story message of the batch is opened; a batch of only tips opens the oldest one.
A message queued while the test is false waits (there is no timeout). **[exe]**

### 3.5 When a message may appear **[exe]**

"Can show" (the flush) requires all of:

1. the global game state is 6 (in game, not front end) and a few disabling globals are zero;
2. the race-status object exists and its state is 0 (free roam, not racing);
3. the front end may open an overlay (the test also refuses when the photo-mode screen or a fade screen is up);
4. the speech event `cellcall` (204) is not queued or playing;
5. the player's car exists, is not wrecked or loading, and a last interface check on the car returns zero (it
   reads as "no pursuit"; the exact name of those calls is **[unconfirmed]**).

The second test used by `ECellCall` in a text language is item 3 only (plus the game state in the language
branch). The list a player sees at a safe house is the same screen opened from the front end.

### 3.6 Side effects of opening **[exe]**

Opening the inbox over gameplay pushes a request that pauses the game, sets the audio flag `eSNDPAUSE_SMS_MESSAGE`
through the screen messages `SMSMesPause` / `SMSMesUnPause`, and hides the autosave overlay's "incoming" widget
(`HIDE_INCOMING` / `SHOW_INCOMING`). There is no HUD "new message" icon in the gameplay HUD; the only indicators are
inside the inbox (section 4.2). **[exe]** (no other reference to the icon names was found)

## 4. The inbox screens

Screens `SMS_Mailboxes.fng` (the list; the same name with another case appears) and `SMS_Message.fng` (one message),
both in the front-end packages. The list screen is also opened from the safe-house menus (the factory table of
career-flow.md). **[exe]**

### 4.1 Building the list **[exe]**

```
items = []
for id from 149 down to 0:
    slot = slot(id)
    if slot.id != 0xFF and (slot.state & 6) != 0:  items.append(slot)    # delivered or read
sort items by sequence, larger first           # equal sequences keep insertion order
tab = the tab of the message the screen was opened at (story or tips); the default is the story tab
for slot in items:
    if tab is the tips tab and story(slot.id) == false: add
    if tab is the story tab and story(slot.id) == true:  add
    if slot.state & 2: mark the entry unread (new icon)
```

Per tab the screen remembers the first listed id as the default selection. Two widgets mark "unread in the
other tab": one if any unread story message exists, one for unread tips. Each list row shows the sender
(`..._FROM`) and the subject (`..._SUBJECT`) of its slot. A row shows `NEW_MESSAGE_ICON_<row>` while unread. Entries
with sequence 0 (calls shown as text) therefore sort last. **[exe]**

### 4.2 The message screen **[exe]**

Shows the body label `SMS_MESSAGE_<id>`, the sender and subject. For non-story messages two widgets of the screen
(the voice caption and its replay hint) are hidden.

### 4.3 Actions **[exe]**

The item handles these screen messages (the names are the pad hashes; the numbers are the observed effects):

| Message | Effect |
|---|---|
| select / open (`0x35F8620B`) | if the message is a story message and still unread: play its voice line; then set `state = 4` (read) |
| pad button 4 (`0xC519BFC3`) | replay: story messages only; stops other speech and plays the voice line of the id |
| pad button 5 (`0xC519BFC4`) | opens a confirmation dialog (`InGameDialog.fng`) |
| the dialog's "yes" (`0xD05FC3A3`) | **delete:** `slot.state = 0` and the row is refreshed |
| screen exit (`EXIT_COMPLETE`, `0xE1FDE1D1`) | unpauses and closes the overlay |

Mapping button 4 to replay and button 5 to delete is read from the effects of the handlers; which physical key
carries each hash was not checked. **[unconfirmed]** A deleted message returns to state 0, so a script that sends
the same id again (a repeat of the trigger) delivers it again.

## 5. Who sends what

### 5.1 Complete list of senders **[verified]**

| Ids | Sent by | How |
|---|---|---|
| 10 | Lua, races `16.1.0` and `16.2.1` (`ECellCall`) | the Mia call at the `call_from_mia` trigger |
| 11, 13 | cutscene `EndingNis03` (the Razor scene of the prologue, event `16.8.1`) | `ECellCall` event in the scene |
| 12 | cutscene `IntroNis10` (intro of `16.1.1`) | `ECellCall` event |
| 15 | Lua, `drive_to_target` start (bin 16) | `EAddSMS(15)` unless the intro is skipped |
| 121 | Lua, `drive_to_target`, 120 s after the drive starts | `EAddSMS(121)` |
| 95 | Lua, final race `1.8.1`, 10 s after the start | `ECellCall` |
| 96 | Lua, `1.8.1`, after 300 s and within 500 m of the barrier area | `ECellCall` |
| 93, 94 | Lua, bin 1 when its rival unlocks | `EAddSMS` |
| 120 | native, after a career race finish | the impound countdown of an impounded car reaches 0 (the car record state becomes 4) |
| bin entry, rival, challenge, race-win messages | Lua per-bin handlers and bin attributes | the table of career-scripts.md 3.3 |
| 17 to 91 and the tips 123 to 137 | the same per-bin handlers | in that table; the ids of the bin attributes are the entry and rival messages |

The cutscene events are in the CARP event lists of the NIS scene bundles (ECellCall hash `0x885F756E`, followed by the id
as a u32). Nothing else sends a message: the only native writer of slot state besides delivery is the read/delete
action of the screen. **[verified]**

### 5.2 Calls and races

`basicrace_racing` has a handler for `MNotifyCellCallStarted`: while a flag `WaitForCallLoad` is set (it is set when the
Mia call of a prologue race is requested) the first start message sets a timer `fade` of 8 s that fades the screen out
and ends the race. In `16.2.1` the call is requested at the `call_from_mia` trigger together with a 15 s "backup"
timer that sabotages the engine unless the kill-engine trigger is reached first.

### 5.3 Never sent in this build **[verified]**

Valid ids that no script, cutscene or native code sends: 16, 21, 32, 47, 50, 74, 97, 98, 105, 110, 111, 122, 129.
Of these 21 ("The regs for racing the Blacklist members") is the same text as 137, which is sent. The others are the two
Rog texts about impound and arrest (97, 98), the second Vic texts (32, 74), 16/47/50, two Earl/Razor texts (110, 111),
the released-from-impound notice (122), the backroom note (105) and the rap-sheet hint (129). Of the rest, 120 and
121 are conditional, which accounts for the 15 valid slots left in state 0 in the late-game save.

### 5.4 Milestone and breaker information messages

Messages 21 to 23 (regs, "some info", milestone info), 26 and 27 (cop info, pursuit breakers) are Blacklist-bin
messages, not milestone-unlock triggers: 22 is sent at bin 14 (3 challenges), 23 at bin 13 (1 race win), 26 at bin 13
(3 challenges), 27 at bin 14 (2 race wins); 21 is never sent. No milestone completion sends a message.

## 6. Save and anomalies

- The slot table and the counter are in the profile (section 1.1). The pending list is saved in the "Game" block
  (section 8.2 of career-blacklist.md) as a count and one u32 per entry. The writer steps its output index by two for
  each entry while the reader takes the entries back to back, so a list with two or more ids is saved with every
  second id lost and zeros between; the list is almost always empty at save time because the flush runs every
  update. **[exe]**
- A text-language game and a voice-language game differ in the inbox contents (section 3.3). The late-game save's
  sequence-0 entries come from a text-language run. **[unconfirmed]** for which language that save used.

## 7. For the rewrite

Implement: the 150-slot table with `{id, state, seq}` and a counter; `add_sms` with the immediate set; a pending
list flushed each update under the can-show rules; the two-tab inbox sorted by sequence with read, replay and
delete; state 4 for read. Voice for calls is optional: a take exists for 75 ids (the group/line table). The
messages that the original never sends should stay unsent.

## 8. Open questions

1. The precise identity of the three player-car checks and the pursuit check in the can-show test (section 3.5).
2. Which physical controls the hashes `0xC519BFC3`, `0xC519BFC4` and `0x35F8620B` stand for on keyboard and pad.
3. The exact SPCH rule that picks the take from the selector pair (`cellcall` sentence rules are not decoded).
4. Whether the late-game save was recorded in a text language (the sequence-0 entries suggest it).
