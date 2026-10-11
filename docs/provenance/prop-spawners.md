# Prop placement and attributes

- **Specs:** [format](../formats/prop-spawners.md), [attributes](../specs/prop-attributes.md).
- **Sources read:** the user's PC 1.3 executable and install; the CC0 reference
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), physics and scenery
  record layouts. Full version/hash details are in the format document.
- **Implemented:** 2026-10-11, from the written specification. No decompiled code,
  executable bytes or extracted asset records are included.
- **Checked against the game by:** inspecting all 366 placement packs and joining
  all 13,484 records to scenery hooks and attributes. Positions match exactly.
  Synthetic tests cover alignment, fields and invalid record counts. The ignored
  install test repeats the complete mapping and reports missing bounds explicitly.
- **Limits:** this verifies serialized data, not runtime object lifecycle,
  fragmentation, attachments or full gameplay parity.
