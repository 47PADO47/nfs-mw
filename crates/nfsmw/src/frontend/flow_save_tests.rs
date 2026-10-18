use anyhow::anyhow;

use super::*;

#[test]
fn a_save_says_the_alias_it_wrote() {
    assert_eq!(saved_message("PLAYER", &Ok(())), "PLAYER saved.");
}

#[test]
fn a_failed_save_says_why() {
    let message = saved_message("PLAYER", &Err(anyhow!("disk full")));
    assert!(message.starts_with("The career could not be saved."));
    assert!(message.contains("disk full"));
}

#[test]
fn a_delete_says_the_alias_it_removed() {
    assert_eq!(deleted_message("MWO", &Ok(())), "MWO deleted.");
}

#[test]
fn a_refused_load_names_the_profile_and_the_reason() {
    let message = loaded_failed("NAME", &anyhow!("profile NAME refused: bad magic"));
    assert!(message.contains("NAME"));
    assert!(message.contains("bad magic"));
}
