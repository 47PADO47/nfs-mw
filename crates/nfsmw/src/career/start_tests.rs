use super::{Choice, choose};

#[test]
fn no_profile_starts_a_new_career_on_the_default_alias() {
    assert_eq!(choose(None), Choice::New("PLAYER".to_owned()));
}

#[test]
fn the_latest_profile_is_loaded() {
    assert_eq!(choose(Some("Max")), Choice::Load("Max".to_owned()));
}

#[test]
fn an_empty_alias_is_still_loaded_not_replaced() {
    assert_eq!(choose(Some("")), Choice::Load(String::new()));
}
