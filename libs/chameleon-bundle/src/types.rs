/// The resource type ids the engine registers, with the names it gives them. Ids this crate does not name stay raw
/// in [`crate::Entry::kind`]; [`ResourceKind::from_id`] returns `None` for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Texture,
    Material,
    VertexDescriptor,
    VertexProgramState,
    Renderable,
    MaterialState,
    SamplerState,
    ShaderProgramBuffer,
    GenesysType,
    GenesysObject,
    InstanceList,
    Model,
    Shader,
    PolySoupList,
    Ginsu,
    AnimationList,
    Skeleton,
    Animation,
    VehicleList,
    VehicleGraphicsSpec,
    VehicleSound,
    ConvexHull,
}

impl ResourceKind {
    pub const fn from_id(id: u32) -> Option<Self> {
        Some(match id {
            0x1 => Self::Texture,
            0x2 => Self::Material,
            0x3 => Self::VertexDescriptor,
            0x4 => Self::VertexProgramState,
            0x5 => Self::Renderable,
            0x6 => Self::MaterialState,
            0x7 => Self::SamplerState,
            0x8 => Self::ShaderProgramBuffer,
            0x14 => Self::GenesysType,
            0x15 => Self::GenesysObject,
            0x50 => Self::InstanceList,
            0x51 => Self::Model,
            0x53 => Self::Shader,
            0x60 => Self::PolySoupList,
            0x80 => Self::Ginsu,
            0xB0 => Self::AnimationList,
            0xB2 => Self::Skeleton,
            0xB3 => Self::Animation,
            0x105 => Self::VehicleList,
            0x106 => Self::VehicleGraphicsSpec,
            0x207 => Self::VehicleSound,
            0x303 => Self::ConvexHull,
            _ => return None,
        })
    }

    pub const fn id(self) -> u32 {
        match self {
            Self::Texture => 0x1,
            Self::Material => 0x2,
            Self::VertexDescriptor => 0x3,
            Self::VertexProgramState => 0x4,
            Self::Renderable => 0x5,
            Self::MaterialState => 0x6,
            Self::SamplerState => 0x7,
            Self::ShaderProgramBuffer => 0x8,
            Self::GenesysType => 0x14,
            Self::GenesysObject => 0x15,
            Self::InstanceList => 0x50,
            Self::Model => 0x51,
            Self::Shader => 0x53,
            Self::PolySoupList => 0x60,
            Self::Ginsu => 0x80,
            Self::AnimationList => 0xB0,
            Self::Skeleton => 0xB2,
            Self::Animation => 0xB3,
            Self::VehicleList => 0x105,
            Self::VehicleGraphicsSpec => 0x106,
            Self::VehicleSound => 0x207,
            Self::ConvexHull => 0x303,
        }
    }
}
