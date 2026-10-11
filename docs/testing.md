# Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` / `tests` modules in every crate | no | yes |
| Real-install tests | [`crates/nfsmw-data/tests/real_install/`](../crates/nfsmw-data/tests/real_install) (`#[ignore]`) | yes | no |
| GPU tests (renderer) | `#[ignore = "needs a GPU"]` in `blackbox-render`, scenes from [`blackbox-gfx-testkit`](../libs/blackbox-gfx-testkit); the passes' own in [`blackbox-gpu-passes`](../libs/blackbox-gpu-passes) | no | no (optional job) |
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

## GPU tests

The renderer's tests that need a GPU are `#[ignore = "needs a GPU"]`, run headless (no window) and need no game
data. They skip, not fail, when the machine has no adapter for the API.

```sh
cargo test -p blackbox-render --lib -- --include-ignored          # Vulkan
BLACKBOX_TEST_API=dx12 cargo test -p blackbox-render --lib -- --include-ignored   # or gl
BLACKBOX_GPU_FALLBACK=1 cargo test -p blackbox-render --lib -- --include-ignored  # software adapter (lavapipe, WARP)
```

They take turns on one lock, because creating Vulkan devices on parallel threads crashes some Mesa drivers. The
lock (`serial()`) lives in `blackbox-gpu-passes`' `test-support` feature, so the tests of both crates share it.
The passes crate tests its UI, effect, soft particle, filter and FSR 1 passes against a plain wgpu device, with no
renderer around them (`cargo test -p blackbox-gpu-passes -- --include-ignored`).

- **Structure tests** (`gpu/parity/structure_tests.rs`) check facts that hold on any adapter: depth order out to
  9 km, fog, alpha-test cut-outs, blend order, clip rectangles, and that the UI is never post-processed.
- **API tests** check the native `Capabilities` (FXAA; bilinear and FSR 1; every tone map; bloom; no ray
  tracing; `compressed_bc` from the adapter), `resolve` and `apply_graphics` on a real renderer.
- **Digest tests** guard the native image. Each testkit scene is rendered at 256x144 under five settings (direct
  path, offscreen, FXAA, bloom with ACES, FSR 1 at 67 %) and its 32x18 grid of mean colours is compared to a digest
  stored as a Rust constant (`gpu/parity/digests/`), within +-2 per channel. Any change to what the native
  renderer draws moves some cell, so an intended change updates the digests in the same commit.

Digests depend on the GPU and driver. These were recorded on **Intel Iris Xe (Vulkan, Mesa)**; the digest test
skips on any other adapter (its header comment names the GPU and driver). A different Mesa version can move a few
cells by rasterisation detail: regenerate and review the change.

To regenerate (the test prints the files; it never writes to the repository):

```sh
BLACKBOX_UPDATE_DIGESTS=1 cargo test -p blackbox-render --lib native_digests -- --include-ignored --nocapture > /tmp/digests.txt
python tools/split_digests.py /tmp/digests.txt
```

To add a GPU family, add a variant to `Family` in `gpu/test_support.rs`, a directory under `digests/`, and run
the generator on that GPU. Cross-backend comparisons use the testkit's metrics instead of digests (its README).

## Bevy renderer tests

`cargo test -p blackbox-bevy-render` runs the CPU tests. Its GPU tests (`-- --include-ignored --test-threads=1`, they take
the same `serial()` lock) draw the strict testkit scenes headless on the Bevy renderer and compare them with the
native one through `compare` against the plan's tolerances (max 4, mean 0.5, p99 2 of 255); they print the real
numbers (`--nocapture`) and skip without an adapter. `BLACKBOX_GPU_FALLBACK=1` runs them on lavapipe; where the
environment restricts Vulkan drivers (`VK_LOADER_DRIVERS_SELECT=*intel*`), also set it to `'*lvp*'`. Results and
how to compare the real city: [bevy-backend.md](bevy-backend.md) (`cargo xtask img-diff`).
