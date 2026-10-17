# Runtime garage overlay

Press **F6** in a driving or world scene to open the garage. Press F6 or Escape,
or click the window's close button, to return. The `garage` input action can be
rebound with the existing `bind` / `bind-save` commands. `--open-garage` starts a
world viewer with the overlay open, including screenshot runs. The console and
garage do not take input at the same time; `--open-console` takes precedence.
Use `--open-garage --garage-tab tuning` to start on the tuning controls.

The garage pauses the world's driving simulation while streaming continues.
It takes the keyboard and releases the cursor. Closing restores the previous
cursor capture. The front end receives no garage clicks or closing Escape.
Open the garage from gameplay, rather than the front end's pause menu.

## Available controls

- **Racing cars** and **Police** tabs contain only installed vehicles of those
  categories with usable physics, engine and gearbox data. Traffic, trucks,
  trailers, accessories and vehicles without a working powertrain are excluded.
  Names use the installed language table where available, with readable fallbacks.
- Search by car name, manufacturer or folder name. A grid fills the browser's
  available height; widening the window adds columns and enlarging it reveals
  more cars. Performance and appearance/camera have separate options tabs.
- **Performance** offers Stock, Maximum and Max + Junkman presets, plus all
  seven independent categories: engine, transmission, induction, nitrous,
  suspension, tires and brakes. Each category uses that car's installed limits
  and endpoint records. Intermediate stages and Junkman follow the
  [original package rules](specs/performance-upgrades.md). A variant with
  missing upgrade records keeps its original equipment and explains the limitation.
- Upgrades apply immediately without moving, stopping or repairing the car.
  The HUD, engine/transmission/induction audio and engine-upgrade exhaust
  eligibility follow the selection. Stock restores the original installed
  records, including absence of nitrous on cars without original equipment.
  A car that has factory nitrous keeps it at stock. Junkman eligibility follows
  the installed category and requires a nitrous/induction kit for those categories.
- **Tuning** offers three profiles with eleven positions for steering, handling,
  brake balance, ride height, aerodynamics, nitrous and induction. Zero is
  neutral. Original help directions appear beside each slider. Suspension,
  brake, nitrous and induction upgrades unlock their corresponding controls;
  stock equipment and Junkman alone do not. Aerodynamics requires a selected
  aftermarket body kit or spoiler, which the current overlay cannot install.
  Locked controls explain their requirement. Removing an upgrade retains
  profile values and their physics effects, matching the original; **Reset
  profile** restores the active profile to neutral. Profile and slider changes
  apply immediately without respawning, repairing or stopping the car.
- Select any of the nine existing camera modes. This changes the live camera
  setting immediately. Original camera modes use the existing installed view
  data and retain the existing fallback when that data is absent.
- Choose an RGB body color for cars with generated paint textures. The
  installed gloss and paint material remain unchanged. Fixed liveries keep
  their original textures and have the picker disabled.
- Car and paint changes apply when selected. The window can be resized by
  dragging its edges or corners. Loading validates the selection before replacing
  the current car. Changing cars places the new one at rest on a nearby road.
  Repainting the current car preserves its physics pose, velocity and damage.
  Select **Original paint** to restore the installed paint. Preset swatches and
  **Custom paint** apply live. Dragging the color picker reuses the loaded model
  instead of reading the geometry again.

Selections apply to the current session. Expand **Saved presets**, enter a name
and **Save preset** to retain the complete car, paint, model parts, performance
packages and all three tuning profiles in `garage-presets.toml` beside the user
config. Choose a saved name and **Load preset** to apply it immediately. Saving
the same name replaces it. Presets survive car changes and application restarts;
they do not load automatically at startup. Unavailable cars, stages and parts
are rejected through the normal garage validation before replacing the rig.
No original game assets or career save are modified. Restoring installed paint
reloads the stock assembled model. Career save integration remains separate work.

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
Livery recoloring, vinyl compositing, automatic per-car selection recall and controller
navigation within egui are also deferred. The existing camera modes have their
existing behavior; this contribution does not establish camera parity.
