/// Declares [`ResourceKind`] from one table of `Name = id` rows, so the enum, its id lookup and its names cannot drift.
macro_rules! resource_kinds {
    ($($name:ident = $id:literal,)*) => {
        /// The resource type ids a 2012 bundle uses, with the names the executable registers for them (a few ids the
        /// executable does not name carry the name the community tools use, see `docs/formats/chameleon-bundle.md`).
        /// Ids this crate does not name stay raw in [`crate::Entry::kind`]; [`ResourceKind::from_id`] returns `None`
        /// for them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ResourceKind {
            $($name,)*
        }

        impl ResourceKind {
            /// Every named kind, in id order.
            pub const ALL: &'static [Self] = &[$(Self::$name,)*];

            pub const fn from_id(id: u32) -> Option<Self> {
                Some(match id {
                    $($id => Self::$name,)*
                    _ => return None,
                })
            }

            pub const fn id(self) -> u32 {
                match self {
                    $(Self::$name => $id,)*
                }
            }

            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name),)*
                }
            }
        }
    };
}

resource_kinds! {
    Texture = 0x1,
    Material = 0x2,
    VertexDescriptor = 0x3,
    VertexProgramState = 0x4,
    Renderable = 0x5,
    MaterialState = 0x6,
    SamplerState = 0x7,
    ShaderProgramBuffer = 0x8,
    GenesysType = 0x14,
    GenesysObject = 0x15,
    Font = 0x30,
    InstanceList = 0x50,
    Model = 0x51,
    ColourCube = 0x52,
    Shader = 0x53,
    PolySoupList = 0x60,
    NavigationMesh = 0x68,
    TextFile = 0x70,
    LuaData = 0x74,
    Ginsu = 0x80,
    Wave = 0x81,
    WaveContainerTable = 0x82,
    GameplayLinkData = 0x83,
    WaveDictionary = 0x84,
    MicroMonoStream = 0x85,
    Reverb = 0x86,
    ZoneList = 0x90,
    WorldPaintMap = 0x91,
    AnimationList = 0xB0,
    PathAnimation = 0xB1,
    Skeleton = 0xB2,
    Animation = 0xB3,
    VehicleList = 0x105,
    VehicleGraphicsSpec = 0x106,
    AiData = 0x200,
    LocalisedText = 0x201,
    TriggerData = 0x202,
    RoadData = 0x203,
    DynamicInstanceList = 0x204,
    WorldObject = 0x205,
    ZoneHeader = 0x206,
    VehicleSound = 0x207,
    RoadMapData = 0x208,
    CharacterSpec = 0x209,
    SurfaceSounds = 0x20B,
    ReverbRoadData = 0x20C,
    CameraTake = 0x20D,
    CameraTakeList = 0x20E,
    GroundcoverCollection = 0x20F,
    LightInstanceList = 0x213,
    GroundcoverInstances = 0x214,
    CompoundObject = 0x215,
    CompoundInstanceList = 0x216,
    PropObject = 0x217,
    PropInstanceList = 0x218,
    ZoneAmbienceList = 0x219,
    BearEffect = 0x301,
    BearGlobalParameters = 0x302,
    ConvexHull = 0x303,
    HsmData = 0x501,
    TrafficLaneData = 0x701,
}
