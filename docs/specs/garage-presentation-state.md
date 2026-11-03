# Garage presentation state repairs

This contract covers implementation defects found while following playtest UI-010–018 and UI-020–023.
Evidence is the installed packages, the existing remake source and supplied playtest images. It does not establish
matched original-game appearance, controller feel, camera timing or loading performance.

## Part status

Cart checkmarks keep the package's horizontal spacing but align vertically with the center of rendered label glyphs,
including native font scale and justification. An authored sample string's origin is not a text-center measurement.
Retained My Cars managers replay their complete native INIT tracks before rebinding their grid, selected slot and
mode on Back. Restoring the scrollbar alone does not restore other roots faded by a child's global leave event.
Reapply the selected tile's native highlight after INIT; the runtime's current-button identity may already be
correct while its separate cursor has been reset. Post that highlight once without an automatic focus-sound pair.

The native gauge color palette has a separate selection-frame group. Move that group to the selected visible
swatch while retaining its authored offset, border geometry and highlight animation. Swatch highlight scripts
alone do not move the frame. Preserve its position on idle refresh; update it across channel restoration and
swatch paging in both normal and backroom packages (UI-046). Gauge color-channel targets remain a separate report.

The installed `0xd0582feb` and `0xcffb7033` marker objects start with a hide script that sets both alpha and scale
to zero. Clearing the runtime's hidden flag does not undo these scripted values. When a marker becomes active,
run its existing reveal script `0x0280164f` once. Do not restart the reveal on each refresh or replace native exit fades.
The installed/cart/locked texture and eligibility rules remain those in [garage-game-ui](garage-game-ui.md).

## Performance strip layering

The recorded modded-PC manager shows a bright cyan lit extent over the dim full-width stat strip. Installed
`FILLBAR_*` precedes `BASE_*` at equal depth. The runtime draws later equal-depth siblings on top, so the dim
base covered the lit artwork. Place each lit fill just in front of its base, retaining native RGB, alpha, UV pitch
and the installed-value marker. Repeated updates must keep a stable depth instead of moving the fill each frame.
This covers My Cars and performance customization; other stat presentations need separate reference checks.

## Palette edits

Opening the gauge or vinyl color page records each channel's current cursor without applying its first swatch.
Changing the focused channel also leaves the selection unchanged. Only an explicit swatch selection previews a
color. A missing color identity remains the native default; accepting a style does not synthesize three color edits.
The supplied original-game Lettering Color screenshot supersedes the earlier gear-only interpretation. Lettering
tints the tachometer scale artwork (including its printed numbers), turbo scale and speed-unit label. Digital speed
and gear readouts keep their native colors. Backing tints the separate fill artwork; Needle tints both needles.
Apply the same channel mapping in the preview and driving HUD. Exact rendered style remains acceptance work.

Some garage controls, such as paint swatches, expose HIGHLIGHT/UNHIGHLIGHT without generic gain/lose-focus responses.
Apply those existing tracks explicitly when present, changing the script only when its selection state changes.
Palette swatches retain their palette RGB. Number, saved-car and cart fields retain the existing tint fallback
only when they lack native tracks. Set runtime focus only on a new selection, and let native script clocks advance.

## Preview work

My Cars retains the last successfully uploaded geometry selection. A request with the same car, customization
parts and body color reuses that upload after selection validation. Performance, tuning and gauge values still
update their session state. A failed replacement does not replace this key. Closing the scene clears it.
The garage also retains the two most recently used inactive previews, each with its complete CPU model and
renderer resources. Returning to their car/parts/color key skips geometry parsing, assembly, texture decoding
and upload. The visible preview is validated first; a parked preview resumes the current shading, room lighting
and camera cut. Its fallback orbit rest and intro progress remain with that car; the shared native stage mover
remains outside the cache. Eviction and scene close release every owned mesh, texture and material exactly once.
This is a remake cache policy; it does not establish measured latency.

Standalone My Cars reads its metadata and prepares uncached car models on worker threads. One decoder runs at
a time, with one replaceable latest request. Intermediate requests do not accumulate. A completed result can
publish only when its car, parts and body color still match the latest request, or its saved-slot identity matches
the latest slot read. Session values that do not affect geometry come from that latest request. Stale errors are
discarded too. Returning to an uploaded cached preview can finish immediately without waiting for an old decode.

Entry retains the native My Cars menu instead of opening the full-screen loading package. A local caption marks
metadata/preview work. Once metadata is available, direction browsing may replace the requested car. Back remains
available; accepting, saving, deleting and confirming a pending selection wait until its preview is validated.
The old car, its shadows and its planar reflection are hidden while the new preview is pending. Its room remains.
Failure restores the prior draft and saved-slot identity and exposes the error without publishing partial geometry.
Closing drops the request owner; an already running CPU read may finish but cannot publish into the closed scene.

Entry tries the first saved custom slot before uploading stock fallback, so a valid saved car needs one upload.
An invalid saved slot reports its failure and still loads the available stock car. Room reads and renderer uploads
still run on the render thread. Texture upload conversion can also run there; this change does not make all first
load work asynchronous. The contract covers standalone My Cars, not the separate Career preview loader.

## Part cameras

A screen selected during a stage intro retains its requested pose. After that intro finishes, the mover starts
the requested screen's authored transition. A newer screen request replaces the deferred target. The pose remains
valid when a car upload completes later; the upload must not reset the shared mover.

## Returning to My Cars

A retained standalone garage returning from customization to the car overview keeps its current
camera pose. Changing the room must not restart the main-menu intro. The overview then requests
its installed screen transition from that retained pose. Fresh garage entry still uses its intro;
a missing camera book or pose uses the existing initialization fallback. Repeated overview refreshes
do not restart the movement. This is a remake lifecycle repair, not a claim about every original
camera path or room-loading performance.

## Showcase aspect

Showcase selects its CURRENT_GEN_WIDESCREEN (`0xcb835ee3`) group scripts above the existing aspect threshold of 1.4.
Both this message and `0x3d4b7270` select wide art in the installed package; they are not complementary.
Normal aspect restores only the normal group's INIT and the wide group's HIDE scripts. Apply the choice after
package start/resume and when crossing the threshold. Select both groups' scripts atomically: a queued older choice
must not override a newer aspect before the next update. Child artwork, bounds and entrance/exit alpha tracks remain native.
This repairs missing dispatch; matched original-game vignette appearance remains separate acceptance work.

## Remaining comparisons

- UI-011/012: authored colors, active highlights and footer/item-text bounds need rendered reference comparison.
- UI-014: Showcase's authored aspect dispatch does not certify matched reference appearance, art or opacity.
- UI-015/016: every part/style camera and the reported exact gauge target remain live acceptance work.
- UI-017/018: worker publication, cache reuse and cancellation need installed regression checks. Actual first/repeated
  load latency and frame pacing still need measured playtests, including render-thread upload work.
- UI-020: signed left fill geometry must use forward UVs, keep the installed fill Y and retain centered zero state.
  That geometry correction alone does not certify the reference's layer colors or selection treatment.
