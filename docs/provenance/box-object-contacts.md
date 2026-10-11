# Box contact correction

- Source: the user's PC 1.3 executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Private inspection of geometry setup, box intersection and segment/plane
  intersection, plus CC0 `RigidBody` penetration handling. Executable bytes,
  inspection reports and research scripts remain outside the repository.
- [Specification](../specs/box-object-contacts.md) written before implementation.
- Isolated x86 execution and Rust comparison: 5,012 synthetic cases, comprising
  12 named cases, 1,000 seeded yaw cases and 4,000 seeded arbitrary rotations,
  including 100 flat boxes. No hit/miss, point-ownership or numerical mismatches;
  maximum point/normal/depth component error below `0.000001`.
- In-repo numerical regressions cover centred faces, containment, tangency,
  unsupported corners, thin-object crossing, pair reversal, rotation and tumbling.
- Synthetic separation tests cover incident-point translation, temporary centre
  translation, normal-speed shares, attachment release and effective-mass units.
  Full original compound-body and live gameplay comparisons remain open.
