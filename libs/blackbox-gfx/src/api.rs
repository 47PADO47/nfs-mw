//! The graphics API a renderer draws with (Vulkan, Direct3D 12, OpenGL).

use std::fmt;
use std::str::FromStr;

/// Which graphics API to draw with. This is the *API* (the old `Backend`), not the renderer that uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GraphicsApi {
    /// Let the renderer choose: Vulkan or Direct3D 12 (Metal on macOS), then OpenGL.
    #[default]
    Auto,
    Vulkan,
    /// Windows only.
    Dx12,
    /// OpenGL (GLES 3 / desktop GL), the fallback for old GPUs.
    Gl,
}

impl GraphicsApi {
    pub const ALL: [GraphicsApi; 4] = [GraphicsApi::Auto, GraphicsApi::Vulkan, GraphicsApi::Dx12, GraphicsApi::Gl];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Dx12 => "dx12",
            Self::Gl => "gl",
        }
    }
}

impl fmt::Display for GraphicsApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown graphics API {0:?} (expected one of: auto, vulkan, dx12, gl)")]
pub struct ParseGraphicsApiError(String);

impl FromStr for GraphicsApi {
    type Err = ParseGraphicsApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "vulkan" | "vk" => Ok(Self::Vulkan),
            "dx12" | "d3d12" => Ok(Self::Dx12),
            "gl" | "opengl" | "gles" => Ok(Self::Gl),
            _ => Err(ParseGraphicsApiError(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for b in GraphicsApi::ALL {
            assert_eq!(b.name().parse::<GraphicsApi>().unwrap(), b);
        }
        assert_eq!("D3D12".parse::<GraphicsApi>().unwrap(), GraphicsApi::Dx12);
        assert!("metal2".parse::<GraphicsApi>().is_err());
        assert!("dx11".parse::<GraphicsApi>().is_err());
    }
}
