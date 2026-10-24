# Police integration correction provenance

Spec: [police-ai-parity-core](../specs/police-ai-parity-core.md), written before
implementation. Date: 2026-10-23.

Sources are the existing behavior specifications and their recorded original
sources: `ai-pursuit`, `ai-pursuit-formations`, `ai-pursuit-heat`, `ai-helicopter`,
and `cop-siren-audio`. The CC0 reconstruction is credited in NOTICE. No original
code is translated or copied. Height-sheet normal conversion is ordinary plane
geometry over the already documented format, not an extracted asset.

The PC task registration wrapper, constructor and runner were read through Ghidra
after confirming executable identity. The runner confirms fractional update
rates, fixed elapsed delta divided by rate and the variable elapsed-time bank.
It contradicts the unsolved reconstruction's variable-task argument. Private
native evidence remains outside this repository; no decompiled text is included.

The same identity-checked PC executable's cop-manager constructor at
`0x0043c8b0` initializes maximum cars, platform car budget and maximum active
cars to eight; both helicopter limits to one; and the patrol-limit field to
two. Its clamp and development overrides corroborate the reconstruction's
starting limits. This does not prove that a modded executable retains them.

Validation is recorded in the implementation's final report. Synthetic and
installed-data checks do not establish original-game gameplay or listening
parity. Existing uncertain thresholds and helicopter side steering are not made
verified by these adapter corrections. The fixed-timeline replacement for
presentation scheduling remains an intentional deterministic-host difference.
