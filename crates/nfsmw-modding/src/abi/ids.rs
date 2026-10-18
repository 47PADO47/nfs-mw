//! Interface ids, versions and the limits the host checks (catalogue: docs/modding/api-nfsmw.md).
//!
//! The ids of `nfsmw.*` belong to this crate (ADR 0005 section 1). Versions are `(major, minor)`, as the `bb.*`
//! interfaces have them. This is version 0.1: nothing is promised across minors yet (ADR 0005 section 7).

/// Id of the pursuit interface: heat, the cop tables and the chase.
pub const IFACE_PURSUIT: &str = "nfsmw.pursuit";
/// The version the host implements for [`IFACE_PURSUIT`], as `(major, minor)`.
pub const IFACE_PURSUIT_VERSION: (u16, u16) = (0, 1);

/// Id of the career interface: cash, bounty, rap sheet, blacklist, milestones, garage, safe houses and shops.
pub const IFACE_CAREER: &str = "nfsmw.career";
/// The version the host implements for [`IFACE_CAREER`], as `(major, minor)`.
pub const IFACE_CAREER_VERSION: (u16, u16) = (0, 2);

/// Id of the race interface: the race list, starts and custom races.
pub const IFACE_RACES: &str = "nfsmw.races";
/// The version the host implements for [`IFACE_RACES`], as `(major, minor)`.
pub const IFACE_RACES_VERSION: (u16, u16) = (0, 1);

/// Heat levels. The integer part of a heat value picks the cop wave; a level is `1..=MW_HEAT_LEVELS`.
pub const MW_HEAT_LEVELS: u32 = 10;
/// Most cop records one wave row may hold. The host never reads more than this from a mod.
pub const MW_MAX_COPS_PER_ROW: u32 = 64;
/// Most checkpoints one custom race may hold. The host never reads more than this from a mod.
pub const MW_MAX_CHECKPOINTS: u32 = 256;
/// The first event kind of this crate. The range `MW_EVENT_RANGE_START..MW_EVENT_RANGE_START + 0x1000` is `nfsmw.*`.
pub const MW_EVENT_RANGE_START: u32 = 0x1000;
