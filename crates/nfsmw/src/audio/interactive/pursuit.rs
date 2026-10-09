//! The pursuit sets of `MW_Music.mpf`: where each starts, and the files they play from.

use std::path::PathBuf;
use std::sync::Arc;

use ea_audio::mus::Mpf;
use ea_audio::mus::graph::{Graph, NodeKind};

use super::state::PURSUIT_SETS;

/// The group heads the original's table of pursuit start nodes names (spec `docs/specs/music-graph.md` section
/// 8; the table has a fifth entry, 0, for "no pursuit"). Which entry belongs to which set is not known, so
/// [`starts`] sorts them by the section of the node.
pub const START_TABLE: [usize; PURSUIT_SETS as usize] = [0x199, 0x6A, 0x132, 0xD5];

/// The start node of each pursuit set, `[set 1 .. set 4]`. A set is the section number of its head, 1 to 4
/// (sections 1 to 4 are the pursuit sets **[confirmed-by-data]**). When the table's heads do not cover sections
/// 1 to 4 once each, the table order is used and `Err` is not returned, since the table is all there is.
pub fn starts(graph: &Graph) -> Result<[usize; PURSUIT_SETS as usize], String> {
    for &node in &START_TABLE {
        let Some(found) = graph.nodes.get(node) else {
            return Err(format!("the map has no node {node:#X}: this is not MW_Music.mpf"));
        };
        if found.kind != NodeKind::Head {
            return Err(format!("node {node:#X} is not a group head: this is not MW_Music.mpf"));
        }
    }
    let mut by_section = [None; PURSUIT_SETS as usize];
    for &node in &START_TABLE {
        let section = usize::from(graph.nodes[node].section);
        let Some(slot) = section.checked_sub(1).and_then(|s| by_section.get_mut(s)) else { continue };
        slot.get_or_insert(node);
    }
    if let [Some(a), Some(b), Some(c), Some(d)] = by_section {
        return Ok([a, b, c, d]);
    }
    log::warn!("music: the pursuit start nodes are not one per section 1 to 4; using the table order");
    Ok(START_TABLE)
}

/// What a pursuit track needs: the graph, the stream table and the music file.
pub struct PursuitFiles {
    pub graph: Arc<Graph>,
    pub mpf: Arc<Mpf>,
    pub mus: PathBuf,
    starts: [usize; PURSUIT_SETS as usize],
}

impl PursuitFiles {
    pub fn new(mpf_bytes: &[u8], mus: PathBuf) -> Result<Self, String> {
        let graph = Graph::parse(mpf_bytes).map_err(|e| format!("MW_Music.mpf: {e}"))?;
        let mpf = Mpf::parse(mpf_bytes).map_err(|e| format!("MW_Music.mpf: {e}"))?;
        let starts = starts(&graph)?;
        Ok(Self { graph: Arc::new(graph), mpf: Arc::new(mpf), mus, starts })
    }

    /// The node pursuit set `set` (1 to 4) starts at.
    pub fn start_node(&self, set: u8) -> Result<usize, String> {
        set_index(set).map(|i| self.starts[i])
    }
}

/// Index of pursuit set `set` (1 to 4) in a table of the four sets.
pub(super) fn set_index(set: u8) -> Result<usize, String> {
    let index = usize::from(set).checked_sub(1).filter(|&i| i < usize::from(PURSUIT_SETS));
    index.ok_or_else(|| format!("there is no pursuit set {set} (1 to {PURSUIT_SETS})"))
}
