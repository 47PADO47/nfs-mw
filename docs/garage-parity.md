# Garage implementation and original-game parity

**Draft status:** live playtesting still reports substantial garage bugs. The
implementation is preserved for follow-up work and is not ready for a parity
sign-off. Passing automated checks and selected screen captures do not resolve
those playtest failures. Specific reproduction steps still need to be recorded.

The [2026-10-27 playtest backlog](ui-parity-playtest.md) now lists the reported Back/UI loss, marker, text,
highlight, sound, vignette, camera, gauge-target and loading issues. These remain open despite existing implementation.

The normal garage runs the installed FEng packages through the shared game UI.
Browsing, adding to the shopping cart and purchasing are separate actions. F6 remains
a diagnostic overlay and never changes career cash, markers, heat or saves.
This matrix records implementation scope. It does **not** certify full original
runtime parity. See [garage-shop](specs/garage-shop.md) for evidence and rules,
and [provenance](provenance/garage-shop.md) for sources and comparison limits.
The [original UI contract](specs/garage-game-ui.md) records its screens and flow.

## Entry, selection and persistence

| Capability | Implemented behavior | Comparison still needed |
|---|---|---|
| Main-menu My Cars | Original 5-by-2 MyCarsManager grid, Add last, 20 independent slots and Add Car selector | All imported save variants, progression and camera presentation |
| Career garage | Current owned car only; preview and quote before checkout | Full progression and all safe-house/shop transitions in live play |
| Free-roam shop | Installed shop entry, event/pursuit gates, frozen world and authored exit position/heading | Every installed shop marker in live play |
| Car preview | Customized car on installed crib, customization-shop and backroom platforms; authored camera shots | Original showroom, lights, camera poses and animation timing |
| Discard | Scene restores its entry selection; no purchase or native-record change | Live cancel from every popup and failure state |
| My Cars recall | Profile-scoped independent slots, multiple same-model setups, add/delete and native imports | Original slot limits, imported save variants and full quick-race interaction |
| Career recall | Native customization record plus fingerprint-bound extension data | Imported original saves, modified pack order and every vehicle variant |
| Save failure | Checkout restores failed purchases; journal recovery binds extensions to the full native commit | Power-loss behavior and external concurrent profile writers |
| F6 diagnostics | Immediate session changes and explicitly saved diagnostic presets | No claim that this developer screen matches an original menu |

## Models, materials and visual parts

| Capability | Implemented behavior | Comparison still needed |
|---|---|---|
| Body kits | Compatible installed geometry and dependent parts; stock restoration | Every car, kit, damage model, wheel offset and attachment |
| Spoilers | Installed compatible choices, stock, paintable and carbon variants | Original availability and all attachment transforms |
| Hoods | Installed compatible choices including carbon variants | Original labels, unlocks and every mesh/material combination |
| Roof scoops | Installed compatible choices and attachment requirements | All vehicles and original carbon/material appearance |
| Rims and diameter | Installed wheel identities, including authored sizes; four-wheel replacement | Original brand grouping, size filtering, scaling and all wheel setups |
| Body paint | Installed finish/color choices, custom RGB, stock restoration | Original material response and color palette presentation |
| Paint finishes | Installed gloss/metallic/pearl and any compatible extra finish | Original swatch ordering, highlight appearance and mod-specific finishes |
| Rim paint | Presets/RGB gated by paintable aftermarket rims | Every rim's material mask and original palette |
| Window tint | Installed compatible material choices independent of paint | Glass blending and original tint appearance in all lighting |
| Vinyl artwork | Installed fixed-UV compatible artwork, stock restoration | All car/vinyl combinations and original category grouping |
| Vinyl recolors | Original native swatch identities and supported channels, committed with their parent vinyl | Every palette, alpha and composition appearance |
| Manufacturer decals | Six installed zones, available authored positions, body-dependent controls | Every car/body/zone/position and original decal palette/placement |
| Door numbers | Original two-digit widget, mirrored doors, one paired cart entry and purchase fee | Every body kit, numeral style and transparency |
| Custom gauges | Installed skins, three native color channels and original in-menu gauge samples | All HUD themes, drag gauges and material appearance |
| Stock restoration | Each part row can restore stock/remove; related dependencies retained or cleared | All stock presets, unusual integrated geometry and original refund rules |
| Fixed liveries | Unpaintable installed liveries remain fixed | No arbitrary livery recoloring is claimed |
| Movable artwork | Fixed authored positions and artwork | Arbitrary vinyl/decal movement is not implemented |

Paint finishes and gauge artwork depend on the existing rendering/HUD settings.
Simple car shading does not establish material parity. Custom gauges use the
Original HUD and installed preview artwork. The normal garage uses installed
FEng screens, templates, fonts, textures and animation scripts. F6 alone retains
the diagnostic egui controls. Original captures cover the manager, main menu,
parts, performance, engine, visual, paint and empty quick-race cart; they do not
establish every car, purchase or controller path.

## Performance and tuning

| Capability | Implemented behavior | Comparison still needed |
|---|---|---|
| Engine | Installed car-specific stage range and endpoint interpolation | Original acceleration, power display and every vehicle/stage |
| Transmission | Installed packages and drivetrain data | Original ratios, shift behavior and performance meter |
| Suspension | Installed chassis packages and tuning gates | Original contact response, ride height and handling |
| Nitrous | Installed packages, factory equipment and consumable rules | Original duration, power and all stock/upgrade combinations |
| Tires | Installed grip-related packages | Original traction across surfaces and original meters |
| Brakes | Installed packages and brake-bias tuning gate | Original stopping behavior and all tire/package combinations |
| Induction | Installed turbo/supercharger packages and tuning gate | Original boost response and category labels for every vehicle |
| Junkman | Installed compatibility plus normal shop/backroom eligibility checks | Original marker availability, unlock paths and all stacking cases |
| Performance bars | Original stock/max population, frontend estimate, category weights, Junkman blend and installed-value markers | Matched live values for all cars and modified physics tables |
| Tuning profiles | Original pause-menu screen, three profiles, seven eleven-position sliders, Help and discard confirmation | Imported float values and every live race transition |
| Tuning gates | Required equipment; aero requires aftermarket body/spoiler; pause context restrictions | Every unusual factory aero setup |
| Locked tuning values | Retained values follow existing tuning specification | Original behavior for every downgrade/removal sequence |
| Same-car replacement | Existing replacement preserves pose and driving state | Live damage, audio, nitrous and camera across every part change |

The package and performance-bar implementations have separate original static
evidence. That does not certify full driving physics or all live rating values.

## Career economy and unlocks

| Capability | Implemented behavior | Comparison still needed |
|---|---|---|
| Pricing | Installed frontend category/level records; missing prices reject purchase | Original displayed totals and all authored zero-cost cases |
| Cart identity | Original ShoppingCart packages, per-item enable/remove, clear all, checkout confirmation | Original cart ordering and matched live purchase comparisons |
| Career unlocks | Installed bin awards, independent quick-race levels and persistent profile completion | All original award timing and special rewards in live play |
| Funds | Checked before checkout; confirm owns cash debit | Original corner cases for reductions/refunds and mixed carts |
| Trade-in | Original binary confirms eligible slots, one-quarter credit and signed cash refund | Live purchase/refund comparison, mixed-cart ordering and modded routines |
| Backroom | Alternate purchase mode with category markers | Original one-level-ahead behavior and every marker/category combination |
| Marker consumption | Exact kind/parameter redemption at confirm; original vinyl count wildcard remains separate | Duplicate category items and all reward sources in live play |
| Heat | Normal confirmed career customization uses installed cooling factors | Per-item ordering, number/decal special cases, normal/backroom factor differences |
| Atomic preview | Browsing does not purchase or alter native installed parts | All render/assembly/save failure recovery paths |

The original shopping-cart packages are bound to the transaction backend.
Locked purchases surface a quote error. Purchase remains disabled until the
active cart has a valid quote. Original live purchase comparisons remain separate.

## Input and extensibility

| Capability | Implemented behavior | Comparison still needed |
|---|---|---|
| Pointer | Original package hit targets, icon strips, paint palettes, cart and confirmation buttons | Live interaction at all supported resolutions |
| Keyboard/gamepad | Original category and choice navigation, contextual prompts, grids and per-screen trigger actions | Physical controller playtest and original repeat timing |
| Save/discard input | View Cart opens the original cart; checkout confirms its active items; Back follows the screen stack | Original prompts, controller-specific labels and popup focus |
| Stock input | Original stock choice in each part category; F6 retains its diagnostic restore controls | Original per-screen button assignments |
| Input ownership | Normal screens own the front-end input; driving and F6 cannot bypass their transaction | Held controls and rapid open/close under live input |
| Modded installs | Catalogues read installed compatible cars/parts rather than a fixed public list | Arbitrary new formats, slots, multi-pack ordering and every external mod |
| Mod identities | Native indices stay install-specific; extensions retain extra selection data | Portable identity migration after an installed mod changes |
| New categories | Existing catalogues are data-driven within supported native slots | No general plugin API for arbitrary new slots or garage categories |

Synthetic, installed-data and renderer checks must be recorded separately.
They cannot replace original/remake live comparison. This file deliberately
retains each unresolved comparison after the corresponding feature exists.
