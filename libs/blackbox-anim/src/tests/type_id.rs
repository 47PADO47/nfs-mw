use crate::TypeId;

#[test]
fn every_id_from_0_to_25_maps_to_its_own_variant() {
    for value in 0..=25u16 {
        let id = TypeId::from_u16(value).unwrap_or_else(|| panic!("id {value} has no variant"));
        assert_eq!(id as u16, value, "{id:?} must keep its number");
    }
    assert_eq!(TypeId::from_u16(10), Some(TypeId::DeltaLerp));
    assert_eq!(TypeId::from_u16(15), Some(TypeId::Compound));
    assert_eq!(TypeId::from_u16(25), Some(TypeId::PoseAnim));
}

#[test]
fn ids_past_25_are_unknown() {
    assert_eq!(TypeId::from_u16(26), None);
    assert_eq!(TypeId::from_u16(u16::MAX), None);
}

#[test]
fn the_five_delta_leaves_are_supported() {
    let supported: Vec<u16> =
        (0..=25u16).filter_map(TypeId::from_u16).filter(|id| id.is_supported_leaf()).map(|id| id as u16).collect();
    assert_eq!(supported, [10, 11, 17, 20, 21]);
}
