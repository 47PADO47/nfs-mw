use super::*;

#[test]
fn f8_cannot_close_a_normal_garage_transaction() {
    let mut app = input_app();
    let mut capture = MouseCapture(true);
    let mut focus = UiFocus(false);
    {
        let mut lab = app.world_mut().resource_mut::<GraphicsLab>();
        lab.open = true;
        lab.sync_focus(&mut capture, &mut focus);
    }
    {
        let mut garage = app.world_mut().resource_mut::<crate::garage::Overlay>();
        garage.set_open(true, &mut capture, &mut focus);
        garage.menu = true;
    }
    // Normal garage ownership allows menu actions while capturing driving separately.
    focus.0 = false;
    app.insert_resource(capture).insert_resource(focus);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.update();
    let garage = app.world().resource::<crate::garage::Overlay>();
    assert!(garage.open && garage.menu, "an unpaid preview must retain its transaction owner");
    assert!(!app.world().resource::<GraphicsLab>().open);
    assert!(!app.world().resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::F8));
    assert!(!app.world().resource::<MouseCapture>().0);
    assert!(!app.world().resource::<UiFocus>().0);
}
