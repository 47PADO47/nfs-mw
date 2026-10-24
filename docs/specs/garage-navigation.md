# Garage navigation

## Source and limits

**[decomp]** Behavioral reference:
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
`MyCarsManager.cpp`, `feArrayScrollerMenu.cpp` and `uiQRCarSelect.cpp`.
These establish static navigation rules. They do not establish a matched live
original/remake input or animation comparison. The source marks `ScrollCars`
as unresolved overall; the circular previous/next list operations are explicit.

## Saved My Cars grid

The original manager constructs a five-column, two-visible-row array scroller.
Horizontal navigation wraps within each five-wide row, including a short final
row. Vertical navigation stops at the outer data rows. Down into an absent
column of a short final row selects that row's last tile. The Add tile counts
as a data entry and participates in these rules. Moving the selection previews
the matching saved record; it does not create or save a record.

The ten image slots form a moving two-row window. Its first row is persistent,
not a page number. Moving below its second row advances the window by one
five-item row; moving above its first row moves it up one row. Pointer slots
use this same offset. Entering the manager focuses the active saved record;
when it starts beyond the first ten entries, its row is initially the lower
visible row. Returning from Showcase retains the window when selection fits.

The native scrollbar counts data rows, including Add, with two visible rows.
Hide backing, handle and arrows when no scrolling is needed. Otherwise resize
the handle to `2 / total_rows` of the authored backing height, and place its
top at the matching first-row fraction of the remaining travel. Its native
arrows move one selection row and use authored DIM/NORMAL endpoint states.
The original position/extent rule comes from `FEScrollBar::Update` and
`SetPosResized` in `feWidget.cpp`.

**[verified, modified PC copy]** A later read-only live check observed backing
clicks above/below the thumb move the selected car by one five-car row through
Up/Down, with viewport movement as needed. No proportional jump was observed.
Two controlled thumb-drag attempts instead opened the current car's
customization screen on release. Match ordinary thumb-click acceptance; do
not infer a continuous drag API from those attempts. No purchases, additions,
deletions or saves occurred, and profile hashes remained unchanged. This check
does not establish behavior on other original platform variants.

## Add Car stock list

Customization mode uses the stock-car list. Left/Right use circular previous
and next selection, wrapping between its first and last car. Up/Down are list
switch actions in the general car selector; customization mode ignores them.
This behavior is separate from the manager's five-wide grid navigation.

The initial selection previews a stock setup. Navigation remains transient;
only confirmation creates a custom record, and Back restores the prior record
and preview. See [garage game UI](garage-game-ui.md) for persistence boundaries.

## Paint controls

Body finishes retain their native localized names: Gloss, Metallic and Custom
for the installed GLOSS, METAL and PEARL families. Unknown compatible mod
families retain their supplied name. Changing a finish restores that finish's
swatch cursor. The PC arrow images require explicit pointer hit-testing because
they have no button response. Rim paint hides the filter and uses one fixed
palette; body finishes do not alter its stock swatch.

The native twenty-column paint grid wraps horizontally within each row.
Vertical movement stops at the outer rows and clamps an absent final-row
column to the final swatch. Gauge-color Up/Down wraps the list; its authored
vertical pointer arrows use the same transitions. Changing gauge layers retains
each layer's cursor and native part identity.

## Decal color controls

On return from Showcase or another child page, replay the installed decal
filter group's entrance. The group's native leave animation fades it to zero;
restoring its label alone does not restore the control. The same entrance rule
applies to rim sizes, spoiler families and body/vinyl paint filters. The offscreen
`Toggle Colour` event button retains its authored literal text and position.
