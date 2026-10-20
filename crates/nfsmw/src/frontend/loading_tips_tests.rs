use super::*;
use crate::ui::tips::TIPS;

#[test]
fn consecutive_loads_show_different_tips() {
    let first = next_key();
    let second = next_key();
    assert_ne!(first, second);
    assert!(TIPS.contains(&first) && TIPS.contains(&second));
}

#[test]
fn the_screen_is_made_for_the_tips_package_only() {
    use crate::frontend::factory::slot_for;
    use crate::frontend::ids::screen::{LOADING, LOADING_TIPS};
    use crate::frontend::logic::Args;
    // Neither loading screen is a menu slot.
    assert!(slot_for(LOADING_TIPS, &Args::default()).is_none());
    assert!(slot_for(LOADING, &Args::default()).is_none());
    assert_eq!(LoadingTips::with_key("BOUNTY").key, "BOUNTY");
}
