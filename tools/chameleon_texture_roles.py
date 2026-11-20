#!/usr/bin/env python3
"""List the textures of a Chameleon bundle with what each is used as (diffuse, normal, specular, road colour map...).

A texture has no name in the 2012 files, so what it is comes from the materials: a material imports its textures in the order of
the sampler table of its shader (docs/formats/chameleon-gfx.md), and the sampler name (`DiffuseTextureSampler`,
`Normal_Sampler`, `ColourMap_Sampler`...) says the role. Facts: docs/formats/chameleon-road-textures.md.
Needs numpy and Pillow for `sheet` (contact sheets). Nothing read is stored in the repository.

    python tools/chameleon_texture_roles.py roles <game dir> <bundle under it>          # one line per texture
    python tools/chameleon_texture_roles.py sheet <game dir> <bundle> <out.png> [i,j,k]  # a picture of some textures
"""

import io
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle  # noqa: E402
from chameleon_export import Shaders  # noqa: E402

T_TEXTURE, T_MATERIAL = 0x1, 0x2
FORMATS = {71: "BC1", 74: "BC2", 77: "BC3", 80: "BC4", 83: "BC5", 98: "BC7", 28: "RGBA8", 61: "R8", 10: "RGBA16F"}
BLOCK = {71: 8, 74: 16, 77: 16}


def u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0]


def header(bundle, entry):
    """Format, width, height and mip count of a texture resource."""
    main = bundle.block(entry, 0)
    width, height = struct.unpack_from("<HH", main, 0x24)
    return u32(main, 0x1C), width, height, main[0x2D]


def roles(game, bundle):
    """Texture index -> list of (shader technique, sampler name), from the materials of the same bundle."""
    shaders = Shaders(game)
    textures = bundle.by_type(T_TEXTURE)
    index = {entry.id: i for i, entry in enumerate(textures)}
    found = {}
    for material in bundle.by_type(T_MATERIAL):
        main = bundle.block(material, 0)
        slots = {offset: rid for rid, offset, _ in bundle.imports(material, main)}
        technique, samplers, _ = shaders.table.get(slots.get(0x08), ("?", [], None))
        count, base = u32(main, 0x20), u32(main, 0x2C)
        for i in range(count):
            texture = index.get(slots.get(base + 4 * i))
            if texture is None:
                continue
            sampler = samplers[i] if i < len(samplers) else "?%d" % i
            found.setdefault(texture, []).append((technique, sampler))
    return textures, found


def decode(bundle, entry, longest):
    """A PIL image of a BC1/BC2/BC3 texture, from the smallest mip that is still `longest` pixels on its longer side."""
    from PIL import Image

    fmt, width, height, mips = header(bundle, entry)
    block = BLOCK.get(fmt)
    if block is None:
        return None
    pixels = bundle.block(entry, 1)
    offset, level = 0, 0
    while max(width, height) > longest and level + 1 < mips:
        offset += max(1, (width + 3) // 4) * max(1, (height + 3) // 4) * block
        width, height, level = max(1, width // 2), max(1, height // 2), level + 1
    size = max(1, (width + 3) // 4) * max(1, (height + 3) // 4) * block
    dds = bytearray(128)
    dds[0:4] = b"DDS "
    struct.pack_into("<7I", dds, 4, 124, 0x1007 | 0x80000, height, width, size, 0, 1)
    struct.pack_into("<2I", dds, 76, 32, 4)
    dds[84:88] = {71: b"DXT1", 74: b"DXT3", 77: b"DXT5"}[fmt]
    struct.pack_into("<I", dds, 108, 0x1000)
    image = Image.open(io.BytesIO(bytes(dds) + pixels[offset:offset + size]))
    image.load()
    return image.convert("RGBA")


def sheet(bundle, chosen, out, cell=256, columns=6):
    from PIL import Image, ImageDraw

    textures = bundle.by_type(T_TEXTURE)
    chosen = chosen or list(range(len(textures)))
    rows = (len(chosen) + columns - 1) // columns
    picture = Image.new("RGB", (columns * (cell + 4), rows * (cell + 4)), (90, 0, 90))
    draw = ImageDraw.Draw(picture)
    for k, i in enumerate(chosen):
        fmt, width, height, _ = header(bundle, textures[i])
        x, y = (k % columns) * (cell + 4), (k // columns) * (cell + 4)
        draw.rectangle([x, y, x + cell, y + cell], fill=(30, 30, 30))
        image = decode(bundle, textures[i], cell)
        if image is not None:
            image.thumbnail((cell, cell))
            picture.paste(image.convert("RGB"), (x, y))
        draw.text((x + 2, y), "%d %dx%d" % (i, width, height), fill=(255, 255, 0))
    picture.save(out)


def main(argv):
    if len(argv) < 4 or argv[1] not in ("roles", "sheet"):
        print(__doc__)
        return 1
    game, path = argv[2], os.path.join(argv[2], argv[3])
    bundle = Bundle(path)
    if argv[1] == "roles":
        textures, found = roles(game, bundle)
        for i, entry in enumerate(textures):
            fmt, width, height, mips = header(bundle, entry)
            names = sorted({sampler for _, sampler in found.get(i, [])})
            print("%4d %016X %5dx%-5d %-4s mips %-2d %s" % (i, entry.id, width, height, FORMATS.get(fmt, fmt), mips, ", ".join(names)))
        return 0
    if len(argv) < 5:
        print(__doc__)
        return 1
    chosen = [int(part) for part in argv[5].split(",")] if len(argv) > 5 else None
    sheet(bundle, chosen, argv[4])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
