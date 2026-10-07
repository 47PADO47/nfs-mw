# Shaders (PC, Direct3D 9)

The PC shaders are **not** separate files. They are compiled D3DX **effects** (`fx_2_0` binaries)
embedded in `speed.exe` as Win32 resources. Each material in a solid selects an effect by index
([models.md](models.md)). For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Where they are **[verified]**

- `speed.exe` contains **31 resources of type 10 (`RT_RCDATA`)**, language 1033, named `IDI_*_FX`.
  Every one starts with the D3DX effect magic `01 09 FF FE` (`0xFEFF0901`, `fx_2_0`).
- They lie in `.rsrc` at file offsets **0x50F3E0–0x57E608** (RVA 0x5C83E0…), 455,140 bytes in total.
- The exe contains the string `Microsoft (R) D3DX9 Shader Compiler 9.04.91.0000`, the compiler that
  produced them.
- Any resource editor (or a short PE-resource walker) can extract them byte-exact.

| Resource | File offset | Size | Resource | File offset | Size |
|---|---|---|---|---|---|
| `IDI_WORLD_FX` | 0x50F3E0 | 49,980 | `IDI_FILTER_FX` | 0x556A50 | 21,096 |
| `IDI_WORLDMIN_FX` | 0x51B720 | 708 | `IDI_OVERBRIGHT_FX` | 0x55BCB8 | 5,620 |
| `IDI_WORLDNOFOG_FX` | 0x51B9E8 | 10,068 | `IDI_SCREENFILTER_FX` | 0x55D2B0 | 15,864 |
| `IDI_FE_FX` | 0x51E140 | 10,192 | `IDI_RAIN_DROP_FX` | 0x5610A8 | 2,676 |
| `IDI_FE_MASK_FX` | 0x520910 | 9,716 | `IDI_RUNWAYLIGHT_FX` | 0x561B20 | 11,480 |
| `IDI_WORLDREFLECT_FX` | 0x522F08 | 57,784 | `IDI_VISUALTREATMENT_FX` | 0x5647F8 | 7,068 |
| `IDI_WORLDBONE_FX` | 0x5310C0 | 10,972 | `IDI_WORLDPRELIT_FX` | 0x566398 | 8,272 |
| `IDI_WORLDNORMALMAP_FX` | 0x533BA0 | 55,128 | `IDI_PARTICLES_FX` | 0x5683E8 | 15,524 |
| `IDI_CAR_FX` | 0x5412F8 | 49,748 | `IDI_SKYBOX_FX` | 0x56C090 | 14,724 |
| `IDI_GLOSSYWINDOW_FX` | 0x54D550 | 38,140 | `IDI_WORLDNORMALMAPNOFOG_FX` | 0x56FA18 | 5,480 |
| `IDI_TREE_FX` | 0x570F80 | 11,712 | `IDI_SHADOW_MAP_MESH_FX` | 0x573D40 | 1,468 |
| `IDI_SKYBOX_CG_FX` | 0x574300 | 7,980 | `IDI_SHADOW_CG_FX` | 0x576230 | 9,508 |
| `IDI_CAR_SHADOW_MAP_FX` | 0x578758 | 5,656 | `IDI_WORLDDEPTH_FX` | 0x579D70 | 3,528 |
| `IDI_WORLDNORMALMAPDEPTH_FX` | 0x57AB38 | 3,332 | `IDI_CARDEPTH_FX` | 0x57B840 | 3,344 |
| `IDI_GLOSSYWINDOWDEPTH_FX` | 0x57C550 | 3,332 | `IDI_TREEDEPTH_FX` | 0x57D258 | 3,528 |
| `IDI_SHADOW_MAP_MESH_DEPTH_FX` | 0x57E020 | 1,512 | | | |

## Contents **[verified]**

- **Shader models:** counted from the `CTAB` target strings inside the 31 effects: `vs_1_1` ×69,
  `vs_3_0` ×2, `ps_1_1` ×28, `ps_1_4` ×3, `ps_2_0` ×18, `ps_2_b` ×18, `ps_3_0` ×2. That is 71 vertex and
  69 pixel shaders, so most effects ship SM1.x and SM2.0/2.b variants side by side (quality levels).
- **Parameter names** are kept, e.g. `DiffuseColour`, `SpecularColour`, `SpecularPower`,
  `AmbientColour`, `diffusemap`, `Cull_Mode`, `BlendState`, `BaseMinTextureFilter`, `LUMINANCE_VECTOR`,
  `FilterWeights`, `gainmap`/`rampmap` (screen filter), `displacemap`/`raindropoffset` (rain),
  `WorldViewProj`, `ColorWriteMode`, and the technique `depth_technique_noalpha`.
- **Runtime shader names** (strings in `.rdata`): `WorldShader`, `WorldReflectShader`,
  `WorldBoneShader`, `WorldPrelitShader`, `WorldNoFogShader`, `WorldMinShader`, `WorldDepthShader`,
  `CarShader`, `CarShaderDepth`, `CarShadowMapShader`, `TreeDepthShader`, `skyshader`, `billboardshader`,
  `ParticlesShader`, `RainDropShader`, `RunwayLightShader`, `VisualTreatmentShader`, `OverbrightShader`,
  `FilterShader`, `ScreenFilterShader`, `FEShader`, `FEMaskShader`. Which material effect index maps to
  which resource is not established here; [models.md](models.md) notes effect 2 = `WorldBoneShader`.

## Graphics options **[verified strings]**

`speed.exe` names these settings: `g_RacingResolution`, `g_PerformanceLevel`, `g_FSAALevel`,
`g_TextureFiltering`, `g_VSyncOn`, `g_ShadowDetail`, `g_WorldLodLevel`, `g_CarLodLevel`,
`g_CarEnvironmentMapEnable`, `g_CarEnvironmentMapUpdateData`, `g_RoadReflectionEnable`,
`g_MotionBlurEnable`, `g_OverBrightEnable`, `g_ParticleSystemEnable`, `g_RainEnable`,
`g_VisualTreatment`. Where they are stored is covered in [saves.md](saves.md#configuration).

## Status

- The decomp's PC folder is sparse (`src/Speed/PC/Src/Ecstasy/eLightPlat.hpp`, `eSolidPlat.hpp`,
  `TextureInfoPlat.hpp`, `xSparks.h`), so the D3D9 render pipeline is not decompiled yet **[decomp]**.
- No public tool or doc for extracting or decompiling MW's effects was found.
- noclip.website (`src/NeedForSpeedMostWanted/render.ts`, `postprocess.ts`; MIT with a reverse-engineering
  note) is an independent WebGL re-implementation, not a port of these effects.
- For a rewrite: extract the 31 blobs, disassemble them with the DirectX SDK (`fxc /dumpbin` or
  `D3DXDisassembleEffect`), and re-express them in WGSL/GLSL. **[unconfirmed workflow]**
