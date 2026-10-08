# AEMS module banks (the logic inside `.abk` files)

An `.abk` sound bank (`ABKC`, [audio.md](audio.md#abkc--bnkl-sound-banks-community-offsets-verified)) is more than a
box of samples: it is EA's AEMS **module bank**. Its modules are small dataflow graphs (the "event system" that
decides which samples play, how loud and how high) plus the compiled code that runs them. What a graph does is in
[specs/aems.md](../specs/aems.md); how the car sound drives the engine banks is in
[specs/engine-sound-aems.md](../specs/engine-sound-aems.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

Sources: the structures are the decompilation's `Libs/snd/9/extern/aemsdef.h` (CC0, **[decomp]**); everything below
was then checked on the 301 banks of the PC install with a throwaway parser and an interpreter, and the checks are
kept as tests in `blackbox-aems`. **[verified]** marks those.

## What the files contain **[verified]**

The 301 `.abk` files hold 386 modules (platform byte 0 = PC, version 1 in all of them). Every module is the
logic of one Csis class: `CAR` (the engine, 154 modules in the 70+ `CAR_nn_ENG_MB_EE/_SPU` banks), `CAR_SWTN`,
`CAR_Sputter` and `CAR_SputOutput` (one per `SWTN_CAR_nn_MB`), `CAR_WHINE`, `CAR_TRANNY`, `FX_TURBO_01` (32),
`FX_SHIFTING_01` (16), `FX_SKID` (4), `FX_NITROUS`, `FX_PURGE`, `FX_ROADNOISE`, `FX_ROADNOISE_TRANS`, `FX_WIND`,
`FX_WIND_Weather`, `FX_Scrape`, `FX_MAIN_MEM`, the collision, static and whoosh stitches, `SIREN`, `FX_TRAFFIC`,
`FX_TRUCK_FX`, `FX_Helicopter`, `FX_Weather`, `ENV_STATIC`, `FX_Camera`, `FX_Radar`, `FX_UVES` and the front-end
players.

## The bank header

The first 0x5C bytes of an `.abk` are the original's `ModuleBank` (the `ABKC` header of audio.md is the same
structure). Little-endian; offsets are from the start of the file:

| Offset | Field |
|---|---|
| 0x00 | `ABKC`; 0x04 version (1); 0x05 to 0x07 tool version; 0x08 platform (0 = PC) |
| 0x0A | `u16` number of modules |
| 0x0C / 0x10 | debug CRC / unique id |
| 0x14 | `u32` total size; 0x18 **resident size**: the bytes the game keeps (code, module data, sample groups); the sounds (`BNKl`) follow |
| 0x1C | offset of the first module (always 0x5C) |
| 0x20 / 0x24 | offset and padded size of the sound bank (`BNKl`); 0x28 / 0x2C the same for MIDI (none) |
| 0x30 | offset of the **function fixup** table |
| 0x34 | offset of the **static-data fixup** table |
| 0x38 | offset of the **interface** table |
| 0x3C to 0x5B | run-time fields (handles, stream path, list links, tweak header), zero in the file |

The three tables after the sounds: the function fixups are `{u32 count, u32 offset[count]}` of the 4-byte operands
of the module code's `call` instructions (below); the static-data fixups `{u32 count, u32 offset[count]}` of
pointer words in the module data; the interface table `{u32 count, {i32 handle offset, i32 id offset, u8 type, 3
bytes}[count]}`, where the id is `u16 system crc, u16 interface crc` and a NUL-terminated name (`CAR`, `CAR_Sputter`,
...). Type 1 is a class. The first reference names the module's own class (its handle is the module's `classHandle`
at +0x04); later ones name classes a module creates. **[verified]**: in every bank the function fixups are exactly
the operands of the code's `call` instructions.

## Modules **[verified]**

Starting at the first module, each is a 0x3C-byte header followed by an array of `numPlayers + numClassControllers`
`u32` offsets, and the next module starts right after:

| Offset | Field |
|---|---|
| 0x00 | `u32` module id |
| 0x04 | 8 bytes of run-time class handle; 0x0C 16 bytes of run-time client |
| 0x1C | `i16` instances now (0); 0x1E `i16` most instances (4 for the engine) |
| 0x20 / 0x22 | `u16` number of global variables / of functions |
| 0x24 | `u8` number of players |
| 0x25 / 0x26 | `u8` the module has a class-destructor node / a class-data node |
| 0x27 | `u8` number of class controllers (nodes that create other classes) |
| 0x28 | `u32` offset of the code |
| 0x2C | `u32` offset of the data |
| 0x30 | `u32` size of the data |
| 0x34 | `u32` offset of the destroy node, from the start of an instance |
| 0x38 | run-time list head |
| 0x3C | `u32` offsets, from the start of an instance, of the player nodes and then the class-controller nodes |

An **instance** is a copy of the module data placed at the start of a block whose first 0x18 bytes are the
run-time list links (zero in the file). The code runs with a pointer **0x18 bytes into the instance**, so a node
the code reaches at `esi + x` is at file offset `data + 0x18 + x`. Everything the node offsets and the destroy
offset say is from the start of the instance, i.e. `data + offset` in the file.

The data holds, in this order: the class-destructor node (0x14 bytes, if the flag says so), the global variables
(0x1C each), the class-data node (0x14 bytes and one `i32` per class member: `0x14 + 4 * n`, `n` at byte 0x10 of
the node; 26 for the engine class), the function nodes, then the graph's nodes in the order the code runs them.
After the data (at `data + size`) come the sample groups, and then the rest of the resident part.
**Pointers** inside the data (a player's sample group, a table's data) are file offsets in this PC format; the
static-data fixups list them (37 in `CAR_66_ENG_MB_EE.abk`) and the game adds the load address. A reader that keeps
the file image and follows the offsets needs no fix-up.

A **sample group** is `{u32 count, {u8 type, u8 priority, 2 bytes, u32 index, u32 loop offset}[count]}`: the same
12-byte entries as the sample tables of [audio.md](audio.md). Type 0 is a bank sound (`index` is the `BNKl` entry,
counted from 1); entries with index 0 pad the group.

## The code **[verified]**

A module's code is a straight-line program for a 32-bit x86 CPU, with no loops and only two kinds of branch: it
runs the nodes in dependency order and moves each node's output to the input words of the nodes that read it. Over
all 386 modules (828,489 bytes of code) it uses exactly these forms, `esi` being the walking pointer:

| Form | Meaning |
|---|---|
| `push esi` / `pop esi` / `ret` | enter and leave; `mov esi, [esp+8]` loads the instance data pointer |
| `push esi; call N` | run node function number `N` on the node at `esi` (the operand is the number until the game patches it; table in the spec); the result is in `eax` |
| `mov [esi+d], eax` (`d` signed, 8 or 32 bits) | store a node's output into the input word of another node |
| `mov eax, [esi+d]` / `mov ecx, [esi+d]` | load an input |
| `add / sub / imul eax, [esi+d]` | inline `Add2`, `Subtract` and `Multiply` nodes (`d` = 4) |
| `cmp eax, ecx; jl/jg +2; mov eax, ecx` | inline `Min2` / `Max2` |
| `mov byte [esi+d], imm8` | clear a trigger flag |
| `add esi, imm` (8 or 32 bits) | move on to the next node |
| `add esp, imm` | drop the arguments pushed since |

The nodes follow each other in the data, so `add esi, N` is also the size of the node just called (for the fixed
nodes it equals the size in the spec).

## Class data (the parameters)

The game sets the class-data words (`data + 0x18 + 0x14 + 4k` of the node) from the parameter list of the Csis class,
in the order of its structure. The `CAR` class has 26: car class, RPM, TRQ_ENG, TORQUE, VOL_ENG, VOL_EXH,
TRQ_LFO_AMP, TRQ_LFO_FREQ, RPMLFO_AMP, RPM_LFO_FREQ, VOL_LFO_AMP, VOL_LFO_FREQ, SPU_or_EE, FX_DRY, FX_AMOUNT,
AZIMUTH, PITCH_OFFSET, ROTATION, XOVER_IDLE_2_LO, XOVER_LO_2_MID, XOVER_MID_2_HI, FILTER, FILTER_RND, FILTER_Dist,
FILTER_Trig, MAX_RPM. The lists of the other classes are in
[specs/engine-sound-aems.md](../specs/engine-sound-aems.md).

## Open

- No module of the install has global variables or function nodes (the counts are 0 in all 386); those nodes
  are known from the decompilation only. Class controllers occur only in the 76 sputter modules.
- The sound system's random generator and sine table (`iSNDrandom`, `iSNDsin`) are not in the decompiled sources.
