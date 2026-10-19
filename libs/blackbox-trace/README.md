# blackbox-trace

Framework-independent, on-demand instrumentation. Call `scope("phase")` around
CPU work and `global().counter(...)` or `note(...)` for measurements and bookmarks.
The application owns `start`/`stop` and writes the detached capture on a worker.
No game knowledge, GPU API, logging subscriber or runtime service is required.

Disabled scopes perform an atomic check. Enabled scopes measure monotonic wall
time, including blocking, rather than CPU active time. The event buffer is capped
at 250,000 entries; overflow is counted in exported metadata. Text and thread
metadata are bounded. Old guards cannot leak into later recordings. Unfinished
scopes at capture stop are omitted; normal frame boundaries should close them first.

The original exporter follows the documented Chrome Trace Event JSON format,
readable in [Perfetto](https://ui.perfetto.dev). Numeric counters are sampled at
observation time: a delayed GPU readback is not a calibrated GPU timeline.
Format reference: [Perfetto external formats](https://perfetto.dev/docs/getting-started/other-formats)
(Apache-2.0 documentation; no source code copied).
