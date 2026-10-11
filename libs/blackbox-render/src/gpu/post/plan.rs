//! Which image each pass of the chain reads and writes. Pure, so the ordering is unit-tested.

use super::Extent;

/// What a pass reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    /// The offscreen scene image (HDR, at the render size).
    Scene,
    /// One of the two ping-pong images.
    Scratch(usize),
}

/// What a pass writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    /// One of the two ping-pong images.
    Scratch(usize),
    /// The final output: the swapchain image (or the capture image).
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Step {
    pub source: Source,
    pub target: Target,
}

/// The steps for `pass_count` passes run in order. The first reads the scene, the last writes the
/// output, and every pass in between alternates between the two scratch images so no pass reads
/// the image it writes.
pub(super) fn plan(pass_count: usize) -> Vec<Step> {
    (0..pass_count)
        .map(|i| Step {
            source: match i {
                0 => Source::Scene,
                _ => Source::Scratch((i - 1) % 2),
            },
            target: match i + 1 == pass_count {
                true => Target::Output,
                false => Target::Scratch(i % 2),
            },
        })
        .collect()
}

/// Where a pass of size `new` goes in a chain whose passes write `extents` (the last is the resolve
/// pass), when it is asked to run at `index`. An output-size pass goes right before the resolve pass
/// whatever the index; a render-size pass goes at `index` but stays before the first output-size pass.
pub(super) fn insert_index(extents: &[Extent], index: usize, new: Extent) -> usize {
    let resolve = extents.len().saturating_sub(1);
    if new == Extent::Output {
        return resolve;
    }
    let first_output = extents.iter().position(|e| *e == Extent::Output).unwrap_or(resolve);
    index.min(first_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_passes_no_steps() {
        assert!(plan(0).is_empty());
    }

    #[test]
    fn a_single_pass_reads_the_scene_and_writes_the_output() {
        assert_eq!(plan(1), [Step { source: Source::Scene, target: Target::Output }]);
    }

    #[test]
    fn passes_chain_through_alternating_scratch_images() {
        let steps = plan(4);
        assert_eq!(steps[0], Step { source: Source::Scene, target: Target::Scratch(0) });
        assert_eq!(steps[1], Step { source: Source::Scratch(0), target: Target::Scratch(1) });
        assert_eq!(steps[2], Step { source: Source::Scratch(1), target: Target::Scratch(0) });
        assert_eq!(steps[3], Step { source: Source::Scratch(0), target: Target::Output });
    }

    #[test]
    fn each_pass_reads_what_the_previous_one_wrote_and_never_its_own_target() {
        for count in 1..8 {
            let steps = plan(count);
            assert_eq!(steps.last().map(|s| s.target), Some(Target::Output));
            for (i, step) in steps.iter().enumerate() {
                assert!(i + 1 == count || step.target != Target::Output, "only the last pass writes the output");
                let Target::Scratch(written) = step.target else { continue };
                assert_ne!(step.source, Source::Scratch(written));
                assert_eq!(steps[i + 1].source, Source::Scratch(written));
            }
        }
    }

    #[test]
    fn inserted_passes_stay_before_resolve_and_output_passes_stay_last() {
        use Extent::{Output, Render};
        // Chain: [resolve].
        assert_eq!(insert_index(&[Render], 0, Render), 0);
        assert_eq!(insert_index(&[Render], 99, Render), 0, "before the resolve pass");
        assert_eq!(insert_index(&[Render], 99, Output), 0);
        // Chain: [bloom, upscale, resolve]: render-size passes cannot pass the upscaler.
        let chain = [Render, Output, Render];
        assert_eq!(insert_index(&chain, 99, Render), 1);
        assert_eq!(insert_index(&chain, 2, Render), 1);
        assert_eq!(insert_index(&chain, 0, Render), 0);
        assert_eq!(insert_index(&chain, 99, Output), 2, "a second output pass follows the first");
        assert_eq!(insert_index(&chain, 0, Output), 2, "output passes ignore the index");
    }
}
