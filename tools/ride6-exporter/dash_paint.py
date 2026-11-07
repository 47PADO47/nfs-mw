"""Draws a parsed cluster widget (`dash_widgets.Widget`) once, at the state of a bike at rest, into a picture.

The layout follows Unreal's canvas panel (anchors, offsets, alignment, auto size), horizontal boxes, render transforms and the
three UI materials of the gauges; text is set in a system font (the game's fonts are not used) or, for the `Digital-7` font of the
LCD clusters, in seven segments. Blending is made in encoded (sRGB) colour with 8 bit layers.
"""

import math
import os

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from dash_widgets import material_params

FONT_DIR = os.path.join(os.environ.get("WINDIR", r"C:\Windows"), "Fonts")
SLATE_PX = 96.0 / 72.0  # a Slate font size is in points at 96 dpi
SEGMENTS = {
    "0": "abcdef", "1": "bc", "2": "abdeg", "3": "abcdg", "4": "bcfg", "5": "acdfg", "6": "acdefg", "7": "abc", "8": "abcdefg",
    "9": "abcdfg", "-": "g", "N": "ceg", "n": "ceg", "A": "abcefg", "E": "adefg", "F": "aefg", "H": "bcefg", "L": "def", "P": "abefg",
    "S": "acdfg", "C": "adef", "O": "abcdef", "U": "bcdef", "R": "eg", "T": "defg", "B": "cdefg", "D": "bcdeg", "I": "ef", "K": "bcefg",
    "M": "ace", "G": "acdef", "J": "bcd", "Y": "bcdfg", "°": "abfg", "_": "d",
}


class Layer:
    """An RGBA picture the widgets are drawn into."""

    def __init__(self, size):
        self.size = size
        self.image = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    def over(self, other, opacity=1.0):
        source = other.image
        if opacity < 1.0:
            source = source.copy()
            alpha = source.getchannel("A").point(lambda v: int(v * opacity))
            source.putalpha(alpha)
        self.image = Image.alpha_composite(self.image, source)


def srgb_byte(linear):
    v = max(0.0, min(1.0, linear))
    v = v * 12.92 if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055
    return int(v * 255 + 0.5)


def text_colour(props):
    """The text colour (r, g, b, a bytes): the hex of the specified colour, else its linear floats encoded."""
    colour = props.get("ColorAndOpacity", {}).get("SpecifiedColor")
    if not colour:
        return (255, 255, 255, 255)
    if "Hex" in colour and len(colour["Hex"]) >= 6:
        h = colour["Hex"]
        return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), int(colour.get("A", 1.0) * 255))
    return (srgb_byte(colour["R"]), srgb_byte(colour["G"]), srgb_byte(colour["B"]), int(colour.get("A", 1.0) * 255))


def font_for(props, px):
    """A system font for the text block, bold when the game's font name says so."""
    name = ((props.get("Font", {}).get("FontObject") or {}).get("ObjectName") or "").lower()
    bold = any(t in name for t in ("bold", "black", "semi", "heavy"))
    condensed = "condensed" in name or "bebas" in name or "ktm" in name
    for file in (("bahnschrift.ttf" if condensed else None), "arialbd.ttf" if bold else "arial.ttf", "arial.ttf"):
        if not file:
            continue
        try:
            return ImageFont.truetype(os.path.join(FONT_DIR, file), max(int(px), 4))
        except OSError:
            continue
    return ImageFont.load_default()


def is_segment_font(props):
    name = ((props.get("Font", {}).get("FontObject") or {}).get("ObjectName") or "").lower()
    return "digital" in name


def draw_segments(draw, box, text, px, colour, justification):
    """`text` in seven segment digits, `px` the em: the digits are 0.72 em high, 0.5 em wide."""
    height, width, gap, thick = px * 0.72, px * 0.5, px * 0.12, max(px * 0.1, 1.5)
    advance = width + gap
    count = sum(1 for _ in text)
    total = count * advance - gap
    x0, y0, bw, bh = box
    start = x0
    if justification.endswith("Center"):
        start = x0 + (bw - total) / 2
    if justification.endswith("Right"):
        start = x0 + bw - total
    top = y0 + (bh - height) / 2 if bh > height else y0
    for index, ch in enumerate(text):
        left = start + index * advance
        if ch in ".:":
            dot = thick * 0.9
            draw.rectangle([left + width / 2 - dot / 2, top + height - dot, left + width / 2 + dot / 2, top + height], fill=colour)
            if ch == ":":
                draw.rectangle([left + width / 2 - dot / 2, top + height * 0.3, left + width / 2 + dot / 2, top + height * 0.3 + dot], fill=colour)
            continue
        lit = SEGMENTS.get(ch.upper() if ch.upper() in SEGMENTS else ch, "")
        mid = top + height / 2
        rects = {
            "a": (left + thick, top, left + width - thick, top + thick),
            "d": (left + thick, top + height - thick, left + width - thick, top + height),
            "g": (left + thick, mid - thick / 2, left + width - thick, mid + thick / 2),
            "f": (left, top + thick, left + thick, mid - thick / 2),
            "b": (left + width - thick, top + thick, left + width, mid - thick / 2),
            "e": (left, mid + thick / 2, left + thick, top + height - thick),
            "c": (left + width - thick, mid + thick / 2, left + width, top + height - thick),
        }
        for segment in lit:
            draw.rectangle(rects[segment], fill=colour)


def measure_text(props, text):
    """The size (w, h) of a text block in canvas pixels, for auto-sized slots."""
    px = props.get("Font", {}).get("Size", 24.0) * SLATE_PX
    if is_segment_font(props):
        return (len(text) * px * 0.62, px * 0.9)
    font = font_for(props, px)
    box = font.getbbox(text or " ")
    ascent, descent = font.getmetrics() if hasattr(font, "getmetrics") else (px, 0)
    return (box[2], ascent + descent)


class Painter:
    """Draws widget trees; `textures` maps a texture package to its PNG, `state` has the idle values."""

    def __init__(self, assets, textures, config, state, canvas=1024):
        self.assets = assets
        self.textures = textures
        self.config = config
        self.state = state
        self.canvas = canvas
        self.cache = {}

    # --- sources -------------------------------------------------------------------------------------------------
    def texture(self, package):
        if package not in self.cache:
            path = self.textures.get(package)
            self.cache[package] = Image.open(path).convert("RGBA") if path and os.path.exists(path) else None
        return self.cache[package]

    def brush(self, node):
        """(picture, tint) of an image widget: the texture a brush or its material shows, times the colour the material applies."""
        resource = node.props.get("Brush", {}).get("ResourceObject")
        if not resource:
            return None, (1.0, 1.0, 1.0)
        path = resource["ObjectPath"].rsplit(".", 1)[0]
        if resource["ObjectName"].startswith("Texture2D"):
            return self.texture(path), (1.0, 1.0, 1.0)
        params = material_params(self.assets, path)
        if params is None:
            return None, (1.0, 1.0, 1.0)
        name = params["parent"].lower()
        if "colormultiplier" in name:
            colour = params["vector"].get("DayColor", (1.0, 1.0, 1.0, 1.0))
            intensity = params["scalar"].get("DayColorIntensity", 1.0)
            return self.texture(params["tex"].get("DayTexture")), tuple(c * intensity for c in colour[:3])
        if "progressbars" in name:
            return self.progress_bar(params), (1.0, 1.0, 1.0)
        if "ledsemissive" in name:
            colour = params["vector"].get("EmissiveTextureColor", (1.0, 1.0, 1.0, 1.0))
            intensity = params["scalar"].get("EmissiveIntensity", 1.0)
            return self.texture(params["tex"].get("EmissiveColor Texture")), tuple(c * intensity for c in colour[:3])
        return None, (1.0, 1.0, 1.0)

    def progress_bar(self, params):
        """The bar picture masked to the idle fill: the base texture where the gradient picture is below the fill."""
        base = self.texture(params["tex"].get("BaseTexture"))
        gradient_path = self.textures.get(params["tex"].get("GradientTexture"))
        fill = self.state.get("rpm_fill", 0.0)
        if base is None or not gradient_path or not os.path.exists(gradient_path):
            return None
        gradient = Image.open(gradient_path).convert("L").resize(base.size)
        mask = gradient.point(lambda v: 255 if v / 255.0 <= fill else 0)
        out = base.copy()
        alpha = Image.fromarray(np.minimum(np.array(out.getchannel("A")), np.array(mask)))
        out.putalpha(alpha)
        return out

    # --- layout ---------------------------------------------------------------------------------------------------
    def desired(self, node):
        if node.kind == "Image":
            size = node.props.get("Brush", {}).get("ImageSize")
            if size:
                return (size["X"], size["Y"])
            picture, _ = self.brush(node)
            return picture.size if picture else (32, 32)
        if node.kind == "TextBlock":
            return measure_text(node.props, self.state_text(node))
        return (64, 64)

    def canvas_rect(self, node, parent):
        """The rectangle (x, y, w, h) of a widget in a canvas panel slot, parent = (x, y, w, h)."""
        layout = node.slot.get("LayoutData", {})
        offsets = layout.get("Offsets", {})
        anchors = layout.get("Anchors", {})
        low, high = anchors.get("Minimum", {}), anchors.get("Maximum", {})
        align = layout.get("Alignment", {})
        auto = node.slot.get("bAutoSize", False)
        want = self.desired(node) if auto else (0, 0)
        out = []
        for axis, (start, size) in enumerate(((parent[0], parent[2]), (parent[1], parent[3]))):
            name = "XY"[axis]
            near, far = ("Left", "Right") if axis == 0 else ("Top", "Bottom")
            lo, hi = low.get(name, 0.0), high.get(name, 0.0)
            first, second = offsets.get(near, 0.0), offsets.get(far, 0.0)
            if lo != hi:
                begin = start + size * lo + first
                out.extend([begin, start + size * hi - second - begin])
                continue
            length = want[axis] if auto else second
            out.extend([start + size * lo + first - align.get(name, 0.0) * length, length])
        return (out[0], out[2], out[1], out[3])

    def box_rects(self, node, rect):
        """The rectangles of the children of a horizontal box: fixed ones at their desired size, fill ones share the rest."""
        x, y, w, h = rect
        items = []
        for child in node.children:
            size = child.slot.get("Size", {})
            fill = size.get("SizeRule") == "ESlateSizeRule::Fill"
            pad = child.slot.get("Padding", {})
            items.append((child, fill, size.get("Value", 1.0), pad))
        fixed = sum(self.desired(c)[0] + p.get("Left", 0) + p.get("Right", 0) for c, f, _, p in items if not f)
        weights = sum(v for _, f, v, _ in items if f) or 1.0
        free = max(w - fixed, 0.0)
        cursor = x
        out = []
        for child, fill, value, pad in items:
            width = free * value / weights if fill else self.desired(child)[0]
            width -= 0 if not fill else pad.get("Left", 0) + pad.get("Right", 0)
            height = self.desired(child)[1] if not fill else h
            valign = child.slot.get("VerticalAlignment", "")
            top = y + (h - height) / 2 if "Center" in valign else (y + h - height if "Bottom" in valign else y)
            out.append((child, (cursor + pad.get("Left", 0), top, width, height)))
            cursor += width + pad.get("Left", 0) + pad.get("Right", 0)
        return out

    # --- drawing --------------------------------------------------------------------------------------------------
    def state_text(self, node):
        override = self.state.get("text", lambda n: None)(node)
        if override is not None:
            return override
        text = node.props.get("Text", {})
        return text.get("SourceString") or text.get("LocalizedString") or ""

    def render(self, node, rect, layer):
        if node is None or not node.visible() or self.state["hide"](node):
            return
        transform = node.props.get("RenderTransform")
        angle = self.state.get("angle", lambda n: None)(node)
        opacity = node.props.get("RenderOpacity", 1.0)
        if transform or angle is not None or opacity < 1.0:
            temp = Layer(layer.size)
            self.render_plain(node, rect, temp)
            moved = self.transformed(temp, node, rect, transform or {}, angle)
            layer.over(moved, opacity)
            return
        self.render_plain(node, rect, layer)

    def transformed(self, layer, node, rect, transform, angle_override):
        x, y, w, h = rect
        pivot = node.props.get("RenderTransformPivot", {})
        cx, cy = x + pivot.get("X", 0.5) * w, y + pivot.get("Y", 0.5) * h
        translate = transform.get("Translation", {})
        tx, ty = translate.get("X", 0.0), translate.get("Y", 0.0)
        scale = transform.get("Scale", {})
        sx, sy = scale.get("X", 1.0), scale.get("Y", 1.0)
        angle = math.radians(angle_override if angle_override is not None else transform.get("Angle", 0.0))
        c, s = math.cos(angle), math.sin(angle)
        # out = pivot + t + R S (in - pivot)  =>  in = pivot + S^-1 R^-1 (out - pivot - t)
        a, b = c / sx, s / sx
        d, e = -s / sy, c / sy
        ox, oy = cx + tx, cy + ty
        matrix = (a, b, cx - a * ox - b * oy, d, e, cy - d * ox - e * oy)
        out = Layer(layer.size)
        out.image = layer.image.transform(layer.image.size, Image.AFFINE, matrix, resample=Image.BICUBIC)
        return out

    def render_plain(self, node, rect, layer):
        if node.kind == "CanvasPanel":
            order = sorted(node.children, key=lambda c: c.slot.get("ZOrder", 0))
            for child in order:
                self.render(child, self.canvas_rect(child, rect), layer)
        if node.kind == "HorizontalBox":
            for child, child_rect in self.box_rects(node, rect):
                self.render(child, child_rect, layer)
        if node.kind == "Image":
            self.draw_image(node, rect, layer)
        if node.kind == "TextBlock":
            self.draw_text(node, rect, layer)

    def draw_image(self, node, rect, layer):
        picture, tint = self.brush(node)
        if picture is None:
            return
        x, y, w, h = rect
        if w < 1 or h < 1:
            return
        widget_tint = (node.props.get("ColorAndOpacity") or {}).get("SpecifiedColor") or {}
        extra = (widget_tint.get("R", 1.0), widget_tint.get("G", 1.0), widget_tint.get("B", 1.0))
        picture = picture.resize((max(int(round(w)), 1), max(int(round(h)), 1)), Image.LANCZOS)
        data = np.array(picture).astype(np.float32) / 255.0
        rgb = data[..., :3] ** 2.2  # to linear light: the multipliers are made there
        rgb = rgb * np.array([tint[i] * extra[i] for i in range(3)], dtype=np.float32)
        peak = rgb.max(axis=2, keepdims=True)
        rgb = np.where(peak > 1.0, rgb / np.maximum(peak, 1e-6), rgb)  # hue preserving: an over bright LED keeps its colour
        data[..., :3] = np.clip(rgb, 0, 1) ** (1 / 2.2)
        tile = Image.fromarray((data * 255 + 0.5).astype(np.uint8))
        part = Layer(layer.size)
        part.image.paste(tile, (int(round(x)), int(round(y))))
        layer.over(part)

    def draw_text(self, node, rect, layer):
        text = self.state_text(node)
        if not text:
            return
        x, y, w, h = rect
        props = node.props
        px = props.get("Font", {}).get("Size", 24.0) * SLATE_PX
        colour = text_colour(props)
        justification = props.get("Justification", "ETextJustify::Left")
        part = Layer(layer.size)
        draw = ImageDraw.Draw(part.image)
        if is_segment_font(props):
            draw_segments(draw, (x, y, w, h), text, px, colour, justification)
        else:
            font = font_for(props, px)
            width = draw.textlength(text, font=font)
            left = x
            if justification.endswith("Center"):
                left = x + (w - width) / 2
            if justification.endswith("Right"):
                left = x + w - width
            draw.text((left, y), text, font=font, fill=colour, anchor="la")
        layer.over(part)
