//! The native mod's background job on the user's own exporter output: `RIDE6_MOD_DIR` is a folder holding the mod's
//! `config.toml` (`source`, `physics`, `rider`); the job runs on its own thread like in the game and its result is promoted.
//! `RIDE6_MOD_DIR=C:/r6/e2e/mods/ride6-bikes cargo test -p ride6-bikes --test native_job -- --ignored`.

use std::path::PathBuf;
use std::time::Duration;

use ride6_bikes::job::{self, Finished, Start};

#[test]
#[ignore = "needs the user's exporter output (set RIDE6_MOD_DIR)"]
fn the_job_converts_the_bikes_and_the_rider_and_promotes_them() {
    let Some(dir) = std::env::var_os("RIDE6_MOD_DIR").map(PathBuf::from) else { return };
    let Start::Started { job, .. } = job::start(&dir, None).unwrap() else { panic!("a run should start") };
    let finished = loop {
        if let Some(done) = job.poll() {
            break done;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let Finished::Done(report) = finished else { panic!("the run failed") };
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert_eq!(report.rider_failed, None);
    assert!(job::promote(&dir).unwrap());
    assert!(dir.join("files/RIDER/attach.json").is_file());
    assert!(!report.imported.is_empty());
}
