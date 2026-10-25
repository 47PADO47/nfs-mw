//! Geometry: `docs/formats/models.md`.

pub const GEOMETRY_PACK: u32 = 0x8013_4000;
pub const MESH_CONTAINER_INFO: u32 = 0x8013_4001;
/// Pack name, solid count and file size.
pub const MESH_CONTAINER_HEADER: u32 = 0x0013_4002;
/// `(name hash, 0)` per solid, sorted by hash.
pub const MESH_CONTAINER_KEYS: u32 = 0x0013_4003;
/// `(name hash, file offset, size, size, 0, 0)` per solid, in key order.
pub const MESH_CONTAINER_OFFSETS: u32 = 0x0013_4004;
/// Empty container that closes `MeshContainerInfo`.
pub const MESH_CONTAINER_EMPTY: u32 = 0x8013_4008;
pub const SOLID_PACK: u32 = 0x8013_4010;
pub const SOLID_INFO: u32 = 0x0013_4011;
pub const SOLID_TEXTURES: u32 = 0x0013_4012;
pub const SOLID_LIGHT_MATERIALS: u32 = 0x0013_4013;
/// `ePositionMarker` records: named matrices (light, exhaust, brake positions).
pub const SOLID_MARKERS: u32 = 0x0013_401A;
pub const MESH_INFO_CONTAINER: u32 = 0x8013_4100;
pub const MESH_INFO_HEADER: u32 = 0x0013_4900;
pub const MESH_VERTEX_BUFFER: u32 = 0x0013_4B01;
pub const MESH_SHADING_GROUPS: u32 = 0x0013_4B02;
pub const MESH_INDICES: u32 = 0x0013_4B03;
/// One NUL-terminated material name per shading group (`BODY_CHROME`, `WINDOW_FRONT`, ...).
pub const MESH_VLT_MATERIALS: u32 = 0x0013_4C02;
