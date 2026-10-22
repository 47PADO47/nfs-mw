//! When the egui pass can be skipped. A frame with no panel to draw needs no run, tessellation, conversion or output;
//! this is the decision, as plain data so it is tested without a window.

/// Whether anything egui draws is up this frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Visible {
    /// The scene's own panels (`Scene::wants_ui`).
    pub scene: bool,
    /// The F12 console.
    pub console: bool,
    /// The garage overlay.
    pub garage: bool,
    /// The graphics lab.
    pub graphics_lab: bool,
    /// The metrics overlay, at any level above off.
    pub metrics: bool,
    /// The scene's readout line (bottom left).
    pub readout: bool,
    /// The trace recording indicator.
    pub trace: bool,
    /// The clipboard and browser feedback message.
    pub platform_feedback: bool,
}

impl Visible {
    /// Whether any panel is up.
    pub fn any(self) -> bool {
        let Self { scene, console, garage, graphics_lab, metrics, readout, trace, platform_feedback } = self;
        scene || console || garage || graphics_lab || metrics || readout || trace || platform_feedback
    }
}

/// What the last pass that ran left behind. The default is "not settled": nothing has run yet, so the first pass does.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Settled(bool);

impl Settled {
    /// A pass drew no mesh and left egui neither wanting the pointer nor the keyboard: no panel is mid-fade or holding
    /// focus, so a frame with nothing up can go without a pass.
    pub fn after_pass(drew_nothing: bool, wants_pointer: bool, wants_keyboard: bool) -> Self {
        Self(drew_nothing && !wants_pointer && !wants_keyboard)
    }
}

/// Whether this frame needs the egui pass: a panel is up, or the last pass left something behind (a drawn layer to
/// clear, a focused field to release). The pass that clears it is the one that hands the renderer the empty layer;
/// the frames after it hand over nothing. Input events need no flag of their own: with no panel up and egui settled,
/// nothing could take them.
pub(super) fn egui_needed(visible: Visible, settled: Settled) -> bool {
    visible.any() || !settled.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTLED: Settled = Settled(true);

    #[test]
    fn nothing_up_and_settled_skips_the_pass() {
        assert!(!egui_needed(Visible::default(), SETTLED));
    }

    #[test]
    fn each_panel_runs_the_pass() {
        let each = [
            Visible { scene: true, ..Visible::default() },
            Visible { console: true, ..Visible::default() },
            Visible { garage: true, ..Visible::default() },
            Visible { graphics_lab: true, ..Visible::default() },
            Visible { metrics: true, ..Visible::default() },
            Visible { readout: true, ..Visible::default() },
            Visible { trace: true, ..Visible::default() },
            Visible { platform_feedback: true, ..Visible::default() },
        ];
        for visible in each {
            assert!(egui_needed(visible, SETTLED), "{visible:?}");
        }
    }

    #[test]
    fn the_first_pass_and_the_pass_after_a_drawn_one_run_with_nothing_up() {
        assert!(egui_needed(Visible::default(), Settled::default()));
        assert!(egui_needed(Visible::default(), Settled::after_pass(false, false, false)), "meshes were drawn");
        assert!(egui_needed(Visible::default(), Settled::after_pass(true, true, false)), "the pointer was wanted");
        assert!(egui_needed(Visible::default(), Settled::after_pass(true, false, true)), "a field had focus");
    }

    #[test]
    fn an_empty_pass_that_wants_no_input_settles() {
        assert!(!egui_needed(Visible::default(), Settled::after_pass(true, false, false)));
    }
}
