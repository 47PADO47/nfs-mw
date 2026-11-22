# Custom body-paint pages

UI-039 requested one Custom page instead of individual pages named after internal paint families.
This presentation policy is independently authored from that report and the existing catalog/UI source.
The previously recorded modified game exposes a Custom family with forty swatches. This is a bounded
observation; the remake's grouped palette size follows the installed catalog rather than a fixed forty colors.

Installed PC swatches follow the compatible part list, filtered by the paint brand.
Display names and RGB values do not determine their order. Stock occupies its installed
position. The rim page has a fixed Powder Coat caption and no finish-switch controls.
Each of the three twenty-colour rim rows retains the selected part's light material.
The English reference shows white at Gloss 1, gold at Metallic 47, and matching
blue-grey swatches with different finishes at rim positions 3, 23 and 43.

Native Gloss, Metallic and Custom pages use their installed brands. Mod-defined paints
without native brand metadata retain the family grouping supplied by the mod.
Swatches keep their catalog option indices for preview and cart operations.

Opening a palette resolves the fitted part by identity before falling back to RGB.
Cancel restores the entry selection. Body paint uses the current controller bindings
for its finish icons. The blackletter headings on the two paint screens render the
localized titles in lowercase.

The installed regression covers saved swatches, movement, cancellation, stock paint
and equal-RGB finishes. Large mod palettes and purchase persistence are separate cases.
