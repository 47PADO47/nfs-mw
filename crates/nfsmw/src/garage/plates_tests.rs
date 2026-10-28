use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-plates-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("plates.toml")
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(folder, text)| ((*folder).to_owned(), (*text).to_owned())).collect()
}

#[test]
fn texts_written_are_read_back_by_folder() {
    let file = temp("round-trip");
    let stored = Stored { texts: map(&[("POR911", "MY PLATE"), ("BMWM3", "NFS HERO")]), designs: BTreeMap::new() };
    write(&file, &stored).unwrap();
    assert_eq!(read(&file), stored);
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn designs_are_kept_beside_the_texts_and_the_empty_id_is_a_design_too() {
    let file = temp("designs");
    let stored =
        Stored { texts: map(&[("POR911", "MY PLATE")]), designs: map(&[("POR911", "1863272"), ("BMWM3", "")]) };
    write(&file, &stored).unwrap();
    let back = read(&file);
    assert_eq!(back, stored);
    assert_eq!(back.designs.get("BMWM3").map(String::as_str), Some(""), "the plain plate, picked on purpose");
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn a_file_written_before_designs_existed_reads_with_no_designs() {
    let file = temp("old");
    std::fs::write(&file, "[plates]\nPOR911 = \"OLD\"\n").unwrap();
    let back = read(&file);
    assert_eq!(back.texts, map(&[("POR911", "OLD")]));
    assert!(back.designs.is_empty());
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn a_missing_or_broken_file_holds_nothing() {
    let file = temp("broken");
    assert_eq!(read(&file), Stored::default());
    std::fs::write(&file, "plates = [").unwrap();
    assert_eq!(read(&file), Stored::default());
    std::fs::write(&file, "[plates]\nA = 5\nB = \"X\"\n[plate_designs]\nC = 1\nD = \"7\"\n").unwrap();
    let back = read(&file);
    assert_eq!(back.texts.get("B").map(String::as_str), Some("X"), "a value that is no text is skipped");
    assert_eq!(back.texts.len(), 1);
    assert_eq!(back.designs, map(&[("D", "7")]), "and so is a design that is no id");
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn the_file_is_replaced_whole() {
    let file = temp("replace");
    write(&file, &Stored { texts: map(&[("A", "ONE")]), designs: map(&[("A", "1")]) }).unwrap();
    write(&file, &Stored { texts: map(&[("B", "TWO")]), designs: BTreeMap::new() }).unwrap();
    assert_eq!(read(&file), Stored { texts: map(&[("B", "TWO")]), designs: BTreeMap::new() });
    assert!(!file.with_extension("toml.tmp").exists());
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}
