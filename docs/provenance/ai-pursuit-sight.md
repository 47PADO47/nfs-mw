# Cop sight: occlusion, vertical gate and reversing status (epic E7)

- **Source for the rule:** the decompiled pursuit sources cited in [ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md)
  section 2 and [ai-pursuit.md](../specs/ai-pursuit.md) section 4.1: `CanSeeTarget`, the 0.25 s timer, the heat ranges
  (151/151 heat 1 to 7, 201/101 heat 8 to 10), the rear cone `dot(cop.forward, unit(target - cop)) <= -0.3`.
- **Ghidra (`/speed.exe`, image base 0x400000):** the binary has no debug symbols. `CanSeeTarget` and `AIVehiclePursuit::Update`
  are not named. Class-name strings exist (`AIVehicleCopCar` at 0x0089304c, `AIVehiclePursuit` nearby in `.rdata`), but the
  xref lookup returned generic data items, not the referencing functions, so the vtable and method were not found. The
  occlusion query flags, the vertical test and the 7 s grace were **not** confirmed in the binary.
- **Tools requested but unavailable in this session:** `search_instructions`, `search_byte_patterns`, `get_function_pcode` and
  `analyze_control_flow` did not load (ToolSearch found none). Only `list_data_items_by_xrefs` loaded.
- **Occlusion choice (rewrite):** every face blocks, including floors (the wall probe's `FLOOR_NORMAL` filter is not applied) and
  grouped scenery (`skip_groups` off, `exclude` 0). The one pass-through is a one-sided barrier from behind. This follows the
  spec's "world ray hits geometry" plus the epic's requirement that decks and buildings occlude. The game's real flags are
  **[unconfirmed]**.
- **Vertical gate:** `COP_LEVEL_GATE` 8 m is a design value (**[unconfirmed]**), chosen so a cop 20 m above or below is off level.
