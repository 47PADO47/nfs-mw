use super::*;

fn temp_tree(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("game-install-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("GLOBAL")).unwrap();
    std::fs::create_dir_all(root.join("Cars/BMWM3GTR")).unwrap();
    std::fs::write(root.join("GLOBAL/GlobalB.lzc"), b"x").unwrap();
    std::fs::write(root.join("Cars/BMWM3GTR/GEOMETRY.BIN"), b"y").unwrap();
    root
}

/// A scratch directory for one test. Nothing is created inside it yet.
fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("game-install-vfs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Write `bytes` at `rel` under `dir`, creating the folders.
fn put(dir: &Path, rel: &str, bytes: &[u8]) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// An install with `GLOBAL/GlobalB.lzc` and `Cars/BMWM3GTR/GEOMETRY.BIN`, under `<root>/install`.
fn install(name: &str) -> (PathBuf, GameDir) {
    let root = scratch(name);
    put(&root.join("install"), "GLOBAL/GlobalB.lzc", b"x");
    put(&root.join("install"), "Cars/BMWM3GTR/GEOMETRY.BIN", b"y");
    let dir = GameDir::open(root.join("install")).unwrap();
    (root, dir)
}

/// Make `link` a symlink to the file `target`. Fails where the OS refuses (Windows without privilege).
fn make_file_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
}

#[test]
fn lookups_ignore_case_and_separators() {
    let root = temp_tree("case");
    let dir = GameDir::open(&root).unwrap();
    assert_eq!(dir.file_count(), 2);
    assert!(dir.exists("global/GLOBALB.LZC"));
    assert!(dir.exists("GLOBAL\\globalb.lzc"));
    assert_eq!(dir.read("cars/bmwm3gtr/geometry.bin").unwrap(), b"y");
    assert_eq!(dir.read_range("GLOBAL/GLOBALB.LZC", 0, 1).unwrap(), b"x");
    assert!(dir.read_range("GLOBAL/GLOBALB.LZC", 0, 2).is_err());
    assert!(!dir.exists("GLOBAL/missing.bin"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn files_in_lists_a_folder_without_its_subfolders() {
    let root = temp_tree("files");
    std::fs::write(root.join("Cars/Readme.txt"), b"z").unwrap();
    let dir = GameDir::open(&root).unwrap();
    assert_eq!(dir.files_in("cars"), ["Readme.txt"]);
    assert_eq!(dir.files_in("CARS/bmwm3gtr"), ["GEOMETRY.BIN"]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn subdirectories_keep_original_case() {
    let root = temp_tree("subdirs");
    let dir = GameDir::open(&root).unwrap();
    assert_eq!(dir.subdirectories("CARS"), ["BMWM3GTR"]);
    let top = dir.subdirectories("");
    assert_eq!(top, ["Cars", "GLOBAL"]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlay_replaces_an_install_file() {
    let (root, mut dir) = install("override");
    put(&root.join("mod"), "global/globalb.lzc", b"new");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert_eq!(dir.read("GLOBAL/GLOBALB.LZC").unwrap(), b"new");
    assert_eq!(dir.size("GLOBAL/GLOBALB.LZC").unwrap(), 3);
    assert_eq!(dir.provider("GLOBAL/GlobalB.lzc"), Some(Provider::Overlay("mod.a")));
    assert_eq!(dir.file_count(), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlay_adds_a_new_file_and_folder() {
    let (root, mut dir) = install("add");
    put(&root.join("mod"), "CARS/MYCAR/CAR.TXT", b"mine");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert!(dir.exists("cars/mycar/car.txt"));
    assert_eq!(dir.read("cars/mycar/car.txt").unwrap(), b"mine");
    assert_eq!(dir.provider("cars/mycar/car.txt"), Some(Provider::Overlay("mod.a")));
    assert_eq!(dir.provider("GLOBAL/GlobalB.lzc"), Some(Provider::Install));
    assert_eq!(dir.file_count(), 3);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn listings_merge_layers_and_the_top_layer_names_clashes() {
    let (root, mut dir) = install("listing");
    put(&root.join("mod"), "cars/bmwm3gtr/NEW.BIN", b"n");
    put(&root.join("mod"), "Cars/MYCAR/x.bin", b"x");
    put(&root.join("mod"), "global/globalb.lzc", b"g");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert_eq!(dir.files_in("cars/bmwm3gtr"), ["GEOMETRY.BIN", "NEW.BIN"]);
    assert_eq!(dir.files_in("GLOBAL"), ["globalb.lzc"]);
    assert_eq!(dir.subdirectories("CARS"), ["bmwm3gtr", "MYCAR"]);
    assert_eq!(dir.subdirectories(""), ["cars", "global"]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn higher_priority_wins_and_equal_priority_goes_to_the_later_add() {
    let (root, mut dir) = install("priority");
    put(&root.join("low"), "A.TXT", b"low");
    put(&root.join("tie"), "a.txt", b"tie");
    put(&root.join("high"), "A.txt", b"high");
    dir.add_overlay("low", root.join("low"), 1).unwrap();
    dir.add_overlay("high", root.join("high"), 5).unwrap();
    dir.add_overlay("tie", root.join("tie"), 1).unwrap();
    assert_eq!(dir.read("a.txt").unwrap(), b"high");
    let order: Vec<Provider<'_>> = dir.layers().iter().map(|l| l.provider).collect();
    assert_eq!(
        order,
        [Provider::Overlay("high"), Provider::Overlay("tie"), Provider::Overlay("low"), Provider::Install]
    );

    let (root2, mut without_high) = install("priority-tie");
    put(&root2.join("low"), "A.TXT", b"low");
    put(&root2.join("tie"), "a.txt", b"tie");
    without_high.add_overlay("low", root2.join("low"), 1).unwrap();
    without_high.add_overlay("tie", root2.join("tie"), 1).unwrap();
    assert_eq!(without_high.read("a.txt").unwrap(), b"tie");

    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(root2).unwrap();
}

#[test]
fn overlay_names_match_case_insensitively() {
    let (root, mut dir) = install("case-overlay");
    put(&root.join("mod"), "Global/NewFile.TXT", b"n");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert!(dir.exists("global/newfile.txt"));
    assert!(dir.exists("GLOBAL\\NEWFILE.txt"));
    assert_eq!(dir.read("./GLOBAL/newfile.TXT").unwrap(), b"n");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn paths_that_leave_their_root_are_rejected() {
    let (root, dir) = install("escape");
    put(&root, "secret.txt", b"s");
    assert!(!dir.exists("../secret.txt"));
    assert!(!dir.exists("GLOBAL/../../secret.txt"));
    assert!(!dir.exists("GLOBAL/../GLOBAL/GlobalB.lzc"));
    assert!(dir.read("..\\secret.txt").is_err());
    assert!(dir.resolve("C:\\Windows\\win.ini").is_none());
    assert!(dir.resolve("/../secret.txt").is_none());
    assert_eq!(dir.provider("../secret.txt"), None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlay_links_are_followed_only_inside_the_layer() {
    let (root, mut dir) = install("links");
    put(&root, "secret.txt", b"s");
    put(&root.join("mod"), "real.txt", b"r");
    let alias = make_file_link(&root.join("mod/real.txt"), &root.join("mod/alias.txt"));
    let escape = make_file_link(&root.join("secret.txt"), &root.join("mod/escape.txt"));
    let (Ok(()), Ok(())) = (alias, escape) else {
        eprintln!("symlinks are not available here; skipping the link checks");
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert_eq!(dir.read("alias.txt").unwrap(), b"r");
    assert!(!dir.exists("escape.txt"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlay_root_must_be_a_directory_and_ids_are_unique() {
    let (root, mut dir) = install("roots");
    put(&root, "plain.txt", b"p");
    assert!(matches!(dir.add_overlay("file", root.join("plain.txt"), 0), Err(LayerError::NotADirectory(_))));
    put(&root.join("mod"), "x.txt", b"x");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert!(matches!(dir.add_overlay("mod.a", root.join("mod"), 2), Err(LayerError::DuplicateId(_))));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn provider_reports_the_layer_that_serves_a_path() {
    let (root, mut dir) = install("provenance");
    put(&root.join("mod"), "GLOBAL/NEW.TXT", b"n");
    dir.add_overlay("mod.a", root.join("mod"), 0).unwrap();
    assert_eq!(dir.provider("cars/bmwm3gtr/geometry.bin"), Some(Provider::Install));
    assert_eq!(dir.provider("global/new.txt"), Some(Provider::Overlay("mod.a")));
    assert_eq!(dir.provider("global/missing.txt"), None);
    let layers = dir.layers();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].provider, Provider::Overlay("mod.a"));
    assert_eq!(layers[0].priority, Some(0));
    assert_eq!(layers[0].file_count, 1);
    assert_eq!(layers[1].provider, Provider::Install);
    assert_eq!(layers[1].priority, None);
    assert_eq!(layers[1].file_count, 2);
    std::fs::remove_dir_all(root).unwrap();
}
