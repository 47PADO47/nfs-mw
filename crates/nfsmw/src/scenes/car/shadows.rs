//! Which parts of the menu backdrop cast and take the sun's shadow, and the instance keys that carry that choice
//! (`docs/specs/fe-shadows.md`, section 5). Pure: the scene applies it to its frame.

use blackbox_gfx::{Instance, InstanceKey, ShadowFlags};

use crate::settings::fe_shadows::FeShadows;

/// The bits of an instance key's group that carry the front-end shadow mode. A mode change gives the parts new keys,
/// so the renderer spawns them again with their new flags (its pool keeps an entity's flags from its first spawn).
const MODE_SHIFT: u32 = 30;

/// The kinds of part on the backdrop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The car's body and parts.
    Car,
    /// The car's placeholder floor, drawn only when the stage has no room of its own.
    CarFloor,
    /// The stage's room: its solids, floor included. It receives the car's shadow but does not cast.
    Room,
    /// Authored shadow proxy, excluded from the colour view.
    RoomCaster,
}

/// The shadow flags of a part. With front-end shadows off nothing casts or receives. The car casts and receives
/// only when `fe_shadows_cars` is on. The placeholder floor and the room receive but do not cast: the room's floor
/// would shade itself, since it has no marker to leave out (`docs/specs/fe-shadows.md`, Port status).
pub fn flags(part: Part, options: FeShadows) -> ShadowFlags {
    if !options.enabled {
        return ShadowFlags { cast: false, receive: false };
    }
    match part {
        Part::Car => ShadowFlags { cast: options.cars, receive: options.cars },
        Part::CarFloor => ShadowFlags { cast: false, receive: true },
        Part::Room => ShadowFlags { cast: false, receive: true },
        Part::RoomCaster => ShadowFlags { cast: true, receive: false },
    }
}

/// The instance with its shadow flags for `part`, and its key moved to the group of the current mode.
pub fn shadowed(instance: Instance, part: Part, options: FeShadows) -> Instance {
    let key = salted(instance.key, mode(options));
    Instance { key, ..instance }.with_shadow(flags(part, options))
}

/// The mode bits: front-end shadows on, and the cars' switch.
fn mode(options: FeShadows) -> u32 {
    u32::from(options.enabled) | (u32::from(options.cars) << 1)
}

/// A keyed instance's key with `mode` in the top bits of its group. A transient key stays transient.
fn salted(key: InstanceKey, mode: u32) -> InstanceKey {
    if key.is_transient() {
        return key;
    }
    let (group, index) = key.parts();
    InstanceKey::new(group ^ (mode << MODE_SHIFT), index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;

    const ON: FeShadows = FeShadows { enabled: true, cars: true, bias: 1.0, divider: 1 };
    const OFF: FeShadows = FeShadows { enabled: false, cars: true, bias: 1.0, divider: 1 };
    const NO_CARS: FeShadows = FeShadows { enabled: true, cars: false, bias: 1.0, divider: 1 };

    #[test]
    fn with_the_switch_off_no_part_casts_or_receives() {
        for part in [Part::Car, Part::CarFloor, Part::Room] {
            assert_eq!(flags(part, OFF), ShadowFlags { cast: false, receive: false }, "{part:?}");
        }
    }

    #[test]
    fn the_car_follows_the_cars_switch_and_the_room_only_receives() {
        assert_eq!(flags(Part::Car, ON), ShadowFlags { cast: true, receive: true });
        assert_eq!(flags(Part::Car, NO_CARS), ShadowFlags { cast: false, receive: false });
        assert_eq!(flags(Part::Room, NO_CARS), ShadowFlags { cast: false, receive: true });
    }

    #[test]
    fn the_placeholder_floor_receives_and_never_casts() {
        assert_eq!(flags(Part::CarFloor, ON), ShadowFlags { cast: false, receive: true });
        assert_eq!(flags(Part::CarFloor, NO_CARS), ShadowFlags { cast: false, receive: true });
    }

    #[test]
    fn a_part_keeps_its_mesh_and_transform_and_gets_its_flags() {
        let mesh = blackbox_gfx::MeshHandle::from_raw(7);
        let key = InstanceKey::new(3, 9);
        let base = Instance::keyed(mesh, Mat4::from_translation(glam::Vec3::X), key);
        let out = shadowed(base, Part::Car, NO_CARS);
        assert_eq!((out.mesh, out.transform), (base.mesh, base.transform));
        assert_eq!(out.shadow, ShadowFlags { cast: false, receive: false });
        assert_eq!(out.key.parts().1, 9, "the index stays with the part");
    }

    #[test]
    fn a_mode_change_moves_the_key_and_a_transient_key_stays_transient() {
        let base = Instance::keyed(blackbox_gfx::MeshHandle::from_raw(1), Mat4::IDENTITY, InstanceKey::new(3, 9));
        let on = shadowed(base, Part::Car, ON).key;
        let off = shadowed(base, Part::Car, OFF).key;
        let no_cars = shadowed(base, Part::Car, NO_CARS).key;
        assert_ne!(on, off);
        assert_ne!(on, no_cars);
        assert_eq!(on, shadowed(base, Part::Car, ON).key, "the same mode keeps the key");
        let transient = Instance::new(blackbox_gfx::MeshHandle::from_raw(1), Mat4::IDENTITY);
        assert!(shadowed(transient, Part::Room, ON).key.is_transient());
    }
}
