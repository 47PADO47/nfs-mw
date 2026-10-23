use super::*;
use std::cell::Cell;
use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const REL: &str = "GLOBAL/TABLE.BIN";

/// A scratch folder for one test, holding an install with `GLOBAL/TABLE.BIN` = `bytes` under `<root>/install`.
fn install(name: &str, bytes: &[u8]) -> (PathBuf, GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-data-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("install/GLOBAL")).unwrap();
    std::fs::write(root.join("install").join(REL), bytes).unwrap();
    let dir = GameDir::open(root.join("install")).unwrap();
    (root, dir)
}

/// A build that counts its calls and returns `value`.
fn counted(calls: &Cell<usize>, value: Vec<u8>) -> Result<Vec<u8>> {
    calls.set(calls.get() + 1);
    Ok(value)
}

#[test]
fn memo_reuses_the_result_while_the_file_is_unchanged() {
    let (root, dir) = install("reuse", b"abc");
    let calls = Cell::new(0);
    let first = memo(&dir, &[REL], || counted(&calls, vec![1, 2, 3])).unwrap();
    let second = memo(&dir, &[REL], || counted(&calls, vec![9])).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(calls.get(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memo_rebuilds_when_the_file_changes_size() {
    let (root, dir) = install("resize", b"abc");
    let calls = Cell::new(0);
    let first = memo(&dir, &[REL], || counted(&calls, vec![0])).unwrap();
    std::fs::write(root.join("install").join(REL), b"abcdef").unwrap();
    let second = memo(&dir, &[REL], || counted(&calls, vec![1])).unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(calls.get(), 2);
    assert_eq!(*second, vec![1]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memo_does_not_keep_a_failed_build() {
    let (root, dir) = install("failed", b"abc");
    let calls = Cell::new(0);
    let failed = memo::<Vec<u8>>(&dir, &[REL], || {
        calls.set(calls.get() + 1);
        Err(anyhow::anyhow!("broken"))
    });
    assert!(failed.is_err());
    let built = memo(&dir, &[REL], || counted(&calls, vec![9])).unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(*built, vec![9]);
    std::fs::remove_dir_all(root).unwrap();
}

/// Run `ask` on two threads that start together, and return what each got.
fn race<T: Send>(ask: impl Fn() -> T + Sync) -> [T; 2] {
    let start = Barrier::new(2);
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            start.wait();
            ask()
        });
        let second = scope.spawn(|| {
            start.wait();
            ask()
        });
        [first.join().unwrap(), second.join().unwrap()]
    })
}

#[test]
fn memo_builds_once_when_two_threads_ask_for_the_same_file() {
    let (root, dir) = install("race", b"abc");
    let calls = AtomicUsize::new(0);
    let [first, second] = race(|| {
        memo(&dir, &[REL], || {
            calls.fetch_add(1, Ordering::SeqCst);
            // Long enough for the other thread to arrive while this build runs.
            std::thread::sleep(Duration::from_millis(100));
            Ok(vec![4u8])
        })
        .unwrap()
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(Arc::ptr_eq(&first, &second));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memo_does_not_cache_a_failure_for_a_thread_that_waited_for_it() {
    let (root, dir) = install("race-failed", b"abc");
    let calls = AtomicUsize::new(0);
    // Whichever thread builds first fails; the other waits for it, then builds the entry itself.
    let results = race(|| {
        memo(&dir, &[REL], || {
            std::thread::sleep(Duration::from_millis(100));
            match calls.fetch_add(1, Ordering::SeqCst) {
                0 => Err(anyhow::anyhow!("broken")),
                _ => Ok(vec![5u8]),
            }
        })
    });
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
    assert_eq!(results.iter().flatten().count(), 1);
    // The success is stored; the failure left nothing behind.
    let later = memo(&dir, &[REL], || -> Result<Vec<u8>> { panic!("already built") }).unwrap();
    assert_eq!(*later, vec![5u8]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memo_keeps_result_types_apart() {
    let (root, dir) = install("types", b"abc");
    let number = memo(&dir, &[REL], || Ok(7u32)).unwrap();
    let text = memo(&dir, &[REL], || Ok(String::from("seven"))).unwrap();
    assert_eq!(*number, 7);
    assert_eq!(text.as_str(), "seven");
    let again = memo(&dir, &[REL], || Ok(0u32)).unwrap();
    assert!(Arc::ptr_eq(&number, &again));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_unwrapped_returns_an_uncompressed_file_as_is() {
    let (root, dir) = install("plain", b"plain bytes");
    assert_eq!(read_unwrapped(&dir, REL).unwrap(), b"plain bytes");
    let shared = shared_unwrapped(&dir, REL).unwrap();
    assert_eq!(shared.as_slice(), &b"plain bytes"[..]);
    assert!(Arc::ptr_eq(&shared, &shared_unwrapped(&dir, REL).unwrap()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_unwrapped_strips_a_raww_header() {
    // A RAWW wrapper: magic, 4 flag bytes, decompressed size, total size, then the payload.
    let mut blob = b"RAWW".to_vec();
    blob.extend_from_slice(&[1, 0x10, 0, 0]);
    blob.extend_from_slice(&3u32.to_le_bytes());
    blob.extend_from_slice(&19u32.to_le_bytes());
    blob.extend_from_slice(b"abc");
    let (root, dir) = install("raww", &blob);
    assert_eq!(read_unwrapped(&dir, REL).unwrap(), b"abc");
    std::fs::remove_dir_all(root).unwrap();
}
