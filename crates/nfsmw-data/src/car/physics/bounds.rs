//! A car's collision bounds (`GLOBAL/GlobalB.lzc`, `docs/formats/collision.md`): the root box is the
//! size the rigid body is built from.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use blackbox_attrib::vlt_hash;
use blackbox_collision::{BodyLimits, BoundsSet, Shape, find_bounds, read_bounds_sets};
use game_install::GameDir;
use glam::Vec3;

use crate::{memo, read_unwrapped, shared};

/// The size of a car's body, in physics space (x width, y up, z length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarBounds {
    /// Half dimensions of the root box, metres.
    pub half_dimensions: Vec3,
    /// Where the box's centre is in the car's own space, metres.
    pub pivot: Vec3,
}

/// Every car bounds set in the install, parsed once per session.
pub fn read_car_bounds(dir: &GameDir) -> Result<Arc<Vec<BoundsSet>>> {
    let hit = memo(dir, &[shared::GLOBALB_FILE], || {
        let file = shared::globalb(dir)?;
        let sets = read_bounds_sets(&file).map_err(|e| anyhow!("reading the car bounds: {e}"))?;
        Ok(BoundsSets(Arc::new(sets)))
    })?;
    Ok(Arc::clone(&hit.0))
}

/// The memo entry of [`read_car_bounds`]. Its own type keeps it apart from any other memo of the bounds sets.
struct BoundsSets(Arc<Vec<BoundsSet>>);

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

/// The bounds file of a mod car, relative to the install: `CARS/<FOLDER>/BOUNDS.BIN`. The folder is matched
/// case-insensitively, like every install path.
pub fn mod_bounds_file(folder: &str) -> String {
    format!("CARS/{folder}/BOUNDS.BIN")
}

/// The collision bounds of the mod car in folder `folder`, read from its own `CARS/<FOLDER>/BOUNDS.BIN` through the
/// install (mod overlays included), and nothing else: no stock car's set is consulted. See
/// `docs/modding/car-collision.md` for the file.
///
/// The file is a `BoundsPack` holding the car's set, keyed by the hash of the folder name in upper case. A file with
/// exactly one set is accepted under any key, so a hand-made file needs no hash. The set must be a vehicle body
/// ([`BoundsSet::check_vehicle_body`]: a box root of a plausible size, a valid tree, point clouds for its ground
/// meshes). Every failure names the file and says what is wrong, so a car that cannot be driven can be told why.
///
/// Returns the root box ([`CarBounds`], the size and place the rigid body is built from) and the whole set (for
/// [`super::CarGeometry`]). Read once per session while the file is unchanged.
pub fn mod_car_bounds(dir: &GameDir, folder: &str) -> Result<(CarBounds, BoundsSet)> {
    let file = mod_bounds_file(folder);
    let hit = memo(dir, &[file.as_str()], || {
        let bytes = read_unwrapped(dir, &file).with_context(|| format!("{folder} has no collision bounds"))?;
        let set = pick_set(&bytes, folder).with_context(|| format!("reading {file}"))?;
        Ok(ModBounds(Arc::new(set)))
    })?;
    let set = (*hit.0).clone();
    Ok((
        CarBounds { half_dimensions: Vec3::from(set.root().half_dimensions), pivot: Vec3::from(set.root().pivot) },
        set,
    ))
}

/// The memo entry of [`mod_car_bounds`]; its own type keeps it apart from the other memos of a bounds set.
struct ModBounds(Arc<BoundsSet>);

fn pick_set(bytes: &[u8], folder: &str) -> Result<BoundsSet> {
    let sets = read_bounds_sets(bytes).map_err(|e| anyhow!("{e}"))?;
    let set = match find_bounds(&sets, vlt_hash(&folder.to_ascii_uppercase())) {
        Some(set) => set,
        None if sets.len() == 1 => &sets[0],
        None if sets.is_empty() => bail!("the file holds no bounds set"),
        None => bail!("the file holds {} bounds sets and none is keyed {}", sets.len(), folder.to_ascii_uppercase()),
    };
    set.check_vehicle_body(&BodyLimits::default()).map_err(|e| anyhow!("{e}"))?;
    Ok(set.clone())
}
