# Performance tuning

- **Spec:** [performance-tuning](../specs/performance-tuning.md).
- **Sources:** original PC 1.3 executable, proprietary behavior reference;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
  `PhysicsTunings.h`, `CustomTuning.cpp/.hpp` and `PhysicsUpgrades.hpp`;
  language titles and help read from the user's installed copy at research time.
- **Executable identity:** SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **Evidence:** Ghidra program identity checked before querying. The PC limit
  functions reference fourteen alternating -1/+1 floats. Slider initialization
  multiplies the range by 0.1; display maps the range to 0..10. The frontend
  availability switch checks ordinary installed package levels and compares
  body/spoiler identity with stock. The installed help confirms directions.
  The PC vehicle getter returns the active profile without masking values by
  upgrade availability, matching the reference customization record's separate
  package and tuning-profile storage.
  Isolated execution of original machine code covered seven limit pairs,
  eleven nitrous positions, 66 induction vectors, three active-profile getters
  and three shift-map builds. Different induction strengths retained identical
  naturally aspirated thresholds, confirming reversed RPM/spool argument order
  and untuned cache setup.
- **Implementation rule:** implement from the written spec; no game assets,
  executable bytes or decompiled expression belong in the public source tree.
- **Implemented:** three session-local tuning profiles and active-profile
  selection in the runtime overlay; upgrade removal retains the stored values.
- **Limits:** static PC inspection is not a live driving comparison. Career-save
  persistence remains separate frontend work. Cosmetic-part selection is now
  available in the session-local overlay. Consumer arithmetic coverage is separate from complete
  driving-model parity.
