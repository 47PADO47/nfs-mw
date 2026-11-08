use super::*;

fn lender(name: &str, wheelbase_cm: f64) -> Lender {
    Lender { name: name.to_owned(), wheelbase_cm }
}

#[test]
fn a_rain_tyre_takes_the_nearest_compound_that_has_strips() {
    assert_eq!(nearest_compound("Rain", &["Hard", "Medium", "Soft"]), Some("Medium"));
    assert_eq!(nearest_compound("Rain", &["Hard", "Soft"]), Some("Soft"));
    assert_eq!(nearest_compound("Rain", &["Intermediate", "Medium"]), Some("Intermediate"));
}

#[test]
fn the_compound_choice_does_not_depend_on_the_order_of_the_list_and_ignores_case() {
    assert_eq!(
        nearest_compound("rain", &["Soft", "Medium", "Hard"]),
        nearest_compound("Rain", &["Hard", "Medium", "Soft"])
    );
    // Wet is one step from Intermediate and Road two; Intermediate and Road are each one step from Medium: the wetter one wins.
    assert_eq!(nearest_compound("Intermediate", &["Road", "Wet"]), Some("Wet"));
    assert_eq!(nearest_compound("Medium", &["Road", "Intermediate"]), Some("Intermediate"));
}

#[test]
fn an_unknown_compound_or_an_empty_list_has_no_choice() {
    assert_eq!(nearest_compound("Slime", &["Medium"]), None);
    assert_eq!(nearest_compound("Rain", &[]), None);
    assert_eq!(nearest_compound("Rain", &["Slime"]), None);
}

#[test]
fn a_vehicles_stem_is_split_into_make_model_and_year() {
    assert_eq!(parts_of("Italjet_Dragster200CC_RM_2022"), ("italjet".to_owned(), "dragster200cc".to_owned(), 2022));
    assert_eq!(
        parts_of("Indian_Challenger-RR-OHara-2022"),
        ("indian".to_owned(), "challengerrrohara".to_owned(), 2022)
    );
    assert_eq!(parts_of("Suzuki_Hayabusa"), ("suzuki".to_owned(), "hayabusa".to_owned(), 0));
}

#[test]
fn the_dragster_200_takes_the_dragster_700_not_the_other_italjet_or_a_bike_of_the_same_wheelbase() {
    let target = lender("Italjet_Dragster200CC_2022", 135.0);
    let lenders = [
        lender("Honda_PCX125_2021", 135.0),
        lender("Italjet_Dragster700Twin_2024", 143.5),
        lender("Italjet_Dragster200CC_2022", 135.0),
        lender("Italjet_Torpedo_2018", 135.0),
    ];
    let (chosen, rule) = nearest_lender(&target, &lenders).unwrap();
    assert_eq!(chosen.name, "Italjet_Dragster700Twin_2024");
    assert!(rule.starts_with("the same make"), "{rule}");
}

#[test]
fn of_the_same_make_the_longest_shared_model_start_wins_then_the_nearest_year() {
    let target = lender("Indian_Challenger-RR-2022", 170.0);
    let lenders = [
        lender("Indian_FTRX100RCarbon_2024", 150.0),
        lender("Indian_Challenger-Dark-2020", 166.0),
        lender("Indian_Challenger-Dark-2023", 166.0),
    ];
    assert_eq!(nearest_lender(&target, &lenders).unwrap().0.name, "Indian_Challenger-Dark-2023");
}

#[test]
fn with_no_model_of_the_make_the_nearest_wheelbase_wins_and_a_tie_goes_to_the_first_name() {
    let target = lender("Italjet_Dragster200CC_2022", 135.0);
    let lenders =
        [lender("Honda_PCX125_2021", 131.0), lender("Yamaha_NMAX_2021", 139.0), lender("Kawasaki_Z900_2021", 145.0)];
    let (chosen, rule) = nearest_lender(&target, &lenders).unwrap();
    assert_eq!(chosen.name, "Honda_PCX125_2021");
    assert!(rule.contains("wheelbase"), "{rule}");
}

#[test]
fn nobody_to_borrow_from_is_no_choice() {
    let target = lender("Italjet_Dragster200CC_2022", 135.0);
    assert!(nearest_lender(&target, &[]).is_none());
    assert!(nearest_lender(&target, std::slice::from_ref(&target)).is_none());
}

#[test]
fn the_comment_names_the_field_the_source_and_the_rule() {
    let borrowed = Borrowed {
        field: "FrontTyre_Rain.TireSections".to_owned(),
        source: "Bagger/FrontTyre_Medium".to_owned(),
        rule: "the nearest compound of the family".to_owned(),
    };
    assert_eq!(
        borrowed.comment(),
        "choice: FrontTyre_Rain.TireSections taken from Bagger/FrontTyre_Medium (the nearest compound of the family); the data leaves it unset"
    );
}
