//! The `AnimScene` container of a NIS bundle and its header: the scene's name, type, frame range, overlay and
//! description, and its character records. Layout: `docs/specs/nis-scenes.md`, sections 2 and 3.

use crate::bytes::{cstr_at, fixed_text, i32_at, u32_at};
use crate::{Character, Error, Result, SceneType};

/// `AnimScene (NisScene)`: the container that holds one scene.
pub const ANIM_SCENE: u32 = 0x8003_7020;
/// `AnimSceneHeaderData`: the scene header (the 64-byte fixed part and the description).
pub const SCENE_HEADER: u32 = 0x0003_7030;
/// `AnimSceneEntityData`: one character record.
pub const SCENE_ENTITY: u32 = 0x0003_7040;

/// Length of the fixed part of the header. The description text starts at this offset.
const FIXED_LEN: usize = 0x40;
/// Width of the name and overlay fields in the header.
const NAME_LEN: usize = 16;
const WHAT: &str = "scene header";

/// One scene: its header and its character records.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Hash of the scene name (`bStringHash`, case-sensitive).
    pub name_hash: u32,
    /// The scene name, from its 16-byte field.
    pub name: String,
    /// The `Description` pointer field. Zero in the files; the engine points it at the description text.
    pub description_pointer: u32,
    /// The placement and car rules of the scene.
    pub scene_type: SceneType,
    /// The `HaveLayout` flag.
    pub have_layout: bool,
    /// The `HaveCarAnimation` flag: the scene drives cars from animation.
    pub have_car_animation: bool,
    /// `NumberOfCars`: the number of car channels the scene uses.
    pub number_of_cars: i32,
    /// `StartFrame`: the first frame that plays. Frames run at 30 per second in the files.
    pub start_frame: i32,
    /// `VanishFrame`: the last frame. The scene's length is this frame count over the frame rate.
    pub vanish_frame: i32,
    /// The overlay drawn during the scene (`SeeulatorOverlayName`). Empty when there is none.
    pub overlay_name: String,
    /// The description text after the fixed part of the header.
    pub description: String,
    /// The character records, in file order.
    pub characters: Vec<Character>,
}

impl Scene {
    /// Reads a scene from the payload of its `AnimScene` container. The header chunk and the character chunks are
    /// the container's children; other children (padding, event data) are ignored. A container with no header, or
    /// with two, is an error.
    pub fn read(container: &[u8]) -> Result<Self> {
        let mut header = None;
        let mut characters = Vec::new();
        for child in blackbox_chunk::chunks(container) {
            let child = child?;
            match child.id {
                SCENE_HEADER => {
                    if header.is_some() {
                        return Err(Error::Invalid("the scene has two header chunks".into()));
                    }
                    header = Some(child.payload);
                }
                SCENE_ENTITY => characters.push(Character::read(child.payload)?),
                _ => {}
            }
        }
        let Some(header) = header else {
            return Err(Error::Invalid("the scene has no header chunk".into()));
        };
        Self::from_header(header, characters)
    }

    fn from_header(data: &[u8], characters: Vec<Character>) -> Result<Self> {
        if data.len() < FIXED_LEN {
            return Err(Error::Truncated { what: WHAT, offset: data.len() });
        }
        let raw_type = i32_at(data, 0x18, WHAT)?;
        let scene_type = SceneType::from_raw(raw_type)
            .ok_or_else(|| Error::Invalid(format!("scene type {raw_type} is not defined")))?;
        Ok(Self {
            name_hash: u32_at(data, 0x00, WHAT)?,
            name: fixed_text(data, 0x04, NAME_LEN, WHAT)?,
            description_pointer: u32_at(data, 0x14, WHAT)?,
            scene_type,
            have_layout: flag(i32_at(data, 0x1C, WHAT)?, "HaveLayout")?,
            have_car_animation: flag(i32_at(data, 0x20, WHAT)?, "HaveCarAnimation")?,
            number_of_cars: i32_at(data, 0x24, WHAT)?,
            start_frame: i32_at(data, 0x28, WHAT)?,
            vanish_frame: i32_at(data, 0x2C, WHAT)?,
            overlay_name: fixed_text(data, 0x30, NAME_LEN, WHAT)?,
            description: cstr_at(data, FIXED_LEN, "scene description")?,
            characters,
        })
    }
}

/// Finds the `AnimScene` container anywhere in a bundle (depth-first) and reads its scene. `Ok(None)` when the bundle
/// has no scene container.
pub fn find_scene(bundle: &[u8]) -> Result<Option<Scene>> {
    let Some(container) = blackbox_chunk::find(bundle, ANIM_SCENE) else {
        return Ok(None);
    };
    Scene::read(container.payload).map(Some)
}

/// A header flag. The format stores 0 or 1; any other value is an error rather than a guess.
fn flag(value: i32, field: &str) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(Error::Invalid(format!("{field} is {other}, not 0 or 1"))),
    }
}
