# Speech sentences: which recordings say a line, for which parameters

How the police radio turns an *event* and the *parameters* of a request ("the suspect is a red Porsche heading east at
140") into the recordings it plays. [speech.md](speech.md) says when a request may be said; this says what it says.
The data layouts are in [formats/speech-evt.md](../formats/speech-evt.md).

- **Sources:** the install's `copspeech.evt`, `copspeech.idx` and `copspeech.csi` (bytes, measured); the CC0 decompilation
  ([dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)) `EAXSound/SND_GEN/COPSPEECH.{hpp,cpp}` (the call structure of
  every event and the values of every parameter), `EAXSound/Stream/SpeechManager.cpp` (how a request reaches the speech
  library: the parameters are copied from the call structure, then the library chooses and plays) and `Libs/spch`
  (header only: the library's source is not in the decomp). The library's own choice rules were read in `speed.exe` with
  Ghidra, for behaviour only; nothing was copied. Provenance: [provenance/speech.md](../provenance/speech.md).
- **Tags:** **[verified]** measured on the install; **[decomp]** the CC0 decompilation; **[speed.exe]** read in the
  executable's speech library, then confirmed against the data where a check exists; **[unconfirmed]** not checked.
  Nobody compared the rewrite's output with the running original.

## 1. The request

A request is an **event** (its number in `copspeech.evt`: the `SpeechID` of the `speech` class), a **speaker** (voice 1 to
9, see [ai-pursuit-speech.md](ai-pursuit-speech.md#1-structure)) and the **arguments** of the event's call in the game's
speech interface. The interface fixes, for each of the 139 events, an ordered list of typed arguments; for most of the
events the first is the speaker. The type of an argument names a set of choices, each worth one bit
(`Type_car_color`: red 1, black 2, white 4, ... beige 4096), and a call passes one choice (or, rarely, several or none).
The argument list and values are in `crates/nfsmw/src/audio/speech/params/` (section 6). **[decomp]**

Arguments are numbered from 1 in call order, the speaker first when the event has one. So for `Setup_AttmptVehStp`:
1 speaker, 2 pursuit type, 3 number of suspects, 4 speaker battalion, 5 speaker call sign. The library receives the
whole structure and keeps the values by this number. **[speed.exe]** **[verified]**: the tests of the sentences number the
arguments this way (events with no speaker, such as the cell call and the roadblock position, test argument 1 on; events with a speaker
test 2 and above), and the mask
of every test is a subset of the choices of the parameter at that position, for all 175 tests of the file.

## 2. Choosing a sentence version to say

The event's record lists one or two **versions** (blocks). **[speed.exe]**

1. Versions with a weight above 0 are put in a random order, each pick weighted by the versions' weights (the weight
   code `0x39` is 100, so equal weights give an even chance); versions of weight 0 follow in file order.
2. The versions are tried in that order. A version is considered with the probability `chance` percent (100 in this file),
   and is skipped if it has no entries.
3. The first version for which every entry can be said (section 3) is the sentence. If none can, **nothing is said** (the
   request fails; the speech manager drops it).

**Two versions are not two wordings at random.** In the 8 records with two versions the second often covers the choices
the first lacks: the vehicle report (event 155) has the colours red to green in one version (phrase `0x155`, takes with
colour 0 to 5) and the rest in the other (`0x156`, colours 6 to 12). A request for a blue car fails the first version
(no take) and is said by the second. **[verified]**

## 3. Building a version: one take per entry

For each entry of the version, in order: **[speed.exe]**

1. **Find the bank.** The phrase is the event number of a bank in `copspeech.idx`.
   - voice kind 1: the bank whose speaker equals the value of the entry's voice argument (the speaker). If the speaker
     recorded no bank of that phrase, the version fails.
   - voice kind 0: any bank of the phrase (in this file the phrase has one bank, for speaker `0xFFFF`).
2. **Find the fitting takes.** A take fits when, for the nth test of the entry and the nth selector `b` of the take (a bit
   index): `b` is below 32, `mask` has bit `b`, and (argument number not 0) the request's argument has bit `b`. A bank
   whose takes have fewer selectors than the entry has tests fits nothing. (The installed data always has equal counts.)
3. **Avoid repeats.** The bank remembers the last `memory` takes it said (header byte 6), newest first. Takes of the fitting
   set that are not among them are *fresh*. If there are fresh takes, one is chosen at random; if all fitting takes were said
   lately, the one said longest ago is chosen (the first of them if several). If nothing fits, the version fails.
4. **Remember.** When a version is complete, each of its takes is pushed into its bank's memory.

The sentence is the chosen takes joined in entry order. The entries between the spoken ones (phrases for any voice, with no
test or a test on no argument: `0x109`, `0x10D`, `0x1AB`, ...) are small recordings such as the radio click, pauses and
call-sign numbers; they are said like any other. **[verified]**: every phrase of every entry has banks (223 of 223), and
the number of tests of each entry equals the selector count of each of its banks.

### An example

`Setup_VehicleReport` for a red (bit 0) Porsche (bit 1) at 140 mph ("in excess of 140", bit 3), imperial (bit 1), by cop 3:

| Entry | Phrase | Voice | Tests | Takes that fit |
|---|---|---|---|---|
| 0 | `0x155` | speaker 3 | colour (arg 2): selector 0; car type (arg 3): selector 1 | the takes of bank (`0x155`, speaker 3) labelled (0, 1): the cop saying "red Porsche" |
| 1 | `0x1AB` | any | none | any of the 10 takes of that bank |
| 2 | `0x109` | any | mask 1, no argument | the takes labelled 0 |
| 3 | `0xE3` | speaker 3 | measurement (5): selector 1, speed (4): selector 3 | "in excess of 140 miles per hour" |

A silver car (bit 6) fails version 0 at entry 0 (its takes are labelled with colours 0 to 5) and is said by version 1, whose
phrase `0x156` has the colours 6 to 12. A car of type SUV (bit 21) or pickup (22) fails both: nobody recorded them.

## 4. What the original does when it cannot say a line

The speech manager asks the library to choose; when no version works, or the speaker has no recording, the request is
dropped (`ERR_NOTFOUND`) and nothing sounds. The original also has a second chance in the library: a caller-supplied
"re-parameter" function that may change the arguments and try once more; the game's version does nothing. **[speed.exe]**
A parameter field the caller leaves at 0 selects no choice (no recording fits), so a half-filled structure silences the
line; the game fills every field. **[speed.exe]**

## 5. What the rewrite does differently

| Case | Original | Rewrite | Why |
|---|---|---|---|
| A parameter not set in the request | its field is 0: no take fits | **any** take fits (the test is skipped) | callers that do not know a parameter yet (the first pursuit flows) still get a line, in a random variant |
| A parameter set to 0 | no take fits | no take fits | same as the original |
| The speaker did not record the sentence, or no speaker was asked for | the request fails | another voice that recorded every phrase of the version says it (`Options::speaker_fallback`, on by default; off gives the original) | the rewrite's flows do not know who recorded what; a silent request is worse than another voice |
| Randomness | the library's generator with a short-term memory of picks | the game's seeded `Rng`; repeats are avoided by the banks' memory only | not observable except as a different sequence |
| Event number with banks but no record | not a speech event | said as one bank of the requested speaker (the console's `speech play`) | keeps the tooling working |

## 6. Parameters in the rewrite

`ea_audio::speech::Params` holds the numbered values of one request; it knows no names. The game names them in
`crates/nfsmw/src/audio/speech/params/`: `Param` (47 parameters named as in the speech interface: `pursuit_type`,
`num_suspects`, `speaker_battalion`, `speaker_call_sign_id`, `car_type`, `car_color`, `speed`, `measurement`, `direction`,
`location_region`, `location`, `encounter`, `code`, `backup_type`, `disp_backup_eta`, `self_strategy_type`, `intensity`,
`world_object_type`, `heli_hazard_alert_type`, `heat_level`, ...), `values` (the bit of every choice), `signature(event)` (the
argument list of each event by its collection name, such as `setup_vehiclereport`) and `SpeechParams` (what a request sets).
`lib_params` puts the speaker and the named values at their argument numbers. **[verified]** by an installed test: for every
event of the attribute database that has a sentence, every parameter test's mask lies inside the choices of the parameter
the interface puts at that number.

How a pursuit flow derives each value (the speed in steps of 20 from 100, the direction from the heading, the colour from the
paint, the call sign from the battalion's pool) is in [ai-pursuit-speech.md §4](ai-pursuit-speech.md#4-requesting-a-line); the
helpers `speed_for`, `call_sign`, `location` and `heat_level` do the bit arithmetic.

## 7. Not read, because the data never uses it

The library supports more than this file uses; the reader rejects what it cannot interpret and ignores the rest.
**[speed.exe]** **[verified: absent]**

- *Rule variables* (tests with argument number `0xFE`/`0xFF`, block variable count above 0): entries whose choice of take
  depends on the take chosen for an earlier entry. None occur. The reader refuses such a test.
- *Voice kind 2* (a group of speakers) and the block mode bits: none occur; the reader refuses kind 2.
- *Sentence rules* and *argument constraints* on a record (counts at `+7` and the high nibble of `+8`): all 0.
- *Event repeat limit* (low nibble of `+8`): 0.
- A *length budget* mode of the speech manager that prefers shorter sentences when the sample cache is short: set by the
  game's configuration, not by the data. The rewrite has no cache pressure.

## 8. How to check it

- `cargo test --release -p ea-audio --test real_speech -- --ignored` (library): 139 sentences, every entry has banks and as
  many tests as selectors, about 8,870 of 10,008 random requests have a version that works (the others ask for a take nobody
  recorded, such as an SUV in the vehicle report), and 21,137 of 21,162 takes are found again from their own parameters.
- `cargo test --release -p nfsmw speech::tests::sentences -- --ignored` (game): the argument lists agree with the
  sentences; for 18 pursuit events the value of a parameter selects exactly the take recorded for it; 1,377 sentences of
  random parameters decode to audio of up to 29 s (the cell calls) with at most one take per entry.
- In the console: `speech play setup_vehiclereport 3 car_color=1 car_type=2 speed=8 measurement=2`, then change
  `car_color=2` and listen.

## Open questions

- The library's pick-with-short-memory random number generator was not reproduced; the rewrite's choice differs in sequence
  only. **[unconfirmed]**
- The table after the records in `copspeech.evt` (section "The table after the records" of the format note) is not read.
- `collworld_spin` has the same `SpeechID` (98) as `collisionworld` in the attribute database and the file has one record 98,
  which tests the world-object-type argument of `collisionworld`: a `collworld_spin` request (intensity) would be matched against
  the wrong choices. The file has no record for it; nobody asks for it yet.
