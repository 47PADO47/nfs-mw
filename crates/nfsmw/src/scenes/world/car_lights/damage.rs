//! Original marker-to-VehicleFX identities, independent of panel damage zones.

use blackbox_hash::bstring_hash;

pub(super) fn marker_flags(marker: u32) -> u32 {
    [
        ("LEFT_HEADLIGHT", 1),
        ("RIGHT_HEADLIGHT", 2),
        ("CENTRE_HEADLIGHT", 4),
        ("LEFT_BRAKELIGHT", 8),
        ("RIGHT_BRAKELIGHT", 16),
        ("CENTRE_BRAKELIGHT", 32),
        ("BRAKELIGHT_CENTRE", 32),
        ("LEFT_REVERSE", 64),
        ("RIGHT_REVERSE", 128),
        ("COPLIGHTRED", 4096),
        ("COPLIGHTBRIGHTRED", 4096),
        ("COPLIGHTBLUE", 8192),
        ("COPLIGHTBRIGHTBLUE", 8192),
        ("COPLIGHTWHITE", 16384),
    ]
    .into_iter()
    .find_map(|(name, bit)| (bstring_hash(name) == marker).then_some(bit))
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_broken_brake_bits_disable_the_shared_surface_without_changing_head_admission() {
        use crate::scenes::world::car_lights::LampState;
        let state = LampState { headlights: true, brake: 1.0, ..Default::default() };
        assert!(state.brake_surfaces_on());
        assert!(LampState { broken: 8, ..state }.brake_surfaces_on());
        let destroyed = LampState { broken: u32::MAX, ..state };
        assert!(!destroyed.brake_surfaces_on());
        assert!(destroyed.headlights);
        assert!(!LampState { brake: 0.0, ..state }.brake_surfaces_on());
    }

    #[test]
    fn opposite_sides_remain_independent_and_bright_police_markers_share_color_flags() {
        assert_eq!(marker_flags(bstring_hash("LEFT_HEADLIGHT")), 1);
        assert_eq!(marker_flags(bstring_hash("RIGHT_HEADLIGHT")), 2);
        assert_eq!(marker_flags(bstring_hash("BRAKELIGHT_CENTRE")), 32);
        assert_eq!(marker_flags(bstring_hash("COPLIGHTBRIGHTRED")), marker_flags(bstring_hash("COPLIGHTRED")));
        assert_eq!(marker_flags(bstring_hash("COPLIGHTBRIGHTBLUE")), marker_flags(bstring_hash("COPLIGHTBLUE")));
        assert_eq!(marker_flags(bstring_hash("unknown")), 0);
    }
}
