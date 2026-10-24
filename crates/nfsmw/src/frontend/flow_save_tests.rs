use anyhow::anyhow;

use super::*;

#[test]
fn a_save_says_the_alias_it_wrote() {
    assert_eq!(saved_message("PLAYER", &Ok(())).show(), "PLAYER saved.");
}

#[test]
fn a_failed_save_says_so_and_leaves_the_reason_to_the_log() {
    let message = saved_message("PLAYER", &Err(anyhow!("disk full"))).show();
    assert_eq!(message, "The career could not be saved.");
    assert!(!message.contains("disk full"), "the error chain is not shown to the player");
}

#[test]
fn a_delete_says_the_alias_it_removed() {
    assert_eq!(deleted_message("MWO", &Ok(())).show(), "MWO deleted.");
}

#[test]
fn a_refused_load_names_the_profile() {
    let message = loaded_failed("NAME").show();
    assert_eq!(message, "The career NAME could not be loaded.");
}
