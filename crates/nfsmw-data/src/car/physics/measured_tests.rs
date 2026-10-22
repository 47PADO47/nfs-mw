use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;

use blackbox_vehicle::performance::TABLE_SIZE;

use super::*;

/// A scratch install holding the physics files, under `<root>/install`.
fn install(name: &str) -> (PathBuf, GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-data-measured-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("install/GLOBAL")).unwrap();
    for rel in FILES {
        std::fs::write(root.join("install").join(rel), b"abc").unwrap();
    }
    let dir = GameDir::open(root.join("install")).unwrap();
    (root, dir)
}

fn performance_of(top_speed: f32) -> Performance {
    Performance { top_speed, acceleration: [0.0; TABLE_SIZE], start_grip: 1.0, end_grip: 1.0 }
}

/// A measurement that counts its calls.
fn counted(calls: &Cell<usize>, top_speed: f32) -> Option<Performance> {
    calls.set(calls.get() + 1);
    Some(performance_of(top_speed))
}

#[test]
fn a_car_is_measured_once_per_session() {
    let (root, dir) = install("once");
    let calls = Cell::new(0);
    let first = performance(&dir, "BMWM3GTR", || counted(&calls, 80.0));
    let second = performance(&dir, "bmwm3gtr", || counted(&calls, 1.0));
    assert_eq!(first, Some(performance_of(80.0)));
    assert_eq!(second, first);
    assert_eq!(calls.get(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_cars_are_measured_apart() {
    let (root, dir) = install("apart");
    let calls = Cell::new(0);
    let a = performance(&dir, "a", || counted(&calls, 10.0));
    let b = performance(&dir, "b", || counted(&calls, 20.0));
    assert_ne!(a, b);
    assert_eq!(calls.get(), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_car_with_unreadable_physics_is_remembered_too() {
    let (root, dir) = install("unreadable");
    let calls = Cell::new(0);
    let first = performance(&dir, "levin", || {
        calls.set(calls.get() + 1);
        None
    });
    let second = performance(&dir, "levin", || counted(&calls, 5.0));
    assert_eq!((first, second), (None, None));
    assert_eq!(calls.get(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_changed_physics_file_measures_the_car_again() {
    let (root, dir) = install("changed");
    let calls = Cell::new(0);
    let first = performance(&dir, "car", || counted(&calls, 30.0));
    std::fs::write(root.join("install").join(shared::GLOBALB_FILE), b"abcdef").unwrap();
    let second = performance(&dir, "car", || counted(&calls, 40.0));
    assert_eq!(first, Some(performance_of(30.0)));
    assert_eq!(second, Some(performance_of(40.0)));
    assert_eq!(calls.get(), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_set_of_a_file_state_is_shared() {
    let (root, dir) = install("shared");
    let a = memo(&dir, &FILES, || Ok(Measured::default())).unwrap();
    let b = memo(&dir, &FILES, || Ok(Measured::default())).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    std::fs::remove_dir_all(root).unwrap();
}
