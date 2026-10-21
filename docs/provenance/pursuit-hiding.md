# Pursuit hiding spots

- **Spec:** [pursuit hiding](../specs/pursuit-hiding.md); data format [pursuit data](../formats/pursuit-data.md)
  section 5.
- **Sources:** the installed `L2RA.BUN` track path zones and the authored `HUD_SingleRace.fng` package; the CC0
  dbalatoni13/nfsmw reconstruction of the perpetrator vehicle update, the pursuit update and the pursuit level
  attributes, read for behaviour only. No original code, assets or bytes are included.
- **Measured behavior:** the hiding latch (quick 0.05 s latch when the player is not seen on entry, an effectively
  infinite one when it is, timer counted once per update, strict comparison), the feet height, the zero-elevation
  "any height" rule, the scope by first data word, the "how hidden" fill and the evade multiplier for either flag are
  read from the reconstruction. The HIDDEN zones (34, all with elevation and no data) and the authored HIDING objects
  with their message-driven fade scripts are read from the install. Elevation agrees with the road surface where a road
  crosses a zone (installed test).
- **Implementation:** 2026-10-20, from the specification.
- **Limits:** the original feeder of the HUD mark is not in the public sources; binding the latch fill to the
  authored fade scripts is an inference. No live-play or rendered comparison with the original has been made.
