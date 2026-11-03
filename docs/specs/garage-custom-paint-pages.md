# Custom body-paint pages

UI-039 requested one Custom page instead of individual pages named after internal paint families.
This presentation policy is independently authored from that report and the existing catalog/UI source.
The previously recorded modified game exposes a Custom family with forty swatches. This is a bounded
observation; the remake's grouped palette size follows the installed catalog rather than a fixed forty colors.

Group nonstandard body-paint families into one localized Custom page. Preserve the separate Gloss, Metallic
and Pearl pages, catalog ordering, original material hashes and catalog option indices for preview/cart operations.
Rim, vinyl and gauge palettes retain their separate handling.

When reopening, resolve a stored material hash before falling back to RGB. Equal RGB values do not establish
that two paint materials are interchangeable. Opening the palette must not replace a saved material with the
first equally colored swatch. Cancel restores the entry selection through the existing transaction path.

The batch 13 installed regression passes against the modded reference packages. It checks a saved second
custom material, equal-RGB browsing and cancellation. Formatting, changed-file size and whitespace checks pass.
Broader paint checks on both installations and full Cargo guards were queued behind another shared-cache build
and canceled before execution. No full-suite or rendered-reference acceptance is claimed. Palette ordering,
large custom-palette paging and manual apply/return checks remain open.
