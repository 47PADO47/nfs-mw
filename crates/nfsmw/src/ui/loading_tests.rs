//! The background load of the UI assets, without the install: the handle threads wait on, the jobs the main thread
//! polls, and the system that publishes the assets once they are there.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy_ecs::system::RunSystemOnce;
use bevy_ecs::world::World;

use super::*;

/// Polls `job` until it gives its result (a test fails instead of hanging).
fn finish<T: Default + Send + 'static>(job: &mut Job<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(value) = job.take() {
            return value;
        }
        assert!(started.elapsed() < Duration::from_secs(10), "the job never finished");
        std::thread::yield_now();
    }
}

#[test]
fn a_waiting_thread_gets_the_assets_when_the_load_is_over() {
    let load = AssetsLoad::default();
    let waiter = load.clone();
    let mut job = Job::spawn("test-waiter", move || waiter.wait().is_some());
    assert!(load.poll().is_none(), "the load is still running");
    load.finish(Some(Arc::new(UiAssets::empty())));
    assert!(finish(&mut job), "the waiting thread got the assets");
    assert!(load.poll().is_some_and(|outcome| outcome.is_some()));
}

#[test]
fn a_failed_load_wakes_the_waiting_threads_with_nothing() {
    let load = AssetsLoad::default();
    let waiter = load.clone();
    let mut job = Job::spawn("test-waiter", move || waiter.wait().is_none());
    load.finish(None);
    assert!(finish(&mut job));
}

#[test]
fn a_job_gives_its_result_once() {
    let mut job = Job::spawn("test-job", || 7_u32);
    assert_eq!(finish(&mut job), 7);
    assert_eq!(job.take(), Some(0), "after the result was taken the job only gives the default");
}

#[test]
fn a_job_that_panics_gives_the_default() {
    let mut job: Job<Option<u32>> = Job::spawn("test-panic", || panic!("the loader broke"));
    assert_eq!(finish(&mut job), None);
}

#[test]
fn the_loading_parts_keep_the_ui_busy_until_each_ends() {
    let mut loading = UiLoading::default();
    assert!(!loading.busy());
    loading.begin("a");
    loading.begin("b");
    loading.begin("a");
    loading.end("a");
    assert!(loading.busy(), "b still loads");
    loading.end("b");
    assert!(!loading.busy());
}

#[test]
fn the_shared_assets_appear_once_their_load_is_over() {
    let mut world = World::new();
    let load = AssetsLoad::default();
    world.insert_resource(load.clone());
    world.init_resource::<UiLoading>();
    world.resource_mut::<UiLoading>().begin(ASSETS);
    world.run_system_once(publish_assets).expect("the system runs");
    assert!(!world.contains_resource::<SharedAssets>(), "nothing to draw with while the load runs");
    assert!(world.resource::<UiLoading>().busy());
    load.finish(Some(Arc::new(UiAssets::empty())));
    world.run_system_once(publish_assets).expect("the system runs");
    assert!(world.contains_resource::<SharedAssets>());
    assert!(!world.resource::<UiLoading>().busy());
}

#[test]
fn a_failed_load_leaves_the_ui_off_and_not_busy() {
    let mut world = World::new();
    world.insert_resource(AssetsLoad::done(None));
    world.init_resource::<UiLoading>();
    world.resource_mut::<UiLoading>().begin(ASSETS);
    world.run_system_once(publish_assets).expect("the system runs");
    assert!(!world.contains_resource::<SharedAssets>());
    assert!(!world.resource::<UiLoading>().busy());
}
