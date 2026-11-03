use super::*;

#[test]
fn batch12_pause_safe_house_is_contextual_and_keeps_its_disabled_state() {
    for state in [None, Some(false), Some(true)] {
        let row = pause_icons(PauseCtx { safe_house: state, ..Default::default() });
        let homes: Vec<_> = row.iter().filter(|icon| icon.id == Id::JumpToSafeHouse).collect();
        assert_eq!(homes.len(), usize::from(state.is_some()));
        if let Some(enabled) = state {
            assert_eq!(homes[0].enabled, enabled);
            assert_eq!((homes[0].texture, homes[0].name), (QUIT_TEXTURE, 0x3C14_C420));
        }
    }
}
