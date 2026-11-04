# Menu preview lifetime repairs

## Same-stage career selection

Changing only the car look on an existing menu stage must not close the room and replace it with a flat frame.
The existing MenuScene accepts a same-stage look and starts its worker while retaining the current room.
Until the new car is uploaded the room stands empty: the outgoing car, its casters and its floor reflection are
not drawn. The recorded original car select does the same and runs its authored loading LED row meanwhile
(`Car_Select.fng`, messages `0xA05A328E`/`0x913FA282`). The outgoing car releases its renderer resources once the
new one replaces it. A newer request replaces the pending worker owner. Cross-stage changes retain the normal scene
replacement path and start the correct stage camera.

This is a source-lifetime repair for UI-049 and the R2 loading state. It does not establish measured
decode/upload latency, correct partial-load failure messaging, or smooth frame pacing during renderer upload.

## Garage edits of the shown car

In My Cars and customization, a part, paint, gauge or tuning preview of the car already shown keeps that car
visible until the new model is ready, as the recorded original does while browsing spoilers. Only a different car
empties the room. The selected item's caption stays its plain name; the original shows no loading text there.

## Garage capture submissions

GarageBackdrop uses the existing MenuScene still-input key before resubmitting shadow casters and planar captures.
The key includes the camera, instances, surface, graphics, shading, shadow options and receiver texture. Visibility
changes invalidate the remembered key so a hidden preview cannot leave its casters or reflection active and an
unhidden preview republishes both. Geometry or camera changes still update them.

This removes redundant submissions during idle/nonvisual edits. UI-045's visible full-shadow redraw report remains
open: the real backend also caches shadow inputs, so submission counts alone do not prove the reported flicker fixed.

## Wheel height

Car assembly already lowers the static tyres by the front-end sink. Menu stage stands apply no second sink.
Installed wheel-record regressions cover three different cars on all five stages. Exact contact on replacement
wheel meshes and ground geometry still needs rendered acceptance.
