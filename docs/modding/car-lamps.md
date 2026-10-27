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
`HEADLIGHT_LEFT`, `HEADLIGHT_RIGHT`, `BRAKELIGHT_LEFT`, `BRAKELIGHT_RIGHT` or `BRAKELIGHT_CENTRE` is drawn with the car's
own texture for the lamp's state, found by name in the car's `TEXTURES.BIN`:

| Texture | Is drawn when |
|---|---|
| `<ID>_KIT00_HEADLIGHT_ON`, `<ID>_KIT00_HEADLIGHT_OFF` | the head lamps are on, off (a car starts with them on) |
| `<ID>_KIT00_BRAKELIGHT_ON`, `<ID>_KIT00_BRAKELIGHT_OFF` | the brake is held, not held (starts off) |

A state whose texture the car does not have keeps the other one. A broken brake lamp stays dark while the others light up.
`<ID>` is the car's folder in upper case. The two states of the head lamps and of the brake lamps are shared by all their
placeholders, so the left and right lamps of one kind look the same. Every placeholder group should use a light material
that shows a flat surface (the 2012 mod uses `MATTEPLASTIC`).

## The 2012 cars

The [2012 mod](nfsmw-2012-content.md) fills both from each car's own 2012 data
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
- **Surfaces.** The 2012 lamp strips (`Refraction` shader) are split into connected pieces, each given the placeholder of its
  end and side (front pieces `HEADLIGHT_LEFT` or `_RIGHT`, a front piece on the centre plane follows the left; rear pieces
  `BRAKELIGHT_LEFT`, `_RIGHT` or `_CENTRE`), and the car gets four flat textures: a warm white and a dark grey-blue for
  the head lamps, a red and a dark red for the brake lamps. The pattern of the 2012 strips (an emissive map) is not carried
  over. The lamp interiors and reflectors (`Emissive` shaders) are drawn dark grey instead of the pale default they had.
- The head and brake lamp **glass** (a clear head lamp cover, the red rear covers) is the car's blue-grey glass tint.

## Limits

- Any lamp that is not a head, brake or reverse lamp has no marker on the converted cars: the amber position and turn lamps,
  the fog lamps (bit 3) and the tail records of bits 0 and 7 when the car has brake records. The reverse lamp has a flare but
  no lit surface: the game has no reverse placeholder texture, so the white strips of a 2012 lens or lightmap stay as they are.
- The flares are the game's shared profiles: the 2012 coronas' colours (amber position lamps, xenon blue), sizes and shapes
  are not used.
- The 2012 road lights (the spot a lamp throws on the road) are not used: the game's headlight beam applies to every car.
- A lit state is a flat colour. A car whose lamps have a pattern (LED strips) shows the same colour over the whole piece.
- The head lamp flares of a car with several lamp groups on one side are the first two of its records, not a chosen pair.
- A converted car was looked at in the garage view (headlights on, brake off) and in the world while reversing (the 918
  Spyder's white reverse flare); its markers and textures are also checked in the files and by the unit tests of the lamp code.
