# Speech sentences: `copspeech.evt`, the bank selectors of `copspeech.idx` and `copspeech.csi`

How the police radio's *sentences* are stored: the event database `copspeech.evt` (one record per event, listing the
phrases it says and which takes of each phrase fit which parameter values), the *selectors* that label every take in
`copspeech.idx`, and the interface list `copspeech.csi`. The rest of the speech files are in
[audio.md § Speech](audio.md#speech-and-nis-streams-big--idx--evt--csi); how the pieces are used to build a line is the
spec [specs/speech-sentences.md](../specs/speech-sentences.md).

- **Sources:** the install's own `SOUND/SPEECH/copspeech.{evt,idx,csi}`, read as bytes; the CC0 decompilation
  ([dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)) `EAXSound/SND_GEN/COPSPEECH.{hpp,cpp}` for the interface and
  parameter names. Which byte means what was checked against the sentence-building routines of `speed.exe` (read in
  Ghidra for understanding; nothing copied). Provenance: [provenance/speech.md](../provenance/speech.md).
- **Reader:** `libs/ea-audio/src/speech/` (`sentence.rs`, `evt.rs`, `bank.rs`). Evidence tags as in the
  [docs README](../README.md#evidence-tags).

## `copspeech.evt` **[verified]**

```
03 12 3C 07   u32 0x1DF8   u32 0   u32 0xC14705FD   u32 count (139)   u16 100   u16 500
count x u16   record position / 4, ascending, from 0x18
records, 4-byte aligned, from 0x130; after the last record (at 0x1DD4, whose single block ends at 0x1DF8 = the u32 above)
the file goes on to 0x3CBB with a table of 32-byte entries (see below)
```

The first 0x18 bytes other than the count are constants in this file (a hash among them); the game does not use them.
The positions ascend but the event numbers do not: a lookup scans the table for the record whose event number matches.

### The table after the records **[unconfirmed]**

From 0x1DF8 the file holds entries of 32 bytes: a u32 that grows (0x0C, 0x1A, 0x2E, 0x41, ...), the system CRC `A7 5B`, the
CRC of a CSIS interface (`86 2E` is `Acknowledge`), and 20 zero bytes. It looks like the link from an interface to its
record. The rewrite does not read it: a request names its event by the speech database's `SpeechID`, which is the record's
event number.

### Record

```
+0   u16 event number (57 to 263; all 139 distinct)
+2   u16 200   u16 500            same in every record, unused
+6   u8 block count (1 or 2)      +7 u8 0 (the count of sentence rules; none in this file)
+8   u8 0 (low nibble: repeat limit, high nibble: parameter constraint count; both 0 in this file)
+9   u8 100                        unused by the sentence builder
+10  u8 0 (record flags)           +11 u8 number of arguments of the event (see below)
+12  u16 per block: position of the block / 4 from the start of the record
...  blocks
```

The argument count (`+11`) is the number of fields of the event's call in the game's speech interface, including the
speaker: `Setup_AttmptVehStp` (speaker, pursuit type, number of suspects, battalion, call sign) has 5, `Acknowledge` 3,
`CellCall` 2. It matches the interface in the 130 records whose event the attribute database names (the other 9 have no
`speech` collection).

### Block: one version of the sentence

```
+0   u8 weight code            0x39 in 145 blocks, 0 in 2 (both blocks of event 77: they are tried in file order)
+1   u8 chance in percent      100 in every block
+2   u8 entries << 2           (the two low bits are a mode, 0 here)
+3   u8 0                      number of rule variables, 0 here
+4   u32 0
+8   u8 x n   entry positions: where each entry starts, in 4-byte words from the start of the block; padded with 0 to
              a multiple of 4. The first entry is at word 2 + ceil(n / 4)
...  entries, one after another
```

`weight = (code & 0x1F) * [1, 4, 16, 64][code >> 5]`: 0x39 is 25 * 4 = 100. Records with two blocks are two wordings of the
same line (the vehicle report has the colours 0 to 5 in one block and 6 to 12 in the other, see
[the spec](../specs/speech-sentences.md#3-the-versions)). The old notes called the block's `u16` after `39 64` a length;
it is `4 * n`.

### Entry: one phrase

```
+0   u16 phrase           the event number of a bank in copspeech.idx (a phrase is a bank, never another record)
+2   u8  voice parameter  the number of the argument that holds the speaker (always 1)
+3   u8  voice kind       1: the bank of that speaker (165 entries);  0: any bank of the phrase (58 entries)
+4   u32 test count       equals the selector count of the phrase's banks, in all 223 entries
+8   tests, 8 bytes each: u32 mask, u8 argument number, FF FF 00
```

The pair `01 01` that earlier notes called a "said" flag is *voice parameter 1, voice kind 1*; `00 00` is *any voice*.
Phrases for any voice (`speaker 0xFFFF` in the index) are small pieces shared by all cops (a radio click, a pause, a
call-sign number); phrases for a voice exist for the speakers who recorded them, which is why dispatch lines (speaker 1),
helicopter lines (2) and the cross-car lines (9) have one speaker each. Kind 2 (a group of speakers) is read by the game but
never used in this file; the reader rejects it.

A test says: *the take's nth selector must be one of the bits of `mask`, and set in the value of the request's argument
`number`* (argument 0: only the mask). The argument numbers are the position in the event's call, starting at 1 for the
first argument (the speaker, when the event has one). Numbers 0 to 9 occur. Tag values 0xFE and 0xFF would mean a rule
variable; they do not occur.

**Masks** **[verified]**: of the 175 tests in the file, 163 have the mask of *all* the choices of the parameter they test
(`ff 1f 00 00` for the 13 car colours, `ff ff ff 7f` for the 31 car types, `ff 0f` for the 12 speeds, `07` for the three
measurement systems, ...) and 12 a part of them (yes/no, intensity, the five pursuit types split 0xF / 0x10,
the disp-backup types 0x7E / 0x01): the other version of the sentence takes the rest. This is how the argument numbers were
tied to the interface list: every test's mask is a subset of the choices of the parameter at that position.

### Example: the vehicle report (event 155)

```
header     9B 00 C8 00 F4 01 02 00 00 64 00 05 04 00 19 00
block 0    39 64 10 00 00 00 00 00 | 03 09 0B 0F                         4 entries at words 3, 9, 11, 15
 entry 0   55 01 01 01  02 00 00 00  FF 1F 00 00 02 FF FF 00  FF FF FF 7F 03 FF FF 00
           phrase 0x155, speaker voice, 2 tests: colour (argument 2, 13 choices), car type (3, 31 choices)
 entry 1   AB 01 00 00  00 00 00 00                                       phrase 0x1AB for any voice
 entry 2   09 01 00 00  01 00 00 00  01 00 00 00 00 FF FF 00              phrase 0x109 for any voice, mask 1, no argument
 entry 3   E3 00 01 01  02 00 00 00  07 00 00 00 05 FF FF 00  FF 0F 00 00 04 FF FF 00
           phrase 0xE3, speaker voice, tests: measurement (5), speed (4)
block 1    the same with phrase 0x156 in entry 0 (colours 6 to 12)
```

## The selectors in `copspeech.idx` **[verified]**

A bank header (layout in [audio.md](audio.md#idx-the-banks-verified-on-copspeechidx-read-by-ea-audiospeech)) is: event,
speaker, **selector count** `s` (the byte earlier notes called `flags`, 0 to 3), take count `n`, **repeat memory**,
alignment, length, padding, then the take table, then masks and the repeat ring:

| What | Where |
|---|---|
| take table | from byte 14, `n` entries of `s + 2` bytes: the take's `s` selector bytes, then the **next** take's start (big-endian u16, in alignment units); the last entry has only its selector bytes (the earlier notes had `n - 1` entries and counted the last take's selectors as "extra") |
| masks | at `((s + 2) * n + 15) & ~3`: one u32 per selector, the OR of `1 << selector` over all takes (checked on all 579 banks) |
| repeat ring | after the masks: `memory + 1` bytes the game fills while playing, to avoid saying a take that was said among the last `memory` |

A **selector** is a bit index (0 to 31): "this take is the one for the choice `1 << selector` of the nth tested parameter".
Bank 425 (the cell calls) has 80 takes with selectors (bucket 0 to 16, call 0 to 6); bank 227 (the speed) has 30 takes with
(measurement 0 to 2, speed 0 to 8). Several takes may have the same selectors (the actor recorded the phrase twice). The
repeat memory is 0 for 168 banks and up to 100 for the large ones; a bank with memory equal to its take count never repeats
a take until all were said.

## `copspeech.csi` **[verified]**

5,136 bytes. It is the interface list of the CSIS library, and carries no parameter types:

```
"MOIR" 00 02 00 01    u16 count 0x8B (139) at +10    u16 0x5BA7 (the system CRC) at +16    zeros
139 x 12 bytes from 0x2C: u32 position of the name from the start of the file, u32 CRC of the interface, u32 0
the names (NUL-terminated), from 0x6AC to the end
```

The entries are sorted by CRC (`Acknowledge` is 0x2E86, `Setup_SpotterReply` 0x0369, as in `COPSPEECH.cpp`), and so are the
names in the string pool. The parameter
types of every event (`Type_pursuit_type`, `Type_car_color`, ...) and the order of the arguments are *not* in this file:
they are the structures of the generated header `COPSPEECH.hpp` **[decomp]**, compiled into the game. The rewrite keeps
the same list as a table (`crates/nfsmw/src/audio/speech/params/signatures.rs`), and the installed tests check it against
the masks of the sentences.
