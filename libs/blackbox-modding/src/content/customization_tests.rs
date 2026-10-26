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
    assert_eq!(table.spoilers.len() + table.roof_scoops.len() + table.rim_paints.len(), 3);
}

#[test]
fn only_the_paints_are_needed() {
    let table = car(&with_table("paints = [{ name = \"Red\", color = [1, 2, 3] }]\n")).unwrap().customization.unwrap();
    assert_eq!(table.default_paint, 0);
    assert!(table.rims.is_empty() && table.vinyls.is_empty() && table.vinyl_colors.is_empty());
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
