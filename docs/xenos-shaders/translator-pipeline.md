# Translator pipeline: from disc file to packed library

How the fork turns the game's Xbox 360 shader containers into a SPIR-V library, step by step, with the tool and the
gate of each step. It is the fork's design, as read from its source. Nothing here was run.
See [README.md](README.md) for the overview and [container-format.md](container-format.md) for the file layout.

## The steps

```
disc files (.BIN, .xex, .exe)
   │  1. nfsmw_buscar_contenedores      candidates + procedencia.tsv (no validation)
   ▼
containers (<v|p>_NNNNNN.bin)
   │  2. nfsmw_hlsl: Convertir2005      2005 layout -> 2008 layout, control flow checked
   │  3. ShaderRecompiler::recompile    microcode -> HLSL, one file per container
   ▼
HLSL (.hlsl)
   │  4. sed / python rewrites          shadow fetches, radial blur, marker constant
   │  5. dxc -spirv                      ps_6_6 or vs_6_6, Vulkan 1.2
   │  6. spirv-val                      --scalar-block-layout
   ▼
SPIR-V (.spv)
   │  7. nfsmw_empaquetar               library keyed by each container's bytes
   │  8. nfsmw_probar_biblioteca        every container must be found again
   ▼
nfsmw_shaders.nfsp  (library, loaded by the renderer at start-up)
```

## Step by step

### 1. Find the containers

`nfsmw_buscar_contenedores.cpp` walks a folder and reads each `.BIN`, `.xex` and `.exe` file. It looks for the
byte pattern `10 2A 0E 0x` (the 2005 signature, see [container-format.md](container-format.md)), then checks four
header fields (sizes, offsets) against the file. Each hit is written out as a candidate
`<p|v>_NNNNNN.bin`, where `p` is a pixel shader and `v` a vertex shader. Its source file, offset and sizes go to
`procedencia.tsv`.

**Gate:** none beyond the header sanity check. The file says so: "a signature and dimension match is only a
candidate, not a validation of the microcode". The candidates are checked later, in step 2.

### 2. Convert the 2005 layout

`nfsmw_hlsl.cpp` accepts both signatures, `0x102A1100` (XenosRecomp's, 2008) and `0x102A0E00` (2005). A 2005
container goes through `nfsmw::Convertir2005` in `nfsmw_contenedor.h`, which:

- checks every table offset against the virtual part, and every name for a terminator;
- validates the control flow, see [container-format.md](container-format.md#microcode-and-control-flow);
- copies the microcode unchanged and rebuilds the header and tables in the 2008 layout, shifted by 12 bytes;
- rebuilds the immediate constants (`c` registers) and rejects masked definitions it does not implement.

**Gate:** a container that fails any check is skipped and reported by name. The 2005 corpus is rejected, not
guessed at.

### 3. Translate to HLSL

`ShaderRecompiler::recompile` (in `XenosRecomp/shader_recompiler.cpp`) emits one HLSL file per container. The
output includes `shader_common.h`. The translator's own README describes the control-flow scheme: a `while` loop
over a `switch` on a local `pc`, flattened when the flow is simple, so DXC can optimize it. The same README warns
that dynamic register indexing is not implemented, that `INF` and `NaN` results are not handled exactly, and
that the control-flow path was tested on very few shaders. Those three gaps still apply to this fork.

**Gate:** an empty output is a failure. `nfsmw_hlsl` counts `ok` against `total` and returns non-zero if any
container was skipped.

### 4. Rewrite the shadow and blur paths

The bash script `nfsmw_regenerar_biblioteca_pcf.sh` changes three things in the HLSL before compiling. Each one
is an optional path behind a specialization bit, see [feature-bits.md](feature-bits.md):

| Rewrite | What it changes | Gate |
|---|---|---|
| Shadow fetch | Every `tfetch2D(SHADOWMAP_SAMPLER_…` becomes `tfetch2DSombra(…)` (cheap PCF) | The count must be non-zero, or the script stops |
| Shadow minimum | `tfetch2DSombra(…)` becomes `tfetch2DSombraMin(…)` with the second texture index taken from the same register | Every call must have the expected shape and the `#define` must be present, or the script stops |
| Radial blur | In one composition shader (`p_000139`), the blend `r5.xyz = r5.xyz * r0.xxx + r3.xyz;` is wrapped in the `SIN_DESENFOQUE` bit | The line must appear exactly once |

The rewrites match exact strings. That is fast and easy to read, but fragile: a translator change that alters
the text fails the gate, which is the intent, and the file name `p_000139` is hard-coded.

### 5. Compile with DXC

```text
dxc -spirv -T ps_6_6|vs_6_6 -E main -HV 2021 -fspv-target-env=vulkan1.2 -fvk-use-dx-layout -Werror=parameter-usage [-fvk-invert-y] ...
```

Pixel shaders are `ps_6_6`. Vertex shaders are `vs_6_6` and also get `-fvk-invert-y`, which flips Y for Vulkan's
clip space. `-Werror=parameter-usage` turns an unused-parameter warning into a failure.

**Gate:** any DXC error stops the run and names the container.

### 6. Validate the SPIR-V

`spirv-val --target-env vulkan1.2 --scalar-block-layout` checks each output.

**Gate:** an invalid module stops the run.

### 7. Pack the library

`nfsmw_empaquetar.cpp` reads each original container and its SPIR-V, checks the SPIR-V length is a multiple of
4, and packs them with `EmpaquetarShaders`. The library keys each shader by the bytes of the original container,
so the renderer looks a shader up by the bytes of the container it has. The output path must not exist, so a run never
overwrites a previous library.

**Gate:** if a container is missing its `.spv`, or does not match its shader after packing, the run fails. The
tool loads the packed library again and checks that every container can be found in it.

### 8. Test the library

`nfsmw_probar_biblioteca.cpp` tests a built library. Its checks are in the source; they were not read here line by
line.

## Regression checks for the container reader

`nfsmw_probar_contenedor.cpp` runs on a folder of real containers and on a folder of known-bad copies. For each
container it:

- normalizes it and confirms the microcode bytes are unchanged;
- decodes every control-flow word and records the opcodes seen;
- checks the pixel shader's colour-output mask against the registers the microcode exports;
- checks that bit 5 of the output mask is set exactly when the microcode contains a kill (discard) instruction;
- rejects every truncation of its 24-byte header, an overflowing table offset (header words 4, 16 and 20), and a
  zeroed microcode.

It also checks that every known-bad copy in the second folder is rejected. For each one it reports whether an
equivalent constant table exists in the corpus; the ones without one are listed, not failed. The test contains no
game data. Its corpus is passed in on the command line.

**Our take:** the approach fits any reader of a binary format, and we have nothing like it for the formats in
[docs/formats/](../formats/). See idea 4 in [README.md](README.md#ideas-worth-taking).

## Build notes

- The bash script builds with MinGW `g++ -std=c++23` and the flags `-DNFSMW_RECOMP -DXXH_INLINE_ALL
  -DFMT_HEADER_ONLY`. It needs `MESA` set to a source tree (`src/util/xxhash.h`) and `SDK` for the `fmt` headers.
  It is not runnable from this repository, which has none of those trees.
- `nfsmw_hlsl.cpp` explains why it does not use `XenosRecomp/main.cpp`: that file also needs smol-v and zstd
  submodules that are not checked out, and the DXC API. The fork uses the `dxc.exe` command line from the Vulkan
  SDK instead.
- The fork is written in Spanish in its comments and error messages. The English here is a translation.
