use super::*;

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_stat_bars_preserve_base_and_place_absolute_installed_marker() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    for name in ["CustomizePerformance.fng", "MyCarsManager.fng"] {
        let mut rt = crate::ui::runtime();
        let package = rt.load(catalog.find(name).unwrap().clone());
        apply_bars(&mut rt, package, Some([0.25, 0.5, 0.75]), Some([0.75, 0.5, 0.25]));
        for (index, axis) in ["TOPSPEED", "HANDLING", "ACCELERATION"].into_iter().enumerate() {
            let object = |rt: &blackbox_feng::Runtime, prefix| {
                rt.find(package, fe_hash_upper(&format!("{prefix}_{axis}"))).unwrap()
            };
            let base = object(&rt, "BASE");
            assert_eq!(rt.size(base).unwrap().x, 160.0, "{name} base must retain full authored width");
            let fill = object(&rt, "FILLBAR");
            assert_eq!(rt.size(fill).unwrap().x, [40.0, 80.0, 120.0][index]);
            assert_eq!(rt.position(fill).unwrap().x - rt.size(fill).unwrap().x / 2.0, -60.0);
            let handle = object(&rt, "HANDLE");
            assert_eq!(rt.position(handle).unwrap().x, [76.0, 36.0, -4.0][index]);
        }
        apply_bars(&mut rt, package, Some([0.1; 3]), Some([0.75, 0.5, 0.25]));
        let handle = rt.find(package, fe_hash_upper("HANDLE_TOPSPEED")).unwrap();
        assert_eq!(rt.position(handle).unwrap().x, 76.0, "preview changes must not move the installed-value marker");
    }
}
