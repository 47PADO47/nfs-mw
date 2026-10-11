//! The recorded digests, one directory per kind of GPU and one file per setting combination.
//!
//! The files under a family directory are generated (see `digest_tests`); `mod.rs` here and the family's
//! `mod.rs` are written by hand and list them.

mod intel_xe;

use crate::gpu::test_support::Family;

/// The recorded digest (hex) of `scene` under the `combo` settings on `family`, if there is one.
pub(super) fn expected(family: Family, combo: &str, scene: &str) -> Option<&'static str> {
    let table = match family {
        Family::IntelXeVulkanMesa => intel_xe::table(combo)?,
    };
    table.iter().find(|(name, _)| *name == scene).map(|(_, hex)| *hex)
}
