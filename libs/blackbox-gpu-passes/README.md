# blackbox-gpu-passes

Reusable wgpu passes for EA Black Box game renderers. Each pass takes plain wgpu inputs and
[`blackbox-gfx`](../blackbox-gfx) data, and knows nothing about the renderer that calls it, so the native
renderer ([`blackbox-render`](../blackbox-render)) and a renderer built on another engine can share the code.

License: MIT OR Apache-2.0 (the FSR 1 shader keeps AMD's MIT notice).
