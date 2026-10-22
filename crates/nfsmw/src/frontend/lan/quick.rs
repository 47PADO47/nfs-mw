//! The preset quick lines (MP-44): ids into a table the game owns (`nfsmw_net::chat::QUICK_LINES`). The server checks the
//! id; the lines are this table's.

/// The preset lines, by id.
pub const QUICK_LINES: [&str; 8] = [
    "Police behind me!",
    "Roadblock ahead!",
    "Wait for me",
    "Go go go!",
    "Nice one",
    "Sorry!",
    "Good race",
    "Ready when you are",
];

/// The line of quick message `id`.
pub fn quick_line(id: u16) -> Option<&'static str> {
    QUICK_LINES.get(usize::from(id)).copied()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_preset_is_a_quick_id_the_server_accepts() {
        for id in 0..super::QUICK_LINES.len() as u16 {
            assert!(nfsmw_net::chat::is_quick_id(id), "{id}");
            assert!(super::quick_line(id).is_some());
        }
        assert_eq!(super::quick_line(99), None);
    }
}
