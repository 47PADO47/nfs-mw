//! `SolidInfo` version 0x18: NFS: Most Wanted's front-end stages (`FRONTEND/PLATFORMS/PlatformCrib.BIN`).
//! The record is the version 0x16 record with the version byte changed (checked on the crib's five solids:
//! the name hash equals `bStringHash` of the name, the name sits at 0xA0, and the counts and bounds read sane).

use super::SolidLayout;
use super::v16::V16;

pub const V18: SolidLayout = SolidLayout { version: 0x18, used_by: "NFS: Most Wanted front-end stages", ..V16 };
