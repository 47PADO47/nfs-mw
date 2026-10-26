//! Converts every car of the 2012 install into the folder named by `MW2012_MOD_OUT`, for the offline sound measurement
//! (`crates/nfsmw/src/audio/car/measure`). Set `MW2012_GAME_DIR` and `MW2012_MOD_OUT` and run with `--ignored`.

use std::path::Path;

use nfsmw_2012_content::config::Config;

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR) and writes the converted cars to MW2012_MOD_OUT"]
fn every_car_is_converted_into_the_measurement_folder() {
    let (Some(source), Some(out)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_MOD_OUT")) else {
        return;
    };
    let threads = nfsmw_2012_content::import::default_threads();
    let report =
        nfsmw_2012_content::import::run(&Config::default(), Path::new(&source), Path::new(&out), threads).unwrap();
    println!("imported {}, skipped {:?}", report.imported.len(), report.skipped);
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
}
