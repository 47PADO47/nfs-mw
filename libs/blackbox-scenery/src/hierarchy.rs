//! Named model parts and child ranges. Format: `docs/formats/model-hierarchies.md`.

use crate::{Error, Result};
use blackbox_chunk::find_all;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct ModelNode {
    pub name_hash: u32,
    pub model_hash: u32,
    pub flags: u8,
    pub parent: u8,
    pub child_count: u8,
    pub first_child: u8,
}

#[derive(Clone, Debug)]
pub struct ModelHierarchy {
    pub name_hash: u32,
    pub nodes: Vec<ModelNode>,
}

impl ModelHierarchy {
    /// Drawing a selected internal root is allowed; internal descendants remain reserved.
    pub fn visible_nodes(&self, root: usize) -> Vec<usize> {
        let mut pending = vec![root];
        let mut out = Vec::new();
        while let Some(index) = pending.pop() {
            let Some(node) = self.nodes.get(index) else { continue };
            out.push(index);
            let start = usize::from(node.first_child);
            for child in (start..start + usize::from(node.child_count)).rev() {
                if self.nodes[child].flags & 1 == 0 {
                    pending.push(child);
                }
            }
        }
        out
    }

    pub fn find(&self, name: u32) -> Option<usize> {
        self.nodes.iter().position(|n| n.name_hash == name)
    }
}

/// Container/record identifiers belong to the caller's game layout.
pub fn read_model_hierarchies(bytes: &[u8], tree_id: u32, record_id: u32) -> Result<Vec<ModelHierarchy>> {
    let mut out = Vec::new();
    for tree in find_all(bytes, tree_id) {
        for child in tree.children() {
            let child = child?;
            if child.id != record_id {
                continue;
            }
            let fail = |detail: &str| Error::Section { offset: child.offset, detail: detail.to_owned() };
            let data = child.payload;
            if data.len() < 8 {
                return Err(fail("truncated model hierarchy"));
            }
            let count = usize::from(data[4]);
            if count == 0 || data.len() < 8 + count * 16 {
                return Err(fail("invalid hierarchy node count"));
            }
            let word = |b: &[u8]| u32::from_le_bytes(b.try_into().unwrap());
            let nodes: Vec<_> = data[8..8 + count * 16]
                .as_chunks::<16>()
                .0
                .iter()
                .map(|n| ModelNode {
                    name_hash: word(&n[0..4]),
                    model_hash: word(&n[4..8]),
                    flags: n[12],
                    parent: n[13],
                    child_count: n[14],
                    first_child: n[15],
                })
                .collect();
            for (index, node) in nodes.iter().enumerate() {
                let start = usize::from(node.first_child);
                let end = start + usize::from(node.child_count);
                if node.child_count > 0 && (end > count || (start..end).contains(&index)) {
                    return Err(fail("invalid hierarchy child range"));
                }
                let mut pending = vec![index];
                let mut visited = vec![false; count];
                while let Some(at) = pending.pop() {
                    if at >= count || visited[at] {
                        return Err(fail("cyclic hierarchy"));
                    }
                    visited[at] = true;
                    let n = &nodes[at];
                    pending.extend(usize::from(n.first_child)..usize::from(n.first_child) + usize::from(n.child_count));
                }
            }
            out.push(ModelHierarchy { name_hash: word(&data[0..4]), nodes });
        }
    }
    Ok(out)
}
