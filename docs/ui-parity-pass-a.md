# UI repair pass A

Four repair candidates address the first pass of the repair plan. User acceptance remains open.

1. **Pause backdrop:** retain the authored per-vertex colors for ColoredImage objects. The shared
   presenter now combines these colors with inherited opacity, including repeated images. The pause
   side panels fade from black to transparent instead of drawing white. No replacement rectangle or
   arbitrary opacity override is used. See [format contract](specs/feng-coloured-images.md).
2. **Challenge Series footer:** use the shared binding-aware prompt path. Back, Accept and Tutorial
   follow selected-row eligibility. Suppress complete legacy hint groups in the final presentation
   so native animation cannot restore keyboard hints over controller prompts.
3. **Blacklist category feedback:** route pad acceptance through the existing accepted-action sound
   helper, matching pointer acceptance. Held Accept does not request repeated sounds during entry.
4. **Gauge selection frame:** center the authored frame on the selected swatch instead of retaining
   its initial vertical offset. Preserve depth, parent coordinates and the native opacity pulse.
   A faint frame midway through that pulse is intentional. Digital gauge color behavior is unchanged.

## Checks and limits

Each reported defect had a failing regression before its production fix. Focused checks cover both
reference installations, the shared FEng/UI layers, footer device switching, locked/unlocked rows,
Blacklist category acceptance and gauge rows/channels/pages in normal and backroom variants.

GPU captures exercise the actual native presenter and Vulkan renderer. Pause background captures
cover 4:3, 16:9 and a wide viewport over a controlled background. Challenge captures cover keyboard
and controller hints on locked and unlocked selections. Gauge captures retain the original pulse.
Existing pause stack tests cover Options/tuning retention and removal on return to the main menu.
These are controlled captures, not a full gameplay-to-pause recording or complete reference replay.

The local validation record retains commands, source/binary identities, installation identities,
captures and audio evidence. Physical-controller checks, listening comparison, tutorial-return visual
acceptance and full gameplay composition remain open. No playable build or shortcut is updated.
