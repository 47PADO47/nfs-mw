//! The finder against font files written to a temporary folder: the bundled egui fonts stand in for the system's, so the
//! tests do not depend on the fonts of the machine they run on.

use std::path::{Path, PathBuf};

use ab_glyph::Font;

use super::{Candidate, Role, find, linux, windows};

/// A fresh, empty temporary folder for the test `name`.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-system-fonts-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary folder is writable");
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().expect("a file in a folder")).expect("the folder is writable");
    std::fs::write(path, bytes).expect("the file is writable");
}

#[test]
fn the_first_candidate_that_exists_serves_the_role() {
    let dir = scratch("first");
    write(&dir.join("second.ttf"), epaint_default_fonts::UBUNTU_LIGHT);
    write(&dir.join("third.ttf"), epaint_default_fonts::HACK_REGULAR);
    let candidates = [
        Candidate::new(dir.join("first.ttf"), 0, &[]),
        Candidate::new(dir.join("second.ttf"), 0, &[]),
        Candidate::new(dir.join("third.ttf"), 0, &[]),
    ];
    let (candidate, font) = find(&candidates).expect("the second file loads");
    assert_eq!(candidate.path, dir.join("second.ttf"));
    assert!(font.glyph_id('8').0 != 0, "the font draws digits");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_is_not_a_font_falls_through() {
    let dir = scratch("junk");
    write(&dir.join("junk.ttf"), b"not a font");
    write(&dir.join("good.ttf"), epaint_default_fonts::UBUNTU_LIGHT);
    let candidates = [Candidate::new(dir.join("junk.ttf"), 0, &[]), Candidate::new(dir.join("good.ttf"), 0, &[])];
    let (candidate, _) = find(&candidates).expect("the good file loads");
    assert_eq!(candidate.path, dir.join("good.ttf"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_candidate_leaves_the_role_to_the_fallback() {
    let dir = scratch("missing");
    let candidates = [Candidate::new(dir.join("a.ttf"), 0, &[]), Candidate::new(dir.join("b.ttc"), 1, &[])];
    assert!(find(&candidates).is_none());
    assert!(find(&[]).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn axes_a_font_lacks_do_not_stop_it_loading() {
    let dir = scratch("axes");
    write(&dir.join("static.ttf"), epaint_default_fonts::UBUNTU_LIGHT);
    let candidates = [Candidate::new(dir.join("static.ttf"), 0, windows::NUMERIC_AXES)];
    assert!(find(&candidates).is_some(), "a static font with variation axes asked of it still loads");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn windows_tries_bahnschrift_first_with_the_axes_of_each_role() {
    let fonts = Path::new(r"C:\Windows\Fonts");
    let [numeric, label, wide] = windows::candidates(fonts);
    let bahnschrift = fonts.join("bahnschrift.ttf");
    assert_eq!(numeric[0], Candidate::new(bahnschrift.clone(), 0, windows::NUMERIC_AXES));
    assert_eq!(label[0], Candidate::new(bahnschrift.clone(), 0, windows::LABEL_AXES));
    assert_eq!(wide[0], Candidate::new(bahnschrift, 0, windows::LABEL_WIDE_AXES));
    assert_eq!(numeric[1].path, fonts.join("arialnb.ttf"), "Arial Narrow is the numbers' fallback");
    assert_eq!(label[1].path, fonts.join("segoeui.ttf"), "Segoe UI is the labels' fallback");
    assert_eq!(windows::NUMERIC_AXES[0], (*b"wdth", 75.0));
    assert_eq!(windows::NUMERIC_AXES[1], (*b"wght", 600.0));
}

#[test]
fn linux_orders_the_found_files_by_preference_not_by_folder() {
    let dir = scratch("linux");
    // The listing order puts the less wanted file first.
    write(&dir.join("a/dejavu/DejaVuSans.ttf"), epaint_default_fonts::UBUNTU_LIGHT);
    write(&dir.join("z/dejavu/DejaVuSansCondensed-Bold.ttf"), epaint_default_fonts::UBUNTU_LIGHT);
    let [numeric, label, _] = linux::candidates(std::slice::from_ref(&dir));
    let paths: Vec<&Path> = numeric.iter().map(|candidate| candidate.path.as_path()).collect();
    let condensed = dir.join("z/dejavu/DejaVuSansCondensed-Bold.ttf");
    let regular = dir.join("a/dejavu/DejaVuSans.ttf");
    assert_eq!(paths, [condensed.as_path(), regular.as_path()]);
    assert_eq!(label.len(), 1, "labels do not take the condensed bold face");
    assert_eq!(find(&numeric).expect("the first file loads").0.path, condensed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_linux_search_stops_at_its_depth_and_prefers_the_first_root() {
    let dir = scratch("depth");
    write(&dir.join("system/1/2/3/4/DejaVuSans.ttf"), b"too deep");
    write(&dir.join("system/1/NotoSans-Regular.ttf"), b"system");
    write(&dir.join("user/NotoSans-Regular.ttf"), b"user");
    let roots = [dir.join("system"), dir.join("user")];
    let found = linux::search(&roots, &["DejaVuSans.ttf", "NotoSans-Regular.ttf"], linux::SEARCH_DEPTH);
    assert!(!found.contains_key("DejaVuSans.ttf"), "a folder past the search depth is not read");
    assert_eq!(found.get("NotoSans-Regular.ttf"), Some(&dir.join("system/1/NotoSans-Regular.ttf")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_roles_index_their_arrays_in_order() {
    let indices: Vec<usize> = Role::ALL.iter().map(|role| role.index()).collect();
    assert_eq!(indices, [0, 1, 2]);
}
