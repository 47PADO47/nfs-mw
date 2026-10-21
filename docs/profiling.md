# Profiling and build profiles

General profiling notes. The Bevy renderer's Tracy workflow is in [profiling-bevy.md](profiling-bevy.md).

## In-game trace recorder

Available in ordinary builds, with either renderer and no DLSS or Tracy requirement:

| Control | Action |
|---|---|
| **F9** | Start recording; press again to stop and save early. |
| **F10** | Add a numbered bookmark at a hitch or interesting moment. |
| **F8 → Diagnostics → Performance trace** | Record, stop, bookmark, choose 5/15/30/60 seconds, copy the saved path. |
| Console `trace start [seconds]` | Start a 1–60 second capture (default 15). |
| Console `trace stop`, `trace mark [label]`, `trace status` | Save early, annotate, inspect status. |

Hotkeys require the game window to be focused and work with the menus closed.
Recording stops automatically on a frame boundary. A recording/saving indicator
appears even with the performance overlay off. Capture data stays in a bounded
4,000,000-event buffer; excess events are counted in JSON metadata and the save
message. Normal shutdown saves an active capture; forced termination cannot.
File export runs on a worker after recording stops. Only shutdown waits for it.

Files go in the active profile's log directory, under `traces/trace-*.json`.
F8 Diagnostics and the console show the full saved path. Open that JSON file in
[Perfetto](https://ui.perfetto.dev); opening the viewer does not upload the file.
The original exporter follows Perfetto's documented
[Chrome Trace Event JSON support](https://perfetto.dev/docs/getting-started/other-formats).

The trace contains nested CPU wall-time scopes for the frame, scene update, UI,
world simulation, streaming/upload work, visibility, caster preparation,
reflections, effects and native render recording/submission. Acquiring a surface,
presenting/FG dispatch, Reflex pacing and the game limiter have separate scopes.
Ordered Update phases also have enclosing scopes. Individual scopes cover HUD
telemetry, FEng animation/theme composition, CPU image masks and text rasterization,
audio, mod callbacks, frontend/cursor work, input, live settings and metric collection.
Native recording separates SMAA preparation/resolve, encoder finalization, queue
submission, temporal setup, UI encoding and timestamp polling/readback. Parent
phase time includes scheduler/wait overhead; it is not exclusive CPU active time.
Settings snapshots are included initially and on changes, with active backend,
GPU/driver identity, requested settings and effective graphics. Frame observations
include dimensions, focus, base-frame IDs, waits and cumulative SDK-reported
presentations where supported. Presented FPS can be derived from changes in
the presentation counter; no multiplier estimate is substituted.

Native GPU base/shadow/surface/AO durations appear as **delayed readback counters**
at CPU observation time, with readback IDs. They are not GPU execution slices or
calibrated to the CPU clock, and exclude auxiliary queues and present-time FG.
OS main-thread activity and busiest-process-GPU-engine activity are interval
samples, with sample IDs and age; they include work on SDK/auxiliary queues.
Unsupported measurements are omitted. CPU scopes include blocking/driver time:
a long scope alone does not prove CPU saturation. Bevy's internal rendering is
not covered by the native scopes/counters; use its existing Tracy workflow.

Start in a representative scene, mark slow moments, change one setting at a time,
and compare nested scopes alongside activity and waits. Instrumentation adds
some overhead while recording; compare runs with identical capture settings.
The recorder does not sample native call stacks or GPU shader instructions.
Use a sampling profiler or PIX/Nsight for hot functions and vendor-internal queues.
Generic instrumentation is in `libs/blackbox-trace`; additional subsystems can
add scopes/counters without depending on Bevy or the game's devtools.

Clipboard and browser buttons are handled by the host on a bounded platform
worker. A toast reports success/failure; copying text does not open or upload a
trace. Browser actions use the configured default browser. The clipboard bridge
uses [arboard](https://docs.rs/arboard/latest/arboard/); web links use
[open](https://docs.rs/open/latest/open/). Their native APIs are called outside
the render loop. The clipboard object is retained for X11 ownership semantics.

## Build profiles

The root `Cargo.toml` defines these profiles. Pick by what you are measuring.

| Profile | Command | Use it for |
|---|---|---|
| `release` | `cargo build --release -p nfsmw` | Everyday timing. |
| `profiling` | `cargo build --profile profiling -p nfsmw` | Profiling. Same codegen as `release` plus line tables, so Tracy, VTune, Superluminal and `perf` show function and line names. Output: `target/profiling/`. |
| `dist` | `cargo build --profile dist -p nfsmw` | Shipped builds. Fat LTO, one codegen unit. Much slower to build; use it for releases, not iteration. Output: `target/dist/`. |

`profiling` adds debug info only, so its timings match `release`. Compare numbers across the same profile.

## Allocator: `mimalloc` feature

`nfsmw` can use [mimalloc](https://crates.io/crates/mimalloc) (MIT) as its global allocator. It is behind the
`mimalloc` cargo feature, which is on by default: in the headless world frame cost benchmark (`world_frame_costs`) it
took 3-5 % less CPU per frame than the system allocator in every scene, with and without the mirror. Build without it
to compare against the system allocator (this also drops the default renderers, so add back the ones you want):

```bash
cargo build --release -p nfsmw --no-default-features --features renderer-bevy,fsr3
```

The allocator is set in `crates/nfsmw/src/main.rs` with `#[global_allocator]`. It needs a C compiler (it builds
the mimalloc C sources), so a target without one builds with the feature left out.

The buffer grows on demand up to four million events, enough for longer high-FPS
captures. A full capture can occupy several hundred MiB before JSON export.
Overflow remains bounded and is disclosed in the saved trace metadata.

### Hitch and memory-growth evidence

The recorder also writes `trace-*.context.json` beside each Perfetto trace. Keep
both files when sharing a capture. The companion includes the executable SHA-256,
package-declared source revision/tree hash and package-metadata hash. Package
claims remain distinct from the measured executable identity; absent metadata is
reported as unknown. Hashing and export run on the writer, outside live frames.

A worker retains up to 600 approximately one-second health samples, including
samples before F9. Windows records process working set, private committed memory,
peak working set, cumulative page faults, process I/O totals, available physical
memory and process commit headroom. Failed/unsupported counters stay absent.
Page faults include soft faults and are not proof of disk paging; process RAM is
not VRAM. The trace buffer itself uses memory while recording, so the history
marks recording intervals for comparison with the earlier baseline. This history
is bounded in RAM and saved only with a requested trace; it is not a continuous
on-disk log. The worker keeps sampling memory while the main thread is blocked.

Live snapshots include mesh/texture counts, effect-buffer vertex capacities,
shadow cascade/draw/instance counts, scene status (resident/loading tiles and
props where available), and camera pose. Snapshot age makes stale resource data
visible. The frame sends these snapshots without waiting for the worker. History
relative times use a monotonic clock: negative seconds precede the trace start.
Wall-clock milliseconds allow correlation with external recordings. Only retained
history at or before capture stop is exported.

`Hitch observation (25 ms threshold)` marks long frame intervals with observation
and delayed-GPU readback IDs, focus and acquisition/presentation/pacing waits.
It is a marker at observation time, not a diagnosis or native CPU stack sample.
Reflection visibility faces and shadow CPU cascades have separate nested scopes.
Start F9 before reproducing a hitch and let it continue afterward; earlier memory
history cannot recover CPU/GPU events that occurred before recording began.
