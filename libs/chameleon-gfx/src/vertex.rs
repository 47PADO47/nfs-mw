use crate::bytes::Block;
use crate::error::{Error, Result};

/// Positions are stored as four signed-normalised 16-bit values; this scale takes them to metres.
/// Verified on every car part of one bundle: a renderable's bounding sphere matches the decoded extent at 10.
pub const POSITION_SCALE: f32 = 10.0;

/// What an element holds (the low byte of its semantic).
pub mod usage {
    pub const POSITION: u8 = 1;
    pub const COLOR: u8 = 4;
    pub const TEXCOORD: u8 = 5;
}

/// One element of a vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    /// The usage in the low byte and an index in the rest (0x106 is usage 6, index 1).
    pub semantic: u32,
    /// A DXGI format number.
    pub format: u32,
    /// Byte offset in the vertex.
    pub offset: u32,
    pub slot: u32,
    pub stride: u32,
}

impl Element {
    pub fn usage(&self) -> u8 {
        (self.semantic & 0xFF) as u8
    }

    pub fn index(&self) -> u32 {
        self.semantic >> 8
    }
}

/// The layout of a vertex buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexDescriptor {
    pub elements: Vec<Element>,
}

/// The attributes a car part needs from a vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Metres, in the engine's space (x left, y up, z forward).
    pub position: [f32; 3],
    /// RGBA, 255 when the layout has no colour.
    pub color: [u8; 4],
    /// The first texture coordinate pair.
    pub uv: [f32; 2],
}

impl VertexDescriptor {
    pub fn parse(main: &[u8]) -> Result<Self> {
        let block = Block::new(main, "vertex descriptor");
        let count = block.u32(0xC)? as usize;
        if count == 0 || count > 32 {
            return Err(Error::Invalid { what: "vertex descriptor", detail: format!("{count} elements") });
        }
        let mut elements = Vec::with_capacity(count);
        for n in 0..count {
            let at = 0x10 + 0x14 * n;
            elements.push(Element {
                semantic: block.u32(at)?,
                format: block.u32(at + 4)?,
                offset: block.u32(at + 8)?,
                slot: block.u32(at + 12)?,
                stride: block.u32(at + 16)?,
            });
        }
        Ok(Self { elements })
    }

    /// Bytes per vertex.
    pub fn stride(&self) -> usize {
        self.elements.first().map_or(0, |element| element.stride as usize)
    }

    fn find(&self, usage: u8, index: u32) -> Option<&Element> {
        self.elements.iter().find(|element| element.usage() == usage && element.index() == index)
    }

    /// Decode `data` (a whole vertex buffer) into vertices. The buffer's size is rounded up to the loader's alignment,
    /// so bytes after the last whole vertex are padding. A layout without a position is an error.
    pub fn decode(&self, data: &[u8]) -> Result<Vec<Vertex>> {
        let stride = self.stride();
        let position = self.find(usage::POSITION, 0);
        let Some(position) = position else {
            return Err(Error::Invalid { what: "vertex buffer", detail: "the layout has no position".into() });
        };
        if stride == 0 || data.len() < stride {
            return Err(Error::Invalid {
                what: "vertex buffer",
                detail: format!("{} bytes holds no {stride}-byte vertex", data.len()),
            });
        }
        let (color, uv) = (self.find(usage::COLOR, 0), self.find(usage::TEXCOORD, 0));
        let block = Block::new(data, "vertex buffer");
        let mut out = Vec::with_capacity(data.len() / stride);
        for n in 0..data.len() / stride {
            let base = n * stride;
            out.push(Vertex {
                position: read_position(&block, base, position)?,
                color: match color {
                    Some(element) => read_color(&block, base, element)?,
                    None => [255; 4],
                },
                uv: match uv {
                    Some(element) => read_uv(&block, base, element)?,
                    None => [0.0; 2],
                },
            });
        }
        Ok(out)
    }
}

fn snorm16(value: u16) -> f32 {
    (f32::from(value as i16) / 32767.0).max(-1.0)
}

fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let (exponent, fraction) = (i32::from((bits >> 10) & 0x1F), f32::from(bits & 0x3FF));
    if exponent == 0 {
        return sign * fraction * 2f32.powi(-24);
    }
    if exponent == 31 {
        return if fraction == 0.0 { sign * f32::INFINITY } else { f32::NAN };
    }
    sign * (1.0 + fraction / 1024.0) * 2f32.powi(exponent - 15)
}

fn read_position(block: &Block, base: usize, element: &Element) -> Result<[f32; 3]> {
    let at = base + element.offset as usize;
    match element.format {
        13 => Ok([0, 2, 4].map(|n| snorm16(block.u16(at + n).unwrap_or(0)) * POSITION_SCALE)),
        6 => block.f32s::<3>(at),
        2 => {
            let [x, y, z, _] = block.f32s::<4>(at)?;
            Ok([x, y, z])
        }
        other => Err(Error::Invalid { what: "vertex buffer", detail: format!("position format {other}") }),
    }
}

fn read_color(block: &Block, base: usize, element: &Element) -> Result<[u8; 4]> {
    let bytes = block.slice(base + element.offset as usize, 4)?;
    match element.format {
        28 => Ok([bytes[0], bytes[1], bytes[2], bytes[3]]),
        87 => Ok([bytes[2], bytes[1], bytes[0], bytes[3]]),
        _ => Ok([255; 4]),
    }
}

fn read_uv(block: &Block, base: usize, element: &Element) -> Result<[f32; 2]> {
    let at = base + element.offset as usize;
    match element.format {
        34 => Ok([half(block.u16(at)?), half(block.u16(at + 2)?)]),
        16 => block.f32s::<2>(at),
        37 => Ok([snorm16(block.u16(at)?), snorm16(block.u16(at + 2)?)]),
        _ => Ok([0.0; 2]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_decode() {
        assert_eq!(half(0x3C00), 1.0);
        assert_eq!(half(0xC000), -2.0);
        assert_eq!(half(0x3800), 0.5);
        assert_eq!(half(0), 0.0);
    }

    #[test]
    fn positions_scale_from_snorm() {
        let descriptor =
            VertexDescriptor { elements: vec![Element { semantic: 1, format: 13, offset: 0, slot: 0, stride: 8 }] };
        let mut data = Vec::new();
        for value in [0x7FFFu16, 0x8001, 0, 0x7FFF] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        let vertices = descriptor.decode(&data).unwrap();
        assert_eq!(vertices.len(), 1);
        assert_eq!(vertices[0].position, [10.0, -10.0, 0.0]);
        assert_eq!(vertices[0].color, [255; 4]);
    }

    #[test]
    fn a_layout_without_a_position_is_refused() {
        let descriptor =
            VertexDescriptor { elements: vec![Element { semantic: 5, format: 34, offset: 0, slot: 0, stride: 4 }] };
        assert!(descriptor.decode(&[0; 4]).is_err());
    }
}
