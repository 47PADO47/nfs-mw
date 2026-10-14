# Feature bits, shadow minimum and cheap PCF

The fork's `shader_common.h` switches shader features with **specialization-constant bits**: one 32-bit value is
set when a pipeline is created, and the compiler removes the code that a clear bit disables. This page lists the bits,
the two shadow techniques they support, and what we can take from them. Source: `shaders/shader_common.h` in
[codepdbh/nfsmw-android](https://github.com/codepdbh/nfsmw-android) (GPL-3.0, see the licence notes in
[README.md](README.md)). Performance figures on this page are the fork's own; we have not measured them.

## The bits

| Bit | Name | What it does | Source's note |
|---|---|---|---|
| 0 | `R11G11B10_NORMAL` | Decodes normals stored in R11G11B10 | — |
| 1 | `ALPHA_TEST` | Enables the alpha test | — |
| 8 | `CONSTANTES_UBO` | Reads shader constants from a uniform buffer rather than a pointer | Constant bank on the Maxwell GPU |
| 9 | `INV_TAMANO_TEX` | Scales texel offsets by `1/size` taken from the shared constants, not by querying the texture | Saves a size query per sample. The library has 160 queries for 435 samples |
| 15 | `PCF_BARATO` | Cheap PCF: all nine shadow samples land on one texel, so the compiler keeps one | "Edge smoothness is lost" |
| 16–18 | Alpha function | The alpha comparison, chosen when the pipeline is created | Was a seven-way switch in every shader that tests alpha |
| 19 | `SIN_DESENFOQUE` | Removes the radial speed blur of the final composition | "The edge blur when accelerating and with NOS is lost. The image is sharper" |
| 23 | `SOMBRA_MINIMO` | Shadow map as the minimum of two textures (see below) | — |
| 24–30 | (unused) | The marker constant is kept out of this range | — |

The fork also defines `NFSMW_MARCA_SOMBRA_MINIMO = 0x5E3B1A84`. It appears as an `OpConstant` in the SPIR-V of
every shader that uses the shadow minimum, and nowhere else. The host can check that a library has the feature
without running a shader. The source of the value is the fork's own choice; it has no other meaning.

Three more bits (2, 3 and 4) exist only under the upstream `UNLEASHED_RECOMP` macro and are not used here.

### How the bits reach the shader

Each bit is read with `g_SpecConstants()`, and the code is gated with `[branch] if (...)`. The fork's comments
explain why `[branch]` is needed: without it DXC keeps both sides of a ternary, so the dead branch's size query
stays in the code.

## Shadow minimum

The fork's comment describes the technique as follows. Treat it as the fork's account, not as a verified result.

- The game resolves the **same 1600×1600 shadow map twice**: once without cars (the car bodies sample this map),
  and once with the cars drawn on top (the world samples this map).
- With `SOMBRA_MINIMO`, the cars are drawn into a separate render target cleared to **1.0** (the far value). The
  world map is not copied underneath.
- Shaders that sample the map with cars take `min(world, cars)` per texel. The fork says this is exactly what
  drawing the cars on top produces, **because the game's depth test is LESS or LEQUAL**.
- The second texture is passed in the **3D index word** of the same shader register. The shadow map is 2D, so
  that word is otherwise unused.
- Both textures are sampled with the same point sampler, the same coordinates and the same offsets, so the minimum
  is per texel.

The rewrite that turns a shadow fetch into this form is a string replacement in the bash script
([translator-pipeline.md](translator-pipeline.md#4-rewrite-the-shadow-and-blur-paths)).

### What the design depends on

- **The depth test.** The result equals "draw the cars on top" only for LESS or LEQUAL. Any other test changes the
  answer, and the fork does not say what happens then.
- **Identical sampling.** Both maps must be sampled the same way. A different filter, a different offset or a
  different clamp breaks the per-texel minimum.
- **No blending.** A blended car pass would not be a minimum at all.

**Our situation:** the renderer has no real-time shadow map (see the table in [README.md](README.md#ideas-worth-taking)).
So this is a design input for when shadows are built. Before that, check whether the PC game's own shadow effects
(`IDI_SHADOW_MAP_MESH_FX`, `IDI_CAR_SHADOW_MAP_FX`, listed in [formats/shaders.md](../formats/shaders.md)) draw the
cars in the same way. Only then is the technique a match for the game and not just for the Xbox build.

## Cheap PCF

The 3×3 shadow filter takes nine samples at half-texel offsets. `PCF_BARATO` sets the eight outer offsets to zero,
so all nine samples read the same texel and the compiler keeps one. The fork's figures: one draw, the smoke and
wheel spray in `p_000101`, has eleven shadow samples and was **21 % of the scene** on Xbox, so the cost is real.

The trade is visible: shadow edges become hard. The fork's comment says so: "Edge smoothness is lost."

This is a good shape for a **quality tier**. Our [low-end guide](../low-end.md) lists the settings that trade
quality for speed. A shadow-filter setting would fit there once shadows exist.

## Lessons for our shader work

These come from the fork's notes and its build script. Each one is a rule to check, not a result we have measured.

1. **Setting a factor to zero does not remove the work.** The fork's radial blur is `r5 * factor + r3`. Setting the
   factor to 0 keeps the seven taps, because `x * 0` is not folded for floats (NaN and infinity must stay
   correct). The fork removes them by cutting the blend itself: with the bit set the output is the centre tap
   `r3`, so `r5` is unused and the taps die. Expect the same IEEE rule in WGSL and naga, and test it before relying on
   it.
2. **Gate passes, not values.** Put a bit on a whole branch that selects which samples exist, not on a number that
   multiplies them. The removed work is then dead code, and the compiler can drop it.
3. **Specialize only on what the app knows at pipeline creation.** The fork's alpha function comes from the
   `RB_COLORCONTROL` register, which is already part of its pipeline key, so the choice is made once per pipeline and
   costs nothing per pixel. The
   same rule holds for us: a value that is in the pipeline key can become an `override`. A value that changes per
   frame cannot.
4. **Use one shader source for both paths.** The fork keeps a single shader and switches the constant path with a
   bit. It is simpler to test than two shaders.

### What we already do

`libs/blackbox-render/src/shaders/scene.wgsl` declares WGSL `override` constants, `PRELIT` and `FOG`, and
`libs/fsr3-wgpu/src/shaders/common.wgsl` declares `INVERTED_DEPTH`, `HDR_INPUT` and `LOW_RES_MOTION_VECTORS`. So the
pattern in point 3 is already in use. The gap is in the other direction: none of our `override`s switches off a
multi-tap pass. Lesson 1 applies if one ever does.
