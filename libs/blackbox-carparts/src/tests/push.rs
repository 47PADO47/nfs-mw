use blackbox_hash::bstring_hash;

use super::builder;
use crate::layout::MOST_WANTED;
use crate::{Error, NewPart, PartQuery, PartsDb, read_parts_db};

fn db() -> PartsDb {
    read_parts_db(&builder::parts_pack(), &MOST_WANTED).unwrap()
}

fn body<'a>(models: &'a [u32], attributes: &'a [(u32, u32)]) -> NewPart<'a> {
    NewPart {
        type_name_hash: bstring_hash("MODCAR"),
        part_id: 23,
        name: "MODCAR_BODY",
        upgrade_level: 0,
        models,
        attributes,
    }
}

#[test]
fn a_pushed_part_reads_back_like_a_stored_one() {
    let mut db = db();
    let before = db.parts().len();
    let solids =
        [bstring_hash("MODCAR_KIT00_BODY_A"), bstring_hash("MODCAR_KIT00_BODY_B"), u32::MAX, u32::MAX, u32::MAX];
    let kit = (bstring_hash("KITNUMBER"), 0);
    let pushed = db.push_part(&body(&solids, &[kit, (bstring_hash("NAME"), 7)])).unwrap();

    assert_eq!(db.parts().len(), before + 1);
    assert_eq!(db.parts().last(), Some(&pushed));
    let found = db.find(&PartQuery::new(23, bstring_hash("MODCAR")).upgrade_level(0)).unwrap();
    assert_eq!(found, &pushed);
    assert_eq!(db.name(found), "MODCAR_BODY");
    assert_eq!(db.by_name_hash(bstring_hash("MODCAR_BODY")), Some(&pushed));
    assert_eq!(db.type_name_hash(found), bstring_hash("MODCAR"));
    assert_eq!(db.model_hash(found, 0), Some(solids[0]));
    assert_eq!(db.model_hash(found, 1), Some(solids[1]));
    assert_eq!(db.model_hash(found, 2), None);
    assert_eq!(db.model_hash(found, 4), None);
    assert_eq!(db.attribute(found, "KITNUMBER"), Some(0));
    assert_eq!(db.attribute(found, "NAME"), Some(7));
}

#[test]
fn the_stock_parts_are_untouched_and_found_first() {
    let mut db = db();
    let stock: Vec<_> = db.parts().to_vec();
    let car = bstring_hash("CAR");
    let stock_body = *db.find(&PartQuery::new(23, car).upgrade_level(0)).unwrap();
    let mut other = body(&[1], &[]);
    other.type_name_hash = car;
    other.name = "CAR_BODY_EXTRA";
    db.push_part(&other).unwrap();
    assert_eq!(&db.parts()[..stock.len()], &stock[..]);
    assert_eq!(db.find(&PartQuery::new(23, car).upgrade_level(0)), Some(&stock_body));
    assert_eq!(db.name(&stock_body), "BODY_00");
    assert_eq!(db.model_hash(&stock_body, 0), Some(bstring_hash("CAR_KIT00_BODY_A")));
    assert_eq!(db.attribute(&stock_body, "KITNUMBER"), Some(0));
    assert_eq!(db.type_names().len(), 2, "an existing type name is reused");
}

#[test]
fn attributes_are_shared_and_a_part_without_any_has_none() {
    let mut db = db();
    let known = db.attributes.len();
    let kit = (bstring_hash("KITNUMBER"), 0);
    let first = db.push_part(&body(&[1], &[kit])).unwrap();
    assert_eq!(db.attributes.len(), known, "the stock pack already has this pair");
    let mut second = body(&[2], &[]);
    second.name = "MODCAR_WHEEL";
    second.part_id = 67;
    let second = db.push_part(&second).unwrap();
    assert_eq!(db.attributes(&first).collect::<Vec<_>>(), [kit]);
    assert_eq!(db.attributes(&second).count(), 0);
    assert_eq!(db.attribute(&second, "KITNUMBER"), None);
}

#[test]
fn strings_stay_aligned_and_readable() {
    let mut db = db();
    let mut a = body(&[1], &[]);
    a.name = "A";
    let mut b = body(&[1], &[]);
    b.name = "LONGER_NAME_HERE";
    let (a, b) = (db.push_part(&a).unwrap(), db.push_part(&b).unwrap());
    assert_eq!((db.name(&a), db.name(&b)), ("A", "LONGER_NAME_HERE"));
    assert_eq!(db.strings.len() % 4, 0);
}

#[test]
fn a_level_beyond_the_three_bits_is_refused() {
    let mut db = db();
    let mut new = body(&[1], &[]);
    new.upgrade_level = 8;
    assert!(matches!(db.push_part(&new), Err(Error::OutOfRange { .. })));
    assert_eq!(db.parts().len(), 3);
}

#[test]
fn the_type_name_table_is_not_limited_to_a_byte() {
    let mut db = db();
    let stock = db.parts().to_vec();
    // The stored record keeps the type index in a byte (256 car types and groups); in memory it is 16 bits wide, so a
    // game with far more mod cars than that keeps a body part for every one.
    for i in 0..600u32 {
        let mut new = body(&[1], &[]);
        new.type_name_hash = 0x1000 + i;
        new.name = "X";
        db.push_part(&new).unwrap();
    }
    assert_eq!(db.type_names().len(), 602);
    let last = db.find(&PartQuery::new(23, 0x1000 + 599)).unwrap();
    assert_eq!(last.type_index, 601);
    assert_eq!(db.type_name_hash(last), 0x1000 + 599);
    assert_eq!(db.name(last), "X");
    let first = db.find(&PartQuery::new(23, 0x1000)).unwrap();
    assert_eq!(first.type_index, 2);
    // An existing type name is reused, and the stock parts are still first and unchanged.
    let mut again = body(&[1], &[]);
    again.type_name_hash = 0x1000;
    again.part_id = 67;
    assert_eq!(db.push_part(&again).unwrap().type_index, 2);
    assert_eq!(&db.parts()[..stock.len()], &stock[..]);
}

#[test]
fn a_string_table_past_the_stored_width_still_addresses_every_name() {
    let mut db = db();
    let mut new = body(&[1], &[]);
    // 4-byte units: a 16-bit offset reaches 262 kB, about 4,500 names of a mod car's size on top of the game's own.
    db.strings.resize(70_000 * 4, 0);
    new.name = "BEYOND";
    let part = db.push_part(&new).unwrap();
    assert!(part.name_offset > u32::from(u16::MAX));
    assert_eq!(db.name(&part), "BEYOND");
}

#[test]
fn the_model_tables_past_the_stored_width_still_resolve() {
    let mut db = db();
    db.model_tables.resize(
        usize::from(u16::MAX) + 10,
        crate::parts::ModelTable { templated: false, middle: None, entries: Vec::new() },
    );
    let solid = bstring_hash("MODCAR_KIT00_BODY_A");
    let part = db.push_part(&body(&[solid], &[])).unwrap();
    assert!(part.model_table > u32::from(u16::MAX));
    assert_eq!(db.model_hash(&part, 0), Some(solid));
}

#[test]
fn a_full_attribute_table_is_refused() {
    let mut db = db();
    db.attributes.resize(i16::MAX as usize, (1, 1));
    assert!(matches!(db.push_part(&body(&[1], &[(9, 9)])), Err(Error::Full("attribute"))));
    // A pair that is already stored needs no room.
    assert!(db.push_part(&body(&[1], &[(1, 1)])).is_ok());

    let mut db = self::db();
    db.attribute_lists.resize(usize::from(u16::MAX) * 2, 0);
    assert!(matches!(db.push_part(&body(&[1], &[(bstring_hash("KITNUMBER"), 0)])), Err(Error::Full("attribute list"))));
    // No list is needed for a part without attributes.
    assert!(db.push_part(&body(&[1], &[])).is_ok());
}

#[test]
fn a_group_goes_in_whole_or_not_at_all() {
    let mut db = db();
    let parts = db.parts().len();
    let (one, two, name) = ([1], [2], [(bstring_hash("NAME"), 3)]);
    let mut wheel = body(&two, &name);
    wheel.part_id = 67;
    wheel.name = "MODCAR_WHEEL";
    let stored = db.push_parts(&[body(&one, &[]), wheel]).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(db.parts().len(), parts + 2);

    // The second of the pair does not fit (the attribute lists are full and it needs one): nothing stays.
    let mut db = self::db();
    db.attribute_lists.resize(usize::from(u16::MAX) * 2, 0);
    let mut other = body(&one, &[]);
    other.type_name_hash = 0xABCD;
    let mut wheel = body(&two, &name);
    wheel.part_id = 67;
    wheel.name = "MODCAR_WHEEL";
    let before =
        (db.strings.len(), db.parts().len(), db.type_names().len(), db.attributes.len(), db.model_tables.len());
    assert!(matches!(db.push_parts(&[other, wheel]), Err(Error::Full("attribute list"))));
    let after = (db.strings.len(), db.parts().len(), db.type_names().len(), db.attributes.len(), db.model_tables.len());
    assert_eq!(after, before);
}
