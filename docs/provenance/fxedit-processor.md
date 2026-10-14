# FXEDIT reader and software processor

- Date: 2026-10-14.
- Spec: [fxedit-processor.md](../specs/fxedit-processor.md).
- Implementation: `libs/blackbox-fxedit`.
- Sources: matching original PC executable sound-library constructor,
  eight installed processor types, full math helper instruction ranges,
  connection/mix/reset routines; reference
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  `SFXObj_Reverb` and game patch selection.
- Specification written first; new implementation follows the behavioral
  contract. No decompiled code, original tables, installed coefficients,
  patches, audio or binaries copied into public source.
- Independent verification runs complete original patch constructors and
  processor graphs in isolated memory with declared allocator/free stubs.
  Parsed structure, original coefficients and PCM evidence stay private.
- Numeric limits: f64 reconstruction approximates PC x87 extended arithmetic;
  explicit regressions bound differences. Initial runtime supports mono/stereo
  and16..256 samples in multiples of16. Live patch activation, host send/group
  ordering and final output conversion need separate evidence.
