# Original camera environment data provenance

- **Attribute identities:** the private reference `smackable` and `simsurface`
  schema declarations resolve the names, hashes and types of `CAMERA_AVOIDABLE`
  and `CAMERA_NOISE`. These declarations are behavioral references only.
- **Native use:** the PC camera obstruction path checks the object flag; the
  terrain camera path consumes the surface float pair. The independent mover
  and obstruction specifications describe the consuming calculations.
  The native terrain producer reads indices zero and one separately, multiplying
  the speed-dependent amplitude and frequency respectively. See
  `docs/specs/original-camera-noise.md` for the consumer contract.
- **Implementation:** independently written Rust adds the original boolean to
  game-specific prop data and preserves a per-surface pair in the existing
  surface table. No original executable, decompilation, asset table or extracted
  data is included.
- **Runtime adapters:** the native driving action obtains the collision body's
  center of gravity, excludes its lateral component, rotates the remaining
  offset and adds rigid-body position before updating the camera anchor. This
  establishes a physical anchor independently of drawable model pivots. Native
  action inputs also establish the brake/handbrake gate and the engine redline
  minus 500 RPM overrev gate. The host reads those existing simulation fields;
  its current free-roam mode has no drag-race input.
  The native gear producer compares the transmission integer with its retained
  prior value without filtering direction. Reference GearID declarations and
  the host transmission agree on reverse0/neutral1/first2, so the adapter uses
  the raw simulation value rather than the HUD's value minus one.
- **Collision severity:** independently executed native contact-solver vectors
  separate tangential velocity from the closing vector delivered by the rigid
  body's collision callback. A 30 m/s tangential slide with a 5 m/s normal
  approach delivers 5 m/s closing speed; a 15 m/s slide with a 30 m/s normal
  approach delivers 30 m/s. Adapter regressions preserve those distinctions and
  reject separating contacts. Response impulse is not the camera input.
- **Limits:** missing-data recovery follows existing reader policy and is not
  an assertion about malformed-data handling by the original executable. Data
  loading checks do not establish the behavior of the consuming camera query.
