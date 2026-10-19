use super::*;

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn hotkeys_require_focus_and_bookmarks_share_the_capture() {
    let _lock = LOCK.lock().unwrap();
    let mut app = App::new();
    app.init_resource::<GraphicsLab>().init_resource::<ButtonInput<KeyCode>>();
    app.world_mut().spawn((Window { focused: false, ..Default::default() }, PrimaryWindow));
    app.add_systems(PreUpdate, hotkeys);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F9);
    app.update();
    assert!(!app.world().resource::<GraphicsLab>().recorder.recording());
    app.world_mut().query::<&mut Window>().single_mut(app.world_mut()).unwrap().focused = true;
    app.update();
    assert!(app.world().resource::<GraphicsLab>().recorder.recording());
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F10);
    app.update();
    assert_eq!(app.world().resource::<GraphicsLab>().recorder.bookmarks, 1);
    let capture = blackbox_trace::global().stop().unwrap();
    app.world_mut().resource_mut::<GraphicsLab>().recorder.started = None;
    assert!(capture.events() >= 3);
}

#[test]
fn export_is_valid_and_failures_are_reported() {
    let root = std::env::temp_dir().join(format!(
        "trace-test-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let recorder = blackbox_trace::Recorder::default();
    recorder.start(100);
    recorder.note("Bookmark", "test");
    let path = root.join("capture.json");
    assert_eq!(save(recorder.stop().unwrap(), path.clone()).unwrap(), path);
    assert!(std::fs::read_to_string(&path).unwrap().contains("Bookmark"));
    assert!(!path.with_extension("json.partial").exists());
    // A file cannot be used as an output directory; propagate the failure instead of claiming success.
    recorder.start(10);
    assert!(save(recorder.stop().unwrap(), path.join("invalid.json")).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn console_rejects_unbounded_or_invalid_capture_requests() {
    let mut recorder = Recorder::default();
    assert!(recorder.command(&["start".into(), "0".into()]).is_err());
    assert!(recorder.command(&["start".into(), "100000".into()]).is_err());
    assert!(recorder.command(&["mark".into()]).is_err());
}
