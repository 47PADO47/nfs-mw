//! The operations of a BearEffect: the table of the executable (`NFS13.exe`) lists 78 of them by name, and an effect names the
//! ones it uses by their index in that table (docs/formats/chameleon-effects.md, "Operations").

/// The names of the operations, by the id an effect uses for them.
pub const NAMES: [&str; 78] = [
    "EmitterConePlus",
    "EmitterCollisionMesh",
    "EmitterNull",
    "EmitterTrail",
    "EmitterVolume",
    "EmitterWheel",
    "EmitterWheelContacts",
    "EmitterWheelContactPlus",
    "EmitterWheelTrack",
    "EmitterWheelTracks",
    "Initialise2dRotation",
    "Initialise3dRotation",
    "InitialiseAgeAndLifetime",
    "InitialiseAlpha",
    "InitialiseAndSubmitLight",
    "InitialiseAndSubmitLightLocalSpace",
    "InitialiseBakedLighting",
    "InitialiseCollisionRadius",
    "InitialiseRadius",
    "InitialiseBouncePlane",
    "InitialisePosOffset",
    "InitialisePosVelToWorldSpaceWithLocalTransform",
    "InitialiseVelocityInheritance",
    "ParametricApplyPerParticleAlpha",
    "ParametricEmitterVolume",
    "ParametricEmitterVolumeSubUV",
    "ParametricPosVelToWorldSpace",
    "ParametricPosVelToWorldSpaceWithLocalTransform",
    "ParametricSetColourGradient",
    "ParametricSetSubUV",
    "Render3dSprite",
    "Render3dSpriteSubUV",
    "RenderCubeMapSprite",
    "RenderLightBeam",
    "RenderMesh",
    "RenderMeshNoPaintColour",
    "RenderSprite",
    "RenderSpriteMotionBlur",
    "RenderSpriteShadowMapSubUV",
    "RenderSpriteSubUV",
    "RenderSpriteUVDistortion",
    "RenderSpriteVelocity",
    "RenderTrail",
    "SuperEmitter",
    "Update2dRotation",
    "Update3dRotation",
    "Update3dRotationBasedOnVelocity",
    "UpdateAge",
    "UpdateAndSubmitLight",
    "UpdateAxis",
    "UpdateAxisLocalSpace",
    "UpdateBindings",
    "UpdateBindings_Wheel",
    "UpdateOccluders",
    "UpdateParametric2dRotationAlignToVelocity",
    "UpdateParametricColour",
    "UpdateParametricFadeOverDistance",
    "UpdateParametricLighting",
    "UpdateParametricLightingWithOccluders",
    "UpdateParametricLightingWithTunnels",
    "UpdateParametricRadius",
    "UpdatePosVel",
    "UpdatePosVelWithBouncePlane",
    "UpdatePosVelWithVortex",
    "UpdatePosVelWithWorldCollision",
    "UpdateTrail",
    "UpdateWheelBindings",
    "RenderSpark",
    "UpdateBucketBoundingBox",
    "FrustumCullBucket",
    "FreezeOffscreen",
    "InitialiseRandomSeed",
    "UpdateParametricRandomTint",
    "PackVelocity",
    "UnpackVelocity",
    "InitialisePackedVelocity",
    "EmitterSplatter",
    "RenderSplatter",
];

pub const EMITTER_CONE_PLUS: u8 = 0;
pub const EMITTER_WHEEL: u8 = 5;
pub const INITIALISE_2D_ROTATION: u8 = 10;
pub const INITIALISE_AGE_AND_LIFETIME: u8 = 12;
pub const INITIALISE_AND_SUBMIT_LIGHT: u8 = 14;
pub const INITIALISE_RADIUS: u8 = 18;
pub const INITIALISE_POS_VEL_TO_WORLD_SPACE_WITH_LOCAL_TRANSFORM: u8 = 21;
pub const INITIALISE_VELOCITY_INHERITANCE: u8 = 22;
pub const PARAMETRIC_POS_VEL_TO_WORLD_SPACE: u8 = 26;
pub const PARAMETRIC_POS_VEL_TO_WORLD_SPACE_WITH_LOCAL_TRANSFORM: u8 = 27;
pub const PARAMETRIC_SET_COLOUR_GRADIENT: u8 = 28;
pub const PARAMETRIC_SET_SUB_UV: u8 = 29;
pub const RENDER_SPRITE: u8 = 36;
pub const RENDER_SPRITE_SUB_UV: u8 = 39;
pub const RENDER_SPRITE_UV_DISTORTION: u8 = 40;
pub const UPDATE_AND_SUBMIT_LIGHT: u8 = 48;
pub const UPDATE_PARAMETRIC_RADIUS: u8 = 60;
pub const UPDATE_POS_VEL: u8 = 61;
pub const UPDATE_POS_VEL_WITH_BOUNCE_PLANE: u8 = 62;

/// The name of operation `id`.
pub fn name(id: u8) -> Option<&'static str> {
    NAMES.get(usize::from(id)).copied()
}
