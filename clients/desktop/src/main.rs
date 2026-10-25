mod app;
mod browser;
mod events;
mod icons;
mod login;
mod session;
mod theme;
mod transfers;

const APP_TITLE: &str = "Home Space";
const APP_ID: &str = "home-space";

fn main() -> iced::Result {
    iced::application(app::App::new, app::App::update, app::App::view)
        .title(APP_TITLE)
        .subscription(app::App::subscription)
        .settings(iced::Settings {
            id: Some(APP_ID.to_owned()),
            ..iced::Settings::default()
        })
        .run()
}
