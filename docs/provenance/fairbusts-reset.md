# Provenance: the busted reset penalty and the FairBusts settings

What the reset penalty (the stock "flashing" busted fill) is, where each constant was read, and what is still a guess.
The rules are in `libs/blackbox-pursuit/src/busted.rs`; the settings and the penalty mapping in
`crates/nfsmw/src/settings/fairbusts.rs`; the wiring in `crates/nfsmw/src/scenes/world/ai/police/busted.rs`. Spec:
[fairbusts-reset.md](../specs/fairbusts-reset.md) and [ai-pursuit.md](../specs/ai-pursuit.md) section 5.1.

## Ghidra pass (`/speed.exe`, image base 0x400000)

The program was opened in the Ghidra project as `/speed.exe` (the other open program, `/New Traces/pydbg/speed.exe`,
is a copy and was not used). The executable's functions are unnamed (`FUN_*`), and Ghidra defines no function at the
pattern, so the reading below is of the instructions only.

| Address | Bytes / instruction | What it does | Value |
|---|---|---|---|
| `0x0044447C` | `83 F8 01 75 02 8A D8 84 DB` (`CMP EAX,1; JNZ; MOV BL,AL; TEST BL,BL`) | start of `kResetPenaltyBlock`; the only match of the 9-byte pattern in the image (search by `findBytes`) | |
| `0x00444485` | `FLD dword [0x00890DAC]` | normal-range reach (pattern +0x09) | `0x41700000` = **15.0 m** |
| `0x0044448F` | `FLD dword [0x00892FA8]` | reset-range reach, flashing (pattern +0x13) | `0x42B40000` = **90.0 m** |
| `0x00444495` | `JMP 0x004444AD` (`EB 16`) | skips the speed gate (pattern +0x19, two bytes); the flashing branch falls through to the distance test | |
| `0x00444497`–`0x004444AB` | `CALL 0x00409ED0`, `FCOMP [ESP+0x2C]`, `FNSTSW`, `TEST AH,5` | the speed gate: the player's speed (through the car's vtable) against the heat's `BustSpeed` | |
| `0x004444C2` | `FMUL dword [0x008910F0]` | base fill per unit of `dT` (pattern +0x46) | `0x3E800000` = **0.25** |
| `0x004444D0` | `FMUL dword [0x00890E98]` | flashing fill multiplier, applied only when `BL` is set (pattern +0x54) | `0x40800000` = **4.0** |

The four constants (15.0, 90.0, 0.25, 4.0) are read from the install's memory, so they are verified against the binary.
The pattern offsets agree with the FairBusts mod's own notes (+0x09, +0x13, +0x19, +0x54), so the mod and this reading
find the same block.

### The flashing flag `BL`

`BL` is set at `0x00444468`–`0x00444481`: `CALL 0x0040E6C0` with the address of a local, then, when that returns non-zero,
a virtual call on the same object (`CALL [EDX+0x14]`), and `BL = (result == 1)`. The helper `0x0040E6C0` reads an
object at `[ECX+0x1C]`, looks up the name hash `0x004039E0` through `0x005D59F0`, and stores the result. **The name behind
hash `0x004039E0` is not resolved**, so the flag is taken as the spec's "invulnerable from manual reset" state
(`docs/specs/ai-pursuit.md` section 5.1) without confirming the name. A `[ESI+0x129]` test at `0x00444457` takes another
branch before this; its meaning is not read.

`0x00409ED0` returns the player's speed through a virtual call (`[EAX+0x54]`, then `[EDX+0x30]`); the fallback constant at
`0x00890968` is used when the object is absent. It is read as the speed getter, not as a named function.

## Not verified in this pass

- **The blink duration (2 s).** The reset penalty only runs while the player is flashing. The 2 s comes from the
  rigid-body spec (`docs/specs/vehicle-rigid-body.md` section 7: a reset sets the invulnerability timer to 2 s), which is
  decompiled behaviour. It was not measured in the binary here. The model clears the blink when its timer reaches 0; the
  spec also waits for 1 s after the last refused contact, which this model does not track.
- **The cop cap (`MAX_COPS`).** The stock car loader's limit of 8 cars is from the Bartender notes, not from a Ghidra
  search in this pass.
- **Stock `FairBusts` default.** The mod's INI sets `ResetBustMultiplier = 1.0` and documents that as "normal rules". The
  brief for this rewrite sets the default to 4.0, which is what this implementation uses. A user who wants the mod's
  behaviour sets `ResetBustMultiplier = 1.0` in `NFSMWFairBustsSettings.ini`.
- **The bust rule of the mod's patch.** The mod patches the `JMP` at `0x00444495` (`EB 16`) to `90 90`, so the flashing
  branch also runs the speed gate; this is inferred from the code layout, not tested in the game.

## Values in the rewrite

| Value | Used as | Source |
|---|---|---|
| Normal reach 15 m | `Busted.Reach` (Bartender), the fair reset's reach | Ghidra `0x00890DAC`; `ai-pursuit.md` 5.1 |
| Stock reset reach 90 m | `ResetPenalty::STOCK` and `ResetAutoBust = 1` | Ghidra `0x00892FA8` |
| Base fill 0.25 per dT | the busted timer's rate in the spec (`BustTuning` fill 1.0 per second net) | Ghidra `0x008910F0` |
| Reset multiplier 4.0 | `ResetBustMultiplier` default (the task brief) and `STOCK` | Ghidra `0x00890E98` |
| Blink 2 s | `RESET_BLINK` | `vehicle-rigid-body.md` section 7 (decomp) |
| Speed gate skip | `ResetAutoBust = 1` and the stock penalty | Ghidra `0x00444495` |
