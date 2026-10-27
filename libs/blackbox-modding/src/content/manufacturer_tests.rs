use super::ContentError;
use super::manufacturer::{Manufacturer, brand_key, parse};

fn manufacturer(text: &str) -> Result<Manufacturer, ContentError> {
    parse("bentley", text)
}

#[test]
fn a_manufacturer_has_a_name_a_logo_and_defaults_to_no_override() {
    let found = manufacturer("name = \"Bentley\"\nlogo = \"UI/MAKERS/BENTLEY.PNG\"\n").unwrap();
    assert_eq!(found, Manufacturer { name: "Bentley".into(), logo: "UI/MAKERS/BENTLEY.PNG".into(), overriding: false });
    assert!(manufacturer("name = \"Subaru\"\nlogo = \"a.png\"\noverride = true\n").unwrap().overriding);
}

#[test]
fn a_bad_name_logo_id_or_key_is_refused() {
    assert!(manufacturer("name = \"\"\nlogo = \"a.png\"\n").is_err());
    assert!(manufacturer("name = \"X\"\nlogo = \"a.jpg\"\n").is_err());
    assert!(manufacturer("name = \"X\"\nlogo = \"../a.png\"\n").is_err());
    assert!(manufacturer("name = \"X\"\nlogo = \"/a.png\"\n").is_err());
    assert!(manufacturer("name = \"X\"\nlogo = \"C:/a.png\"\n").is_err());
    assert!(manufacturer("name = \"X\"\nlogo = \"a.png\"\nextra = 1\n").is_err());
    assert!(parse("bad id", "name = \"X\"\nlogo = \"a.png\"\n").is_err());
}

#[test]
fn brands_are_compared_without_case_spaces_or_punctuation() {
    assert_eq!(brand_key("Aston Martin"), brand_key("ASTON_MARTIN"));
    assert_eq!(brand_key("Mercedes-Benz"), "MERCEDESBENZ");
}
