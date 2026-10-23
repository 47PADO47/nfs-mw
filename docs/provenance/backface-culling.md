# Provenance: back-face culling

Spec: [docs/specs/backface-culling.md](../specs/backface-culling.md).

## Method

- **Tool:** Ghidra MCP on `/speed.exe` (PC build; image base `0x400000`). Strings by regex, references by inline
  Ghidra scripts (references to an address, instructions with a given operand), decompile by function address.
  Decompiled code was read for understanding only.
- **Search path:** the string `CULL_MODE` led to the effect parameter table; the table entry's handle offset led to
  the two functions that set the parameter; their callers and the globals they read gave the rest. A scan for
  `SetRenderState(D3DRS_CULLMODE, ...)` with a literal state found one call, in `FUN_006d7f70`, outside the scene
  draws; it was not followed.
- **Install measurements:** a throwaway Python script in the scratchpad (not in the repo) read the car and world
  geometry, the texture pack info records (decompressing JDLZ texture blobs) and the scenery instances. The car
  winding numbers in the spec come from the Rust test in `libs/blackbox-solid/src/tests/winding.rs`.

## Addresses

| Address | Name in Ghidra | What it is |
|---|---|---|
| `0x008af12c` | string | `CULL_MODE`, the effect parameter name |
| `0x006c3d20` | `FUN_006c3d20` | effect base constructor: the table of parameter names and their handles; `CULL_MODE` is entry 11 |
| `0x006c5e60` | `FUN_006c5e60` | effect start: sets `CULL_MODE` to 1 (both faces) and other defaults |
| `0x006c68b0` | `FUN_006c68b0` | texture state of a draw: blend states, texture addressing, z write and `CULL_MODE` from the texture's info record, the current effect id, the two globals and the draw flags |
| `0x006c6cc0`, `0x006c6d10` | | callers of the above when a draw's textures change; the second passes the draw's flag word |
| `0x006d7660` | `FUN_006d7660` | one draw of the solid renderer: binds the textures with the draw's flag word, then `DrawIndexedPrimitive` |
| `0x008fae44` | global byte | culling enabled; 1 in the file, no writer found |
| `0x008fae6c` | global dword | cull mode of the current view: 1 at start, written 2 or 3 by the render paths below |
| `0x006de300` | `FUN_006de300` | world frame render: 3 before the road reflection and the environment-map faces, 2 before the player view |
| `0x006e44c0`, `0x006cfce0`, `0x006e7220` | | other render paths that set mode 2 (the first saves and restores it) |
| `0x00723fa0` | `FUN_00723fa0` | scenery draw list of a view: turns instance flag `0x400000` into draw flag `0x40000` |
| `0x0074e160` | `FUN_0074e160` | car renderer: four draw calls add draw flag `0x40000` |
