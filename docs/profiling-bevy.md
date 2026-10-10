# Profiling the Bevy renderer with Tracy

How to run the Bevy renderer, capture a Tracy trace, and turn it into numbers an agent can read. The background
(why the fixed CPU cost matters, the candidate fixes) is in [bevy-backend.md](bevy-backend.md).

## Run it

From the repository root:

```bash
cargo xtask run bevy
```

Starts the world view on the Bevy renderer, driving at once and uncapped (`view-world --renderer bevy --drive
--no-vsync`). It builds with `--release` and the `renderer-bevy` feature.

For a profiling run:

```bash
cargo xtask run trace
```

Same run, built with the `trace` feature: Bevy's per-system `tracing` spans plus a Tracy subscriber. The game
does not wait for a profiler; it just streams to one if one is connected.

To run something else, put the game arguments after `--`:

```bash
cargo xtask run trace -- view-world --renderer bevy --drive SKYLINEZT --no-vsync
```

## The `SymInitialize` message

Starting the trace build with `tracing-tracy` 0.11 (`tracy-client-sys` 0.29) printed the following. It has not been
re-checked with 0.30; if you still see it after the upgrade, the explanation below still applies.

```text
ERROR: SymInitialize FAILED with code 87 (0x57) | The parameter is incorrect.
```

This is harmless. `tracy-client-sys` initialises the Windows symbol handler (`dbghelp`) itself, and Tracy then
calls `SymInitialize` again. The second call fails with 87 (`ERROR_INVALID_PARAMETER`) and Tracy carries on. The
only effect is on symbol names for callstacks, and `tracing-tracy` records no callstacks unless it is configured
to (its default `stack_depth` is 0). The per-system spans are unaffected. The message cannot be turned off from
this repository: `tracy-client-sys` does not expose a switch for Tracy's Windows callstack setup.

The first trace run opens a local listener for the Tracy client. Windows Defender Firewall may ask about
`nfsmw` the first time. Allow it on private networks only; the listener is for the local profiler, not the internet.

## Capture a trace

1. Get Tracy 0.14.1 (wire protocol 82, matching `tracy-client-sys` 0.30) from
   <https://github.com/wolfpld/tracy/releases/tag/v0.14.1>, Windows zip `windows-0.14.1.zip`. It is unpacked to
   `%LOCALAPPDATA%\Programs\Tracy-0.14.1`, which holds `tracy-capture.exe`, `tracy-csvexport.exe` and
   `tracy-profiler.exe` (the GUI).
2. Start `cargo xtask run trace`. Let the drive run for at least 30 seconds of steady state (skip the first few
   seconds of shader and pipeline warm-up).
3. Connect the Tracy GUI to `127.0.0.1` and save the trace (File, Save trace). Or, without the GUI, record it
   headless:

   ```bash
   tracy-capture -o target/trace/run.tracy -f -s 30
   ```

   `-s 30` stops after 30 seconds; `-f` overwrites an existing file.
4. Export the per-zone statistics to CSV. `-e` gives self times (time not spent in child zones), which is what
   attribution needs; leave it off for inclusive totals:

   ```bash
   tracy-csvexport -e target/trace/run.tracy > target/trace/zones_self.csv
   ```

`target/` is ignored by git, so the trace and the CSV stay local.

## Hand it to the agent

Tell the agent the path to `zones.csv` (and, if you have it, the average fps and frame time the game reports).
Compare it with a native-renderer run captured the same way, so the fixed Bevy cost is measured against a
baseline rather than in isolation. The agent reads the CSV and reports, per Bevy schedule (extract, queue,
prepare, render) and per system, the time per frame and its share of the frame.

If a zone name is unclear, the agent can't tell which system it is from the CSV alone. Say so, and the agent will
ask for a GUI screenshot of that zone's parent.

## Results (2026-10-10, RTX 4070 SUPER, borderless 1920x1080, `view-world --drive --no-vsync`)

Benchmark (`--bench-seconds 20`, no trace feature):

| Renderer | fps | p50 | p95 | p99 | worst |
| --- | --- | --- | --- | --- | --- |
| blackbox (native) | 1036 | 0.92 ms | 1.11 ms | 1.21 ms | 1.85 ms |
| bevy | 311 | 3.18 ms | 3.40 ms | 3.72 ms | 7.48 ms |

Traced run (30 s, `bevy_ecs/debug` on, so system names show; ~6,000 frames). Inclusive time per frame, with
tracing overhead:

- The render thread (`RenderApp`) takes 4.41 ms. `RenderGraph` is 1.73 ms and `finish_command_buffers` 0.94 ms.
- `prepare_preprocess_bind_groups` (bevy_pbr gpu_preprocess) is the largest single system, at 0.63 ms per frame.
  It accounts for most of the roughly 400 `create_bind_group` calls per frame.
- `Queue::write_buffer` runs about 574 times per frame, roughly 0.7 ms self time. `StagingBuffer::new` runs about
  585 times per frame.
- `prepare_clusters_for_gpu_clustering` takes 0.16 ms, `hud::plugin::present` 0.08 ms, `run_ui` 0.06 ms, and
  `blackbox_bevy_render::apply` 0.02 ms per frame. The HUD and the blackbox bridge are small.

Reading: the gap to native is about 2.3 ms per frame. Per-frame GPU resource churn (bind groups, buffer uploads,
command encoding) accounts for most of the render thread's time.
