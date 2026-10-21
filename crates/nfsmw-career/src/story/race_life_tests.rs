use super::*;
use crate::story::StoryFields;

/// A story with an intro movie and intro NIS (either may be empty), and an optional intro message.
fn intro(movie: Option<&str>, nis: Option<&str>, message: Option<i32>) -> StoryFields {
    StoryFields {
        intro_message_id: message,
        intro_movie: movie.map(str::to_owned),
        intro_nis: nis.map(str::to_owned),
        ..StoryFields::default()
    }
}

/// A career race that is not D-Day, not a boss race and not in split screen.
fn career() -> IntroContext {
    IntroContext { career: true, ..IntroContext::default() }
}

fn nis(scene: &str, movie: Option<&str>) -> IntroStep {
    IntroStep::Nis { scene: scene.to_owned(), movie: movie.map(str::to_owned) }
}

#[test]
fn intro_steps_follow_the_prologue_and_the_other_races() {
    let cases: Vec<(&str, StoryFields, IntroContext, Vec<IntroStep>)> = vec![
        (
            "16.1.0 is a D-Day race: no tutorial, the movie is the NIS argument (CSE 2.1, F-01)",
            intro(Some("storyfmv_rac01"), Some("IntroNisDD"), None),
            IntroContext { dday: true, ..career() },
            vec![nis("IntroNisDD", Some("storyfmv_rac01"))],
        ),
        (
            "16.2.2 runs the tutorial, then its NIS with the movie",
            intro(Some("storyfmv_cro06_coh06a"), Some("GenericStart"), None),
            career(),
            vec![IntroStep::Tutorial, nis("GenericStart", Some("storyfmv_cro06_coh06a"))],
        ),
        (
            "16.2.3 runs the tutorial, then its NIS with the movie",
            intro(Some("storyfmv_raz08"), Some("IntroNis05"), None),
            career(),
            vec![IntroStep::Tutorial, nis("IntroNis05", Some("storyfmv_raz08"))],
        ),
        (
            "16.1.1 shows message 2 after the tutorial, then its NIS with no movie",
            intro(None, Some("IntroNis10"), Some(2)),
            career(),
            vec![IntroStep::Tutorial, IntroStep::Message(2), nis("IntroNis10", None)],
        ),
        (
            "16.2.1 runs the tutorial, then its NIS with the movie (the D-Day flag is off here)",
            intro(Some("storyfmv_pin11"), Some("IntroNis04"), None),
            career(),
            vec![IntroStep::Tutorial, nis("IntroNis04", Some("storyfmv_pin11"))],
        ),
        (
            "1.8.1 epic pursuit has a movie and no NIS: the movie, then the empty NIS warps (1.2 rule 5)",
            intro(Some("Storyfmv_her136"), None, None),
            IntroContext { boss_race: true, ..career() },
            vec![IntroStep::Tutorial, IntroStep::Movie("Storyfmv_her136".to_owned()), IntroStep::WarpToStart],
        ),
        (
            "a quick-race boss race plays QuickRaceNIS and never the movie (1.2 rule 2)",
            StoryFields {
                quick_race_nis: Some("BossQuickNis".to_owned()),
                ..intro(Some("Storyfmv_her136"), Some("IntroNisBL01"), None)
            },
            IntroContext { career: false, boss_race: true, ..IntroContext::default() },
            vec![IntroStep::Tutorial, nis("BossQuickNis", None)],
        ),
        (
            "a quick-race boss race with no QuickRaceNIS only warps the player",
            intro(None, Some("IntroNisBL01"), None),
            IntroContext { career: false, boss_race: true, ..IntroContext::default() },
            vec![IntroStep::Tutorial, IntroStep::WarpToStart],
        ),
        (
            "a career boss race does not use the quick-race rule (1.2 rule 3)",
            intro(None, Some("IntroNisBL01"), None),
            IntroContext { boss_race: true, ..career() },
            vec![IntroStep::Tutorial, nis("IntroNisBL01", None)],
        ),
        (
            "split screen skips the NIS and its movie, and warps the player (1.1 intronis)",
            intro(Some("storyfmv_raz08"), Some("IntroNis05"), None),
            IntroContext { split_screen: true, ..career() },
            vec![IntroStep::Tutorial, IntroStep::WarpToStart],
        ),
        (
            "split screen still plays the movie of a movie-only intro (the intromovie state is separate)",
            intro(Some("Storyfmv_her136"), None, None),
            IntroContext { split_screen: true, ..career() },
            vec![IntroStep::Tutorial, IntroStep::Movie("Storyfmv_her136".to_owned()), IntroStep::WarpToStart],
        ),
        (
            "a quick-race boss race in split screen warps the player",
            intro(None, None, None),
            IntroContext { career: false, boss_race: true, split_screen: true, ..IntroContext::default() },
            vec![IntroStep::Tutorial, IntroStep::WarpToStart],
        ),
        (
            "no movie and no NIS warps the player (rule 3 with an empty NIS)",
            intro(None, None, None),
            career(),
            vec![IntroStep::Tutorial, IntroStep::WarpToStart],
        ),
        (
            "an empty NIS name counts as not set, so the movie plays alone and warps",
            intro(Some("storyfmv_raz08"), Some(""), None),
            career(),
            vec![IntroStep::Tutorial, IntroStep::Movie("storyfmv_raz08".to_owned()), IntroStep::WarpToStart],
        ),
        (
            "an intro message of -1 is not set",
            intro(None, Some("IntroNis10"), Some(-1)),
            career(),
            vec![IntroStep::Tutorial, nis("IntroNis10", None)],
        ),
        (
            "a D-Day race still shows its message and movie, and warps with no NIS",
            intro(Some("storyfmv_rac01"), None, Some(7)),
            IntroContext { dday: true, ..career() },
            vec![IntroStep::Message(7), IntroStep::Movie("storyfmv_rac01".to_owned()), IntroStep::WarpToStart],
        ),
    ];
    for (name, story, ctx, expected) in cases {
        assert_eq!(intro_steps(&story, ctx), expected, "{name}");
    }
}

#[test]
fn outro_steps_follow_the_outro_fields() {
    let cases: Vec<(&str, StoryFields, bool, Vec<OutroStep>)> = vec![
        (
            "16.2.1 has only an outro movie, storyfmv_bus12 (CSE 2.1)",
            StoryFields { outro_movie: Some("storyfmv_bus12".to_owned()), ..StoryFields::default() },
            false,
            vec![OutroStep::Movie("storyfmv_bus12".to_owned())],
        ),
        (
            "16.8.1 NIS shell plays EndingNis03 when the race has an outro marker (CSE 2.1)",
            StoryFields { outro_nis: Some("EndingNis03".to_owned()), ..StoryFields::default() },
            true,
            vec![OutroStep::Nis("EndingNis03".to_owned())],
        ),
        (
            "16.8.1 without its outro marker plays no outro NIS",
            StoryFields { outro_nis: Some("EndingNis03".to_owned()), ..StoryFields::default() },
            false,
            vec![],
        ),
        (
            "16.1.1 has no outro at all (F-12)",
            StoryFields { outro_message_id: None, ..StoryFields::default() },
            true,
            vec![],
        ),
        (
            "an outro message of -1 is not set",
            StoryFields { outro_message_id: Some(-1), ..StoryFields::default() },
            true,
            vec![],
        ),
        (
            "outro message comes before the outro movie",
            StoryFields {
                outro_message_id: Some(5),
                outro_movie: Some("storyfmv_bus12".to_owned()),
                ..StoryFields::default()
            },
            false,
            vec![OutroStep::Message(5), OutroStep::Movie("storyfmv_bus12".to_owned())],
        ),
        (
            "the full outro runs NIS, then message, then movie",
            StoryFields {
                outro_nis: Some("EndingNis03".to_owned()),
                outro_message_id: Some(5),
                outro_movie: Some("storyfmv_bus12".to_owned()),
                ..StoryFields::default()
            },
            true,
            vec![
                OutroStep::Nis("EndingNis03".to_owned()),
                OutroStep::Message(5),
                OutroStep::Movie("storyfmv_bus12".to_owned()),
            ],
        ),
    ];
    for (name, story, marker, expected) in cases {
        assert_eq!(outro_steps(&story, marker), expected, "{name}");
    }
}

#[test]
fn the_intro_nis_exits_to_prerace_or_to_the_outro_for_the_nis_shell() {
    let cases = [
        ("a normal race starts at prerace", false, AfterIntroNis::Prerace),
        ("the NIS shell 16.8.1 goes to the outro", true, AfterIntroNis::Outro),
    ];
    for (name, nis_shell, expected) in cases {
        assert_eq!(after_intro_nis(nis_shell), expected, "{name}");
    }
}

#[test]
fn the_post_race_chain_runs_only_after_a_win() {
    let cases: Vec<(&str, bool, Option<&str>, Option<&str>)> = vec![
        ("16.1.0 won starts 16.2.2 (CSE 2.3)", true, Some("16.2.2"), Some("16.2.2")),
        ("a won race with no activity ends the chain", true, None, None),
        ("an empty activity name counts as no activity", true, Some(""), None),
        ("a lost race stops the chain before its activity (F-08, F-09)", false, Some("16.2.2"), None),
        ("a lost race with no activity stops too", false, None, None),
    ];
    for (name, won, activity, expected) in cases {
        assert_eq!(chain_after(won, activity).as_deref(), expected, "{name}");
    }
}
