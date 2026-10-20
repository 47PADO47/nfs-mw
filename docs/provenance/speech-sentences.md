# Speech sentences

- **Spec:** [specs/speech-sentences.md](../specs/speech-sentences.md), [formats/speech-evt.md](../formats/speech-evt.md)
  (extends [speech.md](speech.md), which covers the queue and the bank index).
- **Restricted sources read:**
  - dbalatoni13/nfsmw (decompilation, CC0-1.0): `src/Speed/Indep/Src/EAXSound/SND_GEN/COPSPEECH.hpp` and `.cpp` (the call
    structure of each event, the choices of each parameter type, the interface names and CRCs), `EAXSound/Stream/SpeechManager.cpp`
    (a request copies its structure into the speech library, which then chooses and plays; the game's rule callbacks do
    nothing), `Libs/spch/dev/include/spch/spch.h` and `Libs/csis/dev/include/csis/csis.h` (headers only; the libraries'
    sources are not in the decomp). Read for understanding; no code copied.
  - The original PC executable `speed.exe` 1.3, in Ghidra: the speech library (SPCH, build string 3.18) routines that find a
    record, order its versions, check an entry's tests against a take's selectors, collect the takes that fit, avoid repeats,
    and pick one. Read for behaviour only, to settle what the bytes of `copspeech.evt` and the bank headers mean where the
    data alone was ambiguous (the weight code, the voice kind, the use of the selector bytes, the repeat ring). No
    decompiler output, function names or addresses are in the repository, and the rewrite was written from the spec.
  - The install's own `SOUND/SPEECH/copspeech.evt`, `.idx`, `.csi`, as bytes.
- **Implemented:** 2026-10-20, from the spec: `libs/ea-audio/src/speech/{sentence,evt,assemble,params,bank}.rs` (generic: no
  game names), `crates/nfsmw/src/audio/speech/{lines.rs,params/}` and the request path (`Request`, `Voice`, `Speaker`,
  `Audio::say_with`).
- **Checked against the game by:** the layout was derived from the bytes first (every record parses to its exact end; every
  entry's phrase has banks; each entry's test count equals the selector count of its banks; each bank's masks equal the OR of
  its selectors) and then matched with the library's behaviour. Installed tests: `libs/ea-audio/tests/real_speech.rs`,
  `crates/nfsmw/src/audio/speech/tests/sentences.rs` (see the spec's "How to check it"). In particular the argument order of
  the interface (from the CC0 header) was confirmed independently by the masks of the data: 163 of 175 tests equal the
  full choice set of the parameter at that position and the other 12 are subsets. No comparison with the running original,
  and nobody has listened to the output.
- **Known differences from the original:** an unset parameter lets every variant through (the original has no unset
  parameters: a zero field fits nothing); another voice says a sentence the requested speaker did not record
  (`Options::speaker_fallback`; off gives the original's behaviour); the random choices use the game's own generator, not
  the library's short-term pick memory; the library's rule variables, speaker groups, record constraints and sample-cache
  length mode, which the data never uses, are not read.
