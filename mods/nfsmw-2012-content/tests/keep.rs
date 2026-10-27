//! Converts the cars named in `KEEP_CARS` (ids, comma separated) into the mod folder `KEEP_DIR` and keeps it, with a
//! `mod.toml` the game takes, so `nfsmw view-car <ID>` can draw them. Set `MW2012_GAME_DIR` and run with `--ignored`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use nfsmw_2012_content::job::{self, Finished, Start};

const MOD_TOML: &str = "[mod]\nid = \"nfsmw-2012-content\"\nname = \"2012 cars\"\nversion = \"0.1.0\"\nauthors = [\"test\"]\nlicense = \"MIT OR Apache-2.0\"\n\n[api]\nblackbox_modding = \"0.1\"\n\n[games]\nsupported = [\"nfsmw\"]\n";

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR) and writes KEEP_DIR"]
fn convert_the_cars_and_keep_them() {
    let (Some(source), Some(keep), Ok(cars)) =
        (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("KEEP_DIR"), std::env::var("KEEP_CARS"))
    else {
        return;
    };
    let dir = PathBuf::from(keep);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = PathBuf::from(source).to_string_lossy().replace('\\', "/");
    let ids: Vec<String> = cars.split(',').map(|id| format!("'{id}'")).collect();
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = [{}]\n", ids.join(", "))).unwrap();
    std::fs::write(dir.join("mod.toml"), MOD_TOML).unwrap();
    let Start::Started { job: running, .. } = job::start(&dir, None).unwrap() else { panic!("the first start converts") };
    let started = Instant::now();
    let finished = loop {
        if let Some(finished) = running.poll() {
            break finished;
        }
        assert!(started.elapsed() < Duration::from_secs(600), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    };
    let Finished::Done(report) = finished else { panic!("the conversion failed") };
    println!("imported {:?}, skipped {:?}", report.imported.iter().map(|c| &c.id).collect::<Vec<_>>(), report.skipped);
    assert!(job::promote(&dir).unwrap());
}
