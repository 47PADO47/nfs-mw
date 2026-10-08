# Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` / `tests` modules in every crate | no | yes |
| Real-install tests | [`crates/nfsmw-data/tests/real_install/`](../crates/nfsmw-data/tests/real_install) (`#[ignore]`) | yes | no |
| Leak + size checks | `cargo xtask check` | no | yes, first job |
| Python tool tests | [`tests/`](../tests) | no | yes |

```sh
NFSMW_GAME_DIR="D:/Need For Speed Most Wanted Black Edition" cargo test --release -p nfsmw-data -- --ignored
```

The real-install tests check:

- all 100 cars (15,781 solids) parse, with every index in range;
- the BMW M3 GTR matches [models.md](formats/models.md);
- the car and global texture packs decode;
- all 720 world sections parse (20,377 solids, 3,644 textures, 77,783 scenery instances), and tile scenery
  resolves;
- the visible-section tables parse (515 boundaries, 435 zones, 39 loading sections), zones never overlap,
  and only the panoramas `A91`, `A94`, `C99`, `O93` are in no zone's list.

The whole stream parses in under a second in release builds.

`nfsmw view-car … --screenshot out.png` and `nfsmw view-world … --wait-for-load --screenshot out.png`
render one frame off-screen. Use them to check rendering changes and backends without a window.
