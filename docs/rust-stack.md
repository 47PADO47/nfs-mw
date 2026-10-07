# Rust stack

Which crates the rewrite uses or plans to use, and why. Versions and licenses were checked on crates.io on
2026-10-07. License policy: [licensing.md](licensing.md); enforced by [`deny.toml`](../deny.toml).

## In use (milestone 1)

| Crate | Version | License | Used for |
|---|---|---|---|
| wgpu | 30 | MIT OR Apache-2.0 | Rendering on Vulkan, Direct3D 12, OpenGL (and Metal). Uploads DXT directly with `TEXTURE_COMPRESSION_BC` |
| winit | 0.30 | Apache-2.0 | Windows, input, event loop on Windows/X11/Wayland |
| glam | 0.34 | MIT OR Apache-2.0 | Math. Since 0.34 the projection and view matrices live in `glam::camera` (we use `rh::proj::directx`, which matches wgpu's clip space) |
| bytemuck | 1 | Zlib OR Apache-2.0 OR MIT | Vertex structs to GPU buffers |
| texture2ddecoder | 0.1 | MIT OR Apache-2.0 | CPU DXT decode when a texture can't be uploaded as BC |
| winreg | 0.56 | MIT | Windows registry lookup of the install |
| directories | 6 | MIT OR Apache-2.0 | Per-user config path |
| serde, toml | 1, 0.9 | MIT OR Apache-2.0 | Config file |
| sha2 | 0.10 | MIT OR Apache-2.0 | Identifying `speed.exe` |
| clap | 4 | MIT OR Apache-2.0 | Command line |
| png | 0.18 | MIT OR Apache-2.0 | `--screenshot` |
| thiserror, anyhow, log, env_logger, pollster | — | MIT OR Apache-2.0 | Errors, logging, blocking on wgpu futures |

Written in-house because no crate exists: **JDLZ** and **HUFF** ([`nfsmw-compress`](../crates/nfsmw-compress)),
**bChunk** parsing, the **TPK** and **solid** readers.

## Planned

| Need | Choice | License | Notes |
|---|---|---|---|
| Memory-map the 533 MB `STREAML2RA.BUN` | memmap2 | MIT OR Apache-2.0 | Needs `unsafe`; confined to `nfsmw-install` |
| EAGL4 animation ELF objects | object | Apache-2.0 OR MIT | ELF32 sections, symbols, relocations |
| Collision queries (wheel raycasts, trimesh) | parry3d | Apache-2.0 | parry 0.31 pins glam 0.33 through glamx; align glam versions when adding it |
| Audio output and mixing | kira (on cpal) | MIT OR Apache-2.0 | Its `Sound` trait fits a custom granular engine-sound synth |
| EA-XA, EA-XAS, MicroTalk decoders | port from vgmstream | ISC-style | Keep vgmstream's notice in each ported file |
| VP6 video | nihav-vp6 (git, ruffle-rs) | MIT | Plus our own MVhd/MV0K/MV0F demuxer |
| Debug UI | egui | MIT OR Apache-2.0 | |
| Third-party notices | cargo-about | MIT OR Apache-2.0 | Generated from `Cargo.lock` for releases |

## Rejected

| Option | Why not |
|---|---|
| Bevy | Too opinionated for a faithful port of an engine with its own loop and streaming; breaking changes every release. iw4L uses only its ECS. |
| FFmpeg bindings (VP6, EA audio) | LGPL; heavy runtime DLLs. nihav-vp6 + our own demuxer cover MW's videos. |
| NihAV upstream | AGPL-3.0. Only the MIT VP6 subset (relicensed for Ruffle) is usable. |
| vgmstream through FFI | Its EALayer3 path needs mpg123 (LGPL); MW doesn't use EALayer3 anyway, and the decoders we need are small to port. |
| Unlicensed Rust EA-format code (EA-Playground, vgmstream-rs) | No license: reference only. |
| rapier3d | MW's vehicle physics is custom; at most useful for debris later. |
