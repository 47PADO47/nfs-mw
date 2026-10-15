# Reversing workflow

How behaviour is taken from the original `speed.exe` (NFS:MW, the PC build) with Ghidra. The rules in
[CONTRIBUTING.md](../CONTRIBUTING.md) and [docs/licensing.md](licensing.md#spec-first) apply: decompiled
code and disassembly never enter the repo. Only prose specs (`docs/specs/`) and provenance records
(`docs/provenance/`) do.

## Ghidra setup

- Program: `speed.exe`, x86 32-bit, image base `00400000`, about 26k functions.
- The Ghidra MCP groups load on demand. `load_tool_group("xref")` and `load_tool_group("function")` are
  needed for cross-references and `force_decompile`.
- `run_script_inline` failed with a class-loading error in this setup. Use the MCP query tools instead.

## Method

1. **Start from a string or a global.** Search strings for the feature's name, then read the bytes around
   the hit. Strings in this binary are sometimes packed back to back, so the pointer table may point at
   a neighbouring address, not the string start. Check xrefs at a few offsets (`-8` to `+16`) before
   concluding there are none.
2. **Follow xrefs.** `get_xrefs_to` on the table or global gives the reader and writer functions.
   A table filled by a constructor is a sign of per-object state, not a global setting.
3. **Read the consumer.** `force_decompile` for the logic, `disassemble_function` when the decompile is
   unclear. Note offsets within the object, not raw code.
4. **Write it down.** Record what was learned as a spec: the rule in plain words, the inputs, and the
   evidence tag. Do not paste code or names that only exist in Ghidra's output.
5. **Stop when there is nothing new to follow.** A lead is closed when every reader is either explained or
   shown to be irrelevant to the feature.

## Evidence tags

Use the same tags as [docs/formats](formats): **[verified]** (matched against game behaviour or data),
**[decomp]** (read from the binary, not yet tested in game), **[community]**, **[unconfirmed]**.

## Lessons

- A string near a hit is not automatically the thing you're looking for. Read the bytes on both sides and
  check the xrefs to the exact address before naming a table after it. The cullable-part lead was
  a data-overlap false positive.
- A float constant read by a function is a strong hint for a threshold. Check its context with
  `get_assembly_context` (it takes `xref_sources`) before reading it as data.

- Hand-decoding a binary record layout from a dump is error-prone. Verify the field offsets against the
  port's reader on a known record before trusting any field. The shading-group record only made sense once
  the port's reader gave the right values.
- A material name that looks like a shading parameter can be a lookup key. Hash the name with
  `bstring_hash` and match it against the light-material hashes to find the group it drives.

## Shader-parameter class (narrowed)

- The ~16 constructors and destructors I counted all store one vtable address, so they belong to one class,
  not 16 shader families. Its vtable has 16 slots: destructors plus methods.
- The per-draw method in that vtable sets shadow, matrix and light constants on the effect. It does not set
  `DIFFUSERANGE`. So the writer of `BOTTOM`'s values is another method of that class, or a caller of it.
- Next: find the vtable slot that writes the diffuse handle (offset `0x278` in the class), and which caller
  passes the light material values.

## Vtable of the shader-parameter class (read)

- Slots that are bare `RET` stubs (no-op virtuals): `006c5150`, `006c5160`. They are not defined as functions in
  the Ghidra database, so earlier decompile attempts failed on them.
- Destructors: `006d5bb0`, `006d6480`, `006d64f0`, `006d6560`, `006d65d0`, `006d6640`.
- Real per-draw method: `006d6aa0`. It calls `006c6300`, `006c6450`, and sets shadow and matrix constants. No
  diffuse-range write.
- `006c60f0`: car-level lighting (sun, ambient, fog, sky), no light material.
- Conclusion: no slot in this vtable writes `DIFFUSERANGE`. The writer of `BOTTOM`'s values is outside this
  class, in a caller that passes the light material. Next: find callers of the class's draw method, or of the
  glossy material setup (the code that reads the light-material record).

## Vtable, slot by slot (complete)

- `006c5150`, `006c5160`, `006c5190`, `006c5c70`, `006c57d0`: bare `RET` stubs.
- `006c57e0`: a real method under the `006c57d0` slot's neighbourhood. It looks up the name `HEADLIGHTS`, so it
  belongs to the headlight shading, not to the car body diffuse.
- `006d7400`: a real method. It reads a global (`0x982c88`), calls a lookup (`0x503400`), and stores the result
  in `0x988568`. No diffuse-range write.
- `007c5200`: a thunk that calls through a function pointer (the delete path).
- Destructors and the one draw method are listed above.
- Result: no virtual method of this vtable writes `DIFFUSERANGE`. The writer must be in a generic
  name-to-handle setter elsewhere, or in another class.

## Callers checked (lead 1)

- `006d6aa0` has no direct callers. It's dispatched through the vtable.
- `006d71f0` calls `006c6450` and `006c6300`. It is a per-frame setup that writes fade and matrix constants from
  a per-pass block (indexed by `DAT_00982a20`). It sets no `DIFFUSERANGE` and reads no light-material record.
- Still unread: the glossy material setup that reads a light-material record. The next step is to find the
  function that reads the `BOTTOM` material's fields, which may be on the port's side of the data (`LightMaterial`
  parsing), not in the binary.

## Resolved: port side of lead 1

- The port's `glossy_material` (in `crates/nfsmw/src/scenes/car/shading.rs`) maps light material fields to the
  shader constants `diffuse_min` and `diffuse_range`. These match the binary's `DIFFUSEMIN` and `DIFFUSERANGE`
  names. For `BOTTOM` (colours 1,1,1; scales 0.833 and 0.840) the range is 0.007, so the underside is nearly flat.
  Whether the original's formula matches is still unconfirmed. See
  [docs/specs/car-cullable-parts.md](specs/car-cullable-parts.md).

## Dead ends (don't repeat)

- Per-draw setters in the class's vtable (`006c60f0`, `006c6450`) set car-level values (sun, ambient, fog,
  and sky constants from a global lighting state). They don't take a light material, so they aren't the
  writer of `BOTTOM`'s values.

- The light-material pool string (`LightMaterialPlatInfoSlotPool`) is referenced only by a one-line
  initializer that creates a pool. It is not the writer of material values.

## Closed leads

- The underside material `BOTTOM`: it drives shading group 5 of the body shell, which uses texture `CARBOTTOM`
  and a near-flat diffuse range. See [docs/specs/car-cullable-parts.md](specs/car-cullable-parts.md).

- Cullable car part names: the string has no pointer references; the nearby bytes are other data
  (see [docs/specs/car-cullable-parts.md](specs/car-cullable-parts.md)).

## Open leads

- Original's use of the diffuse range and environment power for `BOTTOM`. Progress: the shader-parameter
  constructor registers `DIFFUSEMIN`, `DIFFUSERANGE`, `ENVMAPPOWER` and related names. Its vtable is set by
  about 16 constructors in the `006d5bb0`–`006d6a50` range, which suggests one class per shader family. Next:
  find which of those classes covers the glossy car body, then the function that writes light-material
  values into its handles, and what the diffuse range multiplies.
- In-game comparison of the underside at the same camera angle as the original.
- P8 palette layout in the texture decoder (unconfirmed assumption).
