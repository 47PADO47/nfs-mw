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
#[cfg(test)]
pub(super) fn plan(pass_count: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    plan_into(pass_count, &mut steps);
    steps
}

/// [`plan`] into `steps`, which is cleared first, so a chain that runs every frame reuses one buffer.
pub(super) fn plan_into(pass_count: usize, steps: &mut Vec<Step>) {
    steps.clear();
    steps.extend((0..pass_count).map(|i| Step {
        source: match i {
            0 => Source::Scene,
            _ => Source::Scratch((i - 1) % 2),
        },
        target: match i + 1 == pass_count {
            true => Target::Output,
            false => Target::Scratch(i % 2),
        },
    }));
}

/// How many encodes in a row a ping-pong image may go unused before it is freed. A chain that is shorter on some
/// frames (the rear-view mirror runs without the temporal upscaler) must not free the image the next frame needs.
pub(super) const SCRATCH_IDLE_FRAMES: u32 = 120;

/// Count one encode against the idle counter of a ping-pong slot: `used` resets it, else it grows. Returns whether
/// the slot has been idle for [`SCRATCH_IDLE_FRAMES`] and its images should be freed.
pub(super) fn slot_expired(idle: &mut u32, used: bool) -> bool {
    if used {
        *idle = 0;
        return false;
    }
    *idle = idle.saturating_add(1);
    *idle >= SCRATCH_IDLE_FRAMES
}

/// A hash of the pass names in order, to notice that the set of passes changed (not just which of them run).
pub(super) fn signature<'a>(names: impl Iterator<Item = &'a str>) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    names.fold(0xcbf2_9ce4_8422_2325, |hash, name| {
        let hash = name.bytes().fold(hash, |h, byte| (h ^ u64::from(byte)).wrapping_mul(PRIME));
        // A separator, so ["ab", "c"] and ["a", "bc"] differ.
        (hash ^ 0xff).wrapping_mul(PRIME)
    })
}

/// The size a pass that is not last writes: its own `extent`, except that once a pass has upscaled (`upscaled`)
/// every pass after it writes the output size too. Spatial upscalers sit last, so for them this is the pass's own
/// extent; a temporal upscaler sits first and the effects that follow it run on the upscaled image.
pub(super) fn written_extent(upscaled: bool, extent: Extent) -> Extent {
    match upscaled {
        true => Extent::Output,
        false => extent,
    }
}

/// Whether a fragment written straight into `format` is clamped to [0, 1] on the way, as the SDR resolve copy
/// clamps it: true for the 8-bit normalised formats. A float target is not clamped, so it always takes the copy,
/// whose HDR entry point passes the range through.
pub(super) fn clamps_on_write(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
    )
}

/// Whether the output needs the resolve copy after the chain whose passes write `extents`. Without any pass the
/// copy is the whole chain. A temporal upscaler last (`temporal_last`) keeps its copy (`post/fsr3.rs`). Any other
/// last pass writes the output itself when that is exact: it is output-sized (an upscaler scales its own input), or
/// it reads an image already the output size, and the output format clamps on write like the SDR copy does.
/// `render` is the size of the scene, `output` the size of the output.
pub(super) fn needs_resolve(
    extents: &[Extent],
    temporal_last: bool,
    render: (u32, u32),
    output: (u32, u32),
    format: wgpu::TextureFormat,
) -> bool {
    let Some(last) = extents.last() else { return true };
    if temporal_last || !clamps_on_write(format) {
        return true;
    }
    if *last == Extent::Output {
        return false;
    }
    // The last pass reads what the one before it wrote: the output size once any pass has upscaled.
    let upscaled = extents[..extents.len() - 1].contains(&Extent::Output);
    let input = match upscaled {
        true => output,
        false => render,
    };
    input != output
}

/// Where a pass of size `new` goes in a chain whose passes write `extents`, when it is asked to run at
/// `index`. An output-size pass goes last whatever the index; a render-size pass goes at `index` but stays
/// before the first output-size pass.
pub(super) fn insert_index(extents: &[Extent], index: usize, new: Extent) -> usize {
    if new == Extent::Output {
        return extents.len();
    }
    let first_output = extents.iter().position(|e| *e == Extent::Output).unwrap_or(extents.len());
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
    fn plan_into_reuses_the_buffer_and_matches_plan() {
        let mut steps = Vec::new();
        for count in [4, 1, 0, 3] {
            plan_into(count, &mut steps);
            assert_eq!(steps, plan(count));
        }
    }

    #[test]
    fn a_slot_expires_only_after_it_has_been_idle_for_many_frames() {
        let mut idle = 0;
        for _ in 0..SCRATCH_IDLE_FRAMES - 1 {
            assert!(!slot_expired(&mut idle, false));
        }
        assert!(!slot_expired(&mut idle, true), "use resets the count");
        for _ in 0..SCRATCH_IDLE_FRAMES - 1 {
            assert!(!slot_expired(&mut idle, false));
        }
        assert!(slot_expired(&mut idle, false));
    }

    #[test]
    fn the_signature_follows_the_names_and_their_order() {
        let sig = |names: &[&str]| signature(names.iter().copied());
        assert_eq!(sig(&["bloom", "fxaa"]), sig(&["bloom", "fxaa"]));
        assert_ne!(sig(&["bloom", "fxaa"]), sig(&["fxaa", "bloom"]));
        assert_ne!(sig(&["ab", "c"]), sig(&["a", "bc"]));
        assert_ne!(sig(&[]), sig(&["bloom"]));
    }

    #[test]
    fn passes_after_an_upscale_write_the_output_size() {
        assert_eq!(written_extent(false, Extent::Render), Extent::Render);
        assert_eq!(written_extent(false, Extent::Output), Extent::Output);
        assert_eq!(written_extent(true, Extent::Render), Extent::Output);
        assert_eq!(written_extent(true, Extent::Output), Extent::Output);
    }

    #[test]
    fn inserted_passes_stay_before_output_passes_and_output_passes_stay_last() {
        use Extent::{Output, Render};
        // Chain: [].
        assert_eq!(insert_index(&[], 99, Render), 0);
        assert_eq!(insert_index(&[], 99, Output), 0);
        // Chain: [bloom]: a render-size pass goes at its index, past the end included.
        assert_eq!(insert_index(&[Render], 99, Render), 1);
        // Chain: [bloom, upscale, effect]: render-size passes cannot pass the upscaler.
        let chain = [Render, Output, Render];
        assert_eq!(insert_index(&chain, 99, Render), 1);
        assert_eq!(insert_index(&chain, 2, Render), 1);
        assert_eq!(insert_index(&chain, 0, Render), 0);
        assert_eq!(insert_index(&chain, 99, Output), 3, "a second output pass goes last");
        assert_eq!(insert_index(&chain, 0, Output), 3, "output passes ignore the index");
    }

    #[test]
    fn the_copy_is_needed_only_when_the_last_pass_cannot_write_the_output() {
        use Extent::{Output, Render};
        let rgba = wgpu::TextureFormat::Rgba8UnormSrgb;
        let (small, big) = ((4, 4), (8, 8));
        assert!(needs_resolve(&[], false, small, small, rgba), "with no pass the copy is the whole chain");
        assert!(!needs_resolve(&[Render], false, small, small, rgba), "same size: the pass writes the output");
        assert!(needs_resolve(&[Render], false, small, big, rgba), "a render-size pass would be scaled by the copy");
        assert!(!needs_resolve(&[Output], false, small, big, rgba), "an output-size pass scales its own input");
        assert!(!needs_resolve(&[Render, Output, Render], false, small, big, rgba), "its input is output-sized");
        assert!(needs_resolve(&[Output], true, small, big, rgba), "a temporal last pass keeps its copy");
        let float = wgpu::TextureFormat::Rgba16Float;
        assert!(needs_resolve(&[Render], false, small, small, float), "a float output takes the copy's HDR path");
    }
}
