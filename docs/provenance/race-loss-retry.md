# Provenance: the retry after a lost career race

- **Spec:** [race-loss-retry](../specs/race-loss-retry.md).
- **Sources:** the existing specs [story-flow](../specs/story-flow.md) 1.1, 2.1 and 2.4 and [story-flow-2](../specs/story-flow-2.md) F-08, F-09,
  F-16 (the Lua state graphs of `gameplay.bin`, read in earlier passes, and `ShowLosingPostRaceScreen`, a stub at `0x0060e230`).
  Nothing new was read from `speed.exe` for this change; the Ghidra project was reachable but the stub was not re-opened.
- **What is new and ours:** showing the results first and asking for the retry afterwards (the original goes straight from the
  Lua `raceover` wait to its native pause and summary choices), the Offer/Require split by `PostRaceActivity`, and the wording.
- **Not verified:** whether any prologue race besides 16.2.1 and 16.8.1 ends by script; the native retry screen of the original.
