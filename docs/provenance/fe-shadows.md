# Provenance: front-end shadows

Spec: [docs/specs/fe-shadows.md](../specs/fe-shadows.md). Program: `/speed.exe` (image base `0x00400000`,
26,084 functions). Names below are Ghidra's; they belong here only, never in the spec.

Reading notes: decompile text was read through the Ghidra tools and not saved. Constants were decoded by hand from `read_memory`
bytes and cross-checked against the decompile (for example `5.960465e-08` is `2^-24`). Decompiled code and
disassembly are not in the repo.

## 1. Addresses

### 1.1 Globals

| Address | Ghidra name | Meaning used in the spec | How found |
|---|---|---|---|
| `0x00925e90` | `DAT_00925e90` | Game flow state: 3 = front-end session, 6 = driving (world) | 101 reads and writes (`search_instructions` on `925e90`). Writers: `FUN_006617e0` (7), `FUN_0064b6d0` (3, no direct caller), `FUN_00664250` (8), `FUN_00664b80` (1), `FUN_00666d00` (5, no caller), `FUN_00666de0` and `FUN_00667010` (from a register), unnamed code at `0x006627e7` (6). |
| `0x00901830` | `lpData_00901830` | Registry value `g_ShadowDetail`; shadow pass gate and level | Registry names in `FUN_006c1c00` (read) and `FUN_006c1f10` (write). |
| `0x008faed0` | `DAT_008faed0` | Byte gate before the shadow matrix builder | Static bytes `01 00 00 00` (read_memory). One read, at `0x006e5521` in `FUN_006e54e0`. No writer in the xref index. |
| `0x008faecc` | `DAT_008faecc` | Selects shadow draw flags | Reads at `0x006e515b` and `0x006e5341` in `FUN_006e44c0`. |
| `0x008fae50` | float | Gate value for the shadow pass in `FUN_006de300` (first of a pair) | Static `0.1` (`cdcccc3d`). |
| `0x008fae68` | float | Second of the pair | Static `1.0`. Pair decoded from `read_memory` at `0x008fae50`, 32 bytes. |
| `0x008faeb8` | float | Scale of the light offset in `FUN_006e44c0` | Static `2000.0` (`0000fa44`). Three reads, all in `FUN_006e44c0`. |
| `0x009b392c + 0x180` | object vector | Sun or light direction for the world shadow frame (normalised by `FUN_0045f490`) | Read in `FUN_006e44c0`. Who fills it in the front end is not read. |
| `0x0091cb10` | `DAT_0091cb10` | UI or front-end object (0x148 bytes, made by `FUN_00525df0` through `FUN_00517520`). Its `+0x11c` field is a mode. | Readers: `FUN_00517a20`, `FUN_00517070`, `FUN_00599db0`, `FUN_005642c0`, `FUN_00525b20`, `FUN_00525df0` (write of the pointer). |
| `0x009018b8` | `DAT_009018b8` | Upper clamp for `g_ShadowDetail` | Static `0`. Writers: `FUN_006d2100` at `0x006d24b8`, `0x006d24c7`, `0x006d24d5`. Read in `FUN_006c1780` at `0x006c1942`. |
| `0x00982c00` | `DAT_00982c00` | Registry `g_PerformanceLevel` | Read in `FUN_006c1990`. |
| `0x00901ab8` ... `0x00901ad0` | floats and ints | `0x901ab8` = `1.0`; `0x901abc` = `4.0`; `0x901ac0` = int `4` (depth bias count); `0x901ac4` = `0.1` | Decoded. The bias count times `2^-24` is the depth bias written in `FUN_006e54e0`. |
| `0x00901a84` ... `0x00901a90` | floats | Orthographic box for `D3DXMatrixOrthoLH`: `2.0`, `1.0`, `0.5`, `2.5` | Decoded. Used in `FUN_006e44c0`. |
| `0x00901a94` ... `0x00901a9c` | floats | Perspective scale factors: `1.5`, `10.0`, `10.0` | Decoded. |
| `0x00901aa0` ... `0x00901aac` | floats | `20.0`, `0.5`, `1.0`, `0.01` (near) | Decoded. |
| `0x00901ab0` | float | `50.0`, the far value in `far / (far - near)` | Decoded. |
| `0x009017f4` | `lpData_009017f4` | Registry `g_WorldLodLevel`; read by `FUN_006e7220` | Registry names in `FUN_006c1c00`. |
| `0x008ec2b8` | `DAT_008ec2b8` | Set to 1 when `g_ShadowDetail` is 0 | Written in `FUN_006de300`. |
| `0x00903320`, `0x00903328` | flags | Read in `FUN_006e44c0` and in `FUN_0074e160` (car draw); set in `FUN_006e44c0` | Not decoded further. |

Other registry names read in the same functions, for the spec's settings table: `g_CarEnvironmentMapEnable`
(`0x009017b8`), `g_RoadReflectionEnable` (`0x009017d4`), `g_MotionBlurEnable` (`0x009017dc`),
`g_ParticleSystemEnable` (`0x009017ec`), `g_CarLodLevel` (`0x009017f8`), `g_OverBrightEnable` (`0x009017fc`),
`g_FSAALevel` (`0x00901808`), `g_RainEnable` (`0x00901810`), `g_TextureFiltering` (`0x00901818`),
`g_RacingResolution` (`0x0090181c`), `g_VSyncOn` (`0x00901824`), `g_VisualTreatment` (`0x00901828`).

### 1.2 Functions

| Address | Ghidra name | Size | Role |
|---|---|---|---|
| `0x006de300` | `FUN_006de300` | 5580 | World scene draw. Calls the shadow driver inside the block that tests `g_ShadowDetail` and the float pair. Also has a `DAT_00925e90 == 3` branch (car block) and a `== 6` branch (world). |
| `0x006e7220` | `FUN_006e7220` | 1108 | Per-frame render. Calls `FUN_006de300` only when `FUN_00517a20()` is 0. Callers: `0x0064a7c0`, `0x006617e0`, `0x00663c90`, `0x00664780`. |
| `0x00517a20` | `FUN_00517a20` | 39 | Mode predicate: returns 1 when `DAT_0091cb10 + 0x11c` is 3, 4 or 5. |
| `0x006e54e0` | `FUN_006e54e0` | 170 | Shadow pass driver. Sets depth bias (render state 195) to `4 * 2^-24`, resets slope bias (175) and Z-enable is restored (7). Calls `FUN_006e44c0` when `DAT_008faed0` is non-zero. |
| `0x006e44c0` | `FUN_006e44c0` | 4124 | Shadow matrix builder and caster draw. Uses `FUN_00723fa0` with flags `0x100 | DAT_00982b9c` (when `DAT_008faecc` is 0), `0x2000` (when `DAT_008faecc` is set and `g_ShadowDetail` is 1) or `0x200`. Second loop: objects of list `DAT_009b37cc` with a count above 1, drawn through vtable slot `+0xc`. Two projection blocks of 54 floats each, at `0x00986df0` and `0x00987ce0`. |
| `0x00723fa0` | `FUN_00723fa0` | - | Scene traversal with draw-filter flags. Not read. |
| `0x006d6aa0` | `FUN_006d6aa0` | - | Per-draw method of the shader-parameter class (receiver side). Reads `g_ShadowDetail` at `0x006d6b38`. The class is described in [docs/reversing.md](../reversing.md) (shader-parameter section). |
| `0x006d71f0` | `FUN_006d71f0` | - | Per-frame setup: fade and matrix constants from the pass block at `DAT_00982a20`. Reads `g_ShadowDetail` at `0x006d7383`. |
| `0x006e6e40` | `FUN_006e6e40` | 989 | Called before and after the world draw. With argument 0 it has a front-end-only block: gate `FUN_006e6380(...) == 0x188` and `DAT_00919738`. Constants `450.0`, `19.0`, `190.0`, `89.0` are stored, then `FUN_006c0a30` initialises a 0x24-word block. Not yet mapped to a light. |
| `0x00663d30` | `FUN_00663d30` | 455 | Frame dispatch. Calls `FUN_0064a7c0` (which calls `FUN_006e7220`) when `DAT_00925854 == 0`. |
| `0x006c1990` | `FUN_006c1990` | 451 | Defaults for registry globals. `g_ShadowDetail` = `1` only when `g_PerformanceLevel > 3`. |
| `0x006c1780` | `FUN_006c1780` | 511 | Clamps each registry global to its cap (`DAT_0090xxxx`). |
| `0x006c1c00` | `FUN_006c1c00` | 770 | Reads the registry block (`Software\EA Games\Need for Speed Most Wanted`). |
| `0x006c1f10` | `FUN_006c1f10` | 546 | Writes the registry block. |
| `0x00517070` | `FUN_00517070` | 145 | Jukebox FNG load. Runs only when the mode is not 3, 4 or 5, and `DAT_00925e90 == 3`. |
| `0x00599db0` | `FUN_00599db0` | 683 | Runs only at mode 5. Builds a 320 by 240 view (constants `0x43a00000`, `0x43700000`). |
| `0x0045f490` | `FUN_0045f490` | 117 | Normalises a 3-vector. |
| `0x006d2100` | `FUN_006d2100` | - | Writes the cap `DAT_009018b8`. Not read. |
| `0x006617e0` | `FUN_006617e0` | 455 | Writes state 7, then calls `FUN_006e7220`. |
| `0x00662b80` | `FUN_00662b80` | 126 | Stage loader named in [menu-backdrop](../specs/menu-backdrop.md). Close to the unnamed code that writes state 6 at `0x006627e7`; not confirmed to be the same function. |

## 2. Verification facts from the GPL mod

The mod's addresses were used only to check our own readings (the rule is behaviour only). They match:

- `0x00925e90` compared with 3 for the front end. Our state global is the same one (section 1.1).
- `0x00901830` as the shadows-enabled flag. Our `g_ShadowDetail` is the same global.
- `0x00982a20`, `0x00982c80`, `0x009195e4` and `0x00919c70` are used by our decompiles in the same roles (pass block, bound object, slot state, slot array).
- A hook at `0x006e5186` sits inside `FUN_006e44c0`, 0x34 bytes after the `g_ShadowDetail` read at `0x006e5152`. A hook at `0x006e551b` sits inside `FUN_006e54e0`, next to the `DAT_008faed0` gate at `0x006e5521`.
- Not checked: the mod's `ShadowResDivider` and `ShadowBiasMultiplier` values, its object tags `0x2EE02501` and `0x1BF93EF9`, and the other caves. These are not used in the spec.

## 3. Lessons

- A global read in 100 places: `search_instructions` with `operand_pattern "925e90"` gave every reader with its
  compare constant in one call. Paging the xref list was slower. The pattern must not include `0x`, because the
  disassembly prints `[0x00925e90]`.
- A gate with a static non-zero value and no writer is always open. `DAT_008faed0` is `1` in the file and has no
  writer, so the shadow driver is not disabled by any runtime flag. Check the bytes first, then the writers.
- Registry-backed globals name their own settings. The `RegQueryValueExA` calls in `FUN_006c1c00` give the
  setting names directly, which is faster than a string search for `shadow`.
- Render-state numbers show up as small integers in device vtable calls. Here `195` and `175` are the depth and
  slope bias states, and `7` is Z-enable (D3D9 enum values, from the SDK, not from the binary).
- The decompiler drops trailing arguments of vtable calls, for example the `0xc3` and `0xaf` calls in
  `FUN_006e54e0` with only two or three values shown. Read the disassembly before relying on a value.
- Float decode by hand worked for the small table, but use a script for any large block. The check was the
  decompile constant `5.960465e-08`.
- `get_functions` by address failed for `0x006627e7`: no function contains it in Ghidra's database, although the
  xref names one. Treat the owner as unknown.
- A single function can serve as the render entry and the front-end gate. `FUN_006e7220` is that case.
- The mode names 3, 4 and 5 are read from the code shape (the jukebox check, the 320 by 240 view). They are not
  named in the binary.

## 4. Dead ends

- Searching the strings for `shadow` (33 hits) finds no front-end shadow string. The shadow code uses shared
  effect names: `shadow_map_mesh`, `car_shadow_map`, `shadow_cg`, `shadow_map_mesh_depth`.
- Searching for the immediate bytes of the object tags `0x2EE02501` and `0x1BF93EF9` returned no match. Not
  pursued, since these come only from the GPL mod.
- `FUN_006c0a30` initialises a 0x24-word state (a light or fog set). It is not a light direction.
- `FUN_006e44c0` has no shadow map size constant. The map size is set elsewhere.
- The front-end-only block in `FUN_006e6e40` uses constants `450.0`, `19.0`, `190.0` and `89.0`. They are not yet
  mapped to any light.
- `FUN_00517070` does not run in menu modes 3 to 5, so the jukebox does not show a shadow path either way.

## 5. Closed leads

- The shadow pass has one call site, `FUN_006de300` to `FUN_006e54e0`. The driver has one caller. Closed.
- The `DAT_008faed0` gate is always open (static `1`, no writer). Closed.
- The defaults for `g_ShadowDetail` come from `FUN_006c1990`. Closed.
- The front end skips the world draw in modes 3 to 5 (`FUN_006e7220` and `FUN_00517a20`). Closed for the world path.

## 6. Open items (in the spec's Next leads)

- Name modes 3, 4 and 5 of `DAT_0091cb10 + 0x11c`, and find the writer of `+0x11c`.
- Find the path that draws the front-end stage while menus are up. `FUN_006e6e40` (argument 0) and `FUN_0064a7c0`
  are the candidates.
- Find the shadow map size: the render-target creation near the shadow matrix code, or the `DAT_00982bdc` device
  calls in `FUN_006e54e0`.
- Decode the draw flags `0x100`, `0x200`, `0x1000`, `0x2000` and `DAT_00982b9c` in `FUN_00723fa0`.
- Identify the object at `0x009b392c + 0x180` and whether the front end sets it.
- Read `FUN_006d2100` to learn how the `g_ShadowDetail` cap is set at start-up.
- Map the front-end light block in `FUN_006e6e40` (`450`, `19`, `190`, `89`).
