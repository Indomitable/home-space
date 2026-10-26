mod app;
mod browser;
mod events;
mod icons;
mod locale;
mod login;
mod session;
mod theme;
mod transfers;

const APP_TITLE: &str = "Home Space";
const APP_ID: &str = "home-space";
const APP_ICON: &[u8] = include_bytes!("../packaging/home-space.png");

fn main() -> iced::Result {
    iced::application(app::App::new, app::App::update, app::App::view)
        .title(APP_TITLE)
        .subscription(app::App::subscription)
        .settings(iced::Settings {
            id: Some(APP_ID.to_owned()),
            ..iced::Settings::default()
        })
        .window(iced::window::Settings {
            icon: window_icon(),
            ..iced::window::Settings::default()
        })
        .run()
}

/// Taskbar/title-bar icon for Windows and Linux. macOS ignores it and takes
/// the Dock icon from the `.app` bundle instead (see `[package.metadata.bundle]`).
fn window_icon() -> Option<iced::window::Icon> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(APP_ICON))
        .read_info()
        .ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut rgba).ok()?;
    rgba.truncate(info.buffer_size());
    iced::window::icon::from_rgba(rgba, info.width, info.height).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_icon_decodes() {
        assert!(super::window_icon().is_some());
    }
}
