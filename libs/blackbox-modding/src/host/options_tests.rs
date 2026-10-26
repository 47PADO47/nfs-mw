use super::*;

fn game() -> OptionsModel {
    OptionsModel::new(vec![
        OptionDef::game_list("nitrous_variant", "Nitrous variant", &[("original", "Original")]),
        OptionDef::fixed("turn_signals", OptionKind::Toggle),
        OptionDef::fixed("speed_units", OptionKind::List),
    ])
}

fn list<'a>(id: &'a str, values: &[(&'a str, &'a str)]) -> OptionSpec<'a> {
    OptionSpec {
        id,
        kind: OptionKind::List,
        label: "Label",
        values: values.iter().map(|(id, label)| ValueSpec { id, label }).collect(),
        default_on: false,
    }
}

fn ids(option: &OptionDef) -> Vec<&str> {
    option.values.iter().map(|value| value.id.as_str()).collect()
}

#[test]
fn a_list_with_one_value_is_hidden_and_with_more_is_shown() {
    let mut model = game();
    assert!(model.visible().is_empty(), "the game's own nitrous variant has one value");
    let (_, merge) =
        model.add_option("mod.a", &list("nitrous_variant", &[("mw2012", "NFS Most Wanted 2012")])).unwrap();
    assert_eq!(merge, Merge::Integrated { added: vec!["mw2012".into()], skipped: 0 });
    let shown = model.visible();
    assert_eq!(shown.len(), 1);
    assert_eq!(ids(shown[0]), ["original", "mw2012"]);
}

#[test]
fn a_missing_option_is_appended_with_the_given_values() {
    let mut model = game();
    let (_, merge) = model.add_option("mod.a", &list("flame_color", &[("blue", "Blue")])).unwrap();
    assert_eq!(merge, Merge::Created);
    let option = model.get("flame_color").unwrap();
    assert_eq!(option.owner.as_deref(), Some("mod.a"));
    assert!(!option.visible(), "one value: hidden even for a mod's own list");
    model.add_option("mod.b", &list("flame_color", &[("red", "Red")])).unwrap();
    assert!(model.get("flame_color").unwrap().visible());
    assert_eq!(model.options().last().unwrap().id, "flame_color", "appended after the game's options");
}

#[test]
fn values_integrate_in_order_without_duplicates_and_the_games_stay_first() {
    let mut model = game();
    model.add_option("mod.a", &list("nitrous_variant", &[("b", "B"), ("original", "Mine"), ("a", "A")])).unwrap();
    let (_, merge) = model.add_option("mod.b", &list("nitrous_variant", &[("a", "A2"), ("c", "C")])).unwrap();
    assert_eq!(merge, Merge::Integrated { added: vec!["c".into()], skipped: 1 });
    let option = model.get("nitrous_variant").unwrap();
    assert_eq!(ids(option), ["original", "b", "a", "c"]);
    assert_eq!(option.values[0].label, "Original", "the game's own value keeps its label");
    assert_eq!(model.value_owner("nitrous_variant", "a"), Some("mod.a"));
    assert_eq!(model.value_owner("nitrous_variant", "original"), None);
}

#[test]
fn another_kind_is_refused_and_nothing_changes() {
    let mut model = game();
    let toggle =
        OptionSpec { id: "nitrous_variant", kind: OptionKind::Toggle, label: "x", values: vec![], default_on: true };
    assert_eq!(model.add_option("mod.a", &toggle), Err(AddError::KindMismatch { existing: OptionKind::List }));
    assert_eq!(ids(model.get("nitrous_variant").unwrap()), ["original"]);
    let as_list = list("turn_signals", &[("x", "X")]);
    assert_eq!(model.add_option("mod.a", &as_list), Err(AddError::KindMismatch { existing: OptionKind::Toggle }));
    assert_eq!(model.options().len(), 3);
}

#[test]
fn a_fixed_row_of_the_game_takes_no_values() {
    let mut model = game();
    let (_, merge) = model.add_option("mod.a", &list("speed_units", &[("x", "X")])).unwrap();
    assert_eq!(merge, Merge::Fixed);
    assert!(model.get("speed_units").unwrap().values.is_empty());
    assert!(model.visible().is_empty(), "fixed rows are drawn by the game, not the model");
}

#[test]
fn a_bad_spec_is_refused() {
    let mut model = game();
    assert!(matches!(model.add_option("m", &list("Bad Id", &[("a", "A")])), Err(AddError::Invalid(_))));
    assert!(matches!(model.add_option("m", &list("ok", &[])), Err(AddError::Invalid(_))));
    assert!(matches!(model.add_option("m", &list("ok", &[("a", "A"), ("a", "B")])), Err(AddError::Invalid(_))));
    assert!(matches!(model.add_option("m", &list("ok", &[("a", "")])), Err(AddError::Invalid(_))));
    let toggle = OptionSpec {
        id: "t",
        kind: OptionKind::Toggle,
        label: "x",
        values: vec![ValueSpec { id: "a", label: "A" }],
        default_on: false,
    };
    assert!(matches!(model.add_option("m", &toggle), Err(AddError::Invalid(_))));
    assert_eq!(model.options().len(), 3);
}

#[test]
fn a_mod_that_leaves_takes_its_values_and_options_with_it() {
    let mut model = game();
    let (handle, _) = model.add_option("mod.a", &list("nitrous_variant", &[("mw2012", "M")])).unwrap();
    model.add_option("mod.a", &list("own", &[("x", "X")])).unwrap();
    assert!(model.remove("mod.a", handle));
    assert!(!model.remove("mod.a", handle), "twice does nothing");
    assert_eq!(ids(model.get("nitrous_variant").unwrap()), ["original"]);
    assert!(model.visible().is_empty(), "the option is hidden again");
    model.remove_owner("mod.a");
    assert!(model.get("own").is_none());
    assert!(!model.remove("mod.b", 99));
}

#[test]
fn resolve_and_step_walk_the_values() {
    let mut model = game();
    model.add_option("m", &list("nitrous_variant", &[("mw2012", "M")])).unwrap();
    let option = model.get("nitrous_variant").unwrap();
    assert_eq!(option.resolve(None), "original");
    assert_eq!(option.resolve(Some("mw2012")), "mw2012");
    assert_eq!(option.resolve(Some("gone")), "original", "a mod that is not loaded leaves the original");
    assert_eq!(option.step("original", true), "mw2012");
    assert_eq!(option.step("mw2012", true), "original");
    assert_eq!(option.step("original", false), "mw2012");
    assert_eq!(option.step("unknown", true), "mw2012");
    let toggle = OptionDef { default_on: true, ..OptionDef::fixed("t", OptionKind::Toggle) };
    assert_eq!(toggle.resolve(None), "on");
    assert_eq!(toggle.resolve(Some("garbage")), "on");
    assert_eq!(toggle.step("on", true), "off");
}
