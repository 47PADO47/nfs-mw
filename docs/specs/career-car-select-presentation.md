# Career owned-car selection presentation

This covers the safe house's Car Select (`Car_Select.fng` in career mode), including the same selector after a bust.
It is separate from the standalone My Cars editor and the car lot. Rules for impound, release and markers remain in
[busted-aftermath.md](busted-aftermath.md). Camera evidence is in
[menu-backdrop-looks.md](menu-backdrop-looks.md), section 4.3; native customization storage is in
[garage-records.md](garage-records.md).

## Selected identity and actions

**[verified: installed package; ours: binding policy]** Each displayed owned-car handle supplies the car identity,
manufacturer/model logos, preview, heat, bounty and impound values. The package's sample Mitsubishi/350Z images,
raw folder text and duplicate `Debug:` label are hidden or replaced. The two logos use the existing car-lot identity
resolver and installed artwork, including its fallback when a secondary logo is absent.
The bounty/fines and release/cash values, their captions and both marker-count copies use package name hashes.
They must not be passed to the car lot's GUID-only setters: that mismatch leaves the authored numeric samples visible.

Left/Right selects an owned slot without wrapping. Accept keeps the chosen free car, or offers that slot's release
dialog. Back restores the entry car unless its impound state forbids leaving. Reopening starts at the accepted current
car. These rules already existed; presentation must follow their handle rather than a separate preview cursor.

The footer uses the active keyboard/controller bindings inside the package's authored bar. It shows Select,
Accept/Release/Use Marker as applicable, Back when allowed, and the Extra Impound Strike marker action when available.
The marker action is `MenuButton2` (`PAD_BUTTON4`). Sale belongs to the lot mode; the career selector neither advertises
nor handles it. Showcase is absent because this selector has no Showcase action. Native sample PC captions are
suppressed to avoid duplicate or unsupported actions.

## Preview data and lifetime

**[ours: implementation contract]** Career selection runs in the live menu stage with the safe house backdrop. It asks
for the installed `career_car_select` camera collection (`angle4_d`), already identified in the camera provenance.
Each look captures the owned handle, alias and native customization record. A later selection cannot replace the
record while the asynchronous backdrop worker decodes it.

The worker resolves the selected installed car, reads its parts catalogue, and applies the same native record decoder
as the driving customization shop. A matching versioned garage extension restores RGB and remake fields only when
its alias, handle and native-record fingerprint match. The car name must also match. Native palette identities use
the existing extension merge policy. Only the installed catalogue is memoized; records and extensions are read anew
for each request. Geometry assembly uses the restored part selection and explicit saved RGB paint.

A failed selected-record load leaves the backdrop without a car and records the error; it must not display the
unrelated default BMW under the chosen car's identity. Replacing the backdrop discards the old worker receiver, so
an older result cannot replace a newer selection. Closing returns camera ownership to the next screen.
Entering this selector marks the existing safe-house prelude: Resume Free Roam starts a new driving scene with the
selected career car, rather than trying to resume the replaced scene. Non-career bust payment keeps its direct resume.

## Evidence and limits

The 2026-10-28 user playtest showed a Carrera GT beside Mitsubishi/sample identity, raw `Debug:` text, unsupported
Showcase and missing input icons. Read-only `dump-screen Car_Select.fng` inspection identified the two logo images,
duplicate strings and authored footer roots. This batch used existing save readers and source inspection; it did not
introduce a new binary-derived customization format or copy installed artwork.

Prepared `career_batch13_*` regressions cover several installed cars, Back/Accept/reentry, bound keyboard/Xbox prompts,
the safe-house command route, authored camera request, saved body-kit assembly, and RGB extension isolation. These
are package/data checks; rendered pixel parity, physical-controller input and selection latency require a playtest.
The backdrop still rebuilds the room when its selected look changes. It does not yet show the native car-loading bar.
