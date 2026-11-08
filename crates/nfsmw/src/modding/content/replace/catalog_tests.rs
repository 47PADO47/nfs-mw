use super::*;

fn mod_with(id: &str, sets: &[(&str, &str)]) -> ModDir {
    let root = std::env::temp_dir().join(format!("nfsmw-catalog-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let folder = root.join(REPLACE_FOLDER);
    std::fs::create_dir_all(&folder).unwrap();
    for (name, text) in sets {
        std::fs::write(folder.join(format!("{name}.toml")), text).unwrap();
    }
    ModDir { id: id.to_owned(), dir: root, priority: 0 }
}

fn set(kind: &str, value: &str, extra: &str) -> String {
    format!(
        "format = 1\n[set]\nkind = \"{kind}\"\nvalue = \"{value}\"\n{extra}\n[files]\ngeometry = \"A.BIN\"\n[[model]]\ntarget = \"S_{value}\"\nmodel = \"M\"\n"
    )
}

#[test]
fn sets_are_listed_in_load_order_then_file_name_and_grouped_by_kind() {
    let first = mod_with(
        "first",
        &[("b", &set("trees", "tall", "kind_label = \"Trees\"")), ("a", &set("bushes", "round", ""))],
    );
    let second = mod_with("second", &[("c", &set("trees", "short", ""))]);
    let catalog = Catalog::scan(&[first, second]);
    let ids: Vec<&str> = catalog.sets.iter().map(|record| record.set.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);
    let kinds = catalog.kinds();
    assert_eq!(kinds.iter().map(|kind| kind.option.as_str()).collect::<Vec<_>>(), ["bushes_variant", "trees_variant"]);
    assert_eq!(kinds[0].label, "Bushes", "the kind, capitalised, when no set names the option");
    assert_eq!(kinds[1].label, "Trees");
    assert_eq!(kinds[1].values.iter().map(|value| value.owner.as_str()).collect::<Vec<_>>(), ["first", "second"]);
    assert_eq!(catalog.find("trees", "short").unwrap().label(), "second:c");
    assert!(catalog.find("trees", "round").is_none());
}

#[test]
fn a_second_set_with_the_same_kind_and_value_and_a_set_for_another_game_are_refused() {
    let first = mod_with("first", &[("a", &set("trees", "tall", ""))]);
    let second = mod_with(
        "second",
        &[("b", &set("trees", "tall", "")), ("c", &set("trees", "x", "source_game = \"nfsu2\"")), ("d", "format = 1")],
    );
    let catalog = Catalog::scan(&[first, second]);
    assert_eq!(catalog.sets.len(), 1);
    assert_eq!(catalog.refused.len(), 3);
    let reasons: Vec<&str> = catalog.refused.iter().map(|refused| refused.reason.as_str()).collect();
    assert!(reasons.iter().any(|reason| reason.contains("already provided by first:a")), "{reasons:?}");
    assert!(reasons.iter().any(|reason| reason.contains("no reader")), "{reasons:?}");
}

#[test]
fn always_sets_have_no_kind_and_a_rescan_keeps_the_sets_that_did_not_change() {
    let always =
        "format = 1\n[set]\nalways = true\n[files]\ngeometry = \"A.BIN\"\n[[model]]\ntarget = \"S\"\nmodel = \"M\"\n";
    let m = mod_with("m", &[("fix", always), ("a", &set("trees", "tall", ""))]);
    let mut catalog = Catalog::scan(std::slice::from_ref(&m));
    assert_eq!(catalog.always().count(), 1);
    assert_eq!(catalog.kinds().len(), 1, "the always set is in no kind");
    let before = catalog.sets.iter().find(|record| record.set.id == "a").unwrap().set.clone();
    std::fs::write(m.dir.join(REPLACE_FOLDER).join("new.toml"), set("trees", "other", "")).unwrap();
    catalog.rescan(std::slice::from_ref(&m));
    assert_eq!(catalog.sets.len(), 3);
    let kept = catalog.sets.iter().find(|record| record.set.id == "a").unwrap();
    assert!(Arc::ptr_eq(&before, &kept.set), "the unchanged set is the same one, so it is not read again");
}
