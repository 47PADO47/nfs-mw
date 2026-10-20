# Petrol-station explosion audio provenance

The matched original PC executable's MomentStrm initializer associates the
gas-station marker with `aud_moment_strm/expl_gas_station`. Its moment recipient
and NIS stream controller select SoundFX recordings, collection volume slots,
positioned azimuth and same-current retrigger behavior. The CC0
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference names these
structures in `SFXObj_MomentStrm.cpp`, `NISSFXModule.cpp` and `NISAudio.hpp`;
it is already credited in NOTICE. The prior claim that the explosion recording
required decoding the generic collision CSI events was incorrect.

Implementation follows [the behavioral spec](../specs/petrol-station-explosion.md),
reusing existing installed AttribSys, speech-index and PCM readers. All audio
and sample selectors remain in the user's installation, loaded at runtime.
No original implementation, extracted data tables or recordings are included.

Static analysis establishes the event path. Installed replay tests establish
selection, decoded sound, mixer gain and lifecycle. Full native gameplay PCM,
moment-stream priority arbitration and backend spatial DSP equivalence remain
unverified; do not describe those as completed parity.
