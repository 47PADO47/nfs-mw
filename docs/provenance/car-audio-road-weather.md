# Road transitions and weather wind provenance

Implemented from [the behavioral specification](../specs/car-audio-road-weather.md)
on 2026-10-14. Source references: original PC 1.3 producers and installed assets,
with CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) behavior/struct
cross-references. No decompiled code or game assets were copied.

- PC comparisons confirmed recreate-on-road-change, the special stitch-road
  collection, transition member layout and base gains. The reference transition
  gains differ from PC; the implementation uses the PC values.
- Weather slope initialization and producer processing were checked with four
  isolated original-instruction cases at 0, 35, 70 and 100 mph, without mocked
  callees. Width 4000 and intensity zero were observed.
- Installed metadata confirms the ordinary road, transition and weather modules
  have supported instructions, player kinds and member counts.
- Synthetic and installed tests are supplied; their execution is delegated to
  the serialized workspace validation owner. No new build/test pass is claimed
  in this record before that validation completes.
- Original full numeric graph replay, with bounded player/destroy host callbacks,
  confirms initialized gravel retirement and the release-before-first-update
  constructor edge. Selected graphs retain their own constructor parameters;
  unticked graphs with no host resources are discarded as a caller adaptation.
  No interpreter arithmetic change or guessed release timeout was introduced.
- Surface transition metadata, tyre damage edges and weather mixer input require
  actual game context. Missing context stays explicit. Original random sequence,
  variable audio-period equality and environmental reverb remain unverified.
