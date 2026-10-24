# Garage camera input and gauge preview

Sources: the user's installed `GLOBAL/FE_ATTRIB.bin`, `attributes.bin`,
`CustomHUD*.fng`, and gauge texture packs; the matched retail PC executable,
SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
the `dbalatoni13/nfsmw` CC0 reference, read only. See
[garage-game-ui](garage-game-ui.md) and [provenance](../provenance/garage-inspection.md).

## Camera policy and lookup [verified data, static executable]

A category-specific screen collection first falls back to the package's
generic collection, then to `default`. For example, a missing
`customize_customhud_775` uses `customize_customhud`, preserving its camera.
Backroom package names retain their `_backroom` suffix during this lookup.

The screen's `cam_user_rotate` is `EA::Reflection::Bool`, one byte at layout
offset 0x5A, field key `0x998DD0DB`. Missing or wrongly typed policy disables
inspection. It belongs to the screen, independently of its `cam_angle` reference.
The inspected normal customization screens disable it; `showcase` and
`customize_showcase` enable it. There is no `cam_auto_rotate` field in the
inspected schema.

Manual input does not interrupt an authored camera move. Normal menu arrows
remain selection controls. The Showcase arrows control inspection; mouse drag
and the right stick feed the existing orbit actions. The shared action layer
expresses a full right stick as 700 pixels per second; the adapter converts
that representation back to the original normalized rotation rate.

## Native motion and idle behavior [static executable]

The SelectCar mover integrates horizontal and vertical normalized speed at
60 degrees per second. Its radius integrates normalized zoom at 7.5 metres
per second. Ordinary inspection clamps pitch to 94.5..110 degrees and radius
to 4.65..6.65 metres. The original's separate `CarGuysCamera` debug mode bypasses
those clamps; this port does not infer that debug mode from the Showcase screen.

Three hundred idle frontend ticks correspond to five seconds. When the input
axes and zoom are zero, the counter advances toward returning to the selected
screen. That return uses the selected installed camera angle's duration,
decay and order. Afterwards a user-rotatable screen starts horizontal input
0.5 through the same mover, which gives 30 degrees per second. New manual input
stops that automatic motion and restarts the idle counter.

The native speed setters also start a 0.555-second inspection-entry transition
of roll, field of view and target when entering manual control from rest: roll
0 degrees, FOV 45 degrees, target `(0, 0, 0.75)`. The implementation uses the
selected camera's damping and periods for that transition. Rotation and radius
continue independently at their native rates. Releasing the input does not
stop the lens transition; another screen starts from its current visible pose.
The same entry transition applies when idle automatic rotation starts.

## Installed gauge sample [verified packages]

`CustomHUD.fng`, `CustomHUDColor.fng` and both `_BACKROOM` variants contain the
same gauge sample objects. Controllers update those existing objects; they do
not draw a replacement gauge UI.

| Object name hash | Runtime binding |
|---|---|
| `0xC0721EB9` | Tach face: `<scale>_LINES_<skin:02>` |
| `0x05D19F25` | Fill: `TACH_FILL_<skin:02>` |
| `0xD312F0CB` | Tach needle: `TACH_NEEDLE_<skin:02>` |
| `0xC62AD685` | Turbo face: `TURBO_LINES_<skin:02>` |
| `0x8FE2A217` | Turbo needle: `TURBO_NEEDLE_<skin:02>` |
| `0xC5D551B7` | Turbo sample group; hidden without forced induction |

The face scale uses the same engine `MAX_RPM` rule as the driving HUD:
the first of 7000, 8000 and 9000 strictly above `MAX_RPM`, otherwise 10000.
The selected performance package supplies the preview's engine and induction
metadata. Previewing a gauge never changes the installed files or commits a
car customization record.

Backing RGB colors the tach and turbo faces. Needle RGB colors both needles.
Digit RGB colors the speed, gear, units and their dimmed backing strings.
Set RGB after package scripts while preserving each object's current alpha.
Removing an override restores that object's authored RGB. Units use the
installed km/h or mph language label from the current setting. Missing skin
textures fall back to installed stock textures where available.

## Validation boundary

Synthetic tests cover category-name fallback, inspection policy, native rates,
entry easing, release and screen-change behavior, idle return, automatic rotation,
texture bindings and RGB/alpha restoration.
Installed tests cover four gauge packages and eleven styles, image decoding,
screen policy and actual selected-category camera poses. Native renderer
screenshots and physical controller play remain separate checks.
