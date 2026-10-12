# Weather

- **Spec:** [weather](../specs/weather.md).
- **Sources read for the spec:** the installed original PC executable (`speed.exe`), read in Ghidra
  (decompiled and disassembled) and checked by name-hash scans of the install's data files. No decompiled
  source from another project was read.
- **Implemented:** from the spec only, in `crates/nfsmw/src/weather/` (`mod.rs`, `drops.rs`, `tests.rs`).
  The model is pure: callers pass the frame time, the zone samples and the base roll.
- **Checked against the game by:** unit tests of the script hook, the gate and the squared roll, the easing
  and snapping, the heading wrap, and the drop pool. Nothing has been compared with a running race yet.
- **Known differences from the original:**
  - The drop motion constants (fall speed, spawn box, ground limits, wind scale, spawn rate) are not measured.
    They are parameters (`Tuning`), with no values from the original.
  - The spawn rule (drops per second per unit of rain) is the rewrite's own choice, not taken from the original.
  - The zone map's compare and output fields are modelled as `ZoneSample::compare` and `ZoneSample::value`.
    The original reads two fields and their names are not known.
  - The ramp value (`+0x369c`, `+0x36a0` in the spec) is not implemented; its use is not confirmed.
  - Lightning and thunder are out of scope: none were found in the build.

No game asset is stored in the repository.
