# blackbox-damage

Persistent visual damage for one vehicle. Each impact zone keeps three levels (panel deformation,
paint scrape and glass crack), every level capped at [`MAX_LEVEL`], and each wheel keeps a
punctured flag.

- **Pure.** No I/O, no allocation after `Damage::new`, no game data. The caller maps its own zone
  numbers and "this zone has glass" flags onto [`Damage::impact`].
- **Monotonic.** Impacts only raise levels; [`Damage::repair`] is the only reset.
- **Spec:** [`docs/specs/vehicle-damage.md`](../../docs/specs/vehicle-damage.md).

```rust
use blackbox_damage::Damage;

let mut damage = Damage::new(10);
// A hard hit (severity 5 of 6) on zone 0 that is glazed.
damage.impact(0, 5, true);
assert_eq!(damage.zone(0).glass, blackbox_damage::MAX_LEVEL);
```
