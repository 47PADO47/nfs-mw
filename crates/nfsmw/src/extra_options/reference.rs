//! The F5 pursuit reference: the bounty places to jump to, the testing heat the Page Up and Page Down keys step, and
//! the state of the pursuit trace recorder. Pure data and rules, so they can be tested without a world.

use nfsmw_data::gameplay::BountyJump;

/// The number `NN` of the bounty place the reference uses by default: `BLACKLIST_BOUNTY_SPAWN_POINT_11` reads
/// "Riverfront Stadium" in the English table of the install. Used when the language has no label of that name.
pub const RIVERFRONT_STADIUM: i32 = 11;
/// The English name the default place is found by, before the number above is used.
const DEFAULT_NAME: &str = "riverfront stadium";
/// The highest heat of the unmodded game; the testing heat goes on to 10 as the mods' extended levels.
pub const ORIGINAL_MAX_HEAT: u8 = 5;
/// The testing heat range.
pub const MIN_HEAT: u8 = 1;
pub const MAX_HEAT: u8 = 10;

/// One bounty place of the install, with every place listed whatever the career stage.
#[derive(Clone, Debug, PartialEq)]
pub struct BountySpot {
    /// The `NN` of `BLACKLIST_BOUNTY_SPAWN_POINT_NN`.
    pub number: i32,
    /// The place's name in the game's language, or `Bounty place NN` when it has none.
    pub name: String,
    /// Where the car goes, render space x and y.
    pub at: [f32; 2],
    /// The marker the car is put on, as the rap sheet's jump does: its place and its `Rotation`.
    pub jump: crate::career::JumpPoint,
}

/// The label key of a place's name.
pub fn label(number: i32) -> String {
    format!("BLACKLIST_BOUNTY_SPAWN_POINT_{number:02}")
}

/// Every place with a position, in number order. `name` resolves a place number to its text.
pub fn spots(jumps: &[BountyJump], name: impl Fn(i32) -> Option<String>) -> Vec<BountySpot> {
    let mut out: Vec<BountySpot> = jumps
        .iter()
        .filter_map(|jump| {
            let p = jump.position?;
            let name = name(jump.number).filter(|text| !text.is_empty());
            Some(BountySpot {
                number: jump.number,
                name: name.unwrap_or_else(|| format!("Bounty place {:02}", jump.number)),
                at: [p.z, -p.x],
                jump: crate::career::JumpPoint::from_marker(p, jump.rotation_deg),
            })
        })
        .collect();
    out.sort_by_key(|spot| spot.number);
    out
}

/// The place selected first: the one named Riverfront Stadium, else number [`RIVERFRONT_STADIUM`], else the first.
pub fn default_spot(spots: &[BountySpot]) -> Option<i32> {
    if let Some(spot) = spots.iter().find(|spot| spot.name.trim().eq_ignore_ascii_case(DEFAULT_NAME)) {
        return Some(spot.number);
    }
    if spots.iter().any(|spot| spot.number == RIVERFRONT_STADIUM) {
        return Some(RIVERFRONT_STADIUM);
    }
    spots.first().map(|spot| spot.number)
}

/// The place's row in the list ("11/12 · Riverfront Stadium").
pub fn row_text(spots: &[BountySpot], number: i32) -> String {
    let Some(at) = spots.iter().position(|spot| spot.number == number) else {
        return format!("Bounty place {number:02}");
    };
    format!("{}/{} · {}", at + 1, spots.len(), spots[at].name)
}

/// `heat` moved by `delta` levels, kept in 1 to 10.
pub fn step_heat(heat: u8, delta: i8) -> u8 {
    let next = i16::from(heat) + i16::from(delta);
    next.clamp(i16::from(MIN_HEAT), i16::from(MAX_HEAT)) as u8
}

/// The note for a heat above the original's levels.
pub fn heat_note(heat: u8) -> Option<&'static str> {
    if heat <= ORIGINAL_MAX_HEAT {
        return None;
    }
    Some("above 5 is modded (extended heat levels)")
}

/// The toast shown after a heat key: the new level, whether a running chase took it, and the modded note.
pub fn heat_toast(heat: u8, applied: bool) -> String {
    let mut text = format!("Testing heat {heat}");
    if applied {
        text.push_str(" · applied to the pursuit");
    }
    if !applied {
        text.push_str(" · used by the next pursuit");
    }
    if let Some(note) = heat_note(heat) {
        text.push_str(" · ");
        text.push_str(note);
    }
    text
}

/// What the F5 panel shows of the pursuit trace recorder; the world fills it every frame.
#[derive(Clone, Debug, Default)]
pub struct TraceView {
    pub recording: bool,
    /// The folder the next session goes into.
    pub root: Option<std::path::PathBuf>,
    /// The folder of the session being written, or of the last one.
    pub session: Option<std::path::PathBuf>,
    /// Samples and events handed to the writer, records dropped, and the writer's error.
    pub samples: u64,
    pub events: u64,
    pub dropped: u64,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn jump(number: i32, position: Option<Vec3>) -> BountyJump {
        BountyJump { key: number as u32, name: format!("bounty_jump/{number}"), number, position, rotation_deg: None }
    }

    #[test]
    fn heat_steps_stay_within_one_to_ten() {
        assert_eq!(step_heat(1, -1), 1);
        assert_eq!(step_heat(1, 1), 2);
        assert_eq!(step_heat(5, 1), 6);
        assert_eq!(step_heat(10, 1), 10);
        assert_eq!(step_heat(10, -1), 9);
        assert_eq!(step_heat(3, i8::MIN), 1);
        assert_eq!(step_heat(3, i8::MAX), 10);
        assert_eq!(step_heat(0, 0), 1, "an out-of-range heat is brought back into range");
    }

    #[test]
    fn heats_above_five_are_marked_modded() {
        assert!(heat_note(5).is_none());
        assert!(heat_note(6).is_some());
        assert!(heat_toast(7, true).contains("modded"));
        assert!(heat_toast(3, false).contains("next pursuit"));
        assert!(heat_toast(3, true).contains("applied"));
    }

    #[test]
    fn every_place_is_listed_in_number_order_whatever_the_stage() {
        let mut jumps: Vec<BountyJump> = (1..=12).rev().map(|n| jump(n, Some(Vec3::new(1.0, 0.0, 2.0)))).collect();
        jumps.push(jump(13, None));
        let spots = spots(&jumps, |n| (n == 11).then(|| "Riverfront Stadium".to_owned()));
        assert_eq!(spots.iter().map(|s| s.number).collect::<Vec<_>>(), (1..=12).collect::<Vec<_>>());
        assert_eq!(spots[0].name, "Bounty place 01");
        assert_eq!(spots[0].at, [2.0, -1.0], "physics (x, y, z) goes to render (z, -x)");
        assert_eq!(default_spot(&spots), Some(11));
        assert_eq!(row_text(&spots, 11), "11/12 · Riverfront Stadium");
    }

    #[test]
    fn the_default_place_falls_back_to_its_number_and_then_the_first() {
        let named = |_: i32| Some("Elsewhere".to_owned());
        let all: Vec<BountyJump> = (1..=12).map(|n| jump(n, Some(Vec3::ZERO))).collect();
        assert_eq!(default_spot(&spots(&all, named)), Some(RIVERFRONT_STADIUM));
        let few: Vec<BountyJump> = (1..=4).map(|n| jump(n, Some(Vec3::ZERO))).collect();
        assert_eq!(default_spot(&spots(&few, named)), Some(1));
        assert_eq!(default_spot(&[]), None);
    }

    #[test]
    #[ignore = "needs a game install; set NFSMW_GAME_DIR"]
    fn the_installs_riverfront_stadium_is_bounty_place_eleven() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR is required");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let data = nfsmw_data::gameplay::cached(&dir).unwrap();
        let table = blackbox_text::StringTable::from_file(&nfsmw_data::shared::language(&dir).unwrap()).unwrap();
        let spots = spots(&data.bounty_jumps, |n| table.get(blackbox_text::label_hash(&label(n))));
        for spot in &spots {
            println!("{} at ({:.1}, {:.1})", row_text(&spots, spot.number), spot.at[0], spot.at[1]);
        }
        assert_eq!(spots.len(), 12);
        let riverfront = spots.iter().find(|s| s.number == RIVERFRONT_STADIUM).unwrap();
        assert_eq!(riverfront.name, "Riverfront Stadium");
        assert_eq!(default_spot(&spots), Some(RIVERFRONT_STADIUM));
        // The original's car after this jump (stock install, two captures): standing at physics (595.59, 110.84,
        // 939.84) and facing physics (0.9745, -0.0470, -0.2196).
        let jump = riverfront.jump;
        let heading = jump.heading.expect("the place has a Rotation");
        let captured = (-0.2196_f32).atan2(0.9745);
        // Render (cos h, sin h) is physics (-sin h, cos h) in x and z.
        let physics = (-heading.sin(), heading.cos());
        let off = (physics.1.atan2(physics.0) - captured).to_degrees();
        println!("Riverfront heading {:.2} deg render, {off:.2} deg from the capture", heading.to_degrees());
        assert!(off.abs() < 2.0, "{off} degrees from the captured heading");
        let captured_at = glam::Vec3::new(939.84, -595.59, 110.84);
        assert!(jump.at.truncate().distance(captured_at.truncate()) < 0.5, "{:?}", jump.at);
    }
}
