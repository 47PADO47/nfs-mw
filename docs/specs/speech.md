# Speech

The police dispatch's radio chatter: what asks for a line, when a line may be said, how lines are kept from
overlapping and where the recordings are. The rules of *when* are written from the decomp; the *what* (which
recordings make up a sentence) is only partly known and is marked as such.

- **Sources read:** dbalatoni13/nfsmw (decompilation, CC0-1.0): `EAXSound/Stream/SpeechManager.cpp/.hpp`,
  `SpeechModule.hpp`, `GameSpeech.cpp`, `EAXSound/SND_GEN/COPSPEECH.cpp` (event names only), the generated AttribSys
  class header `speech`, and the header-only `spch.h`, `csis.h`. The install's `copspeech.idx/.evt/.csi/.big` and
  `ATTRIBUTES.BIN`, measured. Provenance: [provenance/speech.md](../provenance/speech.md).
- **Data inputs:** AttribSys class `speech` (133 collections, 28 fields) and `speechtune` (1 collection, field
  `SpeechDropoffRamp`); the recordings in [formats/audio.md § Speech](../formats/audio.md#speech-and-nis-streams-big--idx--evt--csi).

## 1. Shape of the system

Gameplay code (the police AI, `SoundAI` and its flows for pursuit, roadblocks, backup, air support) calls
`ScheduleSpeech` with an *event* and the unit that speaks. There are 139 events in `copspeech.evt`, each a function of
the CSIS library with typed parameters (colors, directions, unit counts). The `speech` class holds scheduling data
for 130 of them (the other collections are the template `default`, a second collection numbered 0 and a repeat of
98), keyed by the lowercased event name (`staticroadblock_callforrb`). An event's `SpeechID` is its number in the
event database; nine more event numbers (81, 82, 120, 144, 197, 211, 242, 255, 256) have no collection and are only
parts of sentences.

The speech library (SPCH) turns an event and its parameters into a *sentence*: a run of recorded phrases, each a
stream of `copspeech.big`. The speech manager plays the streams back to back on one voice, so only one line sounds
at a time. A second module plays the cut-scene audio (`NISAudio.big`) and is not covered here.

## 2. Requests

`ScheduleSpeech(event, data, actor)`, in this order:

1. Refused (returns nothing) when speech is off, when the speech volume is zero, in split-screen mode, or when the
   event pool is full.
2. An event that is already waiting is not queued twice; its entry time is refreshed.
3. Unless the event has `DoNotDropout`: if the event has been said before, the request is kept with the probability
   `p` of the pursuit (below) and dropped otherwise. An event never said is always kept.
4. Every event of the `RecallList` that is waiting is withdrawn.
5. The request is queued with the priority `priority`, plus 100 when `interrupt` is set; it is *delayed* when
   `InitDelay` is above 0.

`p` is 1 while the pursuit is younger than the first value of `SpeechDropoffRamp` (30 s), 0 once it is older than the
second (500 s), and falls linearly in between.

## 3. Whether a request may be said now

Each frame every waiting request is judged. In the original's order (each check is a pass or a postponement, except
the two marked *drop*):

1. *Delay*: wait until `InitDelay` has passed since the request; the request's clock starts again when it ends.
2. *Expiry* (drop): the request is older than `expiry` seconds.
3. *Limit* (drop): the event has been said more than `MaxPlayback` times (negative: no limit). The original compares
   with `>`, so a limit of 1 lets the event through twice.
4. *Dead air*: `DeadAir` is above 0 and fewer seconds than that have passed since the last line ended.
5. *Interval*: the event was said less than `Interval` seconds ago.
6. *Range*: the speaking unit is farther than `CullingRange` meters. Requests without a unit skip this and the
   next two checks.
7. *Follows*: `DepFollow` lists events. With `BackTime` above 0, one of them must have been said within that many
   seconds, or be playing. With `BackTime` 0, the last event said (of the last ten, kept in order) must be one of them,
   or one of them must be playing; with nothing said yet, the check fails.
8. *On screen*: `OnScreenOnly`, and the unit is not drawn.
9. *Line of sight*: `reqLOS`, and the unit cannot see the player.
10. *Heat*: the pursuit's heat is outside `MinHeat` to `MaxHeat`.
11. *Speed*: the player's speed in mph, without sign, is outside `MinPlayerSpeed` to `MaxPlayerSpeed`.

## 4. Playing a line

Four lists carry a request from "asked" to "sounding": new requests, those that passed the checks, those whose
recordings are being fetched, and interrupts. The speech manager's `Deduce` runs them every frame. What matters for
behaviour:

- **Order.** Interrupting events (priority above 100) go before everything else; the rest by priority. (The
  original's sort comparators are not in the decomp's readable parts; equal priorities are taken in order of
  arrival here.)
- **One voice.** A normal request starts only when the voice is idle. After a line ends the event's
  `EnforceDeadAir` seconds of silence must pass before the next normal line starts.
- **Interrupts.** An interrupt does not wait. It may cut the line in progress when that line's event is
  `Interruptable`, or when the line is itself an interrupt of lower priority; otherwise it waits (and expires like any
  request). An interrupt also ignores the enforced silence of the line that just ended.
- **History.** A line counts as said (for the interval, the limit, `DepFollow` and the thinning) when it has sounded
  for 3 seconds or when it ends, whichever comes first.
- **Dead air** is the time since a line last sounded; it is 0 while one does and large before the first.
- A request whose recordings cannot be found is dropped.
- The original also sets, per line: the radio click before and after (`RadioChirp`), the filter strength
  (`Clarity`, a low-pass), panning to the unit (`Pan`) and a per-speaker volume read from the dynamic mixer.

## 5. Recordings

`copspeech.idx` is a table of 579 *banks*; a bank holds the takes of one phrase by one speaker (voices 1 to 9, or
`0xFFFF` for any), 13,562 takes in all, each a stream of the `.big`. A phrase is named by an event number. Of the 139
events, 28 have banks of their own (`cellcall`, `arrest_disparrestreply`, `anytimeevents_bailoutdeny`,
`anytimeevents_weatherreport`, ...); those can be said as they are by choosing a bank of the requested speaker (any
bank when that speaker did not record it) and a take that was not said lately. The other events are *sentences*:
their records in `copspeech.evt` list the phrases (other event numbers, with banks of their own) and rules on the
parameters, which choose among variants. That rule language is not decoded: see the format notes.
The SPCH library's rule for avoiding repeated takes is not in the decomp (header only).

## 6. This rewrite

Implemented (`crates/nfsmw/src/audio/speech/`): the request, the checks of section 3, the order and the single voice
of section 4 (as one queue, not four lists), the thinning, the history, the speech volume, recordings of the 28 events
with banks of their own. Not implemented: sentences built from several phrases, the radio click, the filter, panning,
the per-speaker volumes, the cut-scene module, the track-streamer gating of the original, and every gameplay trigger
(there is no pursuit yet: `Audio::say` is the entry point a pursuit would call, `speech say <event>` the console's).
`speech_volume` (the original's *Speech Volume* row, `0x9E5FB82A`) is a setting and a row of the audio options.

## 7. How to check it

- Play `speech play anytimeevents_weatherreport` and compare with the original's weather report line.
- In the original, trigger a pursuit and note which lines overlap or cut each other; the rules above say none
  overlap except an interrupt cutting an interruptable line.
- Run `speech say <a, b>` for two events in quick succession and watch `speech status`.
