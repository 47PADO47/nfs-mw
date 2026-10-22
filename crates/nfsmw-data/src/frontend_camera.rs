//! The front end's camera poses: the `angle*` collections of the `frontend` class in `GLOBAL/FE_ATTRIB.bin`, with the
//! numbers that shape the move onto each. Spec: `docs/specs/menu-backdrop-looks.md` section 4.2 and the provenance file,
//! section 3.

use anyhow::{Context, Result, anyhow};
use blackbox_attrib::{CollectionRef, Database, Value};
use game_install::GameDir;

use crate::career_cars::FE_ATTRIB_FILE;
use crate::read_unwrapped;
use crate::shared;

/// The class holding the poses (and the screens that name them).
const CLASS: &str = "frontend";

/// Field keys of the `frontend` class (name hashes), with the layout offset the mover reads them from.
const DECAY: u32 = 0x2581_B8BA; // 0x1C
const DURATION: u32 = 0xF624_544D; // 0x20
const FOV: u32 = 0xC220_D5AB; // 0x24
const TARGET: [u32; 3] = [0xA8D9_A1F4, 0xFE9B_5900, 0x7C9B_89D3]; // 0x2C, 0x30, 0x34
const PITCH: u32 = 0xC8F9_6E5A; // 0x3C
const ROLL: u32 = 0x05B5_6526; // 0x44
const ORDER: u32 = 0x7986_24CA; // 0x48
const DISTANCE: u32 = 0xA10C_312C; // 0x50
const YAW: u32 = 0x803F_A14C; // 0x54

/// The field of a screen collection that refers to its pose collection (layout offset 0).
const POSE_REFERENCE: u32 = 0x28B5_440B;
/// The screen collection a name with no collection of its own falls back to.
const DEFAULT_SCREEN: &str = "default";

/// One camera pose and the shape of the move that ends on it. Angles are degrees, lengths metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraAngle {
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub roll: f32,
    pub fov: f32,
    pub target: [f32; 3],
    /// The move's decay `k` (`[0x1C]`): how fast the overshoot dies away.
    pub decay: f32,
    /// Seconds the move takes (`[0x20]`); zero or less jumps straight onto the pose.
    pub duration: f32,
    /// The move's order `n` (`[0x48]`): how many times it swings past the pose before it settles.
    pub order: i32,
}

/// The `frontend` attributes of `GLOBAL/FE_ATTRIB.bin`, read for their camera poses.
pub struct PoseBook {
    db: Database,
}

impl PoseBook {
    /// Reads the install's `GLOBAL/FE_ATTRIB.bin` over the session's class definitions.
    pub fn load(dir: &GameDir) -> Result<Self> {
        // The session's class definitions are shared; the collections go into this book's own copy.
        let attributes = shared::attributes(dir)?;
        let mut db = Database::clone(&attributes);
        let fe_attrib = read_unwrapped(dir, FE_ATTRIB_FILE)?;
        db.load(&fe_attrib).with_context(|| format!("parsing {FE_ATTRIB_FILE}"))?;
        Ok(Self { db })
    }

    /// The pose in the collection called `name` (`angle_final_main_fe`), or `None` when it is missing or lacks a field.
    pub fn angle(&self, name: &str) -> Option<CameraAngle> {
        angle(&self.db, name)
    }

    /// The pose the screen collection called `name` (`mainmenu`, `career_manager`) refers to, which may be inherited from the
    /// collection's parents. A name with no collection gets the `default` collection's pose, as the original does.
    pub fn screen(&self, name: &str) -> Option<CameraAngle> {
        let screen = self.db.collection(CLASS, name).or_else(|| self.db.collection(CLASS, DEFAULT_SCREEN))?;
        let pose = self.db.resolve(screen.get_by_key(POSE_REFERENCE)?.as_ref_spec()?)?;
        read(pose)
    }
}

/// The pose in the `frontend` collection called `name` of `db`, or `None` when it is missing or lacks a field.
pub fn angle(db: &Database, name: &str) -> Option<CameraAngle> {
    read(db.collection(CLASS, name)?)
}

/// The poses called `names`, in that order, read from the install's `GLOBAL/FE_ATTRIB.bin`. A missing pose is an error.
pub fn load(dir: &GameDir, names: &[&str]) -> Result<Vec<CameraAngle>> {
    let book = PoseBook::load(dir)?;
    names
        .iter()
        .map(|name| book.angle(name).ok_or_else(|| anyhow!("{FE_ATTRIB_FILE} has no camera pose {name}")))
        .collect()
}

fn read(c: CollectionRef<'_>) -> Option<CameraAngle> {
    let float = |key: u32| c.get_by_key(key).and_then(Value::as_f32);
    Some(CameraAngle {
        pitch: float(PITCH)?,
        yaw: float(YAW)?,
        distance: float(DISTANCE)?,
        roll: float(ROLL)?,
        fov: float(FOV)?,
        target: [float(TARGET[0])?, float(TARGET[1])?, float(TARGET[2])?],
        decay: float(DECAY)?,
        duration: float(DURATION)?,
        order: c.get_by_key(ORDER).and_then(Value::as_i32)?,
    })
}
