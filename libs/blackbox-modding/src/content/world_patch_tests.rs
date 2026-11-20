use super::*;

const TEXT: &str = r#"
format = 1

[patch]
target = "Okayama"
priority = 3
name = "Okayama at dusk"
start = { pos = [1.0, 2.0, 3.0], heading = 45.0 }
environment = { hour = 19.0 }
remove_packs = ["g017"]
replace_packs = ["g003"]

[[patch.grid]]
pos = [4.0, 5.0, 6.0]
heading = 46.0
"#;

fn patch(text: &str) -> WorldPatch {
    parse("dusk", text).unwrap()
}

fn base() -> WorldManifest {
    WorldManifest::new("okayama", "Okayama", [-230.0, -214.0, 26.0], 95.0)
}

fn keys(list: &[&str]) -> Vec<String> {
    list.iter().map(|key| (*key).to_owned()).collect()
}

#[test]
fn a_patch_parses_and_lower_cases_its_target() {
    let patch = patch(TEXT);
    assert_eq!((patch.target.as_str(), patch.priority), ("okayama", 3));
    assert_eq!(patch.name.as_deref(), Some("Okayama at dusk"));
    assert_eq!(patch.start, Some(Slot { pos: [1.0, 2.0, 3.0], heading: 45.0 }));
    assert_eq!(patch.environment.and_then(|env| env.hour), Some(19.0));
    assert_eq!((patch.remove_packs.clone(), patch.replace_packs.clone()), (keys(&["g017"]), keys(&["g003"])));
    assert_eq!(patch.grid.len(), 1);
}

#[test]
fn bad_patches_are_refused() {
    for (what, text) in [
        ("another format", TEXT.replace("format = 1", "format = 2")),
        ("a target that is not an id", TEXT.replace("\"Okayama\"", "\"Not an id\"")),
        ("a priority out of range", TEXT.replace("priority = 3", "priority = 3000")),
        ("a key removed and replaced", TEXT.replace("[\"g003\"]", "[\"g017\"]")),
        ("a key listed twice", TEXT.replace("[\"g017\"]", "[\"g017\", \"G017\"]")),
        ("an unknown key", TEXT.replace("priority = 3", "priority = 3\nspeed = 1")),
        ("an empty name", TEXT.replace("Okayama at dusk", "")),
    ] {
        assert!(parse("p", &text).is_err(), "{what}");
    }
}

#[test]
fn patches_apply_from_the_lowest_priority_and_the_last_to_set_a_thing_wins() {
    let low = parse(
        "low",
        "format = 1\n[patch]\ntarget = \"okayama\"\npriority = -5\nname = \"Low\"\nstart = { pos = [9.0, 9.0, 9.0] }\n",
    )
    .unwrap();
    let high = patch(TEXT);
    let late = parse("late", "format = 1\n[patch]\ntarget = \"okayama\"\nname = \"Late\"\n").unwrap();
    let inputs = [
        PatchInput { owner: "b", order: 1, patch: &high },
        PatchInput { owner: "a", order: 0, patch: &low },
        PatchInput { owner: "c", order: 2, patch: &late },
    ];
    let applied = apply(&base(), &inputs);
    // Applied in the order low (-5), late (0), high (3): the highest priority speaks last.
    assert_eq!(applied.manifest.name, "Okayama at dusk");
    assert_eq!(applied.manifest.start, [1.0, 2.0, 3.0]);
    assert_eq!(applied.manifest.heading, 45.0);
    assert_eq!(applied.manifest.environment.and_then(|env| env.hour), Some(19.0));
    assert_eq!(applied.manifest.grid.len(), 1);
    assert_eq!(applied.report.len(), 3, "two names and one start were overridden: {:?}", applied.report);
    assert!(applied.report.iter().any(|line| line.contains("mod b") && line.contains("name")));
    let none = apply(&base(), &[]);
    assert_eq!(none.manifest, base());
}

#[test]
fn start_slots_are_appended_or_replace_what_is_there() {
    let mut start = base();
    start.grid = vec![Slot { pos: [0.0; 3], heading: 0.0 }];
    let append = patch(TEXT);
    assert_eq!(apply(&start, &[PatchInput { owner: "m", order: 0, patch: &append }]).manifest.grid.len(), 2);
    let replace = parse("r", &TEXT.replace("remove_packs", "grid_replace = true\nremove_packs")).unwrap();
    let applied = apply(&start, &[PatchInput { owner: "m", order: 0, patch: &replace }]);
    assert_eq!(applied.manifest.grid, replace.grid);
}

#[test]
fn packs_are_removed_added_and_replaced_in_order() {
    let patch = patch(TEXT);
    let own = keys(&["g003", "extra"]);
    let inputs = [PackInput { owner: "m", patch: &patch, keys: &own }];
    let plan = plan_packs(&keys(&["g001", "g003", "g017"]), &inputs);
    assert_eq!(
        plan.packs,
        [
            PackOrigin::World("g001".into()),
            PackOrigin::Patch { index: 0, key: "g003".into() },
            PackOrigin::Patch { index: 0, key: "extra".into() },
        ],
        "g017 is gone, g003 is the patch's and keeps its place, extra is last"
    );
    assert!(plan.report.is_empty(), "{:?}", plan.report);
}

#[test]
fn a_clash_of_keys_is_refused_and_reported() {
    let patch =
        parse("p", "format = 1\n[patch]\ntarget = \"w\"\nremove_packs = [\"nothing\"]\nreplace_packs = [\"ghost\"]\n")
            .unwrap();
    let own = keys(&["g001", "new"]);
    let inputs = [PackInput { owner: "m", patch: &patch, keys: &own }];
    let plan = plan_packs(&keys(&["g001"]), &inputs);
    assert_eq!(plan.packs, [PackOrigin::World("g001".into()), PackOrigin::Patch { index: 0, key: "new".into() }]);
    assert_eq!(plan.report.len(), 3, "{:?}", plan.report);
    assert!(plan.report.iter().any(|line| line.contains("removes the pack nothing")));
    assert!(plan.report.iter().any(|line| line.contains("adds the pack g001") && line.contains("replace_packs")));
    assert!(plan.report.iter().any(|line| line.contains("replace_packs but has no such pack")));
}
