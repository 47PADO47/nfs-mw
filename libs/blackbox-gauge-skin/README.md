# blackbox-gauge-skin

Gauge skins for a car HUD: the ini text that places and colours a speedometer, tachometer, nitrous gauge and similar
elements, the geometry that turns a value into a needle angle, a digit cell or a bar, the sector masks that light part
of an arc, and a DDS decoder for the pictures. Engine-generic: pure text and bytes in, numbers and pixels out. It opens no
file, draws nothing and knows no game.

- `Skin::parse` reads a skin ini; `General::parse` reads the settings file that chooses it.
- `Placement` maps reference pixels (a 1080-line screen, measured from the right and bottom edges) to canvas units.
- `needle` has the angle and pivot rules, `digits` the cell and slot layout, `mask` the sector masks.
- `dds::decode` reads the DDS formats skins use (DXT1, DXT3, DXT5 and 32-bit uncompressed) to RGBA.

The file format is [docs/formats/custom-hud-skin.md](../../docs/formats/custom-hud-skin.md), the behaviour
[docs/specs/custom-hud-gauges.md](../../docs/specs/custom-hud-gauges.md).

License: MIT OR Apache-2.0.
