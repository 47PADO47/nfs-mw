//! Co-op player names (docs/specs/coop-career.md). [ours]: the original has no co-op career.

/// The name a co-op player shows: the alias trimmed, then the player number ("Alias 2").
/// An empty or blank alias falls back to "Player {number}".
pub fn player_name(alias: &str, number: u8) -> String {
    let alias = alias.trim();
    if alias.is_empty() {
        return format!("Player {number}");
    }
    format!("{alias} {number}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_number_the_alias() {
        assert_eq!(player_name("  Rosa ", 2), "Rosa 2");
        assert_eq!(player_name("Rosa", 1), "Rosa 1");
    }

    #[test]
    fn empty_alias_falls_back() {
        assert_eq!(player_name("", 2), "Player 2");
        assert_eq!(player_name("   ", 1), "Player 1");
    }
}
