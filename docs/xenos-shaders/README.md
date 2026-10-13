# Xbox 360 shader translator (`codepdbh/nfsmw-android/shaders`): survey and ideas

A survey of the `shaders/` folder of [codepdbh/nfsmw-android](https://github.com/codepdbh/nfsmw-android), read for
what this rewrite could reuse. The folder translates the Xbox 360 (Xenos) shader microcode of NFS: Most Wanted
into HLSL, compiles it to SPIR-V with DXC, and packs the result into a library for the port's Vulkan renderer. The
repository is a port of the Xbox 360 build, not of the PC build we are rewriting. Snapshot read on 2026-10-14
(`main`, last pushed 2026-10-12).

It is a companion to [ReShade shader pack: survey and ideas](../reshade-shaders/README.md), which surveys a
different source with the same question.

Pages in this folder:

| Page | What it covers |
|---|---|
| [translator-pipeline.md](translator-pipeline.md) | Each step from disc file to packed library, the tools, and the regression checks |
| [container-format.md](container-format.md) | The 2005 Xbox 360 shader container as the fork reads it: header, tables, control flow |
| [feature-bits.md](feature-bits.md) | The specialization-constant bits, the shadow-minimum technique, and the lessons from them |

## Bottom line

- **Nothing here can be ported as code.** The renderer reads Xenos microcode, and our PC game does not use it.
  The PC shaders are D3DX `fx_2_0` effects embedded in `speed.exe`
  ([formats/shaders.md](../formats/shaders.md)), and our renderer writes its own WGSL. The Xenos decoder, the
  container reader and the DXC/SPIR-V steps have no target in this repository.
- **Some design ideas are worth taking.** The best one is the shadow-map minimum: the car shadows are drawn into
  their own map and merged with `min()` at sample time. It is not in our renderer yet, so it is a design input,
  not a fix. The other ideas are ranked below.
- **Format facts are usable with credit.** The layout of the 2005 container and the control-flow rules are facts
  about a file format. Under [licensing.md](../licensing.md) facts can come from any source, so they can be
  recorded with credit. The write-up is in [container-format.md](container-format.md). It is not verified
  against an install here, so it is tagged **[community]**.

## Licences

The folder has two licence sources, and they do not agree with the repository's root licence:

| File | Licence | Source |
|---|---|---|
| `shaders/LICENSE.md` | MIT, "Copyright (c) 2025 hedge-dev and contributors" | Sits beside the XenosRecomp copy and `shader_common.h` |
| Repository root (GitHub's detection) | GPL-3.0 | The licence of the fork as a whole |
| `shaders/XenosRecomp/` | Upstream [hedge-dev/XenosRecomp](https://github.com/hedge-dev/XenosRecomp), which the README says is MIT | Its own header, read as MIT-licensed upstream |

The MIT notice does not say which files it covers, and the fork's own files (`nfsmw_*.cpp`, the scripts) carry no
per-file headers. Read the XenosRecomp files as MIT and the fork's own files as GPL-3.0. Our
[research.md](../research.md) row lists the repository as GPL-3.0. That is right for the repository as a whole,
but it hides the MIT part.

**Consequence for this pass:** no code was copied. The ideas are described in our own words and each one needs a
spec before implementation ([licensing.md, Spec-first](../licensing.md#spec-first)). If a later change ports MIT
code from XenosRecomp, the file keeps its notice and [NOTICE](../../NOTICE) lists it, per the rules in
[licensing.md](../licensing.md). NOTICE already credits the fork as read-only, and now credits hedge-dev's
XenosRecomp as well.

## What is in the folder

| Path | Contents |
|---|---|
| `XenosRecomp/` | Upstream translator, with the fork's changes behind the `NFSMW_RECOMP` macro (about ten guarded blocks in `shader_recompiler.cpp`): `main.cpp`, `shader_recompiler.{h,cpp}`, `shader_code.h`, `shader.h`, `constant_table.h`, `dxc_compiler.{h,cpp}`, `pch*.h`, `CMakeLists.txt`, and its own `shader_common.h` |
| `shader_common.h` | The HLSL helpers that every translated shader includes (copy of the one in `XenosRecomp/`). Spec-constant bits, texture fetch wrappers, shadow fetches |
| `nfsmw_contenedor.h` | The 2005 container adapter: validation and normalization into the layout XenosRecomp reads |
| `nfsmw_buscar_contenedores.cpp` | Finds container candidates in disc files |
| `nfsmw_hlsl.cpp` | Translates every container of a folder to HLSL |
| `nfsmw_lzx.cpp` | LZX decompression of the Xbox executable with libmspack (Xbox-only) |
| `nfsmw_empaquetar.cpp` | Packs the SPIR-V into the library, keyed by each container's bytes |
| `nfsmw_probar_contenedor.cpp`, `nfsmw_probar_biblioteca.cpp` | Regression checks of the container reader and of the built library |
| `nfsmw_regenerar_biblioteca_pcf.sh` | The whole pipeline on a PC, bash, with the rewrites and DXC calls |
| `wasm/` | WebAssembly build of DXC for the browser installer (`build_wasm_tools.bat`, `link_dxc_wasm.bat`, `dxc_web.cpp`) |
| `README.md`, `README.upstream.md`, `LICENSE.md`, `CMakeLists.txt` | Documentation and build files |

## Ideas worth taking

Ranked by value to this renderer. "Already here?" comes from grepping `libs/` and `crates/` and the docs on
2026-10-14.

| # | Idea | Where in the fork | Already here? | Next step |
|---|---|---|---|---|
| 1 | **Shadow-map minimum.** Draw the cars into their own map cleared to 1.0. Shaders that sample the map with cars take `min(world, cars)` at sample time, instead of resolving the world map a second time. The fork says this matches drawing the cars on top with the game's LESS/LEQUAL depth test | `shader_common.h` (`SPEC_CONSTANT_SOMBRA_MINIMO`, `tfetch2DSombraMin`) | **No.** There is no real-time shadow-map pass. The only "shadow" hits are material flags and a baked-occlusion comment | Check on the PC install whether the car shadow effects (`IDI_SHADOW_MAP_MESH_FX`, `IDI_CAR_SHADOW_MAP_FX`, see [formats/shaders.md](../formats/shaders.md)) do the same. Then write a spec before any shadow pass lands |
| 2 | **Cheap shadow filter as a quality tier.** One bit collapses the 3x3 PCF into a single tap. The fork reports that one draw was 21 % of the scene's time on Xbox | `SPEC_CONSTANT_PCF_BARATO` | No shadow filter exists to make cheaper | Spec a shadow-filter setting alongside idea 1, with a row in [low-end.md](../low-end.md) |
| 3 | **Specialize passes away, not just factors.** The radial blur is removed by cutting its blend, not by setting its factor to 0 (see [feature-bits.md](feature-bits.md)) | `SPEC_CONSTANT_SIN_DESENFOQUE`, `nfsmw_regenerar_biblioteca_pcf.sh` | Partly. `scene.wgsl` already uses WGSL `override` constants (`PRELIT`, `FOG`) | Keep the rule in mind for any `override` that switches off a multi-tap pass. Test it with naga before relying on it |
| 4 | **Reject by default, and test the rejection.** The container reader refuses truncated headers, out-of-range offsets, branches that leave the microcode, and empty translations, and a test feeds it damaged copies of real containers | `nfsmw_contenedor.h`, `nfsmw_probar_contenedor.cpp` | Not checked here. Format readers exist in `libs/blackbox-*` | Check whether our readers have truncation and corruption tests. Add them where they are missing |
| 5 | **A provenance index for every extracted candidate.** `procedencia.tsv` records the source file, the exact offset and the sizes of each candidate | `nfsmw_buscar_contenedores.cpp` | Partly: [provenance/](../provenance/README.md) records sources for behaviour, not for byte-level extraction | Use the same columns whenever we extract blobs from the install |
| 6 | **Key derived data by a fingerprint of its source.** The library is keyed by the container's bytes, and the installer checks a SHA-256 of the whole library before running it | `nfsmw_empaquetar.cpp`, `wasm/` installer notes | No cache of derived shader data is keyed this way | Consider it for any cache of compiled pipelines or converted assets |
| 7 | **Keep a fallback for a constant path.** A bit switches the constants between a uniform buffer and a push-constant pointer, with the same shader source | `shader_common.h` (`SPEC_CONSTANT_CONSTANTES_UBO`) | Not needed, since we use WGSL uniforms | Low priority. Only relevant to very old drivers |

### Not worth taking

- The Xenos microcode decoder and its HLSL generator: the target instruction set does not exist in our PC game.
- The container adapter as code: it is the 2005 Xbox layout. The facts are useful; the code is not.
- LZX executable decompression: the Xbox executable is compressed, the PC one is not.
- The DXC SPIR-V steps (`-fvk-invert-y`, `-fvk-use-dx-layout`, smol-v, zstd) and the wasm build: we target WGSL through wgpu, with no SPIR-V step.
- The installer: it targets a Switch NRO build, so nothing in it applies to us.

## Verification

- Read in full: `README.md`, `README.upstream.md`, `LICENSE.md`, `nfsmw_contenedor.h`, `nfsmw_hlsl.cpp`,
  `nfsmw_lzx.cpp`, `nfsmw_empaquetar.cpp`, `nfsmw_buscar_contenedores.cpp`, `nfsmw_probar_contenedor.cpp`,
  `nfsmw_regenerar_biblioteca_pcf.sh` (first 120 lines and the rewrite steps), `shader_common.h`
  (the spec-constant comments and the shadow helpers), and the header of `XenosRecomp/shader_recompiler.h`.
- Not read line by line: `XenosRecomp/shader_recompiler.cpp` (67 KB). Its role is taken from the README and the
  `NFSMW_RECOMP` guards. Also not read: `main.cpp`, `dxc_compiler.cpp`, `shader_code.h`, the `wasm/` sources, and
  the rest of the bash script after the shadow rewrites.
- Nothing was built or run. No game data was used, and no file from the folder was copied into the repository.
- Claims about the Xbox game (the 21 % figure, the two-resolve shadow map, the 160 size queries) are the fork's
  own statements. We have not checked them.

## Re-checking

```bash
git clone --depth 1 https://github.com/codepdbh/nfsmw-android <scratch>/nfsmw-android
```

The fork's folder is `shaders/`. Clone into a scratch folder outside the repository.
