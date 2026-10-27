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
#[cfg(target_os = "macos")]
const APP_ICNS: &[u8] = include_bytes!("../packaging/home-space.icns");

fn main() -> iced::Result {
    #[cfg(target_os = "macos")]
    set_dock_icon();
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
/// the Dock icon from the `.app` bundle (see `[package.metadata.bundle]`) or
/// from [`set_dock_icon`].
fn window_icon() -> Option<iced::window::Icon> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(APP_ICON))
        .read_info()
        .ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut rgba).ok()?;
    rgba.truncate(info.buffer_size());
    iced::window::icon::from_rgba(rgba, info.width, info.height).ok()
}

/// Dock icon for the unbundled binary (`cargo run`), which macOS would
/// otherwise show as a generic "exec" icon. The icon is set once launching is
/// done, when the app gets its Dock tile. A `.app` bundle's icon wins anyway.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSApplicationDidFinishLaunchingNotification,
        NSImage,
    };
    use objc2_foundation::{NSData, NSNotification, NSNotificationCenter};

    let block = RcBlock::new(|_: NonNull<NSNotification>| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        // The Dock tile only exists once the app is a regular app; winit makes
        // it one in this same notification, possibly after this observer runs.
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        let data = NSData::with_bytes(APP_ICNS);
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            unsafe { app.setApplicationIconImage(Some(&image)) };
        }
    });
    // The observer lives for the whole process, so its token is leaked.
    std::mem::forget(unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidFinishLaunchingNotification),
            None,
            None,
            &block,
        )
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_icon_decodes() {
        assert!(super::window_icon().is_some());
    }
}
