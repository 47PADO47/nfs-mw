# World sky shading provenance

- Original source: retail PC Most Wanted executable, embedded SKYBOX effect.
  Proprietary; inspected for behavior only. No original code or assets are copied.
- Cross-install comparison: stock and preferred modded PC executables have
  byte-identical embedded effect resources. This does not establish identical
  runtime constants, active mod hooks or external texture assets.
- Method: shader declarations/register arithmetic were inspected, then the
  installed original vertex/pixel shader pairs were executed in a private D3D9
  offscreen harness with synthetic inputs. Seven pairs and 60 cases cover world,
  tree, no-fog, prelit and the three selected sky layers.
- Measured target: NVIDIA GeForce RTX 5070 Ti; 32 by 32 A8R8G8B8, no sRGB,
  no blending, no alpha test, point sampling. Pixel fixtures are independent
  from the remake implementation. Driver-specific PS 1.1 precision remains a
  limitation; runtime blend state is not inferred from the isolated harness.
- Public specification: [original sky shading](../specs/world-sky-shading.md),
  written before implementation. Private resource offsets, extracted bytecode,
  disassembly, executable identities and harness outputs remain outside source.

Runtime sampler repeat/linear filtering, no culling and reverse-Z depth state
are reconstruction assumptions pending a matched gameplay draw-state trace.
Original shader fixture agreement does not verify those runtime states.
