//! User-selectable graphics backend.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Let the renderer choose: Vulkan or Direct3D 12 (Metal on macOS), then OpenGL.
    #[default]
    Auto,
    Vulkan,
    /// Windows only.
    Dx12,
    /// OpenGL (GLES 3 / desktop GL), the fallback for old GPUs.
    Gl,
}

impl Backend {
    pub const ALL: [Backend; 4] = [Backend::Auto, Backend::Vulkan, Backend::Dx12, Backend::Gl];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Dx12 => "dx12",
            Self::Gl => "gl",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown graphics backend {0:?} (expected one of: auto, vulkan, dx12, gl)")]
pub struct ParseBackendError(String);

impl FromStr for Backend {
    type Err = ParseBackendError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "vulkan" | "vk" => Ok(Self::Vulkan),
            "dx12" | "d3d12" => Ok(Self::Dx12),
            "gl" | "opengl" | "gles" => Ok(Self::Gl),
            _ => Err(ParseBackendError(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for b in Backend::ALL {
            assert_eq!(b.name().parse::<Backend>().unwrap(), b);
        }
        assert_eq!("D3D12".parse::<Backend>().unwrap(), Backend::Dx12);
        assert!("metal2".parse::<Backend>().is_err());
        assert!("dx11".parse::<Backend>().is_err());
    }
}
