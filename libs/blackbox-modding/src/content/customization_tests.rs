use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

const FULL: &str = r#"
[customization]
default_paint = 1
paints = [
  { name = "Red", color = [173, 43, 44] },
  { name = "Silver", color = [200, 204, 197], finish = "metallic" },
]
rim_paints = [{ name = "Black", color = [10, 10, 10] }]
tints = [{ name = "Light", color = [20, 30, 40], opacity = 0.4 }]
rims = [{ name = "Five spoke", model = "FASTCOUPE_RIM1" }]
spoilers = [{ name = "Wing", model = "FASTCOUPE_WING" }]
roof_scoops = [{ name = "Scoop", model = "FASTCOUPE_SCOOP" }]
vinyls = [{ name = "Stripe", bitmap = "FASTCOUPE_STRIPE", mask = "FASTCOUPE_STRIPE_MASK", channels = 2 }]
vinyl_colors = [{ name = "White", color = [255, 255, 255] }]
liveries = [{ name = "Livery 1", texture = "FASTCOUPE_LIV1" }, { name = "Livery 2", texture = "FASTCOUPE_LIV2" }]
"#;

fn with_table(table: &str) -> String {
    format!("[customization]\n{table}")
}

#[test]
fn a_car_has_no_customization_unless_it_gives_the_table() {
    assert_eq!(car("").unwrap().customization, None);
}

#[test]
fn the_full_table_is_read_in_order() {
    let table = car(FULL).unwrap().customization.unwrap();
    assert_eq!(table.default_paint, 1);
    assert_eq!(table.paints[0], Paint { name: "Red".into(), color: [173, 43, 44], finish: Finish::Gloss });
    assert_eq!(table.paints[1].finish, Finish::Metallic);
    assert_eq!(table.tints[0].opacity, 0.4);
    assert_eq!(table.rims[0].model, "FASTCOUPE_RIM1");
    assert_eq!(table.vinyls[0].channels, 2);
    assert_eq!(table.vinyl_colors.len(), 1);
    assert_eq!(
        table.liveries,
        vec![
            Livery { name: "Livery 1".into(), texture: "FASTCOUPE_LIV1".into() },
            Livery { name: "Livery 2".into(), texture: "FASTCOUPE_LIV2".into() },
        ]
    );
    assert_eq!(table.spoilers.len() + table.roof_scoops.len() + table.rim_paints.len(), 3);
}

#[test]
fn only_the_paints_are_needed() {
    let table = car(&with_table("paints = [{ name = \"Red\", color = [1, 2, 3] }]\n")).unwrap().customization.unwrap();
    assert_eq!(table.default_paint, 0);
    assert!(table.rims.is_empty() && table.vinyls.is_empty() && table.vinyl_colors.is_empty());
    assert!(table.liveries.is_empty());
}

#[test]
fn a_table_without_a_paint_is_refused() {
    assert!(car(&with_table("rims = [{ name = \"A\", model = \"X_RIM\" }]\n")).is_err());
    assert!(car(&with_table("paints = []\n")).is_err());
}

#[test]
fn wrong_values_are_refused() {
    let paint = "paints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    let bad = [
        "paints = [{ name = \"Red\", color = [1, 2, 256] }]\n".to_owned(),
        "paints = [{ name = \"Red\", color = [-1, 2, 3] }]\n".to_owned(),
        "paints = [{ name = \"Red\", color = [1, 2, 3], finish = \"matte\" }]\n".to_owned(),
        "paints = [{ name = \"\", color = [1, 2, 3] }]\n".to_owned(),
        "paints = [{ name = \"Red\", color = [1, 2, 3] }, { name = \"Red\", color = [4, 5, 6] }]\n".to_owned(),
        format!("default_paint = 1\n{paint}"),
        format!("{paint}tints = [{{ name = \"T\", color = [0, 0, 0], opacity = 1.5 }}]\n"),
        format!("{paint}rims = [{{ name = \"R\", model = \"has space\" }}]\n"),
        format!("{paint}rims = [{{ name = \"R\", model = \"../X\" }}]\n"),
        format!("{paint}vinyls = [{{ name = \"V\", bitmap = \"A\", mask = \"B\", channels = 4 }}]\n"),
        format!("{paint}unknown = 1\n"),
        format!("{paint}liveries = [{{ name = \"L\", texture = \"has space\" }}]\n"),
        format!("{paint}liveries = [{{ name = \"L\", texture = \"../X\" }}]\n"),
        format!("{paint}liveries = [{{ name = \"\", texture = \"X\" }}]\n"),
        format!("{paint}liveries = [{{ name = \"L\" }}]\n"),
        format!("{paint}liveries = [{{ name = \"L\", texture = \"A\", mask = \"B\" }}]\n"),
    ];
    for table in bad {
        assert!(car(&with_table(&table)).is_err(), "{table}");
    }
}

#[test]
fn a_list_is_capped() {
    let paints: String = (0..65).map(|n| format!("{{ name = \"P{n}\", color = [1, 2, 3] }},")).collect();
    assert!(car(&with_table(&format!("paints = [{paints}]\n"))).is_err());
    let paints: String = (0..64).map(|n| format!("{{ name = \"P{n}\", color = [1, 2, 3] }},")).collect();
    assert!(car(&with_table(&format!("paints = [{paints}]\n"))).is_ok());
}

#[test]
fn a_livery_may_not_repeat_a_livery_or_a_vinyl_name() {
    let paint = "paints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    let twice = "liveries = [{ name = \"L\", texture = \"A\" }, { name = \"L\", texture = \"B\" }]\n";
    assert!(car(&with_table(&format!("{paint}{twice}"))).is_err());
    let vinyl = "vinyls = [{ name = \"L\", bitmap = \"A\", mask = \"B\" }]\n";
    let livery = "liveries = [{ name = \"L\", texture = \"C\" }]\n";
    assert!(car(&with_table(&format!("{paint}{vinyl}{livery}"))).is_err());
    let other = "liveries = [{ name = \"M\", texture = \"C\" }]\n";
    assert!(car(&with_table(&format!("{paint}{vinyl}{other}"))).is_ok());
}

#[test]
fn the_livery_list_is_capped_like_the_others() {
    let paint = "paints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    let list = |count: usize| -> String {
        let items: String = (0..count).map(|n| format!("{{ name = \"L{n}\", texture = \"T{n}\" }},")).collect();
        format!("{paint}liveries = [{items}]\n")
    };
    assert!(car(&with_table(&list(64))).is_ok());
    assert!(car(&with_table(&list(65))).is_err());
}

#[test]
fn a_car_may_wear_one_of_its_liveries_from_the_start() {
    let paint = "paints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    let liveries = "liveries = [{ name = \"A\", texture = \"TA\" }, { name = \"B\", texture = \"TB\" }]\n";
    let dressed = |position: &str| car(&with_table(&format!("{paint}{liveries}default_livery = {position}\n")));
    assert_eq!(dressed("1").unwrap().customization.unwrap().default_livery, Some(1));
    assert!(dressed("2").is_err(), "past the end of the liveries");
    assert_eq!(car(&with_table(&format!("{paint}{liveries}"))).unwrap().customization.unwrap().default_livery, None);
    assert!(car(&with_table(&format!("{paint}default_livery = 0\n"))).is_err(), "no liveries to wear");
}
