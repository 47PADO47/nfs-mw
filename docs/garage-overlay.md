# Diagnostic garage overlay

Press **F6** in a driving or world scene to open the diagnostic garage. Press F6 or Escape,
or click the window's close button, to return. The `garage` input action can be
rebound with the existing `bind` / `bind-save` commands. `--open-garage` starts a
world viewer with the overlay open, including screenshot runs. The console and
garage do not take input at the same time; `--open-console` takes precedence.

The garage pauses the world's driving simulation while streaming continues.
It takes the keyboard and releases the cursor. Closing restores the previous
cursor capture. The front end receives no garage clicks or closing Escape.
F6 is a developer tool: changes apply immediately to the current running rig.
It never purchases career parts, spends career cash or markers, cools career
heat, or writes the career car's customization record. Its explicit named
presets remain separate user files. The normal **My Cars**, career **Garage**
and in-world shop entries use preview, purchase/save and discard instead.
See the [garage parity matrix](garage-parity.md) for those paths and limits.

## Available controls

The F6 garage is about **which vehicle you drive**. Everything about the setup of the vehicle you are in (parts, appearance,
performance, tuning, saved setups, the rider's gear) is on the **vehicle panel (F11)**, which lists what the vehicle's own data offers
and works in the car viewer too: see [devtools-vehicle.md](devtools-vehicle.md). F6 used to have those as tabs on its right; they were
moved, and a button there, **Vehicle options: open the vehicle panel (F11)**, opens the panel.

- **Racing cars** and **Police** tabs contain only installed vehicles of those
  categories with usable physics, engine and gearbox data. Traffic, trucks,
  trailers, accessories and vehicles without a working powertrain are excluded.
  Names use the installed language table where available, with readable fallbacks.
- Search by car name, manufacturer or folder name. A grid fills the browser's
  available height; widening the window adds columns and enlarging it reveals
  more cars. Click a car to drive it.
- **Preview car** hides the browser to see the car; **Browse cars** brings it back.
- **Exhaust** and **Backfire** (the flame styles) stay here: their shape and motion are shared across cars and saved
  automatically, and the previews need the world frozen, as it is while F6 is open.
- Select any of the nine existing camera modes with F5. This changes the live camera
  setting immediately. Original camera modes use the existing installed view
  data and retain the existing fallback when that data is absent.
- Car changes apply when selected. The window can be resized by
  dragging its edges or corners. Loading validates the selection before replacing
  the current car. Changing cars places the new one at rest on a nearby road.

`--open-garage` opens F6; with `--garage-tab performance|tuning|appearance|parts` it opens the vehicle panel on that page
instead (the tabs F6 had), since that is where they are now.

Selections apply to the current session. The named setups (**Saved setups** on the vehicle panel) retain the complete car, paint,
vinyl colors, rim paint, gauges, model parts, performance packages and all three tuning profiles in `garage-presets.toml` beside the
user config; see [devtools-vehicle.md](devtools-vehicle.md). Unavailable cars, stages and parts
are rejected through the normal garage validation before replacing the rig.
No original game assets are modified. Diagnostic changes do not cool career
heat or enter career saves. Normal career checkout owns the separate cash, marker, heat and save work.

Speedbreaker toggles with **Left Ctrl**, the mouse **Back thumb button**, or
**Xbox B / PlayStation Circle**. Wheel buttons and pedals can use the same
saved `speedbreaker` action; see [controller settings](controller-settings.md).
It requires the original activation speed and available meter, and pauses
with the garage. Holding a button does not repeatedly toggle it.

## Deferred

The package resolver has static and isolated original-machine-code evidence.
The driving model also includes documented handling corrections, but complete
original driving parity requires reference driving comparisons across surfaces,
cornering, jumps and contacts. See the vehicle and Speedbreaker specs for
the implemented behavior and remaining comparison limits.
Movable decals and fixed-livery recoloring are unsupported. The normal garage
adds fixed-position decals, paired door numbers, per-car saves and action-based
controller navigation. The legacy F6 widgets retain their pointer controls.
The existing camera modes have their existing behavior; this contribution does
not establish camera parity.

A mod car whose manifest has a `[customization.plate]` also gets a collapsed **License plate** row on the vehicle panel's Appearance page: a text field
(applied on Enter or when it loses focus; the characters the car's glyph sheet draws, at most its slots) and, when the plate offers
designs, a design chooser. Other cars show no row. The console has the same for the car being driven: `plate [TEXT|reset]` and
`plate-design [ID|name|none|list [filter]|reset]` (`this car has no plate` otherwise). Both are session-only like the rest of the diagnostic tools;
the normal garage's License Plate and Plate Design pages keep a text and a design in `plates.toml`
([plates](modding/car-customization.md#license-plates)).

Console captures can select the same identities with `parts paint <index>` and
`parts tint <index>`; `parts` lists available choices and `stock` restores each
category. Existing saved presets retain their original five model-part slots;
paint, tint, rim paint and vinyl use appended slots and remain optional.
Additional console pairs: `rimpaint <index>`, `vinyl <index>`, `gauges <0..10>`,
`bodycolor`, `rimcolor`, `vinylcolor1`–`vinylcolor3`, `gaugebacking`, `gaugeneedle`
and `gaugedigits`. Color values are `RRGGBB` or `stock`.
See [fixed-vinyls.md](specs/fixed-vinyls.md) for composition limits.
