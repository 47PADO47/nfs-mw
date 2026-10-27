use super::super::{CarManifest, parse_car};
use super::*;

const GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789";

fn car(plate: &str) -> Result<CarManifest, ContentError> {
    parse_car(
        "fastcoupe",
        &format!("[customization]\npaints = [{{ name = \"Red\", color = [1, 2, 3] }}]\n[customization.plate]\n{plate}"),
    )
}

fn table(extra: &str) -> String {
    format!("picture = \"FASTCOUPE_PLATE\"\nglyphs = \"{GLYPHS}\"\ncolumns = 9\nrows = 4\nslots = 8\n{extra}")
}

fn plate(extra: &str) -> Plate {
    car(&table(extra)).unwrap().customization.unwrap().plate.unwrap()
}

#[test]
fn a_car_has_no_plate_unless_it_gives_the_table() {
    let text = "[customization]\npaints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    assert_eq!(parse_car("fastcoupe", text).unwrap().customization.unwrap().plate, None);
}

#[test]
fn the_table_is_read() {
    let found = plate("text = \"NEED4SPD\"\naliases = { \"0\" = \"O\" }\n");
    assert_eq!(found.text.as_deref(), Some("NEED4SPD"));
    assert_eq!(found.picture, "FASTCOUPE_PLATE");
    assert_eq!((found.columns, found.rows, found.slots), (9, 4, 8));
    assert_eq!(found.glyphs.len(), 36);
    assert_eq!(found.aliases, vec![('0', 'O')]);
}

#[test]
fn the_text_is_optional_and_the_sheet_shows_as_authored_without_it() {
    assert_eq!(plate("").text, None);
}

#[test]
fn the_glyphs_must_fill_the_sheet_once_each_and_include_the_blank() {
    let wrong_count = table("").replace("columns = 9", "columns = 8");
    assert!(car(&wrong_count).is_err());
    let twice = table("").replace("ABC", "ABA");
    assert!(car(&twice).is_err());
    let no_blank = table("").replace("Z 1", "Z-1");
    assert!(car(&no_blank).is_err());
    let unprintable = table("").replace('B', "\\u00e9");
    assert!(car(&unprintable).is_err());
}

#[test]
fn the_slots_and_the_text_are_limited() {
    assert!(car(&table("").replace("slots = 8", "slots = 0")).is_err());
    assert!(car(&table("").replace("slots = 8", "slots = 17")).is_err());
    assert!(car(&table("text = \"NINELONGS\"\n")).is_err());
    assert!(car(&table("text = \"EIGHT888\"\n")).is_ok());
}

#[test]
fn an_alias_is_one_character_drawn_as_a_glyph_it_is_not_itself() {
    assert!(car(&table("aliases = { \"0\" = \"OO\" }\n")).is_err());
    assert!(car(&table("aliases = { \"0\" = \"~\" }\n")).is_err(), "drawn as a glyph the sheet lacks");
    assert!(car(&table("aliases = { \"A\" = \"B\" }\n")).is_err(), "A is a glyph already");
    assert!(car(&table("aliases = { \"00\" = \"O\" }\n")).is_err());
}

#[test]
fn the_picture_is_a_texture_name_and_a_key_the_table_does_not_know_refuses_the_car() {
    assert!(car(&table("").replace("FASTCOUPE_PLATE", "bad name")).is_err());
    assert!(car(&table("colour = 1\n")).is_err());
}
