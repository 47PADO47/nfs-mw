# EAGL skeletons: loading and bone matrices

How the game loads an EAGL4 skeleton object (chunk `EAGLSkeletons`, `0x00E34009`), what the object holds, and how its bone
transforms become matrices. The container and the symbol naming are in [formats/animation.md](../formats/animation.md);
the animation banks are in [eagl-animation.md](eagl-animation.md). This file covers the skeleton side and the loader they
share.

Evidence tags, as in [docs/README.md](../README.md#evidence-tags): **[verified]** measured on the install, numbers quoted;
**[decomp]** read in the CC0 decompilation (dbalatoni13/nfsmw), file named; **[unconfirmed]** not proven. Where the
decomp body is empty, the item is marked **[unconfirmed – needs Ghidra]** and section 8 lists what to find.

"World" in this file means the skeleton's own frame, the frame of its root bone (the root has no parent). Where the
character or car is placed in the game world is a separate step, not described here.

## 0. Summary

1. A skeleton object is an ELF32 relocatable file (`ET_REL`, `EM_MIPS`) inside a chunk, as for animation banks. It has
   only `.data`, the string tables and the symbol table. It has **no relocations** and no code. **[verified]**
2. The object is not linked by a system loader. The game makes a `DynamicLoader` on the bytes. Its constructor calls
   `Initialize`, `Resolve` and a version check. The bodies of `Initialize` and `Resolve` are not in the decomp. **[decomp]**
3. The skeleton is the single `__Skeleton` symbol. Its value is the offset of a 16-byte header in `.data`. The header is
   followed by one 112-byte record per bone, in parent-first order. **[verified]**
4. Each bone record holds scale (3 floats), a parent index, a unit quaternion (x, y, z, w), translation (3 floats), a
   left/right field, and a 4 by 4 inverse bind matrix. **[verified]**
5. The inverse bind matrix is exactly the inverse of the bind pose built from the record's scale, rotation and
   translation (`T · R · S`, parents multiplied first, column vectors). The worst error over all 2,532 bones is 8.4e-7.
   This fixes the matrix convention. **[verified]**
6. Per frame the game fills a pose buffer with 10 values per bone (scale, quaternion, translation), calls
   `PoseSQTToGlobal` to get world matrices, and, when a flag is set, multiplies each by the bone's inverse bind matrix.
   The body of `PoseSQTToGlobal` is not in the decomp. **[decomp call site; unconfirmed – needs Ghidra]**
7. Install totals: 67 skeleton chunks (64 in NIS scenes, 3 in `GLOBAL`), holding 2,538 bone records. Only 6 distinct
   skeleton payloads exist (5 rigs). The existing docs' totals are wrong; section 7.

## 1. What the object looks like on disk

### 1.1 The payload **[verified]**

The chunk payload is 8 bytes of `0x11` fill, then the ELF file. The ELF file starts 16 bytes after the chunk header, so it
begins on a 16-byte boundary in the file. This holds for all 190 EAGL chunks in the NIS bundles, `TRACKS\L2RA.BUN` and `GLOBAL\InGameB.bun` (skeletons and banks).
The fill is what makes that boundary. **[verified]**

The ELF header is the same in all objects: 32-bit, little-endian, `ET_REL`, `EM_MIPS`, flags `0x20924000`.

### 1.2 Sections **[verified]**

| Section | Skeleton object | Animation object | Notes |
|---|---|---|---|
| (null) | yes | yes | |
| `.data` | yes, 16 + 128 n bytes | yes | `PROGBITS`, flags WRITE and ALLOC, alignment 16, file offset `0x40` |
| `.shstrtab` | yes | yes | |
| `.strtab` | yes | yes | |
| `.symtab` | yes | yes | entries of 16 bytes |
| `.rel.data` | **no** | yes | only `R_MIPS_32` entries, see section 2.5 |

No section is `NOBITS`, so no zero-filled memory is set up by the file itself. No `.text` or init section exists in any
object.

A skeleton object's ELF ends at its section table. The payload then has **40 zero bytes** after the section table in all
64 NIS skeleton chunks. Animation objects have no such tail. The reason is not known. **[verified; unconfirmed reason]**

### 1.3 Symbols **[verified]**

| Symbol | Binding and type | Value | Count per skeleton object |
|---|---|---|---|
| (section symbol of `.data`) | local, section | 0 | 1 |
| `__Skeleton:::<rig>` | global, object, size 4 | offset of the header, always `16 n` | 1 |
| `__Bone:::<rig>.<bone>` | global, object, size 4 | `16 × i`, where `i` is the bone's index | one per bone |
| `__EAGL_TOOLLIB_VERSION:::EAGL_TOOLLIB_VERSION-3` | weak, object | 0 | 1 |

Every named symbol in the skeleton objects is defined in `.data` (section index 1). The skeleton's rig name is the text
after `:::`. Bone symbols carry the same rig name before the dot, then the bone name. The root bone's name equals the rig
name (for example `Bip21.Bip21`).

The type of each symbol is written only as the prefix of its name (`__Skeleton`, `__Bone`, `__AnimationBank`). The decomp
reads the type a different way; see section 2.3.

## 2. Loading

### 2.1 Who calls what **[decomp]**

1. The chunk handler for `0x00E34009` (`LoaderEAGLSkeletons`, file `Animation/AnimSkeleton.cpp`, 4,431 bytes) creates one
   `CAnimSkeleton` per chunk and appends it to a global list. The chunk's aligned payload (the 16-byte boundary described
   in section 1.1) is passed on.
2. `CAnimSkeleton::Initialize` builds a `DynamicLoader` over the payload and its size. It then runs `DynamicLoadResolve`.
3. `DynamicLoadResolve` asks the loader for the first symbol of type `Skeleton` (the constant `SkeletonType`). The symbol's
   data address is the skeleton. The symbol's name is kept as the skeleton's name, and a hash of that name is the lookup
   key (`GetSkeletonFromList`).
4. The unloader finds skeletons by their chunk pointer, removes them, and deletes the loader.

The skeleton therefore stays where the object's bytes are. The loader does not copy the object. **[decomp]**

### 2.2 What "dlopen" returns **[decomp]**

The name is misleading. `DynamicLoader` (`eagl4supportdlopen.h`, 9,629 bytes; `.cpp`, 6,874 bytes) is a small in-memory
linker for one object file:

- It keeps the object's bytes, length, a list of destructor entries and some patch-address bookkeeping.
- Its `handle` is a `HashPointer` record (0x430 bytes). The record holds the ELF header, the section header table, the
  symbol table, the string table, a 256-bucket hash with a chain array, and one "original" flag per symbol. Records of all
  loaders form one linked list.
- The constructor calls `Initialize` (builds the hash), then `Resolve` (fixes addresses), then `DoVersionCheck`, which
  returns true. **`Initialize` and `Resolve` are declared but not defined in the decomp files.**
- `GetCount`, `GetSymbol(i)`, `GetNextSymbol(type, index)` and `GetNextAddr` walk the symbol table. A `Symbol` has a
  name, a type string, a data address and a flag `isInternalRef`.

The data address of a symbol is its `st_value` when the symbol's `st_other` field is 1. Otherwise it is the base address
of the symbol's section plus `st_value`. The section base is the section header's `sh_offset` field, which shares space
with `sh_voffset` (a runtime pointer). So the loader must have overwritten that field with a real address during
`Resolve`. **[decomp; unconfirmed – needs Ghidra: Resolve]**

The static `dlsym` in the same file looks a name up with the ELF hash, folded to 256 buckets, then follows the chain. It
checks the original flag, the name, and that the section index is in range. **[decomp]**

### 2.3 The type of a symbol **[verified: strings; decomp: code]**

The decomp's `GetSymbol` takes the type from the bytes after the symbol name's terminating zero, then steps one byte
forward or back depending on the byte after that. On the install this points into the next symbol's name. For
`__Skeleton:::Bip21` the bytes after its zero are `__Bone:::Bip21.Bip21`. So the code as written cannot return the type
`Skeleton`. **[verified: bytes; decomp: code]**

The types the decomp compares against are `Skeleton` (`SkeletonType`) and `AnimationBank` (`AnimBankType`). Both match
the name prefixes in the object exactly. So the intended rule is most likely: the type is the text between `__` and `:::`.
**[unconfirmed – needs Ghidra: the body that fills `Symbol.type`]**

### 2.4 Symbol pools and constructor pools **[decomp]**

- `SymbolPool` (`eagl4supportsympool.h` and `.cpp`, 1,898 and 3,686 bytes) is a hash table of name to address. The hash
  mixes each character with a shift and XOR, then takes the remainder by the table size. Collisions probe forward with
  wrap-around. The table starts with 256 slots and doubles when it becomes full. Each slot points at an entry holding the
  address and the name. `Search` returns the address stored for the name, and a validity flag.
- `ConstructorPool` (`eagl4supportconspool.h` and `.cpp`, 2,648 and 1,169 bytes) holds two `SymbolPool`s, one for
  constructors and one for destructors, both keyed by type name. `RuntimeAllocConstructorPool` has the same shape with a
  different constructor signature.
- The global `gSymbolPool` (for registered variables) is declared, but `RegisterVar` and `GetRegisteredVar` are not
  defined in these files. **[unconfirmed]**

The skeleton objects have no undefined symbols. So nothing in them needs the symbol pool or the search callback passed to
the loader. **[verified: all named symbols defined in `.data`]**

### 2.5 Relocation rule **[verified: measurements; decomp: no code]**

Skeleton objects have no `.rel.data` section, so nothing is patched in them. Their pointer fields are all zero (section
3.2). **[verified]**

Animation objects do have relocations. Measured over the 127 animation chunks: 4,838 entries, all `R_MIPS_32` (type 2).
Over the 112 distinct animation payloads: 4,495 entries. Every entry targets the `.data` section symbol (value 0). Each
entry is in-place (REL form): the 32-bit word at the offset holds an offset into `.data`, which the loader must add to
the base address of `.data`. The offsets seen are all inside `.data` (largest object: `.data` of 205,120 bytes). The
patching code is `Resolve`, which is not in the decomp. **[verified; unconfirmed – needs Ghidra: Resolve]**

### 2.6 Constructors and destructors **[decomp]**

After relocation, `RunConstructors` goes through the symbols. For each symbol flagged `isInternalRef` whose type has a
registered constructor, it calls the constructor with the symbol's data address, the loader and the name, and records the
destructor. `RunDestructors` calls them in reverse order. The flag is computed from `st_other`. In every object measured,
`st_other` is 0 for every symbol, so the flag is set for all of them. **[verified: st_other; decomp: rule]**

Which types have registered constructors is decided by calls to `AddType`, which are not in these files. So whether a
skeleton gets a constructor is **[unconfirmed – needs Ghidra: AddType callers]**. Skeleton data needs no constructor to
be usable: its bytes are already final once relocation is done (there are no relocations).

## 3. The skeleton object

### 3.1 Layout of `.data` **[verified: 64 of 64 NIS skeleton chunks]**

| Region | Offset in `.data` | Size | Contents |
|---|---|---|---|
| Bone index records | `0` to `16 n − 1` | 16 bytes each | one per bone. The first 4 bytes hold the bone's index `i` (0 to `n−1`). The other 12 bytes are zero. The record for bone `i` is at `16 i`, which is the value of its `__Bone` symbol |
| Header | `16 n` | 16 bytes | section 3.2. Its offset is the value of `__Skeleton` |
| Bone records | `16 n + 16` | `112 n` | section 3.3. Bone `i` starts at `16 n + 16 + 112 i` |

The `.data` size is `16 + 128 n` bytes in all 64 chunks (for `n = 48`, that is 6,160 bytes). **[verified]**

The 16-byte index records are the 16-byte spacing that `formats/animation.md` describes. They are not the bone data.
The bone data is 112 bytes per bone. **[verified]**

### 3.2 Header (16 bytes) **[verified; field names decomp: `AnimationTarget` and `SkeletonData`]**

| Offset | Type | Field | Observed |
|---|---|---|---|
| `0x00` | u16 | rig checksum (`AnimationTarget` checksum) | `0x1658` for `Bip20`, `Bip21`, `Bip23`; `0xDBD5` for `Bip24`; `0x1414` for `ROOT` |
| `0x02` | u16 | number of DOFs (`AnimationTarget` DOF count) | always `10 n` |
| `0x04` | u16 | core skeleton checksum | 0 in all 64 |
| `0x06` | u16 | flags | 0 in all 64 |
| `0x08` | i32 | number of bones `n` | 48 or 3 in NIS, 2 in `GLOBAL` |
| `0x0C` | pointer | inverse bone scales | 0 in all 64 (no relocation needed) |

The three 48-bone rigs have the same bone names (after the rig name) and the same checksum. So the checksum probably
depends on the bone list. The exact rule is **[unconfirmed]**. The decomp names the field a target checksum
(`AnimationTarget`), and the compatibility test that would use it (`IsAnimCompatible`) has no body. **[decomp; unconfirmed]**

### 3.3 Bone record (112 bytes) **[verified]**

| Offset | Type | Field | Meaning and check |
|---|---|---|---|
| `0x00` | 3 × f32 | scale S (x, y, z) | 1.0 for most bones. Non-unit in 104 bone records, e.g. the Eyes and Jaw bones of `Bip21` (X scale 2.13 and 8.82) |
| `0x0C` | i32 | parent index | `-1` for the root (bone 0). Every other parent index is smaller than the bone's own index |
| `0x10` | 4 × f32 | rotation quaternion (x, y, z, w) | unit length within 1e-3 for every bone. The w component comes last |
| `0x20` | 3 × f32 | translation T (x, y, z) | offset of this joint from the parent joint, in the parent's frame |
| `0x2C` | i32 | left/right field | equal to the bone's own index in all 2,532 bones. It does not hold the mirror bone (for example `L Thigh` is 1 and `R Thigh` is 5, but both fields are their own index) |
| `0x30` | 16 × f32 | inverse bind matrix | column-major 4 by 4 (section 4.2). Column 3 is the translation, element 15 is 1 |

Units: translations are in metres. The leg chain of `Bip21` runs thigh to shin 0.416 m and shin to foot 0.435 m. Over the
2,468 non-root bones, the translation length has median 0.141 m and maximum 0.435 m. The root of `Bip21` sits at
z = 0.979 m, so the rig's height axis is most likely +z. **[verified: values; unconfirmed: which axis is up]**

### 3.4 Hierarchy **[verified]**

In all 64 chunks there is exactly one root (bone 0, parent `-1`). Every other bone's parent index is smaller than its own.
So the bones are stored parents first, and one pass in index order can compute world matrices.

### 3.5 Rigs in the install **[verified]**

| Rig | Bones | Chunks in NIS | Bind notes |
|---|---|---|---|
| `Bip21` | 48 | 12 | limbs, spine, and facial bones (brows, cheeks, eyes, jaw, lips, nose) |
| `Bip23` | 48 | 26 | same bone names and checksum as `Bip21` |
| `Bip20` | 48 | 14 | same bone names and checksum as `Bip21` |
| `Bip24` | 3 | 12 | `Bip24` root, `Cuff02` and `Cuff03`; cuff chain of 0.117 m and 0.033 m |
| `ROOT` | 2 | 0 (3 in `GLOBAL`) | identity bind pose; bones `ROOT0` (index 0) and `ROOT` (index 1) |

Totals: 64 NIS skeleton chunks hold 2,532 bones (52 × 48 + 12 × 3). `GLOBAL` holds 3 more chunks of `ROOT` (2 bones
each), so 67 chunks and 2,538 bone records install-wide. The distinct payloads are 6: `Bip20`, `Bip21`, `Bip23`, `Bip24`,
and two `ROOT` variants (the copy in `INGAMEC.BUN` differs from the other two in 8 bytes near the end of the object,
outside `.data`; the bone data is the same). Those 6 payloads hold 151 distinct bones. **[verified]**

## 4. Bind pose and world matrices

### 4.1 The bind pose composition **[verified]**

For each bone `i` in index order, with `S_i` the scale matrix, `R(q_i)` the rotation matrix of the quaternion and
`T_i` the translation matrix:

- local matrix `L_i = T_i · R(q_i) · S_i` (the scale acts first, in the bone's own frame);
- world matrix `W_i = W_parent(i) · L_i`, and `W_0 = L_0` for the root;
- the stored inverse bind matrix equals `W_i⁻¹` for every bone.

The rotation matrix for a unit quaternion (x, y, z, w) is the standard one for column vectors:

| | column 1 | column 2 | column 3 |
|---|---|---|---|
| row 1 | 1 − 2(y² + z²) | 2(xy − zw) | 2(xz + yw) |
| row 2 | 2(xy + zw) | 1 − 2(x² + z²) | 2(yz − xw) |
| row 3 | 2(xz − yw) | 2(yz + xw) | 1 − 2(x² + y²) |

The check ran over the 2,532 bones of the 64 NIS chunks, and over the `ROOT` and `Bip24` rigs. The largest difference
between the stored matrix and `W_i⁻¹` is 8.4e-7. The variants that failed: the quaternion's conjugate (errors of about 2
to 2.9), the reversed composition order (errors up to 2.5), and row-major storage (errors up to 2.0). **[verified]**

### 4.2 Matrix layout **[verified]**

The 16 floats of an inverse bind matrix are column-major: element `c × 4 + r` is row `r`, column `c`. Column 3 (elements
12 to 14) is the translation. So the matrix acts on column vectors. The game's `EAGL4::Transform` is 0x40 bytes and 32-byte aligned. It wraps a
`Matrix4` (`UTypes.h`, 3,883 bytes) of four `Vector4` members, `v0` to `v3`. Those are the same 16 floats, so `v3` is the
translation column. The column reading is inferred from the data. **[verified for the data; decomp for the type; the
column reading is inferred]**

The handedness of the game's world axes is not settled by this data. The matrices are column-vector matrices built from
the standard quaternion formula. Whether the game world is left- or right-handed is **[unconfirmed]**.

### 4.3 Per frame (game side) **[decomp]**

The per-frame code is in `Animation/AnimCtrl.cpp` (8,532 bytes) and `Animation/WorldAnimCtrl.cpp` (10,583 bytes). The
steps, in order:

1. The pose buffer holds 12 floats per bone (`GetPoseBufferLength` gives `12 n`), allocated in `CAnimPart::Init`. It is
   first set to the rest pose with `GetStillPose`.
2. Each animation channel writes its values into that buffer through `EvalSQT`. The rotation channel is slot 1, the
   scale channel slot 2, the translation channel slot 0.
3. `PoseSQTToGlobal(buffer, output, nullptr)` writes one `Transform` per bone (64 bytes each). The mask argument is null.
   Section 5 takes null to mean all bones. That is not yet confirmed. **[unconfirmed]**
4. If bit 0 of the control's flags is set, each output matrix is multiplied in place by the bone's inverse bind matrix.
   The resulting matrices are the skinning matrices. The multiplication helper is `bMulMatrix`.

The header of `Skeleton` declares many more pose functions (`PoseLocalToGlobal`, `PoseSQTToSkin`, `PoseGlobalToSkin`,
`GetStillPose`, and others). Their bodies are empty in the header and `Skeleton.cpp` is 0 bytes. **[decomp: call sites;
unconfirmed – needs Ghidra]**

### 4.4 Pose-time world matrices **[unconfirmed – needs Ghidra]**

The bind check (section 4.1) says the same composition, `T · R · S` with the parent's world matrix applied first, gives
the bind world matrices. The per-frame function probably does the same with the SQT values from the buffer. The step
from step 3 to 4 in section 4.3 implies that the skinning matrix of each bone is `W_i · invBind_i` in column-vector form.
That order is consistent with the bind check, but it depends on `bMulMatrix`'s operand order, which is not in these files.

What to find in Ghidra (`Skeleton::PoseSQTToGlobal`, and `GetStillPose`):

- the order of bones (index order, parent first, as the header implies);
- the layout of the 10 values in the pose buffer per bone (scale, quaternion, translation, as the `BuildSQT` argument
  order suggests) and what the two extra floats per bone (12 per bone, 10 used) are for;
- whether the scale is applied as a 3-vector before the rotation, as the bind check shows;
- how the mask and the output stride (64 bytes per `Transform`) are used;
- the operand order of `bMulMatrix` (in `bWare`).

## 5. BoneMask **[decomp]**

`BoneMask` (`BoneMask.h`, 955 bytes; `BoneMask.cpp` is 0 bytes) is a set of bones with one bit per bone. It has eight
32-bit words, so it covers up to 256 bones. Bone `b` is bit `b mod 32` of word `b div 32`, counting from the least
significant bit. A mask can be created with all bits on or off, set on or off per bone, combined with and, or, xor and not,
and compared for equality. Skeleton functions that take a `BoneMask *` are called with null, which probably means all bones **[unconfirmed]**. The mask builders
(`BuildBoneMask` for a subtree from a root bone, `BuildMirrorBoneMask` and `BuildSymmetricBoneMask` for the left and right
side) are declared in `Skeleton.h` but not defined in the decomp files. The mirror builders would use the left/right field
(section 3.3), but that field equals the bone's own index in this data, so mirroring must come from the names or from
another table. **[decomp; unconfirmed – needs Ghidra]**

## 6. PosePalette **[decomp; not found in the install]**

`PosePalette` (`PosePalette.h`, 1,757 bytes, with no `.cpp`) is 12 bytes: a pointer to pose data (floats), a pointer to a
list of 16-bit DOF indices, and three byte counts (poses, quaternion DOFs, translation DOFs), then one byte of padding.
A `PosePaletteBank` is an 8-byte header: two 16-bit checksums (one for the skeleton, one for the animation), a 16-bit
palette count and two bytes of padding. The accessor `GetPalettes` returns a list of palette pointers. Where that list
sits in the bank is **[unconfirmed]**. The palettes look like a
compact way to store the subset of DOFs that an animation drives for a skeleton. All bodies are empty and no palette type
string is defined in `dlopen.cpp`. The scans found EAGL objects only under the ids `0x00E34009` and `0x00E34010`, so no
separate palette chunk was found. Whether palettes are stored inside animation banks is **[unconfirmed]**.

## 7. Corrections to the existing docs

| Doc | Says | Correct |
|---|---|---|
| `formats/animation.md`, "Symbols" table | the `__Bone` value is an offset into `.data`, and bones are "spaced 16 bytes apart" | the value is correct, and it is the offset of a 16-byte index record. The bone data is 112 bytes per bone, after the header at `16 n` **[verified]** |
| `formats/animation.md`, "EAGL skeletons and animations are ELF object files" | "All 188 EAGL objects ... (64 skeletons + 124 animation banks)" | 67 skeleton chunks and 127 animation chunks install-wide (194). The 64 and 111 are the NIS chunks. The 13 in `TRACKS/L2RA.BUN` and the 3 skeleton and 3 animation chunks in the three `GLOBAL` files make up the rest. Distinct payloads: 6 skeletons and 112 animation objects **[verified]** |
| `formats/animation.md`, "Totals" | "2,532 bone symbols, 64 skeletons, 124 banks" | 2,532 bone symbols is right for the 64 NIS skeleton chunks. Install-wide it is 2,538 (chunks), and 151 in distinct payloads. 124 banks should be 127 chunks |
| `formats/animation.md`, "Relocations" | "every relocation in all 188 objects ... 4,772 in total" | 4,838 entries over 127 animation chunks (4,495 over 112 distinct payloads). Skeleton objects have none. All are `R_MIPS_32` to the `.data` section symbol |
| `formats/animation.md`, "Looking at it yourself" | drop the 8-byte chunk header and the 8 bytes of fill | no change needed: the payload starts 8 bytes past the chunk header, and the ELF starts at offset 16 in the chunk **[verified]** |
| `specs/eagl-animation.md`, section 1 | the airliner object is "1,504 bytes" | 1,488 bytes of ELF. The chunk payload is 1,496 bytes (8 bytes of fill). The bank symbol at `.data` offset `0x330` is correct **[verified]** |

## 8. Open questions (Ghidra list)

1. `DynamicLoader::Initialize` and `DynamicLoader::Resolve` (`eagl4supportdlopen.h` declares them; the `.cpp` does not
   define them). Find: how section base addresses are written into the section headers; the relocation loop; how the hash
   and chain arrays are built; and what happens to undefined symbols (the search callback).
2. `DynamicLoader::GetSymbol`, the code that fills `Symbol.type`. Confirm the type is the text between `__` and `:::`.
3. The bodies of `Skeleton::PoseSQTToGlobal`, `PoseLocalToGlobal`, `PoseSQTToSkin`, `GetStillPose`, `BuildBoneMask` and
   `MirrorPose` (`Skeleton.cpp` is 0 bytes; header bodies are empty). Section 4.4 lists the details.
4. The `EAGL4::Transform` methods (`eagl4runtimetransform.cpp` is 0 bytes): the meaning of `PostMult`, `PreMult`,
   `Append*` and `Prepend*`, and the parameter order of `BuildSQT`.
5. `bMulMatrix` in `bWare`: operand order.
6. `CAnimPart` pose buffer: the order of the 10 values per bone, and the use of the 2 extra floats (section 4.4).
7. `AddType` callers: which types have constructors. Whether `Skeleton` does.
8. The meaning of the 12 zero bytes in each 16-byte index record. They may be filled at run time.
9. The rule for the header checksum (the same for three rigs with the same bone names, and different for `Bip24`).
10. Why 40 zero bytes follow the section table in skeleton objects.
11. The up axis of the rig (section 3.3) and the handedness of the game's world (section 4.2).

## 9. Sources

Decomp files (`src/Speed/Indep/Src/EAGL4Anim/`, dbalatoni13/nfsmw, read through the GitHub API; sizes from the
directory listing; not saved to disk):

| File | Bytes |
|---|---|
| `eagl4supportdlopen.h` / `.cpp` | 9,629 / 6,874 |
| `eagl4supportsympool.h` / `.cpp` | 1,898 / 3,686 |
| `eagl4supportconspool.h` / `.cpp` | 2,648 / 1,169 |
| `eagl4supportdef.h` | 1,744 |
| `eagl4supportbase.h` / `.cpp` | 320 / 591 |
| `Skeleton.h` / `Skeleton.cpp` | 2,915 / 0 |
| `SkeletonData.h` | 2,337 |
| `BoneMask.h` / `BoneMask.cpp` | 955 / 0 |
| `eagl4runtimetransform.h` / `.cpp` | 4,855 / 0 |
| `AnimMemoryMap.h` | 858 |
| `FnAnimMemoryMap.h` / `.cpp` | 703 / 525 |
| `PosePalette.h` | 1,757 |
| `System.h` | 401 |
| `AnimationTarget.h` | 704 |
| `FnAnim.h` | 2,780 |
| `eagl4realmath.h` | 131 (guard only) |

Game side (`src/Speed/Indep/Src/Animation/`):

| File | Bytes |
|---|---|
| `AnimSkeleton.hpp` / `AnimSkeleton.cpp` | 1,635 / 4,431 |
| `AnimCtrl.cpp` | 8,532 |
| `WorldAnimCtrl.cpp` | 10,583 |
| `AnimPart.cpp` | 2,109 |
| `AnimInternal.cpp` | 604 |

Also read: `Libs/Support/Utility/UTypes.h` (3,883 bytes; `Vector4` and `Matrix4`), and `Src/Misc/SpeedChunks.hpp` (chunk
ids `0x00E34009` and `0x00E34010`).

Install files measured (`D:\Need For Speed Most Wanted Black Edition`, read only):

- `NIS\Scene_*_BundleB.bun`, all 87 bundles (uncompressed; the bChunk tree walked to find the two EAGL ids).
- `GLOBAL\InGameB.bun`, `GLOBAL\Ingameb.lzc`, `GLOBAL\INGAMEC.BUN` (the same skeleton and animation chunks appear in all
  three; one copy of `INGAMEC` differs in 8 bytes).
- `TRACKS\L2RA.BUN` (13 animation chunks, including the airliner object at 1,409,088).
- A raw byte search of all 2,084 files under the install for the ELF-after-fill pattern found the same 67 skeleton and 127
  animation chunks.

Method: a short Python 3 script in the scratchpad (run with `python -I`) that walks the bChunk tree, parses the ELF
header, sections, symbols and relocations, and decodes the bone records. It does not use `tools/chunkdump.py`, which
needs its own folder on the import path. The checks (matrix convention, tree, checksums, relocation targets) are the
scripts `verify_all.py`, `skel_conv.py`, `final_check.py` and `anim_rel.py` in that folder.
