use super::*;

#[test]
fn the_manager_lists_the_alias_rows_then_the_career_rows() {
    let titles: Vec<&str> = AliasSetting::ALL.iter().map(|s| s.title()).collect();
    assert_eq!(titles, ["Save Alias", "Load Alias", "Load Career", "Delete Career", "New Career"]);
}

#[test]
fn every_row_says_what_a_press_does() {
    let data: Vec<&str> = AliasSetting::ALL.iter().map(|s| s.data()).collect();
    assert_eq!(data, ["Save", "Load", "Load", "Delete", "New"]);
}

#[test]
fn save_and_load_alias_send_their_commands() {
    assert_eq!(AliasSetting::Save.commands(), vec![Command::SaveAlias]);
    assert_eq!(AliasSetting::Load.commands(), vec![Command::LoadAlias]);
}

#[test]
fn load_career_pushes_the_profile_list() {
    let commands = AliasSetting::LoadCareer.commands();
    assert!(matches!(commands.as_slice(), [Command::Push(name, _)] if name.as_str() == screen::MC_LIST));
}

#[test]
fn delete_career_pushes_the_profile_list() {
    let commands = AliasSetting::DeleteCareer.commands();
    assert!(matches!(commands.as_slice(), [Command::Push(name, _)] if name.as_str() == screen::MC_LIST));
}

#[test]
fn new_career_takes_the_first_free_name() {
    assert_eq!(new_career(&[]), Command::CareerNew("PLAYER".to_owned()));
    assert_eq!(new_career(&["PLAYER".to_owned()]), Command::CareerNew("PLAYER2".to_owned()));
}
