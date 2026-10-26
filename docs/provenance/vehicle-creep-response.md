# Reference chassis creep provenance

Date: 2026-10-26. Spec: [creep response](../specs/vehicle-creep-response.md).

Sources: independently executed original PC 1.3 instructions (proprietary),
retained private player boundary recording, and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `Chassis.cpp` and
`RigidBody.cpp` (CC0-1.0, decompiled, behavioral corroboration only).
Implementation follows the written behavior contract; no restricted code,
binary extracts, original numeric fixtures or local installation paths ship.

Original instruction replay reproduces all twelve body vector components at
seven active recorded creep boundaries exactly when supplied their captured
interface inputs. Synthetic controls confirm additive non-lateral force/torque,
snapshot consumers, epsilon stop and damping subtraction rounding. Interface
substitutions and two tested FPU precision modes are recorded privately.

Expanded instruction replay reproduces all 1,967 complete retained calls,
including 1,960 no-op calls and 181 collision-flag entries. The production Rust
helper also matches all compared body-vector outputs in those 1,967 calls and
ten synthetic controls with explicit cached rotation inputs. No numeric
acceptance tolerance is introduced. The replay fixture data remains private.

The Reference policy previously scaled only lateral force and yaw torque and
multiplied full-stop vectors directly by speed. Those operations differ from
the verified original additive response and damping amount calculation.
Regularized intentionally retains its separate stabilization policy.

Recorded landing boundaries refresh saved angular velocity before jump/creep.
This change preserves that ordering. Full wake/sleep producer parity, complete
jump trajectories and normal-frame-rate driving acceptance remain open.
