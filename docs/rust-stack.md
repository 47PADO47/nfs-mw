# Rust stack

Which crates the rewrite uses or plans to use, and why. Versions and licenses were checked on crates.io on
2026-10-07. License policy: [licensing.md](licensing.md); enforced by [`deny.toml`](../deny.toml).

## In use (milestone 1)

| Crate | Version | License | Used for |
|---|---|---|---|
| wgpu | 30 | MIT OR Apache-2.0 | Rendering on Vulkan, Direct3D 12, OpenGL (and Metal). Uploads DXT directly with `TEXTURE_COMPRESSION_BC` |
| bevy_app, bevy_ecs, bevy_input, bevy_time, bevy_window, bevy_winit, bevy_a11y, bevy_gilrs | =0.20.0 | MIT OR Apache-2.0 | The application shell: schedule, ECS, window and event loop (`bevy_winit` wraps winit 0.30), keyboard, mouse and gamepad input (gilrs 0.11). No `bevy_render`. See [decisions/0001-bevy.md](decisions/0001-bevy.md) |
| glam | 0.34 | MIT OR Apache-2.0 | Math. Since 0.34 the projection and view matrices live in `glam::camera` (we use `rh::proj::directx`, which matches wgpu's clip space) |
| bytemuck | 1 | Zlib OR Apache-2.0 OR MIT | Vertex structs to GPU buffers |
| texture2ddecoder | 0.1 | MIT OR Apache-2.0 | CPU DXT decode when a texture can't be uploaded as BC |
| winreg | 0.56 | MIT | Windows registry lookup of the install |
| directories | 6 | MIT OR Apache-2.0 | Per-user config path |
| serde, toml | 1, 0.9 | MIT OR Apache-2.0 | Config file |
| sha2 | 0.10 | MIT OR Apache-2.0 | Identifying `speed.exe` |
| clap | 4 | MIT OR Apache-2.0 | Command line |
| png | 0.18 | MIT OR Apache-2.0 | `--screenshot` |
| egui | 0.36 | MIT OR Apache-2.0 | Developer UI (metrics overlay, console). Only the context, layout and tessellation are used; `gui/` hands its output to `blackbox-render`'s UI layer. Its bundled fonts (Hack, Ubuntu-Light, Noto Emoji, emoji-icon-font) are OFL-1.1 and Ubuntu Font Licence: `deny.toml` has an exception and `NOTICE` lists them |
| thiserror, anyhow, log, env_logger, pollster | — | MIT OR Apache-2.0 | Errors, logging, blocking on wgpu futures |
| nihav_core, nihav_duck (+ nihav_codec_support) | 0.1.0, git `ruffle-rs/nihav-vp6` @ `12d0bdf` | MIT (repo `COPYING`; no `license` field in the manifests) | VP6 video decoding in [`blackbox-movie`](../libs/blackbox-movie), behind its default `vp6` feature. `deny.toml` allows the git source and clarifies the license |

Written in-house because no crate exists: **JDLZ** and **HUFF** ([`ea-compress`](../libs/ea-compress)),
**bChunk** parsing, the **TPK** and **solid** readers, and the **movie container** demuxer and timeline ([`blackbox-movie`](../libs/blackbox-movie)).

## Planned

| Need | Choice | License | Notes |
|---|---|---|---|
| Memory-map the 533 MB `STREAML2RA.BUN` | memmap2 | MIT OR Apache-2.0 | Needs `unsafe`; confined to `game-install` |
| EAGL4 animation ELF objects | object | Apache-2.0 OR MIT | ELF32 sections, symbols, relocations |
| Collision queries (wheel raycasts, trimesh) | parry3d | Apache-2.0 | parry 0.31 pins glam 0.33 through glamx; align glam versions when adding it |
| Audio output and mixing | kira (on cpal) | MIT OR Apache-2.0 | Its `Sound` trait fits a custom granular engine-sound synth |
| EA-XA, EA-XAS, MicroTalk decoders | port from vgmstream | ISC-style | Keep vgmstream's notice in each ported file |
| Third-party notices | cargo-about | MIT OR Apache-2.0 | Generated from `Cargo.lock` for releases |

## Rejected

| Option | Why not |
|---|---|
| Bevy's renderer | Would replace the working reverse-Z, pre-lit, instanced renderer. The ECS, input and app crates are proposed separately in [decisions/0001-bevy.md](decisions/0001-bevy.md). |
| FFmpeg bindings (VP6, EA audio) | LGPL; heavy runtime DLLs. nihav-vp6 + our own demuxer cover MW's videos. |
| NihAV upstream | AGPL-3.0. Only the MIT VP6 subset (relicensed for Ruffle) is usable. |
| vgmstream through FFI | Its EALayer3 path needs mpg123 (LGPL); MW doesn't use EALayer3 anyway, and the decoders we need are small to port. |
| Unlicensed Rust EA-format code (EA-Playground, vgmstream-rs) | No license: reference only. |
| rapier3d | MW's vehicle physics is custom; at most useful for debris later. |
