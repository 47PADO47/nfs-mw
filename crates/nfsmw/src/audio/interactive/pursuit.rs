//! The pursuit sets of `MW_Music.mpf`: where each starts, and the files they play from.

use std::path::PathBuf;
use std::sync::Arc;

use ea_audio::mus::Mpf;
use ea_audio::mus::graph::{Graph, NodeKind};

use super::state::PURSUIT_SETS;

/// First group head of each pursuit section. Installed PC entries sustain the score;
/// the old 409/106/306/213 table selects internal groups of section 1, including an ending cue.
/// Spec: `docs/specs/pursuit-free-roam.md` and `music-graph.md` section 8.
pub fn starts(graph: &Graph) -> Result<[usize; PURSUIT_SETS as usize], String> {
    let mut starts = [0; PURSUIT_SETS as usize];
    for (index, start) in starts.iter_mut().enumerate() {
        let section = index as u8 + 1;
        *start = graph
            .nodes
            .iter()
            .position(|node| node.kind == NodeKind::Head && node.section == section)
            .ok_or_else(|| format!("MW_Music.mpf has no entry head for pursuit section {section}"))?;
    }
    Ok(starts)
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
