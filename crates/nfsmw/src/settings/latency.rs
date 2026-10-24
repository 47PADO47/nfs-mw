//! Optional NVIDIA presentation controls. Stored requests survive an unavailable runtime.
pub use blackbox_gfx::ReflexMode;

/// Frames the presentation engine may queue ahead of the display (`frame_latency`): 2 keeps the GPU busy, 1 shows each
/// frame sooner and stalls more when the CPU is late. Read when the renderer starts.
pub const DEFAULT_FRAME_LATENCY: u32 = 2;

/// The frame latencies a setting may hold.
pub const FRAME_LATENCIES: std::ops::RangeInclusive<u32> = 1..=2;

/// The `frame_latency` text: 1 or 2.
pub fn parse_frame_latency(text: &str) -> Result<u32, String> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|frames| FRAME_LATENCIES.contains(frames))
        .ok_or_else(|| format!("expected 1 or 2, got {text:?}"))
}

/// How frames reach the display with vsync off (`present_mode`). `Immediate` shows each frame as soon as it is done and
/// can tear; `Mailbox` shows the newest finished frame at each refresh, never tears, and drops the frames in between.
/// The native renderer takes it at start; the Bevy backend always uses Mailbox without vsync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentMode {
    #[default]
    Immediate,
    Mailbox,
}

impl std::fmt::Display for PresentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Immediate => "immediate",
            Self::Mailbox => "mailbox",
        })
    }
}

impl std::str::FromStr for PresentMode {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "immediate" => Ok(Self::Immediate),
            "mailbox" => Ok(Self::Mailbox),
            _ => Err(format!("expected immediate or mailbox, got {value:?}")),
        }
    }
}

/// The swapchain options, read when the renderer starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swapchain {
    /// Frames the swapchain may queue ahead, 1 or 2 (`frame_latency`).
    pub frame_latency: u32,
    pub present_mode: PresentMode,
}

impl Swapchain {
    pub fn from_partial(p: &super::Partial) -> Self {
        Self {
            frame_latency: p.frame_latency.unwrap_or(DEFAULT_FRAME_LATENCY),
            present_mode: p.present_mode.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameGeneration(pub u32);

impl FrameGeneration {
    pub const ALL: [Self; 6] = [Self(0), Self(1), Self(2), Self(3), Self(4), Self(5)];
}

impl std::fmt::Display for FrameGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            return f.write_str("off");
        }
        write!(f, "{}x", self.0 + 1)
    }
}

impl std::str::FromStr for FrameGeneration {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "0" => Ok(Self(0)),
            "2x" => Ok(Self(1)),
            "3x" => Ok(Self(2)),
            "4x" => Ok(Self(3)),
            "5x" => Ok(Self(4)),
            "6x" => Ok(Self(5)),
            _ => Err("expected off, 2x, 3x, 4x, 5x or 6x".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_requests_round_trip_and_merge_without_enabling_generation_by_default() {
        let default = super::super::Settings::from(super::super::Partial::default());
        assert_eq!(default.frame_generation, FrameGeneration(0));
        for mode in FrameGeneration::ALL {
            let parsed = super::super::file::parse(&format!("frame_generation='{mode}'\nreflex='boost'"), "test");
            let s = super::super::Settings::from(parsed);
            let text = super::super::write::merge("master_volume=17", &s.graphics_layer()).unwrap();
            let restored = super::super::Settings::from(super::super::file::parse(&text, "test"));
            assert_eq!(restored.frame_generation, mode);
            assert_eq!(restored.reflex, ReflexMode::Boost);
            assert_eq!(restored.master_volume.0, 17);
        }
    }
}
