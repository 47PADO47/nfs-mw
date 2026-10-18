use super::*;

#[test]
fn the_profile_row_is_the_name_with_a_chevron() {
    assert_eq!(profile_row("Rick"), "> Rick");
    assert_eq!(profile_row(""), "> ");
}

#[test]
fn accept_skips_the_animation_to_the_main_page() {
    let mut first = LoginPage::login();
    first.accept();
    assert_eq!(next_page(first), Page::Main);
    let mut second = LoginPage::login2();
    second.accept();
    assert_eq!(next_page(second), Page::Main);
}

#[test]
fn a_full_first_login_run_shows_the_second_login() {
    assert_eq!(next_page(LoginPage::login()), Page::Login2);
    assert_eq!(next_page(LoginPage::login2()), Page::Main);
}

#[test]
fn back_returns_to_the_safe_house_menu_from_either_page() {
    for mut page in [LoginPage::login(), LoginPage::login2()] {
        page.back();
        assert_eq!(next_page(page), Page::SafeHouseMenu);
    }
}
