# Lamps and lights of a mod car

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) lights its own lamps:
the flares of its head and brake lamps and the lit or unlit look of its lamp surfaces come from its own files. Nothing is
taken from a stock car, and a mod car needs no `ecar` record. There is no manifest table for lamps: the contract is
**markers on the body solid** and **placeholder textures**, the same two things the game's own cars use.

## Lamp markers

The game draws a lens flare at every lamp marker of every solid of the car (`scenes/world/car_lights.rs`). A marker is a
named position in `GEOMETRY.BIN`; a mod car puts them on its body solid, `<ID>_KIT00_BODY_A` to `_D`, next to the tail pipe
markers ([car-customization.md](car-customization.md#tail-pipes)). The marker's translation is the lamp's place in car
space (+x forward, +y left, +z up, metres), its **z axis is the way the lamp shines** (a flare shows only to an eye in front
of the lamp, brightest straight on) and its x axis points up, as on the game's own cars.

| Marker | Lamp | Lit when |
|---|---|---|
| `LEFT_HEADLIGHT`, `RIGHT_HEADLIGHT`, `CENTRE_HEADLIGHT` | head lamps (shine forward) | the headlights are on |
| `LEFT_BRAKELIGHT`, `RIGHT_BRAKELIGHT` | brake and tail lamps (shine backward) | dim with the headlights on or off, full while the brake is held |
| `BRAKELIGHT_CENTRE` (or `CENTRE_BRAKELIGHT`) | the centre high brake lamp | the brake is held |
| `LEFT_REVERSE`, `RIGHT_REVERSE`, `CENTRE_REVERSE` | reverse lamps (shine backward) | the car reverses (the gear is reverse) |

- `CENTRE_REVERSE` is a rewrite extension for a reverse lamp on the centre plane (a single low lamp, as on the 2012 Porsche
  918 Spyder): the game's own cars have only the left and right ones. It is drawn like them and has no damage bit, so it is
  never broken. The reverse flare is white and is the game-wide `reverselight_inner` profile.
- The marker's name also says which lamp it is for **damage**: a broken left head lamp leaves the right one lit
  ([vehicle-lamp-damage.md](../specs/vehicle-lamp-damage.md)). Two markers of one name at different places are two lamps
  (a cluster); closer than 1 cm they are one.
- The flares use the game-wide flare profiles (`light_flares_cg` of the gameplay database: size, colour, texture), the same
  for every car. A mod car cannot change the colour or size of its flares; it chooses where they are and which way they face.
- Turn signals are drawn by the game from the head and brake lamp markers, when the `turn_signals` setting is on; a car
  needs no marker for them.
- A car with none of these markers has no flares. It still drives and shows its lamp surfaces.

## Lamp surfaces

A lamp's surface changes with its state through placeholder textures. A group of a solid whose diffuse texture is named
`HEADLIGHT_LEFT`, `HEADLIGHT_RIGHT`, `BRAKELIGHT_LEFT`, `_RIGHT`, `_CENTRE`, `TAILLIGHT_LEFT`, `_RIGHT`, `_CENTRE` or
`REVERSELIGHT_LEFT`, `_RIGHT`, `_CENTRE` is drawn with the car's own texture for the lamp's state, found by name in the
car's `TEXTURES.BIN`:

| Texture | Is drawn when |
|---|---|
| `<ID>_KIT00_HEADLIGHT_ON`, `<ID>_KIT00_HEADLIGHT_OFF` | the head lamps are on, off (a car starts with them on) |
| `<ID>_KIT00_BRAKELIGHT_ON`, `<ID>_KIT00_BRAKELIGHT_OFF` | the brake is held, not held (starts off) |
| `<ID>_KIT00_TAILLIGHT_ON`, `<ID>_KIT00_TAILLIGHT_OFF` | the head lamps are on, off (starts on): the tail lamps burn with them |
| `<ID>_KIT00_REVERSELIGHT_ON`, `<ID>_KIT00_REVERSELIGHT_OFF` | the gear is reverse, not (starts off) |

A state whose texture the car does not have keeps the other one. A broken brake lamp stays dark while the others light up;
the tail lamps share the damage bit of the brake lamp of their side, and the reverse lamps have none. `<ID>` is the car's
folder in upper case. The two states of each kind are shared by all its placeholders, so the left and right lamps of one
kind look the same. Every placeholder group should use a light material that shows a flat surface (the 2012 mod uses
`MATTEPLASTIC`). The head, brake and tail slots are the game's own (a stock car's lamps use the first two); the tail and
reverse slots are an extension a mod car can use, and the game's own cars never name them. With the **Enhanced vehicle lamps**
shading ([experimental lamp surface](../specs/experimental-lamp-surface.md)) a lit slot also glows: red for the brake and
tail slots, white for the others.

## The 2012 cars

The [2012 mod](../../mods/nfsmw-2012-content/README.md) fills both from each car's own 2012 data
([chameleon-lamps.md](../formats/chameleon-lamps.md)):

- **Markers.** The 2012 game places its lamps with *locators* (numbered placements) and *lamp records* that name a
  locator and a bit of the vehicle's light state. The mod takes the front records of the headlight bits and the rear records
  of the brake bit (or, for a car that has none, of the tail bits), and writes a marker at each place and at its mirror
  image across the centre plane, facing the way the locator faces (corrected to point to the car's end if it does not).
  Amber records (turn and position lamps) are not head lamps. A rear locator on the centre plane is the centre brake lamp.
  At most two markers of one kind are written per side. 92 of the 93 cars have both; the Porsche 911 gets head lamps at
  (2.24, +-0.69, 0.66) and brake lamps at (-2.03, +-0.63, 0.75), plus a centre lamp at (-1.79, 0, 0.96).
- **Reverse markers.** The rear records of light-state bit 2 are the 2012 reverse lamps (89 of the 93 cars have them, white
  or grey-white, see [the format notes](../formats/chameleon-lamps.md#the-reverse-lamp-is-bit-2-verified-by-data)). They give
  `LEFT_REVERSE` and `RIGHT_REVERSE` at each place and its mirror image, or one `CENTRE_REVERSE` for a lamp on the centre
  plane. The Porsche 918 Spyder gets a single centre lamp at (-2.31, 0, 0.31), low at the rear face. The four cars with no
  such record (Cobra 427, Audi A1, Huayra, one unnamed bundle) have no reverse flare.
- **Surfaces.** The 2012 shaders light a lamp from a *lit pattern*, a picture whose four channels are four lamp circuits:
  red the brake, green the head lamps, blue the reverse lamps, alpha the tail lamps
  ([chameleon-lamps.md](../formats/chameleon-lamps.md#lamp-circuits-verified-by-data)). The mod reads it per triangle
  (`lamp_pieces.rs` of `chameleon-vehicle`) and gives each lit piece the placeholder of its circuit and side: the
  `Refraction` lens strips are cut into connected pieces and again by circuit, and the triangles of the lamp interiors (the LED
  rows, the 918 Spyder's whole reverse bar) that a circuit lights are cut out of the interior mesh. A piece's side is its place
  (a front piece on the centre plane follows the left). The car gets eight flat textures (`lamp_surfaces.rs`): warm white
  and dark grey-blue for the head lamps, bright red and dark red for the brake lamps, deep red and dark red for the tail lamps,
  white and dark grey for the reverse lamps. The dots and strips of the 2012 pattern are not carried over, only which
  triangles each circuit lights. The rest of the lamp interiors and reflectors is drawn dark grey.
- The head and brake lamp **glass** (a clear head lamp cover, the red rear covers) is the car's blue-grey glass tint.

## Limits

- Any lamp that is not a head, brake or reverse lamp has no marker on the converted cars: the amber position and turn lamps,
  the fog lamps (bit 3) and the tail records of bits 0 and 7 when the car has brake records. Their surfaces are lit only when
  the lit pattern puts them on one of the four circuits; the number plate lamps (a grey lightmap on other meshes) and the
  plain `Emissive` interior lights are not.
- The flares are the game's shared profiles: the 2012 coronas' colours (amber position lamps, xenon blue), sizes and shapes
  are not used.
- The 2012 road lights (the spot a lamp throws on the road) are not used: the game's headlight beam applies to every car.
- A lit state is a flat colour over every triangle a circuit lights, however small the 2012 lit dots were inside it: an LED
  row is one lit bar. The colours are the same for every car; the 2012 colour constants (xenon blue, HDR intensities) are not used.
- The red channel is the brake and the alpha channel the tail by their brightness and the shader's colour names, not by the
  light-state code; a car whose artist swapped them would light its tail lamps with the brake.
- The head lamp flares of a car with several lamp groups on one side are the first two of its records, not a chosen pair.
- The flares do not follow the lit surface: they keep the game's profiles, so a lamp that is small on the car (the 918
  Spyder's 30 cm reverse bar) still has the game's flare size.
- The lit surfaces and the placeholders are checked on the real data (the 918 Spyder's reverse bar is cut out as a centre
  piece low at the rear, the 911 has brake and reverse lens pieces at the rear corners) and by the unit tests of the lamp
  code; the look in the world with the lamps lit was not yet checked with a screenshot.
