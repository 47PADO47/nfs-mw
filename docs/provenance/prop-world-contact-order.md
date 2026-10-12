# Prop and world contact order

- **Spec:** [Object and world contact order](../specs/prop-attributes.md#object-and-world-contact-order).
- **Sources:** read-only inspection of the user's PC 1.3 executable on 2026-10-12,
  plus `Physics/Behaviors/RigidBody.cpp` and `RBVehicle.cpp` in the CC0 reference.
  Binary identity, raw inspection output and addresses remain private. No
  decompiled code is included.
- **Established:** the global rigid-body update performs begin-frame callbacks
  and broad-phase preparation, object-pair pruning and collision response,
  end-frame callbacks, then articulation. The vehicle and smackable vtables bind
  the inspected end-frame function that dispatches world contacts. The object
  pruning callback reaches the object collision solver before that stage.
- **Rewrite mismatch:** the inspected vehicle step resolved world walls before
  car/prop pairs. A same-tick wall contact could therefore change the velocity
  supplied to breakaway response. This is an ordering defect; it does not by
  itself identify the cause of an individual reported pole collision.
- **Validation boundary:** use a deterministic car/prop collision alongside an
  eligible wall in the same tick, preserving the authored geometry and detach
  threshold. Local ordering is a bounded correction; global body-phase
  scheduling, pair order and full physics parity remain separate work.
