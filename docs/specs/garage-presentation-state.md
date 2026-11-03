# Garage presentation state repairs

This contract covers implementation defects found while following playtest UI-010–018 and UI-020–023.
Evidence is the installed packages, the existing remake source and supplied playtest images. It does not establish
matched original-game appearance, controller feel, camera timing or loading performance.

## Part status

The installed `0xd0582feb` and `0xcffb7033` marker objects start with a hide script that sets both alpha and scale
to zero. Clearing the runtime's hidden flag does not undo these scripted values. When a marker becomes active,
run its existing reveal script `0x0280164f` once. Do not restart the reveal on each refresh or replace native exit fades.
The installed/cart/locked texture and eligibility rules remain those in [garage-game-ui](garage-game-ui.md).

## Palette edits

Opening the gauge or vinyl color page records each channel's current cursor without applying its first swatch.
Changing the focused channel also leaves the selection unchanged. Only an explicit swatch selection previews a
color. A missing color identity remains the native default; accepting a style does not synthesize three color edits.
The Characters channel still covers the gear and speed characters. This repair does not remap that channel based
on an unmatched report; the exact reported gear-versus-speed case still requires a matched reproduction.

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
This is a remake cache policy. It does not make a first new car load asynchronous or establish measured latency.
Entry tries the first saved custom slot before uploading stock fallback, so a valid saved car needs one upload.
An invalid saved slot reports its failure and still loads the available stock car. First-load progress and failures
require separate flow handling.

## Part cameras

A screen selected during a stage intro retains its requested pose. After that intro finishes, the mover starts
the requested screen's authored transition. A newer screen request replaces the deferred target. The pose remains
valid when a car upload completes later; the upload must not reset the shared mover.

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
- UI-017/018: first/repeated vehicle loads need stage timings, visible progress and cancellation/failure checks.
- UI-020: signed left fill geometry must use forward UVs, keep the installed fill Y and retain centered zero state.
  That geometry correction alone does not certify the reference's layer colors or selection treatment.
