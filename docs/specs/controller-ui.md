# Controller UI and menu rendering

This is the enhanced port's policy **[ours]**, rather than a claim of exact Xbox executable parity.
The mod's defaults and a modded PC install are references for A accept, B back, Start pause/resume,
D-pad and left-stick menu navigation. Existing driving bindings and saved response settings remain active.

- Prompts describe the live action bindings. Xbox glyphs come from Kenney Input Prompts 1.5A (CC0);
  no console or mod texture is distributed. Keyboard keys and unsupported custom inputs have text fallbacks.
- Only implemented actions are offered: Continue on the title screen, Accept/Back on icon menus,
  Select/Adjust/Done on settings, and Quit on the main menu. A prompt stays available in every option category.
  The title hint inherits the authored prompt position and fade so it clears the copyright text.
- Prompt device changes on meaningful input, not every poll. Neutral controllers, stick drift, releases,
  and console hotkeys must not overwrite keyboard prompts. A connected controller can be the initial device.
- One active gamepad supplies actions. Another controller takes over on deliberate input; axes from
  different controllers are not combined. Losing the active controller pauses driving and clears its input.
- Down/up pulses within one input frame remain visible for one action frame. Menu direction hysteresis
  engages at 0.55 and releases at 0.35 of raw stick travel, independently of driving deadzone response.
- The FEng runtime supplies direction repetition. Each newly focused screen waits for neutral input,
  preventing a held action or repeated direction from carrying through a screen transition **[ours]**.
- After an option package's initialization scripts complete, restore the selected setting row's focus.
  The pause package can otherwise leave focus on an original footer button **[verified files/runtime]**.
- All group transforms include size, including negative x mirroring and animated selection brackets.
  Additive textures preserve alpha coverage before the compositor's zero-alpha additive encoding.

Reference issue reports, not confirmed bugs of this port:
[missing video prompts](https://github.com/xan1242/NFS-XtendedInput/issues/14),
[Start polling](https://github.com/xan1242/NFS-XtendedInput/issues/8),
[console hotkeys switching prompts](https://github.com/xan1242/NFS-XtendedInput/issues/7),
[device flicker](https://github.com/xan1242/NFS-XtendedInput/issues/4), and
[stale trigger prompts](https://github.com/xan1242/NFS-XtendedInput/issues/35).
The asset source is [Kenney Input Prompts](https://kenney.nl/assets/input-prompts).
The implementation is written independently; no mod code is copied.
