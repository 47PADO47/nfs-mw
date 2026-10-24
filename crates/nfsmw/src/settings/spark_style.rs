//! Original PC particles are separate from the experimental restoration.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SparkStyle {
    #[default]
    OriginalPc,
    RestoredExperimental,
}

impl SparkStyle {
    /// The style's name in the current language, as the options show it.
    pub fn label(self) -> String {
        let id = match self {
            Self::OriginalPc => "options-original-pc",
            Self::RestoredExperimental => "options-sparks-restored",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

impl std::str::FromStr for SparkStyle {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "original-pc" => Ok(Self::OriginalPc),
            "restored-experimental" => Ok(Self::RestoredExperimental),
            _ => Err(format!("expected original-pc or restored-experimental, got {s:?}")),
        }
    }
}

impl std::fmt::Display for SparkStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OriginalPc => "original-pc",
            Self::RestoredExperimental => "restored-experimental",
        })
    }
}
