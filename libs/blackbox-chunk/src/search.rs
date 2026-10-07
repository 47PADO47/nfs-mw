use crate::{Chunk, Chunks, chunks};

pub(crate) fn find_in<'a>(iter: Chunks<'a>, id: u32) -> Option<Chunk<'a>> {
    for c in iter.filter_map(|c| c.ok()) {
        if c.id == id {
            return Some(c);
        }
        if c.is_container()
            && let Some(found) = find_in(c.children(), id)
        {
            return Some(found);
        }
    }
    None
}

/// Depth-first search of a whole buffer for the first chunk with `id`.
pub fn find(data: &[u8], id: u32) -> Option<Chunk<'_>> {
    find_in(chunks(data), id)
}

/// Every chunk with `id` in a buffer, depth-first, including ones nested in containers.
pub fn find_all(data: &[u8], id: u32) -> Vec<Chunk<'_>> {
    fn walk<'a>(iter: Chunks<'a>, id: u32, out: &mut Vec<Chunk<'a>>) {
        for c in iter.filter_map(|c| c.ok()) {
            if c.id == id {
                out.push(c);
            }
            if c.is_container() {
                walk(c.children(), id, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(chunks(data), id, &mut out);
    out
}
