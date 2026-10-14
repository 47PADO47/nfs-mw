use blackbox_chunk::{Chunk, chunks};

use crate::{Error, Layer, Result};

const MAP: u32 = 0x8003_B600;
const LAYER: u32 = 0x8003_B601;
const HEADER: u32 = 0x0003_B602;
const TYPES: u32 = 0x0003_B603;
const OFFSETS: u32 = 0x0003_B604;
const DATA: u32 = 0x0003_B605;
const QUAD8: u32 = 0x0003_B607;
const QUAD16: u32 = 0x0003_B608;

fn invalid(reason: &'static str) -> Error {
    Error::Invalid(reason)
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    let raw = bytes.get(at..at + 4).ok_or_else(|| invalid("short word"))?;
    Ok(u32::from_le_bytes(raw.try_into().map_err(|_| invalid("short word"))?))
}

fn count(bytes: &[u8], at: usize) -> Result<usize> {
    usize::try_from(word(bytes, at)? as i32).map_err(|_| invalid("negative count or offset"))
}

fn exact(bytes: &[u8], count: usize, stride: usize) -> Result<()> {
    if count.checked_mul(stride) != Some(bytes.len()) {
        return Err(invalid("payload size does not match its header"));
    }
    Ok(())
}

pub fn read_all(bytes: &[u8]) -> Result<Vec<Layer>> {
    let mut result = Vec::new();
    let mut pending: Vec<_> = chunks(bytes).collect::<std::result::Result<_, _>>()?;
    while let Some(chunk) = pending.pop() {
        if chunk.id == MAP {
            for child in chunk.children() {
                let child = child?;
                if child.id == LAYER {
                    result.push(layer(child)?);
                }
            }
            continue;
        }
        if chunk.is_container() {
            pending.extend(chunk.children().collect::<std::result::Result<Vec<_>, _>>()?);
        }
    }
    Ok(result)
}

fn layer(chunk: Chunk<'_>) -> Result<Layer> {
    let children = chunk.children().collect::<std::result::Result<Vec<_>, _>>()?;
    let one = |id| -> Result<&[u8]> {
        let mut matches = children.iter().filter(|child| child.id == id);
        let value = matches.next().ok_or_else(|| invalid("missing layer component"))?;
        if matches.next().is_some() {
            return Err(invalid("duplicate layer component"));
        }
        Ok(value.payload)
    };
    let header = one(HEADER)?;
    exact(header, 1, 36)?;
    let name_hash = word(header, 0)?;
    let bounds = [4, 8, 12, 16].map(|at| word(header, at).map(f32::from_bits));
    let bounds = bounds.into_iter().collect::<Result<Vec<_>>>()?;
    let bounds: [f32; 4] = bounds.try_into().map_err(|_| invalid("invalid bounds"))?;
    if bounds.iter().any(|value| !value.is_finite()) {
        return Err(invalid("nonfinite bounds"));
    }
    let node_count = count(header, 20)?;
    let sets = count(header, 24)?;
    let stride = count(header, 28)?;
    let field_count = count(header, 32)?;
    if sets == 0 || stride == 0 {
        return Err(invalid("empty parameter sets"));
    }
    let types = one(TYPES)?;
    let offsets = one(OFFSETS)?;
    exact(types, field_count, 4)?;
    exact(offsets, field_count, 4)?;
    let mut fields = Vec::with_capacity(field_count);
    for at in (0..types.len()).step_by(4) {
        let kind = word(types, at)? as i32;
        let offset = count(offsets, at)?;
        if !matches!(kind, 0 | 1) || offset.checked_add(4).is_none_or(|end| end > stride) {
            return Err(invalid("unsupported field or offset outside a set"));
        }
        fields.push((kind, offset));
    }
    let data = one(DATA)?;
    exact(data, sets, stride)?;
    let quad =
        children.iter().find(|child| child.id == QUAD8).or_else(|| children.iter().find(|child| child.id == QUAD16));
    let mut nodes = Vec::new();
    if let Some(quad) = quad {
        let width = if quad.id == QUAD8 { 1 } else { 2 };
        exact(quad.payload, node_count, width * 4)?;
        for raw in quad.payload.chunks_exact(width * 4) {
            nodes.push(std::array::from_fn(|i| {
                if width == 1 {
                    return usize::from(raw[i]);
                }
                usize::from(u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]))
            }));
        }
    }
    if node_count != 0 && nodes.is_empty() {
        return Err(invalid("missing quadrant nodes"));
    }
    validate_nodes(&nodes, sets)?;
    Ok(Layer { name_hash, bounds, fields, stride, sets, data: data.to_vec(), nodes })
}

fn validate_nodes(nodes: &[[usize; 4]], sets: usize) -> Result<()> {
    for node in nodes {
        if node[0] == 0 {
            if node[1] >= sets {
                return Err(invalid("leaf set index outside parameter data"));
            }
            continue;
        }
        if node.iter().any(|&child| child >= nodes.len()) {
            return Err(invalid("child index outside quadrant data"));
        }
    }
    let mut visited = vec![0u8; nodes.len()];
    for start in 0..nodes.len() {
        let mut pending = vec![(start, false)];
        while let Some((index, leave)) = pending.pop() {
            if leave {
                visited[index] = 2;
                continue;
            }
            if visited[index] == 1 {
                return Err(invalid("cyclic quadrant graph"));
            }
            if visited[index] == 2 {
                continue;
            }
            visited[index] = 1;
            pending.push((index, true));
            if nodes[index][0] != 0 {
                pending.extend(nodes[index].iter().rev().map(|&child| (child, false)));
            }
        }
    }
    Ok(())
}
