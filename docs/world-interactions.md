# World interaction parity work

Target: original Most Wanted PC 1.3 behavior, including car impacts, scenery,
breakable objects, trailers, loose cargo, and pursuit/world set pieces.
Full parity has not been established. Record evidence separately for data,
isolated routines, scene integration and live-game comparisons.

## Base

- Traffic stack: [PR 24](https://github.com/47PADO47/nfs-mw/pull/24),
  [PR 25](https://github.com/47PADO47/nfs-mw/pull/25), and
  [PR 29](https://github.com/47PADO47/nfs-mw/pull/29), ending at
  `291a72e451d3058d9c1f55b115bc7d30708e8de7`.
- Traffic lights are a rewrite extension: disable them for original-parity runs.
  Unrelated renderer PRs are not required by this work.

## Work ledger

| Area | Starting gap | Status |
|---|---|---|
| Car response | Fixed friction/restitution, textbook impulse | 32-iteration solver matches 13 PC reference cases; per-body object materials and grounded player contact centre integrated; AI reaction overrides pending |
| Timing | Player/AI step separately; pairs collide once per render frame | Shared 60 Hz world tick; car-state and impact traces identical at 5/10/30/60/120/144/240 FPS and uneven frames; live validation pending |
| Geometry | Single car box, no swept object contacts | Pending primitive and sweep work |
| Scenery | Name guesses, >150 kg immovable, lighter props disappear for six seconds | Pending original mapping, dynamic response and lifecycle |
| Trailers | Approximate anchors and response | Pending bounds joints and reference comparison |
| Truck cargo | No released logs/crates | Pending hierarchy/attachment data and runtime |
| World/ground | Approximate probes and solver | Pending original comparison |
| Damage/feedback | Partial sparks/audio, incomplete collision events | Pending |
| Set pieces | Missing pursuit breakers, spike strips and scripted objects | Pending |
| Full comparison | No comprehensive repeatable collision suite | Pending live captures and numerical comparison |

## Verification

Run `cargo xtask check`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
Run synthetic checks with no `NFSMW_GAME_DIR`; set that variable explicitly for
ignored install tests. Keep local tool configuration, build output and private
research outside tracked source. Game data and decompiled output are never
published. Compilation and synthetic tests alone do not prove gameplay parity.
