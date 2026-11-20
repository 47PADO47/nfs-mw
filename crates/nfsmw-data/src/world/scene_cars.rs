//! The cars a NIS scene makes for its police channels (`cop1`..`cop8`) from its special car list code. The scene
//! candidate table gives each listed scene a code (`moment.rs` reads it); the code picks one fixed list of eight
//! vehicles, given to `cop1`..`cop8` in order. Nothing is random. Spec: `docs/specs/nis-scenes.md` section 6 and
//! `docs/specs/nis-player.md` section 7.

/// The vehicle (`pvehicle` name) each police channel of a scene with list `code` gets, `cop1` first. `None` for a code
/// with no list (`NONE`, -1, and unknown codes). `ONE_COP` (0) and `ONE_SPORT_COP` (11) make only `cop1`.
pub fn list(code: i32) -> Option<[Option<&'static str>; 8]> {
    let full = |names: [&'static str; 8]| Some(names.map(Some));
    match code {
        0 => Some([Some("copmidsize_nis"), None, None, None, None, None, None, None]),
        1 => full(["copmidsize_nis", LD, LD, LD, LD, LD, LD, LD]),
        2 => full(["copmidsize_nis", LD, LD, "trafminivan", "copghost", "trafsuva", "trafstwag", "traftaxi"]),
        3 => full([LD, LD, "trafficcoup", "trafpizza", "trafminivan", "trafstwag", "trafsuva", "traftaxi"]),
        4 => full([LD, LD, LD, "copsuv", LD, "copsuv", "copmidsize_nis", UNNAMED_A]),
        5 => full([
            "eclipsegt",
            "imprezawrx",
            "trafficcoup",
            "trafficcoup",
            "trafpizza",
            "trafminivan",
            UNNAMED_B,
            "trafpizza",
        ]),
        6 => full([
            "eclipsegt",
            "eclipsegt",
            UNNAMED_A,
            "trafficcoup",
            "traf4dsedc",
            "trafminivan",
            "copmidsize_nis",
            "trafstwag",
        ]),
        7 => full([
            "copsportghost",
            "copsportghost",
            "copsportghost",
            "copsportghost",
            "copsportghost",
            "copsportghost",
            "copsportghost",
            "copcross",
        ]),
        8 => full([LD, LD, LD, "trafstwag", "trafficcoup", "traf4dsedc", "trafminivan", "trafpizza"]),
        9 => full([
            "trafamb",
            "trafstwag",
            "trafpizza",
            "trafminivan",
            "trafficcoup",
            "traf4dsedc",
            "trafstwag",
            "trafpizza",
        ]),
        10 => full([LD, LD, LD, "trafminivan", "trafficcoup", "traf4dsedc", "trafstwag", "trafpizza"]),
        11 => Some([Some("copsport"), None, None, None, None, None, None, None]),
        _ => None,
    }
}

/// The NIS police car the lists use most.
const LD: &str = "copmidsize_nis_ld";
/// Two vehicles of the lists whose names were not recovered (their vault hashes `0x0A92CC67` and `0xA9B509C4`).
const UNNAMED_A: &str = "0a92cc67";
const UNNAMED_B: &str = "a9b509c4";

/// The model folder a list vehicle draws with: the NIS police cars are the install's `COPMIDSIZE`; the others use their
/// own name.
pub fn model_of(vehicle: &str) -> &str {
    match vehicle {
        "copmidsize_nis" | "copmidsize_nis_ld" => "copmidsize",
        other => other,
    }
}

/// Whether a list vehicle is a police car (its model loads as a cop).
pub fn is_police(vehicle: &str) -> bool {
    vehicle.starts_with("cop")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lists_give_eight_cars_in_channel_order_and_the_single_cop_lists_one() {
        let mix7 = list(8).expect("TRAFFIC_MIX7");
        assert_eq!(mix7[0], Some("copmidsize_nis_ld"));
        assert_eq!(mix7[3], Some("trafstwag"), "cop4");
        assert_eq!(mix7[7], Some("trafpizza"), "cop8");
        assert_eq!(list(0).expect("ONE_COP")[1], None);
        assert_eq!(list(-1), None);
        assert_eq!(list(12), None);
        assert_eq!(model_of("copmidsize_nis_ld"), "copmidsize");
        assert_eq!(model_of("trafminivan"), "trafminivan");
        assert!(is_police("copsuv") && !is_police("traftaxi"));
    }
}
