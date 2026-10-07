"""Known bChunk IDs for Need for Speed: Most Wanted (2005).

Sources (both from the dbalatoni13/nfsmw decompilation,
https://github.com/dbalatoni13/nfsmw, CC0-1.0):

- src/Speed/Indep/Src/Misc/SpeedChunks.hpp: the engine's BCHUNK_* identifiers.
  Each entry that has one carries it in a trailing "# BCHUNK_..." comment.
  Display names are CamelCase versions of those identifiers, except where a
  long-standing community name says the same thing more clearly (geometry,
  textures, lights); that name is kept.
- symbols/bchunks.txt: community names, used for IDs that SpeedChunks.hpp
  does not list (no "# BCHUNK_" comment).

Where two names are useful, the display name is "Primary / Alias" or
"Primary (Alias)". Console-only IDs (PS2 / GameCube / Xbox / Xenon) are left
out. Several PC chunks reuse IDs that the decomp names after another platform
(e.g. 0x00134B01, a PC vertex buffer, is BCHUNK_SPEED_ESOLID_XBOX_VERTEX_DATA);
the comment shows the decomp identifier as-is.

Bit 31 (0x80000000) set on an ID means "this chunk contains child chunks".
"""

CHUNK_NAMES = {
    0x00000000: "Padding",
    # --- World: scenery / streaming / track --------------------------------------
    0x00034026: "SmokeableInfo",  # BCHUNK_SPEED_SMOKEABLE_INFO
    0x00034027: "SmokeableSpawners",  # BCHUNK_SPEED_SMOKEABLE_SPAWNER
    0x00034099: "VisibleSectionUserInfo",
    0x80034100: "ScenerySection",  # BCHUNK_SPEED_SCENERY_SECTION
    0x00034101: "ScenerySectionHeader",  # BCHUNK_SPEED_SCENERY_SECTION_HEADER
    0x00034102: "SceneryInfos",  # BCHUNK_SPEED_SCENERY_INFOS
    0x00034103: "SceneryInstances",  # BCHUNK_SPEED_SCENERY_INSTANCES
    0x00034105: "SceneryTreeNodes",  # BCHUNK_SPEED_SCENERY_TREE_NODES
    0x00034106: "SceneryOverrideHooks",  # BCHUNK_SPEED_SCENERY_OVERRIDE_HOOKUPS
    0x00034107: "SceneryPrecullerInfos",  # BCHUNK_SPEED_SCENERY_PRECULLER_INFOS
    0x00034108: "SceneryOverrideInfos",  # BCHUNK_SPEED_SCENERY_OVERRIDE_INFOS
    0x00034109: "SceneryGroups",  # BCHUNK_SPEED_SCENERY_GROUPS
    0x0003410A: "SceneryBoundingBoxes",  # BCHUNK_SPEED_SCENERY_BOUNDING_BOXES
    0x8003410B: "ModelHierarchyTree",  # BCHUNK_SPEED_SCENERY_HEIRARCHIES
    0x0003410C: "ModelHierarchy",  # BCHUNK_SPEED_SCENERY_HEIRARCHY
    0x0003410D: "SceneryLightTexInfos",  # BCHUNK_SPEED_SCENERY_LIGHTTEX_INFOS
    0x00034110: "TrackStreamingSections",  # BCHUNK_SPEED_TRACK_STREAMING_SECTION
    0x00034111: "TrackStreamingInfos",  # BCHUNK_SPEED_TRACK_STREAMING_INFO
    0x00034112: "TrackStreamingBarriers",  # BCHUNK_SPEED_TRACK_STREAMING_BARRIERS
    0x00034113: "TrackStreamingDiscBundle",  # BCHUNK_SPEED_TRACK_STREAMING_DISC_BUNDLE
    0x80034115: "SceneryLighting",  # BCHUNK_SPEED_SCENERY_LIGHTING
    0x00034116: "SceneryLightArray",  # BCHUNK_SPEED_SCENERY_LIGHTARRAY
    0x00034117: "SceneryLightContextHeader",  # BCHUNK_SPEED_SCENERY_LIGHTCONTEXTHEADER
    0x00034118: "SceneryLightContext",  # BCHUNK_SPEED_SCENERY_LIGHTCONTEXT
    0x00034121: "TrackRoutes",  # BCHUNK_SPEED_TRACKROUTES
    0x00034122: "Signposts",  # BCHUNK_SPEED_SIGNPOSTS
    0x00034123: "TrafficIntersections",  # BCHUNK_SPEED_TRAFFIC_INTERSECTIONS
    0x00034124: "CrossTrafficEmitters",  # BCHUNK_SPEED_CROSS_TRAFFIC_EMITTERS
    0x00034146: "TrackPositionMarkers",  # BCHUNK_SPEED_TRACK_POSITION_MARKERS
    0x80034147: "TrackPathManager",  # BCHUNK_SPEED_TRACK_PATH_MANAGER
    0x00034148: "TrackPathPoints",  # BCHUNK_SPEED_TRACK_PATH_POINTS
    0x00034149: "TrackPaths",  # BCHUNK_SPEED_TRACK_PATHS
    0x0003414A: "TrackPathZones",  # BCHUNK_SPEED_TRACK_PATH_ZONES
    0x0003414C: "TrackPathConnGroup",  # BCHUNK_SPEED_TRACK_PATH_CONN_GROUP
    0x0003414D: "TrackPathBarriers",  # BCHUNK_SPEED_TRACK_PATH_BARRIERS
    0x80034150: "VisibleSectionManager",  # BCHUNK_SPEED_VISIBLE_SECTION_CHUNKS
    0x00034151: "VisibleSectionManagerInfo",  # BCHUNK_SPEED_VISIBLE_SECTION_MANAGER_INFO
    0x00034152: "VisibleSectionBoundaries",  # BCHUNK_SPEED_VISIBLE_SECTION_BOUNDARIES
    0x00034153: "DrivableScenerySections",  # BCHUNK_SPEED_VISIBLE_SECTION_DRIVABLE
    0x00034154: "SpecialSectionsPerTrack",  # BCHUNK_SPEED_SPECIAL_SECTIONS_PER_TRACK
    0x00034155: "LoadingSections",  # BCHUNK_SPEED_LOADING_SECTIONS
    0x00034156: "ElevationPolygons",  # BCHUNK_SPEED_ELEV_POLYS
    0x00034158: "VisibleSectionOverlays",  # BCHUNK_SPEED_VISIBLE_SECTION_OVERLAY
    0x00034159: "HeliSheet",  # BCHUNK_SPEED_HELI_SHEET
    0x80034180: "Troughs",  # BCHUNK_SPEED_TROUGHS
    0x00034181: "TroughPerimeter",  # BCHUNK_SPEED_TROUGH_PERIMETER
    0x00034182: "TroughQuadTreeNodes",  # BCHUNK_SPEED_TROUGH_QUAD_TREE_NODES
    0x00034183: "TroughQuadTreeEntries",  # BCHUNK_SPEED_TROUGH_QUAD_TREE_ENTRIES
    0x00034184: "WallClusters",  # BCHUNK_SPEED_WALL_CLUSTERS
    0x00034191: "TrackObjectBounds",
    0x00034201: "TrackInfos",  # BCHUNK_TRACK_INFO_TABLE
    0x00034202: "SunInfos",  # BCHUNK_SUN_INFO
    0x00034250: "WeathermanPack",  # BCHUNK_SPEED_WEATHERMAN_PACK

    # --- World: CARP (collision, road grid, triggers, event sequences) -----------
    0x0003B800: "CarpWGrid (UWorld)",  # BCHUNK_CARP_WGRID; loaded as BCHUNK_UPPLE_UWORLD by WWorld.cpp
    0x0003B801: "CarpWCollisionPack",  # BCHUNK_CARP_WCOLLISIONPACK
    0x0003B802: "CarpWGridIslandData",  # BCHUNK_CARP_WGRID_ISLAND_DATA
    0x0003B803: "CarpTrigger",  # BCHUNK_CARP_TRIGGER
    0x8003B810: "CarpEventSequences",  # BCHUNK_CARP_EVENT_SEQUENCES
    0x0003B811: "CarpEventSequence",  # BCHUNK_CARP_EVENT_SEQUENCE
    0x8003B900: "BoundsPack (CollisionVolumes)",  # BCHUNK_SPEED_BOUNDS_PACK
    0x0003B901: "CollisionBody / Bounds",  # BCHUNK_SPEED_BOUNDS

    # --- Events / emitter triggers -----------------------------------------------
    0x80036000: "EmTriggerPack",  # BCHUNK_SPEED_EMTRIGGER_PACK
    0x00036001: "EmTriggerPackHeader",  # BCHUNK_SPEED_EMTRIGGER_PACK_HEADER
    0x00036002: "EmTriggerPackTree",  # BCHUNK_SPEED_EMTRIGGER_PACK_TREE
    0x00036003: "EmTriggerPackEventTriggers",  # BCHUNK_SPEED_EMTRIGGER_PACK_EVENT_TRIGGERS

    # --- Effects / particles -----------------------------------------------------
    0x80035000: "AcidFxEffectPack",  # BCHUNK_SPEED_ACIDFX_EFFECT_PACK
    0x00035001: "AcidFxEffectPackHeader",  # BCHUNK_SPEED_ACIDFX_EFFECT_PACK_HEADER
    0x00035002: "AcidFxEffectPackEffects",  # BCHUNK_SPEED_ACIDFX_EFFECT_PACK_EFFECTS
    0x80035010: "AcidFxEmitterPack",  # BCHUNK_SPEED_ACIDFX_EMITTER_PACK
    0x00035011: "AcidFxEmitterPackHeader",  # BCHUNK_SPEED_ACIDFX_EMITTER_PACK_HEADER
    0x00035012: "AcidFxEmitterPackEmitters",  # BCHUNK_SPEED_ACIDFX_EMITTER_PACK_EMITTERS
    0x00035020: "AcidFxEmitter2Pack",  # BCHUNK_SPEED_ACIDFX_EMITTER2_PACK
    0x00035021: "AcidFxEmitterDataPack",  # BCHUNK_SPEED_ACIDFX_EMITTERDATA_PACK
    0x0003BB00: "EmitterGroup",  # BCHUNK_SPEED_EMITTER_GROUP
    0x0003BC00: "EmitterLibrary",  # BCHUNK_SPEED_EMITTER_LIBRARY

    # --- Animation ---------------------------------------------------------------
    0x80037020: "AnimScene (NisScene)",  # BCHUNK_SPEED_ANIM_SCENE
    0x00037030: "AnimSceneHeaderData",  # BCHUNK_SPEED_ANIM_SCENE_HEADER_DATA
    0x00037040: "AnimSceneEntityData",  # BCHUNK_SPEED_ANIM_SCENE_ENTITY_DATA
    0x00037045: "AnimSceneEntityClipData",  # BCHUNK_SPEED_ANIM_SCENE_ENTITY_CLIP_DATA
    0x00037047: "AnimSceneEntityUcapData",  # BCHUNK_SPEED_ANIM_SCENE_ENTITY_UCAP_DATA
    0x80037050: "AnimDirectory",  # BCHUNK_SPEED_ANIM_DIRECTORY
    0x00037060: "AnimDirectorySceneLoadData",  # BCHUNK_SPEED_ANIM_DIRECTORY_SCENE_LOAD_DATA
    0x00037070: "AnimDirectorySceneMappingData",  # BCHUNK_SPEED_ANIM_DIRECTORY_SCENE_MAPPING_DATA
    0x00037080: "WorldAnimEntities",
    0x00037090: "WorldAnimInstances",
    0x00037100: "CarPartAnimHeader",  # BCHUNK_SPEED_CARPART_ANIM_HEADER
    0x00037110: "WorldAnimTreeMarkers",
    0x00037140: "WorldAnimEventDirectoryData",  # BCHUNK_SPEED_WORLDANIM_EVENTDIRECTORY_DATA
    0x00037150: "WorldAnimInstanceEntry / WorldAnimCtrl",  # BCHUNK_SPEED_WORLDANIM_INSTANCE_ENTRY
    0x00037160: "WorldAnimPipeInstanceEntry",  # BCHUNK_SPEED_WORLDANIMPIPE_INSTANCE_ENTRY
    0x00037170: "WorldAnimPipeSectionList",  # BCHUNK_SPEED_WORLDANIMPIPE_SECTIONLIST
    0x00037210: "BBGAnimPackHeader",  # BCHUNK_SPEED_BBGANIM_PACKHEADER
    0x00037220: "BBGAnimBlockHeader",  # BCHUNK_SPEED_BBGANIM_BLOCKHEADER
    0x00037230: "BBGAnimPipeKeyframes",  # BCHUNK_SPEED_BBGANIMPIPE_KEYFRAMES
    0x00037240: "BBGAnimKeyframes",  # BCHUNK_SPEED_BBGANIM_KEYFRAMES
    0x00037250: "BBGAnimInstanceNode",  # BCHUNK_SPEED_BBGANIM_INSTANCE_NODE
    0x00037260: "BBGAnimInstanceTree",  # BCHUNK_SPEED_BBGANIM_INSTANCE_TREE
    0x00037270: "BBGAnimEndPackHeader",  # BCHUNK_SPEED_BBGANIM_ENDPACKHEADER
    0x00E34009: "EAGLSkeletons",  # BCHUNK_SPEED_ANIMATION_SKELETON
    0x00E34010: "EAGLAnimations",  # BCHUNK_SPEED_ANIMATION_ANIMBANK

    # --- Splines / parameter maps ------------------------------------------------
    0x8003B000: "SplinePack (QuickSplines)",  # BCHUNK_SPEED_SPLINE_PACK
    0x8003B001: "QuickSpline",  # BCHUNK_SPEED_QUICKSPLINE
    0x0003B002: "QuickSplineInstance",  # BCHUNK_SPEED_QUICKSPLINE_INSTANCE
    0x0003B003: "QuickSplinePoints",  # BCHUNK_SPEED_QUICKSPLINE_POINTS
    0x0003B100: "SplineRangeEnd",  # BCHUNK_SPEED_SPLINE_RANGE_END
    0x8003B600: "ParameterMaps",  # BCHUNK_PARAMETER_MAPS_DATA
    0x8003B601: "ParameterMapLayer",  # BCHUNK_PARAMETER_MAPS_LAYER_DATA
    0x0003B602: "ParameterMapLayerHeader",  # BCHUNK_PARAMETER_MAPS_LAYER_HEADER
    0x0003B603: "ParameterMapLayerFieldTypes",  # BCHUNK_PARAMETER_MAPS_LAYER_FIELD_TYPES
    0x0003B604: "ParameterMapLayerFieldOffsets",  # BCHUNK_PARAMETER_MAPS_LAYER_FIELD_OFFSETS
    0x0003B605: "ParameterMapLayerParameterData",  # BCHUNK_PARAMETER_MAPS_LAYER_PARAMETER_DATA
    0x0003B606: "ParameterMapLayerMapData",  # BCHUNK_PARAMETER_MAPS_LAYER_MAP_DATA
    0x0003B607: "ParameterMapLayerQuadData8",  # BCHUNK_PARAMETER_MAPS_LAYER_QUAD_DATA_8
    0x0003B608: "ParameterMapLayerQuadData16",  # BCHUNK_PARAMETER_MAPS_LAYER_QUAD_DATA_16

    # --- Geometry (solids / meshes) ----------------------------------------------
    0x80134000: "GeometryPack",  # BCHUNK_SPEED_ESOLID_LIST_CHUNKS
    0x80134001: "MeshContainerInfo",  # BCHUNK_SPEED_ESOLID_HEADER_CHUNKS
    0x00134002: "MeshContainerHeader (SolidListInfo)",  # BCHUNK_SPEED_ESOLID_LIST_HEADER
    0x00134003: "MeshContainerKeys",  # BCHUNK_SPEED_ESOLID_LIST_INDEX_TABLE
    0x00134004: "MeshContainerOffsets",  # BCHUNK_SPEED_ESOLID_LIST_STREAM_TABLE
    0x80134008: "MeshContainerEmpty",  # BCHUNK_SPEED_ESOLID_LIST_PLAT_CHUNKS
    0x80134010: "SolidPack",  # BCHUNK_SPEED_ESOLID_CHUNKS
    0x00134011: "SolidInfo (SolidListObjHead)",  # BCHUNK_SPEED_ESOLID
    0x00134012: "SolidTextures (Texture hashes)",  # BCHUNK_SPEED_ESOLID_TEX_TABLE
    0x00134013: "SolidShaders (LightMaterials)",  # BCHUNK_SPEED_ESOLID_LIGHT_TABLE
    0x00134017: "MeshNormalSmoother",  # BCHUNK_SPEED_ESOLID_NORMAL_SMOOTHER
    0x00134018: "MeshSmoothVertices",  # BCHUNK_SPEED_ESOLID_SMOOTH_VERTS
    0x00134019: "MeshSmoothVertexPlats",  # BCHUNK_SPEED_ESOLID_SMOOTH_VERTS_PLAT
    0x0013401A: "SolidMarkers",  # BCHUNK_SPEED_ESOLID_POSITION_MARKERS
    0x80134100: "MeshInfoContainer",  # BCHUNK_SPEED_ESOLID_PLAT_CHUNKS
    0x00134900: "MeshInfoHeader (SolidMeshDescriptor)",  # BCHUNK_SPEED_ESOLID_PC_PLATINFO
    0x00134B01: "MeshVertexBuffer",  # BCHUNK_SPEED_ESOLID_XBOX_VERTEX_DATA
    0x00134B02: "MeshShaderInfos (SolidObjectShadingGroups)",  # BCHUNK_SPEED_ESOLID_XBOX_MESH_ENTRY_TABLE
    0x00134B03: "MeshPolygons (SolidMeshFaces)",  # BCHUNK_SPEED_ESOLID_XBOX_MESH_ENTRY_DATA
    0x00134C02: "MeshVltMaterials",  # BCHUNK_SPEED_ESOLID_MATERIAL_NAME

    # --- Lights ------------------------------------------------------------------
    0x80135000: "LightSourcesPack",  # BCHUNK_SPEED_ELIGHT_CHUNKS
    0x00135001: "LightSourcePackHeader",  # BCHUNK_SPEED_ELIGHT_PACK_HEADER
    0x00135002: "LightAABBNodes",  # BCHUNK_SPEED_ELIGHT_TREE
    0x00135003: "LightSources",  # BCHUNK_SPEED_ELIGHT_LIGHTARRAY
    0x80135100: "LightFlaresPack",  # BCHUNK_SPEED_ELIGHTFLARE_CHUNKS
    0x00135101: "LightFlarePackHeader",  # BCHUNK_SPEED_ELIGHTFLARE_PACK_HEADER
    0x00135102: "LightFlare",  # BCHUNK_SPEED_ELIGHTFLARE_FLARE_ARRAY
    0x00135200: "LightMaterials",  # BCHUNK_SPEED_ELIGHTMATERIAL
    0x0003B650: "ShaperLightRig",  # BCHUNK_SHAPER_LIGHT_RIG

    # --- Textures (TPK) ----------------------------------------------------------
    0xB3300000: "TexturePack",  # BCHUNK_SPEED_TEXTURE_PACK_LIST_CHUNKS
    0xB3310000: "TexturePackInfo",  # BCHUNK_SPEED_TEXTURE_PACK_HEADER_CHUNKS
    0x33310001: "TexturePackInfoHeader",  # BCHUNK_SPEED_TEXTURE_PACK_HEADER
    0x33310002: "TexturePackInfoKeys (Hashes)",  # BCHUNK_SPEED_TEXTURE_PACK_INDEX_TABLE
    0x33310003: "TexturePackInfoEntries",  # BCHUNK_SPEED_TEXTURE_PACK_STREAM_TABLE
    0x33310004: "TexturePackInfoTextures",  # BCHUNK_SPEED_TEXTURE_PACK_INFO_TABLE
    0x33310005: "TexturePackInfoComps",  # BCHUNK_SPEED_TEXTURE_PACK_PLAT_INFO_TABLE
    0xB3312000: "TexturePackBinary",  # BCHUNK_SPEED_TEXTURE_ANIM_CHUNKS
    0x33312001: "TexturePackAnimNames",  # BCHUNK_SPEED_TEXTURE_ANIM
    0x33312002: "TexturePackAnimFrames",  # BCHUNK_SPEED_TEXTURE_ANIM_ENTRIES
    0xB3312004: "TexturePackAnim",  # BCHUNK_SPEED_TEXTURE_ANIM_WRAPPER
    0xB3320000: "TexturePackData",  # BCHUNK_SPEED_TEXTURE_VRAM_DATA_CHUNKS
    0x33320001: "TexturePackDataHeader",  # BCHUNK_SPEED_TEXTURE_VRAM_DATA_HEADER
    0x33320002: "TexturePackDataArray",  # BCHUNK_SPEED_TEXTURE_VRAM_DATA_TABLE
    0x33320003: "TexturePackDataUnknown",  # BCHUNK_SPEED_TEXTURE_VRAM_DATA_TABLE_XENON
    0xB0300100: "TextureAnimPack",  # BCHUNK_PS2_TEXTURE_ANIM_PACK
    0x30300101: "TextureAnimPackHeader",  # BCHUNK_PS2_TEXTURE_ANIM_PACK_HEADER
    0x30300102: "TextureAnimEntry",  # BCHUNK_PS2_TEXTURE_ANIM_ANIMS
    0x30300103: "TextureAnimFrames",  # BCHUNK_PS2_TEXTURE_ANIM_ENTRIES
    0x30300200: "DDSTexture",  # BCHUNK_DDS_TEXTURE_DATA
    0x30300201: "DDSCubemapTexture / ColorCube",  # BCHUNK_DDS_CUBEMAPTEXTURE_DATA
    0x30300300: "PCAWeights",  # BCHUNK_PCA_WEIGHTS
    0xB0300300: "PCAWeightsPack",
    0x0003A100: "CompTPKBlock",
    0x0003BD00: "TPKSettings / XenonTexturePage",  # BCHUNK_SPEED_XENON_TEXTURE_PAGE

    # --- Cameras (ICE) -----------------------------------------------------------
    0x0003B200: "ICECatalog",
    0x8003B200: "ICENisCameras",  # BCHUNK_ICE_NIS_CAMERAS
    0x8003B201: "ICEFmvCameras",  # BCHUNK_ICE_FMV_CAMERAS
    0x8003B202: "ICEMkrCameras",  # BCHUNK_ICE_MKR_CAMERAS
    0x8003B203: "ICEReplayCameras",  # BCHUNK_ICE_REPLAY_CAMERAS
    0x8003B204: "ICEGenericCameras",  # BCHUNK_ICE_GENERIC_CAMERAS
    0x8003B209: "ICECameraShakeData",  # BCHUNK_ICE_CAMERA_SHAKE_DATA
    0x0003B210: "ICECameraGroup (ICETracks)",  # BCHUNK_ICE_CAMERA_GROUP
    0x0003B211: "ICEShakeTracks",

    # --- Frontend / language / movies --------------------------------------------
    0x00030200: "FontInfo",  # BCHUNK_FONT_INFO
    0x00030201: "FEngFont",  # BCHUNK_FENG_FONT / FONTREAL_INFO
    0x00030203: "FEngPackage (FEngFiles)",  # BCHUNK_FENG_PACKAGE
    0x00030210: "FEngCompressedPackage (FNGCompress)",  # BCHUNK_FENG_COMPRESSED_PACKAGE
    0x00030220: "PresetRides",  # BCHUNK_FE_CAR_PRESETS
    0x00030230: "FEMagazines",  # BCHUNK_FE_MAGAZINES
    0x00030231: "SCMagazines",  # BCHUNK_SC_MAGAZINES
    0x00030240: "WideDecals",
    0x00030250: "PresetSkins",  # BCHUNK_FE_SKIN_PRESETS
    0x00030300: "MenusMasterMenu",  # BCHUNK_MENUS_MASTER_MENU
    0x00039000: "Language (STRBlocks)",  # BCHUNK_LANGUAGE
    0x00039001: "LanguageHistogram",  # BCHUNK_LANGUAGE_HISTOGRAM
    0x00039010: "Subtitles",  # BCHUNK_SUBTITLES
    0x00039020: "MovieCatalogEntry",  # BCHUNK_MOVIE_CATALOG_ENTRY

    # --- Cars / parts database ---------------------------------------------------
    0x00034600: "CarTypeInfos",  # BCHUNK_SPEED_CARTYPEINFO_TABLE
    0x00034601: "CarSkinInfos",  # BCHUNK_SPEED_SKININFO_TABLE
    0x80034602: "CarPartPack (DBCarParts)",  # BCHUNK_SPEED_CARPART_PACK_HEADER
    0x00034603: "CarPartPackHeader",  # BCHUNK_SPEED_CARPART_PACK
    0x00034604: "CarPartPartsTable",  # BCHUNK_SPEED_CARPART_PARTS_TABLE
    0x00034605: "CarPartAttributesTable",  # BCHUNK_SPEED_CARPART_ATTRIBUTES_TABLE
    0x00034606: "CarPartStringTable",  # BCHUNK_SPEED_CARPART_STRING_TABLE
    0x00034607: "CarPartTypeNameTable (SlotTypes)",  # BCHUNK_SPEED_CARPART_TYPENAME_TABLE
    0x00034608: "CarPartAnimHookupTable",  # BCHUNK_SPEED_CARPART_ANIMHOOKUP_TABLE
    0x00034609: "CarPartAnimHideTable",  # BCHUNK_SPEED_CARPART_ANIMHIDE_TABLE
    0x0003460A: "CarPartModelNameHashTable",  # BCHUNK_SPEED_CARPART_MODELNAMEHASH_TABLE
    0x0003460B: "CarPartTypeNameHashTable",  # BCHUNK_SPEED_CARPART_TYPENAMEHASH_TABLE
    0x0003460C: "CarPartAttributeTableTable",  # BCHUNK_SPEED_CARPART_ATTRIBUTETABLE_TABLE
    0x0003460D: "DBCarParts_Custom",
    0x00034030: "BodyDamageInfo",  # BCHUNK_SPEED_BODY_DAMAGE_INFO

    # --- Career / performance (Underground-era IDs; none present in the MW PC files) ---
    0x80034A00: "CareerInfo",  # BCHUNK_SPEED_CAREER_INFO
    0x00034A01: "CareerUpgradeInfo",  # BCHUNK_SPEED_CAREER_UPGRADE_INFO
    0x00034A02: "CareerEventList",  # BCHUNK_SPEED_CAREER_EVENT_LIST
    0x00034A03: "CareerRankLadders",  # BCHUNK_SPEED_CAREER_RANK_LADDERS
    0x00034A07: "StyleMomentTable",  # BCHUNK_SPEED_STYLE_MOMENT_TABLE
    0x00034A08: "StyleRewards",  # BCHUNK_SPEED_STYLE_REWARDS
    0x00034A09: "PerfPackInfos",  # BCHUNK_SPEED_PERF_PACK_INFOS
    0x00034A0A: "PerfUpgradePackages",  # BCHUNK_SPEED_PERF_UPGRADE_PACKAGES
    0x80034A10: "UndergroundCareerInfo",  # BCHUNK_UNDERGROUND_CAREER_INFO
    0x00034A11: "UndergroundCareerEventTable",  # BCHUNK_UNDERGROUND_CAREER_EVENT_TABLE
    0x00034A12: "UndergroundCareerShopList",  # BCHUNK_UNDERGROUND_CAREER_SHOP_LIST
    0x00034A13: "UndergroundVisualPartUpgradeList",  # BCHUNK_UNDERGROUND_VISUAL_PART_UPGRADE_LIST
    0x00034A14: "UndergroundBrandList",  # BCHUNK_UNDERGROUND_BRAND_LIST
    0x00034A15: "UndergroundPerfPackageList",  # BCHUNK_UNDERGROUND_PERF_PACKAGE_LIST
    0x00034A16: "UndergroundShowcaseAreaList",  # BCHUNK_UNDERGROUND_SHOWCASE_AREA_LIST
    0x00034A17: "UndergroundSMSMessageList",  # BCHUNK_UNDERGROUND_SMS_MESSAGE_LIST
    0x00034A18: "UndergroundStageData",  # BCHUNK_UNDERGROUND_STAGE_DATA
    0x00034A19: "UndergroundSponsorData",  # BCHUNK_UNDERGROUND_SPONSOR_DATA
    0x00034A1A: "UndergroundCustomTuningsTable",  # BCHUNK_UNDERGROUND_CUSTOM_TUNINGS_TABLE
    0x00034A1B: "UndergroundWorldChallengeTable",  # BCHUNK_UNDERGROUND_WORLD_CHALLENGE_TABLE
    0x00034A1C: "UndergroundCareerUnlockData",  # BCHUNK_UNDERGROUND_CAREER_UNLOCK_DATA
    0x00034A1D: "UndergroundCareerStringTable",  # BCHUNK_UNDERGROUND_CAREER_STRING_TABLE
    0x00034A1E: "UndergroundCareerBankTriggers",  # BCHUNK_UNDERGROUND_CAREER_BANK_TRIGGERS
    0x00034A1F: "UndergroundCareerCarUnlockTable",  # BCHUNK_UNDERGROUND_CAREER_CAR_UNLOCK_TABLE
    0x80034A30: "PerformanceConfigTable",  # BCHUNK_PERFORMANCE_CONFIG_TABLE
    0x00034A31: "PerformanceConfig",  # BCHUNK_PERFORMANCE_CONFIG
    0x00034A32: "PerformanceExtrema",  # BCHUNK_PERFORMANCE_EXTREMA
    0x00034A33: "PerformanceTopAccelTables",  # BCHUNK_PERFORMANCE_TOPACCELTABLES
    0x00034B00: "DifficultyInfo",  # BCHUNK_SPEED_DIFFICULTY_INFO

    # --- Vector vinyls (none present in the MW PC files; MW vinyls are TPKs) -----
    0x8003CE00: "VSVinyl",  # BCHUNK_VS_VINYL
    0x0003CE01: "VSVinylMembers",  # BCHUNK_VS_VINYL_MEMBERS
    0x0003CE02: "VSVinylPathSets",  # BCHUNK_VS_VINYL_PATHSETS
    0x8003CE03: "VSPathSet",  # BCHUNK_VS_PATHSET
    0x0003CE04: "VSPathSetMembers",  # BCHUNK_VS_PATHSET_MEMBERS
    0x0003CE05: "VSPathSetPathData",  # BCHUNK_VS_PATHSET_PATHDATA
    0x0003CE06: "VSPathSetPathVectors",  # BCHUNK_VS_PATHSET_PATHVECTORS
    0x0003CE07: "VSPathSetFillEffect",  # BCHUNK_VS_PATHSET_FILLEFFECT
    0x0003CE08: "VSPathSetStrokeEffect",  # BCHUNK_VS_PATHSET_STROKEEFFECT
    0x0003CE09: "VSPathSetDropShadowEffect",  # BCHUNK_VS_PATHSET_DROPSHADOWEFFECT
    0x0003CE0A: "VSPathSetInnerGlowEffect",  # BCHUNK_VS_PATHSET_INNERGLOWEFFECT
    0x0003CE0B: "VSPathSetInnerShadowEffect",  # BCHUNK_VS_PATHSET_INNERSHADOWEFFECT
    0x0003CE0C: "VSPathSetGradientEffect",  # BCHUNK_VS_PATHSET_GRADIENTEFFECT
    0x8003CE0D: "VSAlignmentDB",  # BCHUNK_VS_ALIGNMENTDB
    0x0003CE0E: "VSAlignmentDBMembers",  # BCHUNK_VS_ALIGNMENTDB_MEMBERS
    0x0003CE0F: "VSAlignmentDBCars",  # BCHUNK_VS_ALIGNMENTDB_CARS
    0x0003CE10: "VSAlignmentDBPoints",  # BCHUNK_VS_ALIGNMENTDB_POINTS
    0x0003CE11: "VSAlignmentDBVinyls",  # BCHUNK_VS_ALIGNMENTDB_VINYLS
    0x0003CE12: "SkinRegionDB",
    0x0003CE13: "VinylMetaData",

    # --- Sound -------------------------------------------------------------------
    0x8003B500: "SndStichBundle (SoundStichs)",  # BCHUNK_SND_STICHBUNDLE
    0x0003B501: "SndStichBundleHeader",  # BCHUNK_SND_STICHBUNDLE_HEADER
    0x0003B502: "SndStichData",  # BCHUNK_SND_STICHDATA
    0x0003B503: "SndSampleRef",  # BCHUNK_SND_SAMPLEREF

    # --- Misc --------------------------------------------------------------------
    0x00038000: "AllocatedHotChunk",  # BCHUNK_ALLOCATED_HOT_CHUNK
    0x00039100: "DemoReplayInfo",  # BCHUNK_SPEED_DEMO_REPLAY_INFO
    0x00039101: "DemoReplaySnapshot",  # BCHUNK_SPEED_DEMO_REPLAY_SNAPSHOT
    0x00039102: "DemoReplayBRepeat",  # BCHUNK_SPEED_DEMO_REPLAY_BREPEAT
    0x00039200: "GJKPolyhedron",  # BCHUNK_SPEED_GJK_POLYHEDRON
    0x00039201: "GJKPolyhedronVerts",  # BCHUNK_SPEED_GJK_POLYHEDRON_VERTS
    0x00039202: "GJKPolyhedronNeighbours",  # BCHUNK_SPEED_GJK_POLYHEDRON_NEIGHBOURS
    0x0003B300: "WorldObjects",  # BCHUNK_WORLD_OBJECTS
    0x0003B400: "IRXBundleFile",  # BCHUNK_IRX_BUNDLE_FILE
    0x0003B700: "AttributeData",  # BCHUNK_ATTRIBUTE_DATA

    # --- Other IDs named in the decomp (not seen in the MW PC files) --------
    0x00015010: "FontthugInfo",  # BCHUNK_FONTTHUG_INFO
    0x00034009: "AnimationSolid",  # BCHUNK_SPEED_ANIMATION_SOLID
    0x00034010: "AnimationRotationKeys",  # BCHUNK_SPEED_ANIMATION_ROTATION_KEYS
    0x00034011: "AnimationLabelledFrames",  # BCHUNK_SPEED_ANIMATION_LABELLED_FRAMES
    0x00034013: "PositionMarkers",  # BCHUNK_SPEED_POSITION_MARKERS
    0x00034014: "AnimationTranslationKeys",  # BCHUNK_SPEED_ANIMATION_TRANSLATION_KEYS
    0x00034021: "PipelinePolyhedron",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON
    0x00034022: "PipelinePolyhedronFaces",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON_FACES
    0x00034023: "PipelinePolyhedronEdges",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON_EDGES
    0x00034024: "PipelinePolyhedronVerts",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON_VERTS
    0x00034025: "PipelinePolyhedronVoronoiPlanes",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON_VORONOI_PLANES
    0x00034060: "PcSolidTest",  # BCHUNK_SPEED_PC_SOLID_TEST
    0x00034160: "PresceneryMesh",  # BCHUNK_SPEED_PRESCENERY_MESH
    0x00034161: "Topology",  # BCHUNK_SPEED_TOPOLOGY
    0x00034162: "Heirarchy",  # BCHUNK_SPEED_HEIRARCHY
    0x00034163: "Light",  # BCHUNK_SPEED_LIGHT
    0x00034440: "CameraCarVortex",  # BCHUNK_CAMERA_CAR_VORTEX
    0x00034450: "HighlightCameraData",  # BCHUNK_HIGHLIGHT_CAMERA_DATA
    0x00034460: "PreraceCameraData",  # BCHUNK_PRERACE_CAMERA_DATA
    0x00034470: "PostraceCameraData",  # BCHUNK_POSTRACE_CAMERA_DATA
    0x00034480: "NisCameraData",  # BCHUNK_NIS_CAMERA_DATA
    0x00034490: "FlybyCameraData",  # BCHUNK_FLYBY_CAMERA_DATA
    0x00034492: "CameraShakeTable",  # BCHUNK_CAMERA_SHAKE_TABLE
    0x00034510: "TrackRadarTrap",  # BCHUNK_TRACK_RADAR_TRAP
    0x00034520: "TrackRoadblock",  # BCHUNK_TRACK_ROADBLOCK
    0x00034530: "TrackCopLandmarks",  # BCHUNK_TRACK_COP_LANDMARKS
    0x0013401B: "EsolidDamageinfoPlat",  # BCHUNK_SPEED_ESOLID_DAMAGEINFO_PLAT
    0x0013401C: "EsolidAuxiliaryLights",  # BCHUNK_SPEED_ESOLID_AUXILIARY_LIGHTS
    0x0013401D: "EsolidMorphtargets",  # BCHUNK_SPEED_ESOLID_MORPHTARGETS
    0x0013401F: "AutosculptSelectionset",  # BCHUNK_SPEED_AUTOSCULPT_SELECTIONSET
    0x00134020: "AutosculptSelectionsetEdgelist",  # BCHUNK_SPEED_AUTOSCULPT_SELECTIONSET_EDGELIST
    0x00134021: "AutosculptSelectionsetEdgelistData",  # BCHUNK_SPEED_AUTOSCULPT_SELECTIONSET_EDGELIST_DATA
    0x00134114: "EsolidMaterialTable",  # BCHUNK_SPEED_ESOLID_MATERIAL_TABLE
    0x00134214: "EsolidMaterial",  # BCHUNK_SPEED_ESOLID_MATERIAL
    0x00134314: "EsolidEmaterial",  # BCHUNK_SPEED_ESOLID_EMATERIAL
    0x00134414: "EsolidMaterialPblock",  # BCHUNK_SPEED_ESOLID_MATERIAL_PBLOCK
    0x00134514: "EsolidMaterialParams",  # BCHUNK_SPEED_ESOLID_MATERIAL_PARAMS
    0x00134614: "EsolidMaterialData",  # BCHUNK_SPEED_ESOLID_MATERIAL_DATA
    0x00134C01: "EsolidVertexStreamTable",  # BCHUNK_SPEED_ESOLID_VERTEX_STREAM_TABLE
    0x00134C03: "EsolidConnectivityData",  # BCHUNK_SPEED_ESOLID_CONNECTIVITY_DATA
    0x00134C04: "EsolidPcaFrameweights",  # BCHUNK_SPEED_ESOLID_PCA_FRAMEWEIGHTS
    0x80034008: "AnimationGroup",  # BCHUNK_SPEED_ANIMATION_GROUP
    0x80034020: "PipelinePolyhedronChunks",  # BCHUNK_SPEED_PIPELINE_POLYHEDRON_CHUNKS
    0x80034405: "HighlightCameras",  # BCHUNK_HIGHLIGHT_CAMERAS
    0x80034410: "PreraceCameras",  # BCHUNK_PRERACE_CAMERAS
    0x80034415: "PostraceCameras",  # BCHUNK_POSTRACE_CAMERAS
    0x80034420: "NisCameras",  # BCHUNK_NIS_CAMERAS
    0x80034425: "FlybyCameras",  # BCHUNK_FLYBY_CAMERAS
    0x80034500: "TrackCops",  # BCHUNK_TRACK_COPS
    0x80134014: "EsolidMaterialChunks",  # BCHUNK_SPEED_ESOLID_MATERIAL_CHUNKS

    # --- Magic-like IDs (first four bytes of non-bChunk files / wrappers) -----
    0x434B4241: "ABKC",           # SOUND/**/*.abk audio bank
    0x48434F4C: "LOCH",           # MEMCARD/*.loc
    0x4B415056: "VPAK",           # AttribSys database
    0x52494F4D: "MOIR",           # SOUND/**/*.csi
    0x53219999: "MEMO",           # GLOBAL/*MemoryFile.bin
    0x55441122: "LZCompressed",
    0x5A4C444A: "JDLZ (Compressed Chunk)",
    0x6468564D: "MVhd",           # MOVIES/*.vp6
    0x75736E47: "Gnsu",           # SOUND/ENGINE/*.gin
    0x000B5846: "FX",             # SOUND/FXEDIT/*.fx ("FX" 0B 00)
}


def chunk_name(chunk_id):
    return CHUNK_NAMES.get(chunk_id, "")
