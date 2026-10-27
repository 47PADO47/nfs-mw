//! Which groups share a vertex buffer, and where each group's vertices start in it.

use std::ops::Range;

use super::source::GroupSource;

/// The reader's rule (`docs/formats/models.md`, "Several vertex buffers per solid"): one buffer per run of
/// consecutive groups with the same effect id, the groups' vertices following each other in group order.
pub(super) struct Plan {
    /// Group index ranges, one per vertex buffer.
    pub runs: Vec<Range<usize>>,
    /// For each group, how many vertices of its run come before it.
    pub base_in_run: Vec<usize>,
}

pub(super) fn plan(groups: &[GroupSource]) -> Plan {
    let mut runs: Vec<Range<usize>> = Vec::new();
    for (i, g) in groups.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if groups[run.start].effect_id == g.effect_id => run.end = i + 1,
            _ => runs.push(i..i + 1),
        }
    }
    let mut base_in_run = Vec::with_capacity(groups.len());
    for run in &runs {
        let mut base = 0;
        for g in &groups[run.clone()] {
            base_in_run.push(base);
            base += g.vertices.len();
        }
    }
    Plan { runs, base_in_run }
}
