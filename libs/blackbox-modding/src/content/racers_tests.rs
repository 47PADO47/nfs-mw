use super::*;

#[test]
fn a_mod_without_a_racers_table_adds_no_names() {
    assert_eq!(parse_racers("[mod]\nid = \"x\"\n"), Ok(None));
}

#[test]
fn the_racers_table_takes_its_names() {
    let section = parse_racers("[racers]\nnames = [\"MAVERICK\", \"Dee Dee\"]\n").unwrap().unwrap();
    assert_eq!(section, RacersSection { names: vec!["MAVERICK".into(), "Dee Dee".into()] });
}

#[test]
fn an_empty_table_is_valid_and_adds_nothing() {
    assert_eq!(parse_racers("[racers]\n").unwrap().unwrap().names, Vec::<String>::new());
}

#[test]
fn a_bad_name_refuses_the_table() {
    let bad_names = [r#""""#, r#""   ""#, r#""bell\u0007""#, r#""ABCDEFGHIJKLMNOPQRSTUVWXYZ""#];
    for bad in bad_names {
        let text = format!("[racers]\nnames = [{bad}]\n");
        assert!(matches!(parse_racers(&text), Err(ContentError::InvalidRacerName(_))), "{bad}");
    }
}

#[test]
fn a_repeated_name_and_an_unknown_key_refuse_the_table() {
    let repeated = parse_racers("[racers]\nnames = [\"A\", \"A\"]\n");
    assert_eq!(repeated, Err(ContentError::RepeatedRacerName("A".into())));
    assert!(matches!(parse_racers("[racers]\nnamez = []\n"), Err(ContentError::Parse(_))));
}
