# World mesh animation provenance

- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0:
  `Animation/AnimWorldTypes.hpp`, `AnimEntity_WorldEntity.cpp`,
  `AnimWorldScene.cpp`, `WorldAnimCtrl.cpp` and `World/SimpleModelAnim.cpp`, used as behavioral reference.
  No reference code or game assets are included.
- The user's installed PC track supplies the animation banks, instance and
  entity records, and the ROOT skeleton from the global animation bundle.
  Installed-data tests check these relationships; they are not a live original
  game comparison.

- Retail PC `speed.exe` SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
  inspected 2026-10-28: controller advance at RVA `0x4BB80`, controller-scenario
  initialization at RVA `0x4D2B0`, and entity initialization at RVA `0x4F340`.
  The PC timing body corroborates the reference's otherwise UNSOLVED function.
  The trigger-zone and forward/reverse-track slots are empty in the inspected
  PC initializer. This is static evidence, not live game capture.
- The follow-up controller was written from the updated behavior specification.
  Synthetic tests exercise strict endpoints, forward/reverse wrapping, ping-pong,
  triggered pause, sequential delays, scaled ranges and bounded hitch handling.
