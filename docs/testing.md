# Testing

| Kind | Where | Needs the game | Runs in CI |
|---|---|---|---|
| Unit tests on synthetic bytes | `#[cfg(test)]` / `tests` modules in every crate | no | yes |
| Real-install tests | [`crates/nfsmw-data/tests/real_install/`](../crates/nfsmw-data/tests/real_install) (`#[ignore]`) | yes | no |
| Real-save tests | [`crates/nfsmw-career/tests/real_saves.rs`](../crates/nfsmw-career/tests/real_saves.rs) (runs only when `NFSMW_SAVE_DIR` points at the profile folder of the game under Documents, or at a folder of profile files; returns at once without it) | yes (saves) | no |
| Installed-game tests of the game itself | `#[ignore = "..."]` tests inside [`crates/nfsmw/src`](../crates/nfsmw/src), beside the code they check | yes | no |
| Installed-data tests of the libraries and mods | `#[ignore]` tests in `libs/` and `mods/` | yes (this game, the 2012 game or RIDE 6 data, by test) | no |
| GPU tests (renderer) | `#[ignore = "needs a GPU"]` in `blackbox-render`, scenes from [`blackbox-gfx-testkit`](../libs/blackbox-gfx-testkit); the passes' own in [`blackbox-gpu-passes`](../libs/blackbox-gpu-passes) | no | no |
| Leak + size checks | `cargo xtask check` | no | yes, first job |
| Python tool tests | [`tests/`](../tests) | no | yes |

The CI workflow ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)) starts on manual dispatch only. Its test
step is `cargo test --workspace`, which skips every `#[ignore]` test; it has no GPU job and no game data.

```sh
NFSMW_GAME_DIR="<your-install>" cargo test --release -p nfsmw-data -- --ignored
NFSMW_GAME_DIR="<your-install>" cargo test -p nfsmw <name filter> -- --ignored --nocapture
```

`nfsmw` is a binary crate, so its tests take no `--lib`. Run its ignored tests by name filter: some need more than
the install (see below).

## Ignored tests

`#[ignore]` attributes counted in the sources on 2026-11-17:

| Where | Ignored tests |
|---|---|
| `crates/nfsmw` | 624 |
| `crates/nfsmw-data` | 210 |
| `libs/` | 468 |
| `mods/` | 56 |
| `crates/nfsmw-career`, `nfsmw-modding`, `nfsmw-net`, `nfsmw-server`, `nfsmw-sim`, `xtask` | 0 |

The 624 in `crates/nfsmw` all carry a reason string; there are 284 different strings and no shared vocabulary. The
most common is `needs the game (set NFSMW_GAME_DIR)` (131 tests). Counting by words in the reason, with overlap:

| Reason mentions | Tests | Examples |
|---|---|---|
| the installed game or its files | 618 | `requires installed original garage packages`, `requires installed audio banks; set NFSMW_GAME_DIR` |
| a GPU, Vulkan, DX12 or rendering | 54 | `requires installed world and GPU`, `requires a Vulkan GPU` |
| a run time or a benchmark | 22 | `needs the installed game; set NFSMW_GAME_DIR (about a minute)`, `CPU benchmark; run explicitly with --nocapture` |
| a private capture, trace or fixture | 12 | `needs installed world and NFSMW_RACER_TRACE player trajectory JSON` |
| a mod folder or converted mod data | 11 | `renders the converted Tesla offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD)` |

A reason does not always name every variable the test reads. Read the test before relying on it.

### Tests that pass without checking

Two patterns make a test pass when its data is missing:

- **Plain `#[test]`s that return early.** Some tests are not ignored; they look for the install and return at once
  without it, so `cargo test --workspace` and CI report them as passed without running a check. Examples:
  `every_look_builds_draws_and_frees_what_it_uploaded` in
  [`frontend/backdrop_tests.rs`](../crates/nfsmw/src/frontend/backdrop_tests.rs) and the tests of
  [`audio/aems/tests.rs`](../crates/nfsmw/src/audio/aems/tests.rs), whose header says "passes without checking
  otherwise". A script that looked for an early `return` on an environment lookup in the first statements of a
  non-ignored test found 78 such tests in 22 files of `crates/nfsmw` (15 of them in `frontend/tests.rs`). That is a
  lower bound: it only follows helpers defined in the same file. The real-save tests of `nfsmw-career` do the same
  without `NFSMW_SAVE_DIR`. The comment in `ci.yml` ("Tests that need the game are `#[ignore]`d and skipped here")
  does not cover these.
- **Ignored tests that return early.** Some `#[ignore]` tests also return when a variable is unset (the
  `nfsmw-data` real-install tests do), so `-- --ignored` without `NFSMW_GAME_DIR` passes them. Others `expect` the
  variable and fail.

A passed run is evidence only when the variables were set and the test's own output shows it ran.

### Environment variables tests read

| Variable | Used by | Value |
|---|---|---|
| `NFSMW_GAME_DIR` | the installed-game tests of `crates/nfsmw`, `crates/nfsmw-data`, several libs and the 2012 content mod | the install folder of the 2005 game |
| `NFSMW_SAVE_DIR` | `crates/nfsmw-career/tests/real_saves.rs` | a folder of profile files |
| `CHAMELEON_GAME_DIR` | tests of `libs/chameleon-*` and one in `mods/nfsmw-2012-content` | the install folder of the 2012 game |
| `MW2012_GAME_DIR` | tests of `mods/nfsmw-2012-content`, `libs/chameleon-*` and `libs/ea-audio` | the install folder of the 2012 game; the two names are not unified |
| `RIDE6_CARS`, `RIDE6_CAR`, `RIDE6_MOD_DIR` | bike tests in `libs/blackbox-modding` and `mods/ride6-bikes` | converted RIDE 6 bike data, which is the user's own |
| `NFSMW_MEASURE_MOD`, `NFSMW_MOD_DIR` | car-sound measurement tests in `crates/nfsmw/src/audio/car/` | a built mod folder |
| `BLACKBOX_TEST_API`, `BLACKBOX_GPU_FALLBACK`, `BLACKBOX_UPDATE_DIGESTS` | GPU tests (below) | graphics API, software adapter, digest regeneration |
| `NFSMW_RACER_TRACE`, `NFSMW_RACER_CATCHUP_TRACE`, `NFSMW_PURSUIT_TARGET_TRACE`, `NFSMW_JUMP_REPLAY`, `NFSMW_CREEP_REPLAY`, `NFSMW_CAPTURE` | comparison tests against private recordings | an input file; formats in [parity-tracing.md](parity-tracing.md#test-inputs-and-outputs-named-by-environment-variables) |
| `NFSMW_*_CAPTURE_DIR`, `NFSMW_*_EVIDENCE_DIR`, `NFSMW_TEST_OUTPUT_DIR`, `NFSMW_SHOT_DIR`, `NFSMW_LAMP_OUT`, `NFSMW_PERF_OUT` | tests that can save pictures, sound or reports | an output folder or file; unset, nothing is saved |

Many more `RIDE6_*`, `MW2012_*` and `NFSMW_MEASURE_*` variables tune single measurement tests; each is documented
at its test.

### Known intermittent test

`a_free_roam_drive_gives_the_same_state_hash_every_second_twice`
([`scenes/world/determinism_tests.rs`](../crates/nfsmw/src/scenes/world/determinism_tests.rs)) runs the installed world
twice under session rules and compares the state hash every simulated second. It has failed on the first run after a
build, with a cold disk cache: twice in about twelve runs before lockstep streaming, and once in six runs on
2026-10-22 after the support-tile fix. The cause is open
([specs/multiplayer/determinism.md](specs/multiplayer/determinism.md), step 5). Run it again before treating a
failure as a regression, and record both results.

### Proposed convention

**A proposal, not current practice.** No test uses these strings today. New ignored tests could start their reason
with one fixed tag, so a run can be filtered and a skipped check is recognisable:

| Tag | Meaning |
|---|---|
| `needs-game` | reads the installed game through `NFSMW_GAME_DIR` |
| `needs-gpu` | needs a graphics adapter |
| `needs-capture` | reads a private recording or fixture named by a variable |

For example `#[ignore = "needs-game needs-gpu: renders the highway at dusk"]`. A test with such a tag would fail,
not return, when its variable is unset.

## Real-install tests of `nfsmw-data`

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
