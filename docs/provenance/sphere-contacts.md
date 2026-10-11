# Sphere contacts

- Source: the user's PC 1.3 `speed.exe`, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Private inspection and isolated synthetic execution: geometry setup `0x4876F0`,
  sphere/box `0x48B5D0`, sphere/sphere `0x48BF50`, and the ray/sphere exit helper
  `0x487500`. Private scripts, executable bytes and inspection reports stay outside
  the repository. Only synthetic inputs and numerical expectations enter tests.
- [Specification](../specs/sphere-contacts.md) written before implementation.
- Cases: positive/negative faces, edge, corner, miss, tangency, containment,
  coincident centres, a flat box, crossing a face/corner, moving box and rotation.
- A seeded private comparison also executes 500 randomized cases, including 50
  flat boxes and 100 sphere pairs. All hit/miss results agree; the largest
  component error in point, normal or depth is below 0.000001.
- These isolated cases do not establish full original world-contact parity.
  Coincident sphere point initialization is intentionally deterministic; the
  original's residual-memory behavior is not reproduced.
