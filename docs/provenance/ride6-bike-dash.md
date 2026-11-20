# RIDE 6 instrument clusters and paint fidelity

Code: [`tools/ride6-exporter/{dash,dash_widgets,dash_paint}.py`](../../tools/ride6-exporter) and
[`mods/ride6-bikes/src/{dash,bake,grade,shading}.rs`](../../mods/ride6-bikes/src).

- **Pages:** [formats/ride6-materials.md](../formats/ride6-materials.md#instrument-clusters) (the clusters),
  [paint and satin metal](../formats/ride6-materials.md#paint-and-satin-metal).
- **Sources read:** the user's own RIDE 6 install (UE 5.6.1 build `5.6.1-1697004`), unencrypted IoStore packages read with CUE4Parse
  and the game's `.usmap` (the exporter's `R6ToolParts`): `Content/assets/data/vehicles/HUD/HUD<chassis>` (the two widget class
  names), `Content/assets/ui/Widgets/gauges/gauge_<n>/gauge_<n>`, `gauge_<n>_E` and `gauge_<n>_configuration` (the widget trees and
  the gauge configuration), `Content/assets/graphics/vehicles/bikes/gauges/gauge<n>/{material_instances,textures}` and
  `bikeShared/material_instances/{M_UI_GaugeWidgetColorMultiplier,M_GaugeProgressBars,M_UI_GaugeLedsEmissive,BASE_UI_GaugeOdometerNumber}`
  (the UI materials and their parameters), the 1,349 livery material instances already in the exporter's `parts.json` and the livery,
  mechanics and cockpit pictures. No decompiled code, no binary was read; no game data is in the repository (the widget textures
  go to `C:\r6work\dash-tex`, the composed pictures to each bike's `tex/`).
- **Facts from the data:** the HUD data asset's two class properties; the widget tree node kinds, slots and properties; the brush
  materials' parameters and parents; the gauge configuration's needle mappings; the 1,024 canvas; the livery materials' lack of any
  decal layer.
- **Inferences and choices** (each marked in the page): that the base colour widget becomes the base colour and the emissive widget the
  emissive input of `mat_gauge` (from the property names and the static picture being the base widget's background); that a
  run-time needle angle replaces the design angle; the at-rest values (1,400 rpm, neutral, speed 0, metric); which widgets are
  hidden by name; the 96/72 pixel size of a Slate font unit (checked against 1,651 text boxes); the sum of the two renders; the paint
  occlusion floor 0.9, the hue-keeping lift and the satin metal material.
- **Not found / not done:** the game's `GameSpecificGauge` code (the run-time values and the lamps' rules), the fonts, the master
  materials' graphs; a live dash (see the page for why the host cannot feed one cheaply).
- **Checked by:** the 155 different composed pictures looked at on contact sheets (TFT, LCD and analogue clusters), the KTM RC 390
  and Ducati Panigale V4 screens on the bike in `view-world --camera-mode hood`, seven bikes (Honda CRF 450 R, KTM RC 390 and 690 SMC R,
  Ducati Hypermotard and Panigale V4, Yamaha R1, Suzuki Hayabusa) rendered with `view-car` before and after the paint changes.
