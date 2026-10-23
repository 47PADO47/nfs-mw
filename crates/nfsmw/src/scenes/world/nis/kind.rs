//! The kind of a NIS scene and how a scene ended.

/// The kind of a scene. It decides whether the player may skip the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NisKind {
    /// A race start or an intro.
    Intro,
    /// A police arrest. The player cannot skip it.
    Arrest,
    /// Any other scene: a replay, a finish, a free-roam scene.
    Outro,
    /// A cinematic moment: a camera on a collapse.
    Moment,
    /// An ending. The player cannot skip it.
    Ending,
}

impl NisKind {
    /// Whether the player may skip a scene of this kind: arrests and endings never are.
    pub fn skippable(self) -> bool {
        !matches!(self, Self::Arrest | Self::Ending)
    }

    /// The kind a scene name implies, for the console: by its prefix (Arrest, Ending, Intro, WM for a moment), without
    /// regard to case. Any other name is an [`NisKind::Outro`].
    pub fn of_scene(name: &str) -> Self {
        const PREFIXES: [(&str, NisKind); 4] = [
            ("arrest", NisKind::Arrest),
            ("ending", NisKind::Ending),
            ("intro", NisKind::Intro),
            ("wm", NisKind::Moment),
        ];
        let lower = name.to_ascii_lowercase();
        PREFIXES.iter().find(|(prefix, _)| lower.starts_with(prefix)).map_or(NisKind::Outro, |&(_, kind)| kind)
    }
}

/// How a scene ended. A scene that is stopped from the console counts as skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NisOutcome {
    /// The scene ran to its end.
    Completed,
    /// The scene ended early: the player skipped it, or it was stopped.
    Skipped,
}
