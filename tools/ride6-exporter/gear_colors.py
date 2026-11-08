"""The average colour of a pair of gloves, from its picture and the mask of where the picture has gloves.

The pictures are the `R6TX` files of the `gtex` mode (docs/formats/ride6-rider-gear.md), block-compressed; a small level of each is
decoded here (BC1 and BC3 colour blocks) and averaged over the mask, in the pictures' own (sRGB) values.
"""

import struct

import r6site  # noqa: F401
import numpy as np

# The longest side of the pictures this reads, pixels.
PROBE = 128


def read(path):
    """`(format code, width, height, bytes)` of the first level of an `R6TX` file."""
    with open(path, "rb") as f:
        data = f.read()
    code, _srgb, count = struct.unpack_from("<IBI", data, 8)
    width, height, length = struct.unpack_from("<III", data, 17)
    return code, width, height, data[29 : 29 + length], count


def decode(code, width, height, blocks):
    """The RGB pixels (height, width, 3) of BC1 or BC3 data."""
    size = 8 if code == 1 else 16
    raw = np.frombuffer(blocks, dtype=np.uint8).reshape(-1, size)[:, size - 8 :]
    c0 = raw[:, 0].astype(np.uint16) | (raw[:, 1].astype(np.uint16) << 8)
    c1 = raw[:, 2].astype(np.uint16) | (raw[:, 3].astype(np.uint16) << 8)

    def expand(c):
        r, g, b = (c >> 11) & 31, (c >> 5) & 63, c & 31
        return np.stack([(r * 255 + 15) // 31, (g * 255 + 31) // 63, (b * 255 + 15) // 31], axis=1).astype(np.float32)

    e0, e1 = expand(c0), expand(c1)
    four = (c0 > c1)[:, None]
    palette = np.stack(
        [
            e0,
            e1,
            np.where(four, (2 * e0 + e1) / 3, (e0 + e1) / 2),
            np.where(four, (e0 + 2 * e1) / 3, np.zeros_like(e0)),
        ],
        axis=1,
    )
    bits = raw[:, 4].astype(np.uint32) | (raw[:, 5].astype(np.uint32) << 8) | (raw[:, 6].astype(np.uint32) << 16) | (raw[:, 7].astype(np.uint32) << 24)
    shifts = np.arange(16, dtype=np.uint32) * 2
    index = (bits[:, None] >> shifts[None, :]) & 3
    pixels = palette[np.arange(len(raw))[:, None], index]
    blocks_wide, blocks_high = width // 4, height // 4
    image = pixels.reshape(blocks_high, blocks_wide, 4, 4, 3).transpose(0, 2, 1, 3, 4)
    return image.reshape(height, width, 3)


def average(picture_path, mask_path):
    """`[r, g, b]` (0 to 255): the mean of the picture where the mask is above half; the whole picture when the mask is empty."""
    p_code, width, height, p_data, _ = read(picture_path)
    m_code, m_width, m_height, m_data, _ = read(mask_path)
    if p_code not in (1, 3) or m_code not in (1, 3) or (width, height) != (m_width, m_height):
        return [64, 64, 64]
    picture = decode(p_code, width, height, p_data)
    mask = decode(m_code, m_width, m_height, m_data).mean(axis=2) > 128
    pixels = picture[mask] if mask.any() else picture.reshape(-1, 3)
    return [int(round(v)) for v in pixels.mean(axis=0)]
