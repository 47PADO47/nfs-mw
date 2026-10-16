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
- Tool names drift between sessions. If a call is refused, run `search_tools` first, then load the group that holds
  the tool, and only then give up on the route.

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
  `bstring_hash` and match it against the light-material hashes to find the group it drives. Hash the upper-cased
  name too: FEng message names are the bstring hash of the upper-cased name.
- When `get_xrefs_to` is missing (it showed as callable in `search_tools` but the calls failed), use `get_functions`
  with `fields=refs` on candidate functions: `refs` lists every address a function touches, so a string or pool word
  can be matched without an xref search. `fields=callers` gives the direct callers.
- Float constants: `read_memory` in blocks of 128 to 448 bytes, then decode the hex with `BitConverter.ToSingle` in
  PowerShell. Hand decoding of singles gave wrong values in planning; the script is a minute's work.
- Packed name lists (behaviour names, vehicle classes, AI class names) are 4-byte aligned strings with no pointers.
  A `read_memory` around them shows that before you expect any xref to the string. The same holds for 4-byte
  tag tables (message tags in a network client): one `read_memory` at the table gave the whole set.
- Two callers of one pool helper can be different features. Tell them apart by the counter they bump (`+0x94` for
  ground cops, `+0x9C` for helicopters in `speed.exe`) and by the spawn distance (60 or 80 m versus 250 m), not by the helper.
- A decompile may show a register argument as a return address (`unaff_retaddr`). Treat it as unresolved and say so.
  Other `unaff_*` and `extraout_*` names are the same garbling: a helper that takes a struct by pointer is often
  shown with the wrong arguments. Check the constructor and the arithmetic instead, and mark the constant **[unconfirmed]**
  until the bytes agree.
- `list_class_members` on a class name and `list_program_items` with `kind=classes` show no AI classes in this binary:
  there are no RTTI names, so class lookup by name is not a route here.
- In a stripped binary, names and strings find nothing (no "offscreen" or "despawn" anywhere). Start from a
  class-name string that a constructor uses as a task key, then read the object's vtable table at the address the
  constructor stores. The task callback is a vtable slot of the object the task was given.
- A task or callback object often stores its id at a fixed offset and checks it before forwarding. Read the raw
  bytes of a short callback when Ghidra has no function there; the forwarded call's target is the real tick.
- Ghidra may have no function at a known address (`Function not found`) even when the bytes are code. Read the bytes
  first: `read_memory` of 64 bytes shows the prologue (`81 ec` is `sub esp`) and the epilogue. Creating a function edits
  the shared Ghidra project, so record in the provenance file which functions were created and do not save the program
  unless that is the intent. Vtable targets are often not function entries; `disassemble_function` refuses them, but
  `read_memory` still gives the bytes. Disassembling at an address inside a new function fails too: use its entry.
- Lookup tables in this binary are a struct of {count, x0, unused, 1 / step, data pointer}. Read the struct, then the
  float array, and match the values to the spec table before trusting either. Little-endian floats are easy to
  misread by hand; decode them with a script in the scratchpad.
- A small data object with a vtable and a flag at +4 can be a visitor query: the matching child sets the flag. Find
  the predicate slot of the child class before naming the flag.
- Two programs named the same can be open. Pass the full project path (`/speed.exe`) to every call.
- Function-level callers come from `get_functions` with `fields=callers`. In this session the xref and call-graph
  tools were not callable, so data references need another route.
- **Start from the factory or the registry.** A factory `switch` or a registration routine names every kind it builds
  or every listener it adds, in one place. Read it before chasing names. Counting the registrations (about 65 types on
  one broadcast subject) is faster than following callers one at a time.
- **A reference to a type name is not always its sender.** A getter that caches a name's hash is used by subscribers,
  so its several callers are several listeners of one type. Find the sender by another route.
- **Name the globals from the script registry.** A function that registers script names with their functions, and
  takes an object global, gives names to the globals its callers read.
- **A factory with no visible caller is named by what it builds.** Match the action set it constructs against the
  name table, and prefer the set that holds one distinctive action.
- **Test "does it do X" with strings first.** One alternation pattern per question (behaviour words, sound words,
  state words) answers most absence questions before any function is read. A string xref to an action or message
  name lists the set that uses it.
- **Hash searches can miss a name that is used.** Screen file names hashed to nothing in `speed.exe`, though the
  screens are plainly used, so those names are plain strings there. Search the strings; the absence of the string is
  the test.
- **Read every branch the index can reach.** A mask router can skip an entry without an error: the overlay name is in
  the table but no action is for it. Before saying a screen is shown or hidden, read each branch the entry's index
  can reach.
- **Trace a callee's pointer to the call site.** A parameter can point a few bytes before the data it reads (a length
  word before the tag). Go back to the call site and the buffer before reading offsets.
- **Check every branch of a compare chain.** A length check can sit in one branch only. Compare each branch's length
  with what the sender writes.
- **Write down which object an offset belongs to.** `this+0x40` (the navigator, through its getter) and `this+0x5c`
  (the own vehicle) are different objects, so a formula read from one can be the other's mass. A mod's offset may name
  a different object than the stock code's offset for the same field; check the stock reader.
- **Read the bytes of every vtable slot first.** Stub slots show at once (`xor al,al; ret 8` is false; `ret` and
  `ret 4` are no-ops). Data xrefs from the vtable entries name the slot functions, and a dead slot makes its path dead
  before any decompile.
- **A constructor's first vtable may be a secondary table.** A three-slot table followed by RTTI strings can be the
  first vtable a constructor writes. Find the main vtable from the object's first write or from the class's typeinfo.
- **The decompiler drops trailing arguments of vtable calls.** When a value matters, read the disassembly; a call can
  show two of its three arguments.
- **Device-call numbers are SDK enums.** Small integers in device vtable calls (render-state values) take their names
  from the SDK, not the binary. Say so in the record.
- **Mode numbers read from code shape are inferred.** A mode name taken from a check or a view size is not named in the
  binary. Mark it as inferred.
- **A static non-zero gate with no writer is always open.** Read the initial bytes before the writers. A flag that is
  `1` in the file and written nowhere is never turned off at run time.
- **Registry-backed globals name their settings.** The calls that read registry values give the setting names directly,
  which is faster than a string search. A pointer test on such a global can mean a size choice rather than on or off:
  read the branch before naming the option.
- **Find every reader of a global in one query.** Search the instruction operands for the address (the hex digits
  without `0x`, because the disassembly prints `[0x...]`). The result gives each reader with its compare constant.
- **A mod's addresses are leads, not evidence.** Check that each one lies inside a stock function and sits on the
  instruction it names. Only a site whose instruction sequence matches counts as confirmed.
- **Defaults sit in the initial data.** The binary's initial data holds option defaults and sizes. A `read_memory` of
  a few dozen bytes near the named sites gives them in one call, and immediates in `mov [reg+offset], imm` show default
  sizes. A registry writer that copies the same values confirms them as defaults.
- **Triage big functions before decompiling them.** Ask for signature, callers and labels first, and decompile one or two
  functions per call. Put a limit on the xref list of a shared global, which can run to hundreds of entries.
- **Each chunk-id hit belongs inside a function.** A byte-search hit must be placed in a function (its start and size).
  A hit in no function is an unanalysed site; create the function at its start before decompiling it.
- **The dispatch order comes from the decompile.** The order in which a dispatcher runs its loaders can be the reverse of
  the file order. Read the dispatcher, not the file.
- **Read the reader before the table.** A region that looks like a table can be a budget record. The record layout
  comes from the reader; the stride cannot be guessed from the bytes.
- **Names of install data are not in the executable.** Search the executable for the little-endian hash words of the
  name with `search_byte_patterns`, not for the name text.
- **Format strings name parts, not places.** A string such as `%s_INTERIOR` can come from the builder of car parts.
  Read the function that builds the name before naming a feature from it.
- **A data table has no code xref.** In a stripped binary, "no xref" does not mean "unused". Find the table's base
  address before reading its consumer; an immediate-address search can miss the table too.
- **Byte-search runs of floats, not single constants.** A six-point run of little-endian floats matches once; a common
  constant such as 0.05 matches dozens. A run that matches also shows whether two tables are distinct.
- **Settle a disputed table in bytes.** When two sources give different values for one table, read the bytes before
  writing a fix.
- **Check copied tables against the executable.** A table copied from the stock binary matches in one byte search and
  one read.
- **Compute a range from the record.** A lookup record has a scale and a point count. When two notes give different
  ranges for one table, compute the range from the scale (a scale of 0.05 with two points spans 20 units, not 80) and
  find which note read the wrong field.
- **Test a hash before brute-forcing names.** Check the hash function against a name with a known value first. Then
  generate candidates from a small vocabulary (prefix times suffix, with and without underscores, in lower and upper
  case) and keep the lists in the scratchpad.
- **A hash-to-label lookup can be checked.** If the code computes a default label, look up its hash and confirm it
  matches a known label.
- **Repo Python tools run without `-I`.** `tools/chunkdump.py` imports a sibling module from its own folder, so run it
  as `python tools/chunkdump.py`. The `-I` flag is for untrusted files only.

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

- Traffic despawn: name searches and string searches for offscreen, despawn, visible and inview return nothing in
  `speed.exe`. Use the task callback chain instead (see [provenance/ai-traffic.md](provenance/ai-traffic.md)).
- Some manager vtable slots are a stub (`xor al,al; ret 8`), not a feature.

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
