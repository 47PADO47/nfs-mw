# World event sequences

- **Specs:** [format](../formats/event-sequences.md),
  [state machines and fragments](../specs/world-sequencers.md).
- **Sources read:** the user's original PC 1.3 executable and install;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0 reference),
  CARP/UGroup layouts, event headers, EventSequencer, DamageVehicle, DrawVehicle,
  DrawCar, SimModel and CarPartNames. The executable hash and reference revision
  are in the format document. No original code or event bytecode is included.
- **Implemented:** 2026-10-11, from the written specifications.
- **Checked:** all 73 installed event packages (75 engines) parse with validated
  references. The ignored install test sends IMPACT_1 to the log trailer's left
  damage zone, dispatches its process-stimulus events, receives all nine original
  fragment requests and one explosion, and verifies that another hit cannot
  release the emptied load again. Seven fragments name present attachments.
  Synthetic tests cover truncation and malformed references, list order, timer
  boundaries, queue capacity, projected states, flush/abort, activation order
  and identical fixed-tick traces across render rates and a 300 ms stall.
- **PC correction:** function `0x645A40` confirms idle invocation and busy queueing;
  the unfinished reference reverses this condition. The runtime follows the PC.
- **Body construction:** isolated execution of `0x672050` and `0x66A890` with four
  synthetic hierarchies checks intact, disabled, selected-subtree and root-only
  geometry. The tests compare the selected names, local offsets, extents, flags
  and orientation (quaternion tolerance 0.00001). Additional synthetic tests
  check fallback boxes/spheres, padding, surface fallback and mesh-only roots.
  The install check constructs geometry for all 13,479 resolved prop placements.
- **Scene integration:** original vehicle strength and damage-zone scaling feed
  collision stimuli. Undelayed EFireEventList expands in place, as measured at
  `0x638540`, retaining its dynamic context. Installed physical rear-end tests
  release seven logs or three carrier cars into the prop solver, with identical
  body traces at 5–240 render FPS. Only successfully spawned uploaded parts are
  hidden; missing meshes preserve the parent. Parent destruction removes child
  bodies. The PC vehicle-fragment limit is 121; the separate global model limit
  of 2297 is not yet tracked by the rewrite.
- **Limits:** expression filters return an explicit unsupported error. Pause,
  rate changes and unload tags remain outside this runtime. Parsing and synthetic
  event dispatch and CPU body tests do not establish live rendering, explosion,
  audio, pursuit-breaker or full original-game parity. Offscreen fragment timers,
  damage visuals, jointed fragments and other event effects remain pending.
