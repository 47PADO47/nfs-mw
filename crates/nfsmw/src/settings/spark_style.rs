//! Original PC particles are separate from the experimental restoration.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SparkStyle {
    #[default]
    OriginalPc,
    RestoredExperimental,
}

impl SparkStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::OriginalPc => "Original PC",
            Self::RestoredExperimental => "Restored (Experimental)",
        }
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
