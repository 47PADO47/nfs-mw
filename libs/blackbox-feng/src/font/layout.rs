use super::Font;

/// How a string is placed.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    /// Justification bits: 1 centre, 2 right, 4 vertical centre, 8 bottom, 0x10 word wrap.
    pub justification: u32,
    pub leading: i32,
    /// 0 = unlimited.
    pub max_width: i32,
}

/// One glyph rectangle, in the string own space (pixels, y down, origin at the string position), and its
/// texture rectangle (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// The laid-out text.
#[derive(Clone, Debug, Default)]
pub struct TextLayout {
    pub quads: Vec<GlyphQuad>,
    /// Width of the widest line and total height, in pixels.
    pub width: f32,
    pub height: f32,
}

/// Baseline offset and leading scale for the fonts the game tweaks, by font name hash.
const EXTRA: [(u32, f32, f32); 4] =
    [(0xDCA5485A, 18.0, 2.0), (0x833A8678, 22.0, 2.0), (0xF88A75F9, 18.0, 1.0), (0x71C777D7, 23.0, 1.0)];

const HCENTER: u32 = 1;
const HRIGHT: u32 = 2;
const VCENTER: u32 = 4;
const VBOTTOM: u32 = 8;
const WRAP: u32 = 0x10;

fn is_newline(c: u16) -> bool {
    c == b'\n' as u16 || c == b'^' as u16
}

fn convert(c: u16) -> u16 {
    let c = if c >= 0xFF80 { c & 0xFF } else { c };
    match c {
        0x99 => 0x2122,
        0x9C => 0x153,
        0xA0 => 0x20,
        other => other,
    }
}

struct Layouter<'a> {
    font: &'a Font,
    text: Vec<u16>,
    style: TextStyle,
    height: f32,
    baseline: f32,
    leading: i32,
}

impl Layouter<'_> {
    fn width_of(&self, c: u16, prev: u16) -> f32 {
        if is_newline(c) || c == b'\r' as u16 {
            return 0.0;
        }
        let Some(g) = self.font.glyph(convert(c)) else { return 0.0 };
        let kern = if prev != 0 { self.font.kern(g, prev) } else { 0 };
        (kern + g.advance_x as i32) as f32
    }

    fn at(&self, i: usize) -> u16 {
        self.text.get(i).copied().unwrap_or(0)
    }

    /// Width of the line starting at `start`, stopping at a newline or, when wrapping, at the last space
    /// before the maximum width.
    fn line_width(&self, start: usize) -> f32 {
        let wrap = self.style.justification & WRAP != 0;
        let max = self.style.max_width as f32;
        let (mut width, mut last_space) = (0.0, 0.0);
        let mut k = 0;
        let mut i = start;
        while self.at(i) != 0 && i < self.text.len() {
            let c = self.at(i);
            if is_newline(c) {
                break;
            }
            if c == b' ' as u16 {
                last_space = width;
            }
            let prev = if k == 0 { 0 } else { self.at(i - 1) };
            width += self.width_of(c, prev);
            if self.style.max_width != 0 && max < width && wrap {
                if last_space > 0.0 {
                    width = last_space;
                }
                break;
            }
            k += 1;
            i += 1;
        }
        width
    }

    fn next_word_width(&self, from: usize) -> f32 {
        let mut size = 0.0;
        let mut i = from;
        loop {
            let prev = if i == 0 { 0 } else { self.at(i - 1) };
            size += self.width_of(self.at(i), prev);
            let next = self.at(i + 1);
            if next == b' ' as u16 || next == 0 || is_newline(next) || i + 1 >= self.text.len() {
                break;
            }
            i += 1;
        }
        size
    }

    fn text_height(&self) -> f32 {
        let wrap = self.style.justification & WRAP != 0 && self.style.max_width != 0;
        let (mut height, mut line_width, mut last_not_return) = (0.0, 0.0, true);
        let mut prev = 0;
        for (i, &c) in self.text.iter().enumerate() {
            if c == 0 {
                break;
            }
            let mut new_line = false;
            if is_newline(c) {
                new_line = true;
            } else if c != b'\r' as u16 {
                if self.font.glyph(c & 0xFF).is_some() {
                    last_not_return = true;
                }
                if wrap {
                    if c == b' ' as u16 && (self.style.max_width as f32) < line_width + self.next_word_width(i) {
                        new_line = true;
                    }
                    line_width += self.width_of(c, prev);
                }
            }
            if new_line {
                last_not_return = false;
                line_width = 0.0;
                height += self.leading as f32 + self.height;
            }
            prev = c;
        }
        if last_not_return { height + self.height } else { height }
    }

    fn x_offset(&self, line_width: f32) -> f32 {
        let j = self.style.justification;
        if j & HCENTER != 0 {
            -line_width * 0.5
        } else if j & HRIGHT != 0 {
            -line_width
        } else {
            0.0
        }
    }

    fn y_offset(&self, h: f32) -> f32 {
        let j = self.style.justification;
        if j & VCENTER != 0 {
            -h * 0.5
        } else if j & VBOTTOM != 0 {
            -h
        } else {
            0.0
        }
    }
}

impl Font {
    /// Lays out `text` the way the game does. `texture_size` is the size of the font texture in pixels.
    /// `$NAME$` button-icon sequences are skipped (no icon is drawn).
    pub fn layout(&self, text: &str, style: TextStyle, texture_size: (u32, u32)) -> TextLayout {
        let (baseline, leading_scale) = EXTRA.iter().find(|e| e.0 == self.hash).map_or((0.0, 1.0), |e| (e.1, e.2));
        let mut units: Vec<u16> = Vec::new();
        let mut in_icon = false;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let mut i = 0;
        while i < utf16.len() {
            let c = utf16[i];
            if c == b'$' as u16 {
                if utf16.get(i + 1) == Some(&(b'$' as u16)) {
                    units.push(c);
                    i += 2;
                    continue;
                }
                in_icon = !in_icon;
                i += 1;
                continue;
            }
            if !in_icon {
                units.push(c);
            }
            i += 1;
        }
        let l = Layouter {
            font: self,
            text: units,
            style,
            height: self.height(),
            baseline,
            leading: (style.leading as f32 * leading_scale) as i32,
        };
        let wrap = style.justification & WRAP != 0;
        let total_height = l.text_height();
        let mut y = l.y_offset(total_height);
        let mut x = l.x_offset(l.line_width(0));
        let mut line_start = x;
        let mut out = TextLayout { quads: Vec::new(), width: 0.0, height: total_height };
        let (tw, th) = (texture_size.0.max(1) as f32, texture_size.1.max(1) as f32);
        let mut i = 0;
        while i < l.text.len() && l.text[i] != 0 {
            let c = l.text[i];
            let skip_leading_space = c == b' ' as u16 && x == line_start && wrap;
            if !skip_leading_space {
                if is_newline(c) {
                    if i + 1 >= l.text.len() || l.text[i + 1] == 0 {
                        break;
                    }
                    x = l.x_offset(l.line_width(i + 1));
                    y += l.height + l.leading as f32;
                    line_start = x;
                } else {
                    if style.max_width != 0 && c == b' ' as u16 && wrap {
                        let word = l.next_word_width(i);
                        if (x - line_start) + word > style.max_width as f32 {
                            x = l.x_offset(l.line_width(i + 1));
                            y += l.height + l.leading as f32;
                            line_start = x;
                        }
                    }
                    let unicode = convert(c);
                    if let Some(g) = self.glyph(unicode) {
                        let prev = if i == 0 { 0 } else { l.text[i - 1] };
                        let kern = if prev != 0 { self.kern(g, prev) as f32 } else { 0.0 };
                        let w = (g.width as f32).max(4.0);
                        let x0 = x + kern + g.offset_x as f32;
                        let y0 = y + g.offset_y as f32 + l.baseline;
                        out.quads.push(GlyphQuad {
                            x0,
                            y0,
                            x1: x0 + w,
                            y1: y0 + g.height as f32,
                            u0: g.u as f32 / tw,
                            v0: g.v as f32 / th,
                            u1: (g.u as f32 + g.width as f32 + 1.0) / tw,
                            v1: (g.v as f32 + g.height as f32) / th,
                        });
                        x += l.width_of(unicode, prev);
                        out.width = out.width.max(x - line_start);
                    }
                }
            }
            i += 1;
        }
        out
    }
}
