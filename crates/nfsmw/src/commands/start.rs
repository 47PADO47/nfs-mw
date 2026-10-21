//! Where `play` starts the front end (`docs/specs/boot-profile-prompt.md`, section 5).

use crate::frontend::Start;

/// The start for `play`'s switches.
///
/// - `drive`: straight into free roam.
/// - `aliases`: the profile step alone, the explicit way to run it without the movies and the title screen.
/// - `scripted` (`--skip-boot` or a `--ui-script`): the main menu with no prompt, so scripts and screenshots start from
///   the same place as before.
/// - otherwise the whole boot flow. `skip_intro` (the setting) only drops the movies: the title screen and the profile
///   step still come, as in the original, whose profile list is not an intro.
pub fn of(drive: bool, aliases: bool, scripted: bool, skip_intro: bool) -> Start {
    if drive {
        return Start::Drive;
    }
    if aliases {
        return Start::Aliases;
    }
    if scripted {
        return Start::Menu;
    }
    Start::Boot { movies: !skip_intro }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_start_runs_the_whole_boot_flow() {
        assert_eq!(of(false, false, false, false), Start::Boot { movies: true });
    }

    #[test]
    fn skip_intro_drops_the_movies_but_keeps_the_title_and_the_profile_step() {
        assert_eq!(of(false, false, false, true), Start::Boot { movies: false });
    }

    #[test]
    fn skip_boot_and_scripts_start_at_the_menu_without_a_prompt() {
        assert_eq!(of(false, false, true, false), Start::Menu);
        assert_eq!(of(false, false, true, true), Start::Menu, "the setting changes nothing for a script");
    }

    #[test]
    fn the_profile_step_can_be_asked_for_alone() {
        assert_eq!(of(false, true, false, false), Start::Aliases);
        assert_eq!(of(false, true, true, true), Start::Aliases, "even in a script, which is how it is tested");
    }

    #[test]
    fn drive_wins() {
        assert_eq!(of(true, true, true, true), Start::Drive);
    }
}
