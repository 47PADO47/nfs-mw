use super::*;

fn texts(list: &[&str]) -> Vec<String> {
    list.iter().map(|text| (*text).to_owned()).collect()
}

fn pair(title: &str, artist: &str) -> (Option<String>, Option<String>) {
    (Some(title.to_owned()), Some(artist.to_owned()))
}

fn song(file: &str, secs: f32, title: Option<&str>, artist: Option<&str>) -> Song {
    Song { file: PathBuf::from(file), secs, title: title.map(str::to_owned), artist: artist.map(str::to_owned) }
}

#[test]
fn a_window_of_two_texts_is_the_title_and_the_artist() {
    let found = captions(&[texts(&["Shake The Dust", "Asherel"]), texts(&["Empathy", "Bassnectar"])]);
    assert_eq!(found, [pair("Shake The Dust", "Asherel"), pair("Empathy", "Bassnectar")]);
}

#[test]
fn a_third_text_is_the_artist_of_the_next_song_when_that_one_has_only_its_title() {
    let found = captions(&[texts(&["The City", "Madeon", "Muse"]), texts(&["Butterflies & Hurricanes"])]);
    assert_eq!(found, [pair("The City", "Madeon"), pair("Butterflies & Hurricanes", "Muse")]);
}

#[test]
fn a_lone_title_has_no_artist_when_nothing_was_carried() {
    let found = captions(&[texts(&["Four Ever Rainbow"])]);
    assert_eq!(found, [(Some("Four Ever Rainbow".to_owned()), None)]);
}

#[test]
fn a_carried_artist_is_used_once() {
    let found = captions(&[texts(&["A", "B", "C"]), texts(&["D"]), texts(&["E"])]);
    assert_eq!(found[1], pair("D", "C"));
    assert_eq!(found[2], (Some("E".to_owned()), None));
}

#[test]
fn a_song_with_no_text_has_no_caption() {
    assert_eq!(captions(&[texts(&[])]), [(None, None)]);
    assert_eq!(captions(&[texts(&["a", "b", "c", "d"])]), [(None, None)]);
}

#[test]
fn the_playlist_captions_a_song_as_artist_dash_title_and_names_its_file() {
    let songs = [
        song("D:/2012/UI/SONGS/2072097.SPS", 153.6, Some("Shake The Dust"), Some("Asherel")),
        song("D:/2012/UI/SONGS/1.SPS", 200.2, Some("Only A Title"), None),
        song("D:/2012/UI/SONGS/2.SPS", 90.0, None, None),
    ];
    let text = playlist_text(&songs);
    assert_eq!(
        text,
        "#EXTM3U\n#EXTINF:154,Asherel - Shake The Dust\nD:/2012/UI/SONGS/2072097.SPS\n\
         #EXTINF:200,Only A Title\nD:/2012/UI/SONGS/1.SPS\nD:/2012/UI/SONGS/2.SPS\n"
    );
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-soundtrack-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn without_a_source_or_when_off_the_playlist_is_removed_and_the_folder_is_still_made() {
    let dir = scratch("off");
    let playlist = dir.join(FOLDER).join(PLAYLIST);
    std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
    std::fs::write(&playlist, "#EXTM3U\n").unwrap();
    sync(&dir, None, true);
    assert!(dir.join(FOLDER).is_dir() && !playlist.exists());
    std::fs::write(&playlist, "#EXTM3U\n").unwrap();
    sync(&dir, Some(&dir), false);
    assert!(!playlist.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_install_without_the_song_table_adds_no_playlist() {
    let dir = scratch("empty-install");
    sync(&dir, Some(&dir.join("not-an-install")), true);
    assert!(!dir.join(FOLDER).join(PLAYLIST).exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_install_gives_the_titled_songs_and_their_files() {
    let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let songs = find(Path::new(&root)).expect("the songs of the install");
    assert!(songs.len() >= 40, "{} songs", songs.len());
    for song in &songs {
        assert!(song.file.is_file(), "{song:?}");
        assert!(song.secs >= MIN_SECS && song.title.is_some(), "{song:?}");
    }
    let artists = songs.iter().filter(|song| song.artist.is_some()).count();
    assert!(artists * 10 >= songs.len() * 8, "{artists} of {} songs have an artist", songs.len());
}

#[test]
fn the_soundtrack_is_on_by_default_and_the_setting_turns_it_off() {
    use crate::config::Config;
    assert!(Config::default().music);
    assert!(!Config::parse("music = false\n").unwrap().music);
}

#[test]
fn the_soundtrack_setting_does_not_change_the_conversion_fingerprint() {
    use crate::config::Config;
    let source = Path::new("definitely/not/here");
    let on = crate::stamp::of(&Config::default(), source);
    let off = crate::stamp::of(&Config { music: false, ..Config::default() }, source);
    assert_eq!(on, off);
}
