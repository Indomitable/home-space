mod app;
mod events;
mod format;
mod icon;
mod listing;
mod pages;
mod route;
mod session;
mod upload;
mod uploads;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
