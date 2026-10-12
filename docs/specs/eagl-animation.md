# EAGL animation banks (work in progress)

Status: **partly mapped, not implemented.** The airliner `ANM_AIRLINERA_10` is still a static pose in the
asset viewer. This spec records what is verified in the install and in `speed.exe`, what is inferred, and what is
still open. It is updated as the Ghidra mapping goes on.

Evidence tags as in [docs/README.md](../README.md#evidence-tags): **[verified]** checked against the install or
the running code's decompilation, **[inferred]** consistent with the evidence but not proven, **[unconfirmed]**
open.

## 1. Where the data is **[verified]**

- The track file `TRACKS/L2RA.BUN` holds the world animation entities (`WorldAnimEntities`, `WorldAnimTreeMarkers`,
  `WorldAnimInstances`) and 13 `EAGLAnimations` chunks (`0x00E34010`). See
  [formats/animation.md](../formats/animation.md).
- Each `EAGLAnimations` payload is 8 bytes of `0x11` padding and then an ELF32 relocatable object (`ET_REL`,
  `EM_MIPS`, relocations `R_MIPS_32` only).
- The airliner's object is the ELF that starts at offset **1,409,088** in `TRACKS/L2RA.BUN` (1,504 bytes).
  Its symbol table holds one `__AnimationBank:::bank` symbol at `.data` offset **0x330**, and no other bank.
- The bank names inside the object are `ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_q`, `..._s` and `..._t`. The
  suffixes name the three channels: rotation (`q`), scale (`s`), translation (`t`). The `LB_10` part is one of
  three parts of the airliner's model (`LB_10`, `LB_11`, `LB_12`).

## 2. The bank structure **[inferred from the relocations and the code]**

The loader adds the `.data` base to each pointer listed in `.rel.data` and nothing else. So the bytes in `.data`
are the in-memory layout after relocation. For the airliner bank (offsets are into `.data`):

| Offset | Content | Evidence |
|---|---|---|
| 0x330 | `+0x00` = 0x1c, `+0x04` = **3** (number of channels) | `FUN_0049dc20` reads `+4` as the count |
| 0x340 | `+0x10` → 0x30c (array of channel objects) | relocation at 0x340 |
| 0x344 | `+0x14` → 0x318 (array of channel name pointers) | relocation at 0x344 |
| 0x318 | name pointers → 0x0e (`_q`), 0x37 (`_s`), 0x60 (`_t`) | relocations at 0x318–0x320 |
| 0x30c | channel object pointers → 0x1a8, 0x1f0, 0x2fc | relocations at 0x30c–0x310 |

The channel objects are at 0x1a0, 0x1e8 and 0x2f4 (the pointers point 8 bytes in, which fits the field offsets
the evaluator uses). Each object's field `+0x0c` points to a **data record**: `q` → 0x90, `s` → 0x1c0, `t` → 0x200.
Each record starts with the same 16-byte header:

```
+0x00 u32  1
+0x04 u16  1          (frame count in the evaluator: a u16 at +4)
+0x06 u8   1          (key count per frame, N, in the evaluator)
+0x07 u8   0          (frame-shift bits in the evaluator)
+0x08 u32  0x3030301e (not a pointer: nothing is relocated here)
+0x0c ...  (the rest of the header differs per record)
```

**Open:** the evaluator (below) reads a pointer at `+0x08` (a frame-time table) and at `+0x0c` (a bone-index
array). The records hold non-pointer values there, so the record layout the evaluator expects is **not yet
confirmed**. This is the main open question.

## 3. The channel evaluators in `speed.exe` **[verified: decompiled]**

Addresses are Ghidra's for `speed.exe` (image base 0x400000).

| Function | Role |
|---|---|
| `FUN_0049dc20` | Assigns each signature entry an evaluator. Signature type 0 → `0x499620` (copy four words), 1 → `FUN_004995d0` (three degree angles → quaternion via `FUN_0048d8e0`), 2 → `0x499660` (copy four words to a second array). Prints `Bad signature channel type` for any other value. |
| `FUN_00497860` | Per-channel setup: reads the descriptor's count and shift, allocates the key arrays, and decodes the base keys with `FUN_00494bc0`. |
| `FUN_00498420` | Channel evaluator (virtual, from the vtable at 0x895954): picks the keys for the current frame, interpolates with `FUN_00496c10`, and writes per-bone transforms to a 0x30-byte-stride output. |
| `FUN_00497bb7` | The other evaluator path; the same logic specialised for `flag == 0` keys. Reached when a dispatcher's first argument is zero (`0x497b9c`). |
| `FUN_00494bc0` | Decodes a **14-byte key** (below). |
| `FUN_00494c70` | Decodes one byte pair into two floats in [−1, 1]: `byte * 0.007843138 − 1` (the 8-bit delta form). |
| `FUN_00494d10` | Decodes a **nibble delta**: high nibble and low nibble scaled by `0.0666…` and offset by the base values in the descriptor. |
| `FUN_00496c10` | Linear interpolation of two quaternions followed by normalisation. |
| `FUN_00497a90` | Quaternion multiply used when a key is in the second form. |
| `FUN_0048d8e0` | Euler angles (radians) → quaternion. |

The `AnimationBank` chunk handler is registered by `FUN_004a8600` (`0x48e3e0` and `0x48e410` are the handler
callbacks). `FUN_004a08f0` prints the statistics (`#Evals`, `#DeltaSums`, `#Frames`, `#Creates`), so the game counts
the delta sums separately from the evaluations.

### 3.0 A table of evaluators **[verified: memory read; the users are not yet found]**

The words at `0x895940` onward are an array of function pointers, not an MSVC vtable: there is no RTTI pointer
before it. Its entries include `0x48d490`, `0x4946b0`, `0x497a20`, `0x494710`, `0x494700`, `0x498420` (at 0x895954,
slot 5), `0x498d40` and `0x48d3c0`. `FUN_00498420` is slot 5. Likely this table is indexed by a codec or channel
type number stored in the channel record. The table base does not appear as an immediate in the code, so the
index is computed through a pointer. Finding that pointer, and the type number in each record, is the next
mapping step.

### 3.0a The factory and the type numbers **[verified: decompiled and disassembled]**

- A method at `0x4a9d10` (its vtable slot is at `0x895590`) creates a channel object from a **type number** passed
  in `EDI`. The type must be `<= 0x18`; anything else goes to the error path at `0x4a9f3a`.
- It switches through a 25-entry jump table at `0x4a9f44`, one entry per type.
- The constructors it reaches set the object's vtable and its `+0x08` field to the type:
  type 0 → vtable `0x8959e8`, type 3 → vtable `0x895d18` (`+0x08 = 3`), type 4 → vtable `0x895958`
  (`+0x08 = 4`). The other types call further constructors (`0x49a240`, `0x49c890`, …).
- The type number is therefore the codec selector. **[inferred]** The rotation record starts with a byte 4 at
  `+0x0c` (`04 00 08 00`), and the translation and scale records carry a 3-component marker (`03 …`). That fits
  type 4 for `_q` and type 3 for `_s`/`_t`, but the field the factory reads from the bank record is not yet
  confirmed.
- Checked and not confirmed: the slot the dispatcher calls for the type-4 object (`[vtable + 0x4c]` at
  `0x8959a4`) holds `0x48d3e0`, not the evaluator `FUN_00498420`. So the evaluator for type 4 is not yet identified.
- The type-4 vtable (`0x895958`, object size `0x2c`) is mostly stubs: slot 0 is the deleting destructor
  `FUN_00498d40`, and the slots that return a constant (`0x48d3c0`, `0x48d3f0`, `0x48d400`, `0x48d410`,
  `0x48d3e0`) are `XOR AL,AL / RET` or `XOR AX,AX / RET`. So slot 19 (`0x4c`) returns 0 for this class. The
  channel evaluator is therefore reached another way: not through the type-4 vtable's slot 19. **[open]**

### 3.1 The 14-byte key **[verified: decompiled `FUN_00494bc0`]**

| Bytes | Field | Decode |
|---|---|---|
| 0–1 | angle 0 | `u16 * K * 2^-16 − π`, with `K` at `DAT_009117fc` (read as 2π) |
| 2–3 | angle 1 | same |
| 4–5 | value 2 | `u16 * 2/65536 − 1` (range [−1, 1)) |
| 6–7 | value 3 | same |
| 8–9 | value 4 | `u16 * 1/32768` (range [0, 2)) |
| 10–11 | value 5 | same |
| 12 | flag | byte; 0 selects the Euler path, 1 the second path, anything else the third |

Each key is for one bone: the flag chooses how the two angles combine into a quaternion (`FUN_0048d8e0` for the
angles, then the two extra values scale or offset it).

### 3.2 The nibble delta **[verified: decompiled `FUN_00494d10`]**

Each byte gives two 4-bit deltas. The high nibble scales the first value, the low nibble the second, each by
`0.0666…` (1/15) and then adds the base from the descriptor (`+0x08`, `+0x0c`, `+0x10`, `+0x14`). This is the
"delta" codec family the docs name (`DeltaQ`, `DeltaF`); the exact mapping of `_q`/`_s`/`_t` to codecs is not
confirmed.

## 4. Testing the rotation record **[verified: the test failed]**

The rotation record at 0x90 declares 4 components at 8 bytes per key (`04 00 08 00` at 0x9c). Its data is 240
bytes, which is 30 keys of 8 bytes. Decoding those keys as 16-bit values gave no smooth motion under any of four
simple mappings:

| Mapping | Mean change between consecutive keys |
|---|---|
| quaternion, [−1, 1] | 1.27 |
| angle, [−π, π) | 4.00 |
| signed /32767 | 1.79 |
| unit, 0–1 | 0.64 |

A correct decode would give small steps between frames. None of these does, so the 8-byte keys are **not** plain
four-component 16-bit values. The rule that turns them into keys is **[unconfirmed]**.

## 5. What is not yet known **[unconfirmed]**

1. The record layout the evaluator uses (the frame-time table and bone-index pointers in §2).
2. Which evaluator (type 0, 1, 2 or the 14-byte path) applies to each of the three channels.
3. The rotation record's key format (§4).
4. The translation record: its header says 3 components and a 524-byte payload, but reading it as floats gives
   values that are not plausible.
5. How the three parts (`LB_10`, `LB_11`, `LB_12`) attach to the airliner solid `ANM_AIRLINERA_10`. The world
   animation entities (`WorldAnimEntities`) should carry that link.

## 6. Tollbooth light (`XO_TOLLBOOTH_LIGHT_1B_00`) **[verified in the asset viewer]**

The model has two shading groups. Group 1 is the lens: two triangles that sit on a face of group 0, and they
use the animated texture `0xA14B54EC` (the first frame of a two-frame animation). The lens's texture coordinates
cover only u 0.84–0.95, v 0.91–1.0, a corner of the 128×128 texture. The frames differ across most of the
texture (938 of 1,024 DXT blocks), but the corner barely changes (a few bytes in six blocks), so the lens looks the
same in every frame. The animation is therefore not visible on this model as authored. Showing it would mean
changing the model's texture mapping, which is a decision for the project owner, not an automatic fix.

## 7. Traffic signal and bobsrobster **[verified in the asset viewer]**

Both animate in the viewer: traffic signal (`XS_TRAFFICSIGNALCROSS_JK_1B_00`) changes 284 pixels between two
frames, and bobsrobster (`XO_BOBSROBSTER_1B_00`) changes 1,601, against zero for a static control (the traffic
cone). The traffic signal needed the decal lift in `nfsmw-data/src/world/assets.rs`: its animated panel is a
decal that sits on a face of the box and loses the depth test.

## 8. Next steps

1. Map the vtable at `0x895954` and the class that owns it, to see which evaluator each channel uses.
2. Confirm the descriptor layout from the code that builds it (the allocator path from `FUN_00497860`).
3. Decode one key by hand against the decompiled function, then check the decode is smooth over the 30 frames.
4. Only then implement the decoder and show the motion in the asset viewer.
