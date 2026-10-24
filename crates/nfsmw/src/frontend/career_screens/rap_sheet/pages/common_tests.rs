use super::*;

#[test]
fn a_template_takes_the_value_at_its_first_placeholder() {
    assert_eq!(value_text(Some("BOUNTY: %$d"), "1,000"), "BOUNTY: 1,000");
    assert_eq!(value_text(Some("NAME: %s"), "Rick"), "NAME: Rick");
    assert_eq!(value_text(Some("CASE FILE: %s"), "MW-1-ABCD"), "CASE FILE: MW-1-ABCD");
}

#[test]
fn a_template_without_a_placeholder_is_returned_as_it_is() {
    assert_eq!(value_text(Some("INFRACTIONS"), "5"), "INFRACTIONS");
    assert_eq!(value_text(Some(""), "5"), "");
}

#[test]
fn without_the_template_the_value_is_shown_alone() {
    assert_eq!(value_text(None, "5"), "5");
}

#[test]
fn numbers_group_by_thousands_and_saturate() {
    assert_eq!(number(0), "0");
    assert_eq!(number(999), "999");
    assert_eq!(number(1_234_567), "1,234,567");
    assert_eq!(number(u64::MAX), "4,294,967,295");
}
