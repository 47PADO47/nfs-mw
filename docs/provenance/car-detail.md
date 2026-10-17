# Car detail provenance

2026-10-17. Ghidra program identity was checked against the same matched retail
PC executable recorded in [original graphics detail](original-graphics-detail.md).
Read-only inspection covered ride initialization, the normal car render selector
and its two embedded threshold arrays. The PC traffic array is 80/40/20/10/0;
the ordinary array is 120/25/20/10/0. First threshold constants agree with the
arrays; the consumer increments while projected size is strictly below them,
then clamps the index plus minimum to the allowed maximum.
The PC projection helper's raw instructions also confirm its half-radius close
threshold and twice-scale close return; this is separate from scenery projection.

Shared CC0 code was read for understanding and names only:
[CarRender.cpp](https://github.com/dbalatoni13/nfsmw/blob/80993f35224c61d2f1b135beba76c6f8234c7940/src/Speed/Indep/Src/World/CarRender.cpp),
[CarInfo.hpp](https://github.com/dbalatoni13/nfsmw/blob/80993f35224c61d2f1b135beba76c6f8234c7940/src/Speed/Indep/Src/World/CarInfo.hpp).
No decompiled code was copied or translated. The behavior contract preceded
the implementation. Binary identity, raw addresses and retained pseudocode
stay in private research, outside this repository.

The minimum/maximum contract is the ordinary ride setup, not proof of every
render usage. No runtime original/modded transition measurements have been
made for this contribution. Explicit remaining gaps and remake fallback
choices are recorded in [the specification](../specs/car-detail.md).
