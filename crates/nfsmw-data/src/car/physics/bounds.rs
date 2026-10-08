//! A car's collision bounds (`GLOBAL/GlobalB.lzc`, `docs/formats/collision.md`): the root box is the
//! size the rigid body is built from.

use anyhow::{Context, Result, anyhow};
use blackbox_attrib::vlt_hash;
use blackbox_collision::{BoundsSet, Shape, find_bounds, read_bounds_sets};
use game_install::GameDir;
use glam::Vec3;

use crate::read_unwrapped;

/// The size of a car's body, in physics space (x width, y up, z length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarBounds {
    /// Half dimensions of the root box, metres.
    pub half_dimensions: Vec3,
    /// Where the box's centre is in the car's own space, metres.
    pub pivot: Vec3,
}

/// Every car bounds set in the install.
pub fn read_car_bounds(dir: &GameDir) -> Result<Vec<BoundsSet>> {
    let file = read_unwrapped(dir, "GLOBAL/GLOBALB.LZC")?;
    read_bounds_sets(&file).map_err(|e| anyhow!("reading the car bounds: {e}"))
}

/// The bounds of car type `type_name` (e.g. `BMWM3GTR`): its root box.
pub fn car_bounds(sets: &[BoundsSet], type_name: &str) -> Result<CarBounds> {
    let set = find_bounds(sets, vlt_hash(&type_name.to_ascii_uppercase()))
        .with_context(|| format!("{type_name} has no collision bounds"))?;
    let root = set.root();
    if root.shape() != Shape::Box {
        anyhow::bail!("{type_name}: the root bounds is not a box");
    }
    Ok(CarBounds { half_dimensions: Vec3::from(root.half_dimensions), pivot: Vec3::from(root.pivot) })
}
