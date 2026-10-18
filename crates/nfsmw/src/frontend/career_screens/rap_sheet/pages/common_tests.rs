use super::*;

#[test]
fn a_template_takes_the_value_at_its_first_placeholder() {
    assert_eq!(fill("BOUNTY: %$d", "1,000"), "BOUNTY: 1,000");
    assert_eq!(fill("NAME: %s", "Rick"), "NAME: Rick");
    assert_eq!(fill("CASE FILE: %s", "MW-1-ABCD"), "CASE FILE: MW-1-ABCD");
}

#[test]
fn a_template_without_a_placeholder_is_returned_as_it_is() {
    assert_eq!(fill("INFRACTIONS", "5"), "INFRACTIONS");
    assert_eq!(fill("", "5"), "");
}

#[test]
fn numbers_group_by_thousands_and_saturate() {
    assert_eq!(number(0), "0");
    assert_eq!(number(999), "999");
    assert_eq!(number(1_234_567), "1,234,567");
    assert_eq!(number(u64::MAX), "4,294,967,295");
}
